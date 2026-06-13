use super::components::{
  BelongsToTable, Customer, CustomerState, GridDirection, GridPosition, GridSize,
  NavigationComplete, RegisterState, SeatedAt, SlotPosition, SlotTarget, TableState,
};
use super::navigation_cmd::NavigateToSlot;
use super::slots::{CustomerSitSlot, ExitSlot, Occupied, QueueSlot};
use bevy::prelude::*;

/// Entering customers find nearest free CustomerSitSlot on chair bound to Empty table.
pub fn customer_find_seat(
  mut commands: Commands,
  customers: Query<(Entity, &GridPosition, &Customer), Without<SlotTarget>>,
  chair_q: Query<&BelongsToTable>,
  sit_slots: Query<(Entity, &ChildOf, &SlotPosition), (With<CustomerSitSlot>, Without<Occupied>)>,
  table_q: Query<&TableState>,
) {
  for (c_entity, c_pos, customer) in customers.iter() {
    if customer.state != CustomerState::Entering {
      continue;
    }

    let mut best: Option<Entity> = None;
    let mut best_dist = i32::MAX;

    for (slot_entity, parent, slot_pos) in sit_slots.iter() {
      let Ok(bound) = chair_q.get(parent.parent()) else {
        continue;
      };
      if table_q
        .get(bound.table)
        .map_or(true, |ts| *ts != TableState::Empty)
      {
        continue;
      }
      let dist = (c_pos.x - slot_pos.x).abs() + (c_pos.y - slot_pos.y).abs();
      if dist < best_dist {
        best_dist = dist;
        best = Some(slot_entity);
      }
    }

    let Some(slot_entity) = best else {
      continue;
    };

    commands
      .entity(c_entity)
      .insert(SlotTarget { slot: slot_entity });
    commands.entity(c_entity).queue(NavigateToSlot {
      slot: slot_entity,
      speed: 3.0,
    });
    info!(
      "Customer at ({},{}) heading to chair slot {:?}",
      c_pos.x, c_pos.y, slot_entity,
    );
  }
}

/// Customer arrived at chair slot → occupy it, place order, mark table Ordered.
pub fn customer_arrive_at_seat(
  mut commands: Commands,
  mut order_queue: ResMut<super::OrderQueue>,
  mut customers: Query<
    (Entity, &mut Customer, &SlotTarget, &NavigationComplete),
    With<NavigationComplete>,
  >,
  sit_slots: Query<&ChildOf, With<CustomerSitSlot>>,
  chair_q: Query<&BelongsToTable>,
  mut table_q: Query<&mut TableState>,
  exit_slot: Query<Entity, With<ExitSlot>>,
) {
  for (entity, mut customer, target, nav) in customers.iter_mut() {
    if customer.state != CustomerState::Entering {
      continue;
    }

    if nav.failed {
      commands
        .entity(entity)
        .remove::<(NavigationComplete, SlotTarget)>();
      customer.state = CustomerState::Leaving;
      if let Ok(exit) = exit_slot.single() {
        commands.entity(entity).queue(NavigateToSlot {
          slot: exit,
          speed: 3.0,
        });
      }
      continue;
    }

    let Ok(parent) = sit_slots.get(target.slot) else {
      continue;
    };
    let Ok(bound) = chair_q.get(parent.parent()) else {
      continue;
    };

    if let Ok(mut ts) = table_q.get_mut(bound.table) {
      *ts = TableState::Ordered;
      order_queue.pending.push_back(bound.table);
    }

    commands.entity(target.slot).insert(Occupied { by: entity });
    commands.entity(entity).insert(SeatedAt {
      sit_slot: target.slot,
    });
    commands.entity(entity).remove::<NavigationComplete>();
    customer.state = CustomerState::WaitingForFood;
    info!("Customer arrived at seat, table {:?} ordered", bound.table);
  }
}

