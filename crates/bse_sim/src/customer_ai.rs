use crate::components::{
  BelongsToTable, Customer, CustomerState, GridPosition, NavigationComplete, RegisterState,
  SlotTarget, TableState, EXIT_POSITION,
};
use crate::navigation_cmd::NavigateTo;
use crate::slots::{customer_sit_cell, queue_cell, CustomerSitSlot, Occupied, QueueSlot};
use bevy::prelude::*;

/// Entering customers find nearest free CustomerSitSlot on chair bound to Empty table.
pub fn customer_find_seat(
  mut commands: Commands,
  customers: Query<(Entity, &GridPosition, &Customer), Without<SlotTarget>>,
  chair_q: Query<(Entity, &GridPosition, &BelongsToTable)>,
  sit_slots: Query<(Entity, &ChildOf), (With<CustomerSitSlot>, Without<Occupied>)>,
  table_q: Query<&TableState>,
) {
  for (c_entity, c_pos, customer) in customers.iter() {
    if customer.state != CustomerState::Entering {
      continue;
    }

    let mut best: Option<(Entity, (i32, i32))> = None;
    let mut best_dist = i32::MAX;

    for (slot_entity, parent) in sit_slots.iter() {
      let Ok((_, chair_pos, bound)) = chair_q.get(parent.parent()) else {
        continue;
      };
      if table_q
        .get(bound.table)
        .map_or(true, |ts| *ts != TableState::Empty)
      {
        continue;
      }
      let dist = (c_pos.x - chair_pos.x).abs() + (c_pos.z - chair_pos.z).abs();
      if dist < best_dist {
        best_dist = dist;
        best = Some((slot_entity, (chair_pos.x, chair_pos.z)));
      }
    }

    let Some((slot_entity, chair_cell)) = best else {
      continue;
    };

    commands
      .entity(c_entity)
      .insert(SlotTarget { slot: slot_entity });
    let target = customer_sit_cell(chair_cell);
    commands
      .entity(c_entity)
      .queue(NavigateTo { target, speed: 3.0 });
    info!(
      "Customer at ({},{}) heading to chair seat ({},{})",
      c_pos.x, c_pos.z, target.0, target.1,
    );
  }
}

/// Customer arrived at chair slot → occupy it, place order, mark table Ordered.
pub fn customer_arrive_at_seat(
  mut commands: Commands,
  mut order_queue: ResMut<crate::OrderQueue>,
  mut customers: Query<(Entity, &mut Customer, &SlotTarget), With<NavigationComplete>>,
  sit_slots: Query<&ChildOf, With<CustomerSitSlot>>,
  chair_q: Query<&BelongsToTable>,
  mut table_q: Query<&mut TableState>,
) {
  for (entity, mut customer, target) in customers.iter_mut() {
    if customer.state != CustomerState::Entering {
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
  reg_q: Query<(Entity, &GridPosition, &crate::components::ApplianceGeometry), With<RegisterState>>,
  reg_queue_slots: Query<(Entity, &ChildOf, &QueueSlot)>,
  reg_occupied: Query<&Occupied>,
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

    for (reg_entity, reg_pos, reg_geo) in reg_q.iter() {
      let mut lowest: Option<(Entity, usize, (i32, i32))> = None;
      for (slot_entity, parent, qslot) in reg_queue_slots.iter() {
        if parent.parent() != reg_entity {
          continue;
        }
        if reg_occupied.contains(slot_entity) {
          continue;
        }
        let cell = queue_cell((reg_pos.x, reg_pos.z), reg_geo, qslot.index);
        if lowest.map_or(true, |(_, idx, _)| qslot.index < idx) {
          lowest = Some((slot_entity, qslot.index, cell));
        }
      }
      let Some((slot_entity, _idx, cell)) = lowest else {
        continue;
      };
      let dist = (c_pos.x - cell.0).abs() + (c_pos.z - cell.1).abs();
      if dist < best_dist {
        best_dist = dist;
        best = Some((slot_entity, cell));
      }
    }

    if let Some((slot_entity, cell)) = best {
      commands.entity(entity).remove::<SlotTarget>();
      commands
        .entity(entity)
        .insert(SlotTarget { slot: slot_entity });
      commands.entity(entity).queue(NavigateTo {
        target: cell,
        speed: 3.0,
      });
      customer.state = CustomerState::WalkingToRegister;
      info!(
        "Customer finished eating, heading to queue at ({},{})",
        cell.0, cell.1
      );
    } else {
      // No free queue slot → go to exit
      commands.entity(entity).remove::<SlotTarget>();
      commands.entity(entity).queue(NavigateTo {
        target: EXIT_POSITION,
        speed: 3.0,
      });
      customer.state = CustomerState::Leaving;
      info!("Customer finished eating, leaving (no queue slot available)");
    }
  }
}

/// Customer arrived at queue cell → occupy slot, set WaitingForPayment.
pub fn customer_arrive_at_queue(
  mut commands: Commands,
  mut customers: Query<(Entity, &mut Customer, &SlotTarget), With<NavigationComplete>>,
  queue_slots: Query<&ChildOf, With<QueueSlot>>,
) {
  for (entity, mut customer, target) in customers.iter_mut() {
    if customer.state != CustomerState::WalkingToRegister {
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

/// Customer at exit → mark table Dirty, release slot, despawn.
pub fn customer_exit(
  mut commands: Commands,
  mut grid: ResMut<crate::world::GridLayers>,
  customers: Query<(Entity, &Customer, &SlotTarget), With<NavigationComplete>>,
  sit_slots: Query<&ChildOf, With<CustomerSitSlot>>,
  chair_q: Query<&BelongsToTable>,
  mut table_q: Query<&mut TableState>,
) {
  for (entity, customer, target) in customers.iter() {
    if customer.state != CustomerState::Leaving {
      continue;
    }

    let Ok(parent) = sit_slots.get(target.slot) else {
      continue;
    };
    let Ok(bound) = chair_q.get(parent.parent()) else {
      continue;
    };

    if let Ok(mut ts) = table_q.get_mut(bound.table) {
      *ts = TableState::Dirty;
    }

    commands.entity(target.slot).remove::<Occupied>();
    grid.release_all(entity);
    commands.entity(entity).remove::<NavigationComplete>();
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
