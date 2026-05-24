use std::collections::HashSet;

use bevy::prelude::*;

use crate::components::{
  ApplianceGeometry, Customer, CustomerState, GridPosition, MovementProgress, NavigationComplete,
  RegisterState, SlotTarget, Staff, StaffState, StaffTarget, StoveState, TableState, EXIT_POSITION,
};
use crate::navigation_cmd::NavigateTo;
use crate::slots::{
  staff_checkout_cell, staff_cook_cell, staff_deliver_cell, Occupied, QueueSlot, StaffCheckoutSlot,
  StaffCookSlot, StaffDeliverSlot,
};
use crate::world::GridLayers;

/// System 1: Idle staff without SlotTarget pick up pending order from OrderQueue,
/// find the nearest stove with a free StaffCookSlot, and navigate to the cook cell.
pub fn staff_pickup(
  mut commands: Commands,
  mut order_queue: ResMut<crate::OrderQueue>,
  mut staff_q: Query<
    (Entity, &GridPosition, &mut Staff),
    (Without<SlotTarget>, Without<NavigationComplete>),
  >,
  cook_slots: Query<(Entity, &ChildOf), (With<StaffCookSlot>, Without<Occupied>)>,
  stove_q: Query<(&GridPosition, &ApplianceGeometry), With<StoveState>>,
) {
  if order_queue.pending.is_empty() {
    return;
  }

  for (staff_entity, staff_pos, mut staff) in staff_q.iter_mut() {
    if staff.state != StaffState::Idle {
      continue;
    }
    let Some(table_entity) = order_queue.pending.pop_front() else {
      break;
    };

    // Find nearest stove with a free cook slot
    let mut best: Option<(Entity, (i32, i32))> = None;
    let mut best_dist = i32::MAX;

    for (slot_entity, parent) in cook_slots.iter() {
      let stove_entity = parent.parent();
      let Ok((stove_pos, geo)) = stove_q.get(stove_entity) else {
        continue;
      };
      let cell = staff_cook_cell((stove_pos.x, stove_pos.z), geo);
      let dist = (staff_pos.x - cell.0).abs() + (staff_pos.z - cell.1).abs();
      if dist < best_dist {
        best_dist = dist;
        best = Some((slot_entity, cell));
      }
    }

    let Some((slot_entity, cell)) = best else {
      // No free stove — push table back to front and skip
      order_queue.pending.push_front(table_entity);
      continue;
    };

    commands.entity(staff_entity).insert((
      SlotTarget { slot: slot_entity },
      StaffTarget {
        target_table: table_entity,
      },
    ));
    commands.entity(staff_entity).queue(NavigateTo {
      target: cell,
      speed: 3.0,
    });
    staff.state = StaffState::WalkingToKitchen;

    info!(
      "Staff at ({},{}) heading to stove cook cell ({},{}) for table {:?}",
      staff_pos.x, staff_pos.z, cell.0, cell.1, table_entity
    );
  }
}

/// System 2: Staff arrived at stove cook slot — occupy slot, start cooking on
/// both the staff and the stove entity.
pub fn staff_arrive_at_stove(
  mut commands: Commands,
  mut staff_q: Query<
    (Entity, &mut Staff, &SlotTarget),
    (With<NavigationComplete>, Without<MovementProgress>),
  >,
  cook_slots: Query<&ChildOf, With<StaffCookSlot>>,
  mut stove_q: Query<&mut StoveState>,
) {
  for (staff_entity, mut staff, target) in staff_q.iter_mut() {
    if staff.state != StaffState::WalkingToKitchen {
      continue;
    }

    let Ok(parent) = cook_slots.get(target.slot) else {
      continue;
    };
    let stove_entity = parent.parent();

    commands
      .entity(target.slot)
      .insert(Occupied { by: staff_entity });
    commands.entity(staff_entity).remove::<NavigationComplete>();
    staff.state = StaffState::Cooking(3.0);

    if let Ok(mut stove) = stove_q.get_mut(stove_entity) {
      *stove = StoveState::Cooking(3.0);
    }

    info!(
      "Staff {:?} arrived at stove {:?}, cooking started",
      staff_entity, stove_entity
    );
  }
}

