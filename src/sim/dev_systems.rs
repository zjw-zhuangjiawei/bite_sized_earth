use super::components::{
  BelongsToTable, ChairState, Customer, CustomerState, GridDirection, GridPosition, GridSize,
  Staff, StaffState, TableState, get_footprint,
};
use super::world::GridLayers;
use bevy::prelude::*;

// =========================================================================
// Staff / Customer spawning — direct calls from shell dev_console
// =========================================================================

/// Spawn a staff agent at `(grid_x, grid_y)` if the cell is walkable.
pub fn spawn_staff(world: &mut World, grid_x: i32, grid_y: i32) {
  let walkable = world.resource::<GridLayers>().is_walkable(grid_x, grid_y);
  if !walkable {
    return;
  }
  world.spawn((
    GridPosition {
      x: grid_x,
      y: grid_y,
    },
    Staff {
      state: StaffState::Idle,
    },
  ));
}

/// Spawn a customer agent at `(grid_x, grid_y)` if the cell is walkable.
pub fn spawn_customer(world: &mut World, grid_x: i32, grid_y: i32) {
  let walkable = world.resource::<GridLayers>().is_walkable(grid_x, grid_y);
  if !walkable {
    return;
  }
  info!("Spawning customer at ({},{})", grid_x, grid_y);
  world.spawn((
    GridPosition {
      x: grid_x,
      y: grid_y,
    },
    Customer {
      state: CustomerState::Entering,
    },
  ));
}

// =========================================================================
// Chair-table binding (still a system: scans world per tick)
// =========================================================================

/// Run condition: skip when no unbound chairs exist.
pub fn has_unbound_chairs(chair_q: Query<&ChairState, Without<BelongsToTable>>) -> bool {
  !chair_q.is_empty()
}

/// Bind new chairs without `BelongsToTable` to the table they face.
pub fn bind_new_chairs_system(
  mut commands: Commands,
  chair_q: Query<
    (Entity, &GridPosition, &GridSize, &GridDirection),
    (With<ChairState>, Without<BelongsToTable>),
  >,
  table_q: Query<(Entity, &GridPosition, &GridSize, &GridDirection), With<TableState>>,
) {
  for (chair_entity, chair_pos, _chair_size, chair_dir) in chair_q.iter() {
    let (dx, dy) = chair_dir.facing_offset();
    let adj = (chair_pos.x + dx, chair_pos.y + dy);
    for (table_entity, table_pos, table_size, table_dir) in table_q.iter() {
      let footprint = get_footprint(table_size, *table_dir, (table_pos.x, table_pos.y));
      if footprint.contains(&adj) {
        commands.entity(chair_entity).insert(BelongsToTable {
          table: table_entity,
        });
        info!(
          "Chair at ({},{}) bound to table {:?}",
          chair_pos.x, chair_pos.y, table_entity
        );
        break;
      }
    }
  }
}
