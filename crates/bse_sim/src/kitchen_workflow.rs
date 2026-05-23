use bevy::prelude::*;
use crate::components::{
  GridPosition, MovementProgress, NavigationComplete, Staff, StaffState, StaffTarget,
  StoveState, TableState,
};
use crate::navigation_cmd::NavigateTo;

/// System 1: Idle staff pick up pending orders and head to a stove.
pub fn staff_pickup_system(
  mut commands: Commands,
  mut order_queue: ResMut<crate::OrderQueue>,
  mut staff_q: Query<(Entity, &GridPosition, &mut Staff), Without<StaffTarget>>,
  stove_q: Query<&GridPosition, (With<StoveState>, Without<NavigationComplete>)>,
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

    // Find nearest idle stove
    let stove_target = stove_q
      .iter()
      .min_by_key(|stove_pos| {
        (staff_pos.x - stove_pos.x).abs() + (staff_pos.z - stove_pos.z).abs()
      })
      .map(|pos| (pos.x, pos.z));

    let Some(target) = stove_target else {
      // No stove -- put the order back
      order_queue.pending.push_front(table_entity);
      continue;
    };

    commands
      .entity(staff_entity)
      .insert(StaffTarget { target_table: table_entity });
    commands.entity(staff_entity).queue(NavigateTo {
      target,
      speed: 3.0,
    });
    staff.state = StaffState::WalkingToKitchen;

    info!(
      "Staff at ({},{}) picking up order for table {:?}, heading to stove at ({},{})",
      staff_pos.x, staff_pos.z, table_entity, target.0, target.1
    );
  }
}

/// System 2: Staff who arrived at stove start cooking. Timer is ticked each frame.
pub fn staff_cooking_system(
  mut commands: Commands,
  time: Res<Time>,
  mut staff_q: Query<
    (Entity, &GridPosition, &mut Staff, &StaffTarget, Option<&NavigationComplete>),
    Without<MovementProgress>,
  >,
  mut stove_q: Query<(&GridPosition, &mut StoveState)>,
  table_q: Query<&GridPosition, With<TableState>>,
) {
  let delta = time.delta_secs();
  let mut to_deliver: Vec<(Entity, Entity)> = Vec::new(); // (staff_entity, target_table)

  for (staff_entity, staff_pos, mut staff, task, nav_complete) in staff_q.iter_mut() {
    // Arrival at stove: WalkingToKitchen staff just finished path to stove
    if staff.state == StaffState::WalkingToKitchen {
      if nav_complete.is_some() {
        // Start cooking
        staff.state = StaffState::Cooking(3.0);
        commands.entity(staff_entity).remove::<NavigationComplete>();
        for (_stove_pos, mut stove) in stove_q.iter_mut() {
          if _stove_pos.x == staff_pos.x && _stove_pos.z == staff_pos.z {
            *stove = StoveState::Cooking(3.0);
            break;
          }
        }
        info!(
          "Staff started cooking at ({},{})",
          staff_pos.x, staff_pos.z
        );
      }
      continue;
    }

    // Cooking timer tick
    if let StaffState::Cooking(ref mut remaining) = staff.state {
      *remaining -= delta;
      if *remaining <= 0.0 {
        staff.state = StaffState::Delivering;
        to_deliver.push((staff_entity, task.target_table));
        info!(
          "Staff finished cooking at ({},{})",
          staff_pos.x, staff_pos.z
        );
      }
    }
  }

  // Tick stove timers too
  for (_stove_pos, mut stove) in stove_q.iter_mut() {
    if let StoveState::Cooking(ref mut remaining) = *stove {
      *remaining -= delta;
      if *remaining <= 0.0 {
        *stove = StoveState::Idle;
      }
    }
  }

  // Navigate delivering staff to their target table
  for (staff_entity, target_table) in to_deliver {
    if let Ok(pos) = table_q.get(target_table) {
      commands.entity(staff_entity).queue(NavigateTo {
        target: (pos.x, pos.z),
        speed: 3.0,
      });
    }
  }
}

/// System 3: Staff arrived at table via NavigationComplete, serve food, return to Idle.
pub fn staff_deliver_system(
  mut commands: Commands,
  mut staff_q: Query<
    (Entity, &GridPosition, &mut Staff, &StaffTarget),
    With<NavigationComplete>,
  >,
  mut table_q: Query<(Entity, &mut TableState)>,
) {
  let mut to_serve: Vec<(Entity, Entity)> = Vec::new(); // (staff_entity, table_entity)

  for (staff_entity, _staff_pos, mut staff, task) in staff_q.iter_mut() {
    if staff.state != StaffState::Delivering {
      continue;
    }

    to_serve.push((staff_entity, task.target_table));
    commands.entity(staff_entity).remove::<StaffTarget>();
    commands.entity(staff_entity).remove::<NavigationComplete>();
    staff.state = StaffState::Idle;
    info!(
      "Staff delivered food to table {:?}, returning to Idle",
      task.target_table
    );
  }

  // Update tables (separate loop avoids borrow conflicts)
  for (_, table_entity) in &to_serve {
    if let Ok((_, mut state)) = table_q.get_mut(*table_entity) {
      *state = TableState::Served;
    }
  }
}