/// System 3: Tick cooking timers on staff and stoves.  When a staff timer
/// expires, find a free StaffDeliverSlot on the target table and navigate there.
pub fn staff_cooking(
  mut commands: Commands,
  time: Res<Time>,
  mut staff_q: Query<(Entity, &mut Staff, &StaffTarget, &SlotTarget), Without<NavigationComplete>>,
  mut stove_q: Query<&mut StoveState>,
  deliver_slots: Query<(Entity, &ChildOf, &StaffDeliverSlot), Without<Occupied>>,
  table_q: Query<&GridPosition>,
) {
  let delta = time.delta_secs();

  // Tick staff cooking timers and transition to delivering when done
  for (staff_entity, mut staff, task, cook_target) in staff_q.iter_mut() {
    if let StaffState::Cooking(ref mut remaining) = staff.state {
      *remaining -= delta;
      if *remaining > 0.0 {
        continue;
      }

      // Find a free deliver slot on the target table
      let mut best: Option<(Entity, (i32, i32))> = None;
      for (slot_entity, parent, dslot) in deliver_slots.iter() {
        if parent.parent() != task.target_table {
          continue;
        }
        let Ok(table_pos) = table_q.get(task.target_table) else {
          continue;
        };
        let cell = staff_deliver_cell((table_pos.x, table_pos.z), dslot.side);
        best = Some((slot_entity, cell));
        break;
      }

      let Some((slot_entity, cell)) = best else {
        continue; // no free deliver slot yet, retry next frame
      };

      // Free the old cook slot
      commands.entity(cook_target.slot).remove::<Occupied>();
      // Swap to deliver slot
      commands.entity(staff_entity).remove::<SlotTarget>();
      commands
        .entity(staff_entity)
        .insert(SlotTarget { slot: slot_entity });
      commands.entity(staff_entity).queue(NavigateTo {
        target: cell,
        speed: 3.0,
      });
      staff.state = StaffState::Delivering;

      info!(
        "Staff finished cooking, heading to deliver at ({},{})",
        cell.0, cell.1
      );
    }
  }

  // Tick all stove Cooking timers
  for mut stove in stove_q.iter_mut() {
    if let StoveState::Cooking(ref mut remaining) = *stove {
      *remaining -= delta;
      if *remaining <= 0.0 {
        *stove = StoveState::Idle;
      }
    }
  }
}

/// System 4: Staff arrived at the deliver slot — free the slot, set table to
/// Served, release the grid cell, and return the staff to Idle.
pub fn staff_deliver(
  mut commands: Commands,
  mut grid: ResMut<GridLayers>,
  mut staff_q: Query<
    (Entity, &GridPosition, &mut Staff, &StaffTarget, &SlotTarget),
    (With<NavigationComplete>, Without<MovementProgress>),
  >,
  mut table_q: Query<&mut TableState>,
) {
  for (staff_entity, staff_pos, mut staff, task, target) in staff_q.iter_mut() {
    if staff.state != StaffState::Delivering {
      continue;
    }

    commands.entity(target.slot).remove::<Occupied>();
    commands
      .entity(staff_entity)
      .remove::<(SlotTarget, StaffTarget, NavigationComplete)>();

    grid.release_cell(staff_pos.x, staff_pos.z, staff_entity);

    if let Ok(mut ts) = table_q.get_mut(task.target_table) {
      *ts = TableState::Served;
    }

    staff.state = StaffState::Idle;

    info!(
      "Staff delivered food to table {:?}, returning to Idle",
      task.target_table
    );
  }
}

/// System 5: Idle staff without SlotTarget look for a register that has both
/// an occupied QueueSlot (customers waiting) and a free StaffCheckoutSlot,
/// then navigate to the checkout cell.
pub fn staff_checkout_start(
  mut commands: Commands,
  mut staff_q: Query<
    (Entity, &GridPosition, &mut Staff),
    (Without<SlotTarget>, Without<NavigationComplete>),
  >,
  checkout_slots: Query<(Entity, &ChildOf), (With<StaffCheckoutSlot>, Without<Occupied>)>,
  occupied_queue: Query<&ChildOf, (With<QueueSlot>, With<Occupied>)>,
  reg_q: Query<(&GridPosition, &ApplianceGeometry), With<RegisterState>>,
) {
  // Precompute: registers that have at least one occupied QueueSlot
  let mut busy_regs: HashSet<Entity> = HashSet::new();
  for parent in occupied_queue.iter() {
    busy_regs.insert(parent.parent());
  }

  if busy_regs.is_empty() {
    return;
  }

  for (staff_entity, staff_pos, mut staff) in staff_q.iter_mut() {
    if staff.state != StaffState::Idle {
      continue;
    }

    // Find nearest register with a free checkout slot and busy queue
    let mut best: Option<(Entity, (i32, i32))> = None;
    let mut best_dist = i32::MAX;

    for (slot_entity, parent) in checkout_slots.iter() {
      let reg_entity = parent.parent();
      if !busy_regs.contains(&reg_entity) {
        continue;
      }
      let Ok((reg_pos, reg_geo)) = reg_q.get(reg_entity) else {
        continue;
      };
      let cell = staff_checkout_cell((reg_pos.x, reg_pos.z), reg_geo);
      let dist = (staff_pos.x - cell.0).abs() + (staff_pos.z - cell.1).abs();
      if dist < best_dist {
        best_dist = dist;
        best = Some((slot_entity, cell));
      }
    }

    let Some((slot_entity, cell)) = best else {
      continue;
    };

    commands
      .entity(staff_entity)
      .insert(SlotTarget { slot: slot_entity });
    commands.entity(staff_entity).queue(NavigateTo {
      target: cell,
      speed: 3.0,
    });
    staff.state = StaffState::WalkingToRegister;

    info!("Staff heading to checkout at ({},{})", cell.0, cell.1);
  }
}