/// WaitingForFood → start Eating when table Served. Eating timer → find queue slot.
pub fn customer_eating(
  mut commands: Commands,
  time: Res<Time>,
  mut customers: Query<
    (Entity, &GridPosition, &mut Customer, &SlotTarget),
    Without<NavigationComplete>,
  >,
  sit_slots: Query<&ChildOf, With<CustomerSitSlot>>,
  chair_q: Query<&BelongsToTable>,
  table_q: Query<&TableState>,
  reg_q: Query<(Entity, &GridPosition, &GridSize, &GridDirection), With<RegisterState>>,
  reg_queue_slots: Query<(Entity, &ChildOf, &QueueSlot, &SlotPosition)>,
  reg_occupied: Query<&Occupied>,
  exit_slot: Query<Entity, With<ExitSlot>>,
) {
  let delta = time.delta_secs();

  for (entity, c_pos, mut customer, target) in customers.iter_mut() {
    // Check if food served
    if customer.state == CustomerState::WaitingForFood {
      let Ok(parent) = sit_slots.get(target.slot) else {
        continue;
      };
      let Ok(bound) = chair_q.get(parent.parent()) else {
        continue;
      };
      if table_q
        .get(bound.table)
        .map_or(false, |ts| *ts == TableState::Served)
      {
        customer.state = CustomerState::Eating(5.0);
        info!("Food served, customer starting to eat");
      }
    }

    // Tick eating timer
    let done = match customer.state {
      CustomerState::Eating(ref mut remaining) => {
        *remaining -= delta;
        *remaining <= 0.0
      }
      _ => false,
    };

    if !done {
      continue;
    }

    // Find nearest register with a free queue slot (lowest index)
    let mut best: Option<(Entity, (i32, i32))> = None;
    let mut best_dist = i32::MAX;

    for (reg_entity, _reg_pos, _reg_size, _reg_dir) in reg_q.iter() {
      let mut lowest: Option<(Entity, usize, (i32, i32))> = None;
      for (slot_entity, parent, qslot, slot_pos) in reg_queue_slots.iter() {
        if parent.parent() != reg_entity {
          continue;
        }
        if reg_occupied.contains(slot_entity) {
          continue;
        }
        let cell = (slot_pos.x, slot_pos.y);
        if lowest.map_or(true, |(_, idx, _)| qslot.index < idx) {
          lowest = Some((slot_entity, qslot.index, cell));
        }
      }
      let Some((slot_entity, _idx, cell)) = lowest else {
        continue;
      };
      let dist = (c_pos.x - cell.0).abs() + (c_pos.y - cell.1).abs();
      if dist < best_dist {
        best_dist = dist;
        best = Some((slot_entity, cell));
      }
    }

    if let Some((slot_entity, _cell)) = best {
      commands.entity(entity).remove::<SlotTarget>();
      commands
        .entity(entity)
        .insert(SlotTarget { slot: slot_entity });
      commands.entity(entity).queue(NavigateToSlot {
        slot: slot_entity,
        speed: 3.0,
      });
      customer.state = CustomerState::WalkingToRegister;
      info!(
        "Customer finished eating, heading to queue slot {:?}",
        slot_entity
      );
    } else {
      // No free queue slot → go to exit
      commands.entity(entity).remove::<SlotTarget>();
      if let Ok(exit) = exit_slot.single() {
        commands.entity(entity).queue(NavigateToSlot {
          slot: exit,
          speed: 3.0,
        });
      }
      customer.state = CustomerState::Leaving;
      info!("Customer finished eating, leaving (no queue slot available)");
    }
  }
}

/// Customer arrived at queue cell → occupy slot, set WaitingForPayment.
pub fn customer_arrive_at_queue(
  mut commands: Commands,
  mut customers: Query<
    (Entity, &mut Customer, &SlotTarget, &NavigationComplete),
    With<NavigationComplete>,
  >,
  queue_slots: Query<&ChildOf, With<QueueSlot>>,
  exit_slot: Query<Entity, With<ExitSlot>>,
) {
  for (entity, mut customer, target, nav) in customers.iter_mut() {
    if customer.state != CustomerState::WalkingToRegister {
      continue;
    }

    if nav.failed {
      commands
        .entity(entity)
        .remove::<(NavigationComplete, SlotTarget)>();
      customer.state = CustomerState::Leaving;
      if let Ok(exit) = exit_slot.single() {
        commands.entity(entity).queue(NavigateToSlot {
          slot: exit,
          speed: 3.0,
        });
      }
      continue;
    }

    let Ok(_parent) = queue_slots.get(target.slot) else {
      continue;
    };

    commands.entity(target.slot).insert(Occupied { by: entity });
    commands.entity(entity).remove::<NavigationComplete>();
    customer.state = CustomerState::WaitingForPayment;
    info!("Customer arrived at queue slot, waiting for payment");
  }
}

/// Customer at exit → mark table Dirty, release sit slot, despawn.
pub fn customer_exit(
  mut commands: Commands,
  mut grid: ResMut<super::world::GridLayers>,
  customers: Query<
    (Entity, &Customer, &SeatedAt, &NavigationComplete),
    (With<NavigationComplete>, With<SeatedAt>),
  >,
  sit_slots: Query<&ChildOf, With<CustomerSitSlot>>,
  chair_q: Query<&BelongsToTable>,
  mut table_q: Query<&mut TableState>,
) {
  for (entity, customer, seated_at, nav) in customers.iter() {
    if customer.state != CustomerState::Leaving {
      continue;
    }

    if nav.failed {
      // Can't reach exit — despawn anyway, but still clean up the sit slot
      commands.entity(seated_at.sit_slot).remove::<Occupied>();
      grid.release_all(entity);
      commands.entity(entity).despawn();
      continue;
    }

    // Find table via SeatedAt → sit slot → chair → BelongsToTable
    let Ok(parent) = sit_slots.get(seated_at.sit_slot) else {
      // Sit slot invalid — still despawn
      grid.release_all(entity);
      commands.entity(entity).despawn();
      continue;
    };
    let Ok(bound) = chair_q.get(parent.parent()) else {
      // Chair invalid — still despawn
      grid.release_all(entity);
      commands.entity(entity).despawn();
      continue;
    };

    if let Ok(mut ts) = table_q.get_mut(bound.table) {
      *ts = TableState::Dirty;
    }

    commands.entity(seated_at.sit_slot).remove::<Occupied>();
    grid.release_all(entity);
    commands.entity(entity).despawn();
    info!(
      "Customer reached exit, table {:?} dirty, despawning",
      bound.table
    );
  }
}

/// Dirty tables → Empty.
pub fn cleanup_tables(mut table_q: Query<&mut TableState>) {
  for mut ts in table_q.iter_mut() {
    if *ts == TableState::Dirty {
      *ts = TableState::Empty;
    }
  }
}
