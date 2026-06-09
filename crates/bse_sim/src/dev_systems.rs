use crate::components::{
  BelongsToTable, ChairState, Customer, CustomerState, GridDirection, GridPosition, GridSize,
  Staff, StaffState, TableState, get_footprint,
};
use crate::messages::{DebugSpawnCustomerRequest, DebugSpawnStaffRequest};
use crate::world::GridLayers;
use bevy::prelude::*;

// =========================================================================
// Staff / Customer spawning
// =========================================================================

pub fn handle_spawn_staff_requests(
  mut commands: Commands,
  grid: Res<GridLayers>,
  mut message_reader: MessageReader<DebugSpawnStaffRequest>,
) {
  for req in message_reader.read() {
    if !grid.is_walkable(req.grid_x, req.grid_y) {
      continue;
    }

    commands.spawn((
      GridPosition {
        x: req.grid_x,
        y: req.grid_y,
      },
      Staff {
        state: StaffState::Idle,
      },
    ));
  }
}

pub fn handle_spawn_customer_requests(
  mut commands: Commands,
  grid: Res<GridLayers>,
  mut message_reader: MessageReader<DebugSpawnCustomerRequest>,
) {
  for req in message_reader.read() {
    if !grid.is_walkable(req.grid_x, req.grid_y) {
      continue;
    }
    info!("Spawning customer at ({},{})", req.grid_x, req.grid_y);

    commands.spawn((
      GridPosition {
        x: req.grid_x,
        y: req.grid_y,
      },
      Customer {
        state: CustomerState::Entering,
      },
    ));
  }
}

// =========================================================================
// Chair-table binding
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
