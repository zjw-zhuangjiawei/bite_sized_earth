use crate::components::{
  ApplianceGeometry, GridFootprint, GridPosition, MovementProgress, NavigationComplete, Staff,
  StaffState, StaffTarget, StaffZone, StoveState, TableState,
};
use crate::navigation_cmd::NavigateTo;
use crate::world::GridLayers;
use bevy::prelude::*;

/// System 1: Idle staff pick up pending orders and head to a stove.
pub fn staff_pickup_system(
  mut commands: Commands,
  mut order_queue: ResMut<crate::OrderQueue>,
  mut staff_q: Query<(Entity, &GridPosition, &mut Staff), Without<StaffTarget>>,
  stove_q: Query<(&GridPosition, &StaffZone), With<StoveState>>,
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

    // Find nearest stove with at least one available interaction cell
    let best = stove_q
      .iter()
      .filter(|(_, zone)| !zone.cells.is_empty())
      .min_by_key(|(stove_pos, _)| {
        (staff_pos.x - stove_pos.x).abs() + (staff_pos.z - stove_pos.z).abs()
      });

    let Some((_stove_pos, zone)) = best else {
      order_queue.pending.push_front(table_entity);
      continue;
    };

    // Pick the closest interaction cell
    let target = zone
      .cells
      .iter()
      .min_by_key(|&&(x, z)| (staff_pos.x - x).abs() + (staff_pos.z - z).abs())
      .copied();

    let Some(target) = target else {
      order_queue.pending.push_front(table_entity);
      continue;
    };

    commands.entity(staff_entity).insert(StaffTarget {
      target_table: table_entity,
    });
    commands
      .entity(staff_entity)
      .queue(NavigateTo { target, speed: 3.0 });
    staff.state = StaffState::WalkingToKitchen;

    info!(
      "Staff at ({},{}) heading to stove interaction cell ({},{}) for table {:?}",
      staff_pos.x, staff_pos.z, target.0, target.1, table_entity
    );
  }
}

/// System 2: Staff who arrived at stove start cooking. Timer tick each frame.
pub fn staff_cooking_system(
  mut commands: Commands,
  time: Res<Time>,
  mut staff_q: Query<
    (
      Entity,
      &GridPosition,
      &mut Staff,
      &StaffTarget,
      Option<&NavigationComplete>,
    ),
    Without<MovementProgress>,
  >,
  mut stove_q: Query<(&GridPosition, &mut StoveState)>,
  table_q: Query<(&GridPosition, &StaffZone), With<TableState>>,
) {
  let delta = time.delta_secs();
  let mut to_deliver: Vec<(Entity, Entity, (i32, i32))> = Vec::new();

  for (staff_entity, staff_pos, mut staff, task, nav_complete) in staff_q.iter_mut() {
    if staff.state == StaffState::WalkingToKitchen {
      if nav_complete.is_some() {
        staff.state = StaffState::Cooking(3.0);
        commands.entity(staff_entity).remove::<NavigationComplete>();
        for (_stove_pos, mut stove) in stove_q.iter_mut() {
          if _stove_pos.x == staff_pos.x && _stove_pos.z == staff_pos.z {
            *stove = StoveState::Cooking(3.0);
            break;
          }
        }
        info!("Staff started cooking at ({},{})", staff_pos.x, staff_pos.z);
      }
      continue;
    }

    if let StaffState::Cooking(ref mut remaining) = staff.state {
      *remaining -= delta;
      if *remaining <= 0.0 {
        staff.state = StaffState::Delivering;
        // Find target table and pick an interaction cell
        if let Ok((_table_pos, zone)) = table_q.get(task.target_table) {
          let target_cell = zone
            .cells
            .iter()
            .min_by_key(|&&(x, z)| (staff_pos.x - x).abs() + (staff_pos.z - z).abs())
            .copied()
            .unwrap_or((_table_pos.x, _table_pos.z)); // fallback: table anchor
          to_deliver.push((staff_entity, task.target_table, target_cell));
        }
        info!(
          "Staff finished cooking at ({},{})",
          staff_pos.x, staff_pos.z
        );
      }
    }
  }

  // Tick stove timers
  for (_stove_pos, mut stove) in stove_q.iter_mut() {
    if let StoveState::Cooking(ref mut remaining) = *stove {
      *remaining -= delta;
      if *remaining <= 0.0 {
        *stove = StoveState::Idle;
      }
    }
  }

  for (staff_entity, _target_table, target_cell) in to_deliver {
    commands.entity(staff_entity).queue(NavigateTo {
      target: target_cell,
      speed: 3.0,
    });
  }
}

/// System 3: Staff arrived at table interaction cell, serve food, return to Idle.
pub fn staff_deliver_system(
  mut commands: Commands,
  mut grid: ResMut<GridLayers>,
  mut staff_q: Query<(Entity, &GridPosition, &mut Staff, &StaffTarget), With<NavigationComplete>>,
  mut table_q: Query<(Entity, &mut TableState)>,
) {
  let mut to_serve: Vec<(Entity, Entity)> = Vec::new();

  for (staff_entity, staff_pos, mut staff, task) in staff_q.iter_mut() {
    if staff.state != StaffState::Delivering {
      continue;
    }

    to_serve.push((staff_entity, task.target_table));
    commands.entity(staff_entity).remove::<StaffTarget>();
    commands.entity(staff_entity).remove::<NavigationComplete>();
    staff.state = StaffState::Idle;

    // Release the interaction cell reservation
    grid.release_cell(staff_pos.x, staff_pos.z, staff_entity);

    info!(
      "Staff delivered food to table {:?}, returning to Idle",
      task.target_table
    );
  }

  for (_, table_entity) in &to_serve {
    if let Ok((_, mut state)) = table_q.get_mut(*table_entity) {
      *state = TableState::Served;
    }
  }
}

/// Zone system: compute StaffZone for tables (adjacent cells within 1 Manhattan distance).
pub fn update_table_zones(
  mut commands: Commands,
  query: Query<(Entity, &GridFootprint), (With<TableState>, Without<StaffZone>)>,
) {
  for (entity, footprint) in query.iter() {
    commands.entity(entity).insert(StaffZone {
      cells: crate::zone_helpers::adjacent_cells(footprint, 1),
    });
  }
}

/// Zone system: compute StaffZone for stoves (front cells, 1 row deep).
pub fn update_stove_zones(
  mut commands: Commands,
  query: Query<(Entity, &GridPosition, &ApplianceGeometry), (With<StoveState>, Without<StaffZone>)>,
) {
  for (entity, pos, geo) in query.iter() {
    commands.entity(entity).insert(StaffZone {
      cells: crate::zone_helpers::front_cells((pos.x, pos.z), geo, 1),
    });
  }
}