/// System 6: Staff arrived at checkout slot — occupy the slot and start the
/// checkout timer.
pub fn staff_arrive_at_checkout(
  mut commands: Commands,
  mut staff_q: Query<
    (Entity, &mut Staff, &SlotTarget),
    (With<NavigationComplete>, Without<MovementProgress>),
  >,
  checkout_query: Query<(), With<StaffCheckoutSlot>>,
) {
  for (staff_entity, mut staff, target) in staff_q.iter_mut() {
    if staff.state != StaffState::WalkingToRegister {
      continue;
    }

    // Sanity-check: the slot is a checkout slot
    if checkout_query.get(target.slot).is_err() {
      continue;
    }

    commands
      .entity(target.slot)
      .insert(Occupied { by: staff_entity });
    commands.entity(staff_entity).remove::<NavigationComplete>();
    staff.state = StaffState::CheckingOut(2.0);

    info!(
      "Staff {:?} arrived at checkout, starting timer",
      staff_entity
    );
  }
}

/// System 7: Tick the CheckingOut timer.  When it expires, serve the front-most
/// occupied QueueSlot on the same register, send that customer to the exit, and
/// return the staff to Idle.
pub fn staff_checkout_tick(
  mut commands: Commands,
  time: Res<Time>,
  mut staff_q: Query<(Entity, &mut Staff, &SlotTarget), Without<NavigationComplete>>,
  checkout_slots: Query<&ChildOf, With<StaffCheckoutSlot>>,
  queue_slots: Query<(Entity, &ChildOf, &QueueSlot)>,
  occupied_q: Query<&Occupied>,
  mut customer_q: Query<&mut Customer>,
) {
  let delta = time.delta_secs();

  for (staff_entity, mut staff, target) in staff_q.iter_mut() {
    if let StaffState::CheckingOut(ref mut remaining) = staff.state {
      *remaining -= delta;
      if *remaining > 0.0 {
        continue;
      }

      // Get the register entity from the checkout slot's parent
      let Ok(parent) = checkout_slots.get(target.slot) else {
        continue;
      };
      let reg_entity = parent.parent();

      // Find the front-most (lowest index) occupied QueueSlot on this register
      let mut front: Option<(Entity, Entity)> = None; // (slot, customer)
      let mut lowest_idx = usize::MAX;

      for (slot_entity, slot_parent, qslot) in queue_slots.iter() {
        if slot_parent.parent() != reg_entity {
          continue;
        }
        let Ok(occupied) = occupied_q.get(slot_entity) else {
          continue;
        };
        if qslot.index < lowest_idx {
          lowest_idx = qslot.index;
          front = Some((slot_entity, occupied.by));
        }
      }

      // Set the front customer to Leaving and send to exit
      if let Some((slot_entity, customer_entity)) = front {
        if let Ok(mut customer) = customer_q.get_mut(customer_entity) {
          customer.state = CustomerState::Leaving;
        }
        commands.entity(customer_entity).remove::<SlotTarget>();
        commands.entity(slot_entity).remove::<Occupied>();
        commands.entity(customer_entity).queue(NavigateTo {
          target: EXIT_POSITION,
          speed: 3.0,
        });
        info!(
          "Customer {:?} checked out, heading to exit",
          customer_entity
        );
      }

      // Clean up staff checkout state
      commands.entity(target.slot).remove::<Occupied>();
      commands.entity(staff_entity).remove::<SlotTarget>();
      staff.state = StaffState::Idle;

      info!("Staff {:?} finished checkout, now Idle", staff_entity);
    }
  }
}
