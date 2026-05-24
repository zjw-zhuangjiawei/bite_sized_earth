use crate::components::{
  get_footprint, ApplianceGeometry, BelongsToTable, ChairState, Customer, CustomerState,
  CustomerZone, GridFootprint, GridPosition, Staff, StaffState, TableState,
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
    if !grid.is_walkable(req.grid_x, req.grid_z) {
      continue;
    }

    commands.spawn((
      GridPosition {
        x: req.grid_x,
        z: req.grid_z,
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
    if !grid.is_walkable(req.grid_x, req.grid_z) {
      continue;
    }
    info!("Spawning customer at ({},{})", req.grid_x, req.grid_z);

    commands.spawn((
      GridPosition {
        x: req.grid_x,
        z: req.grid_z,
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
    (Entity, &GridPosition, &ApplianceGeometry),
    (With<ChairState>, Without<BelongsToTable>),
  >,
  table_q: Query<(Entity, &GridPosition, &ApplianceGeometry), With<TableState>>,
) {
  for (chair_entity, chair_pos, chair_geo) in chair_q.iter() {
    let adj = (
      chair_pos.x + chair_geo.direction.facing_offset().0,
      chair_pos.z + chair_geo.direction.facing_offset().1,
    );
    for (table_entity, table_pos, table_geo) in table_q.iter() {
      let footprint = get_footprint(table_geo, (table_pos.x, table_pos.z));
      if footprint.contains(&adj) {
        commands.entity(chair_entity).insert(BelongsToTable {
          table: table_entity,
        });
        info!(
          "Chair at ({},{}) bound to table {:?}",
          chair_pos.x, chair_pos.z, table_entity
        );
        break;
      }
    }
  }
}

/// Zone system: compute CustomerZone for chairs (chair's own footprint cells).
pub fn update_chair_zones(
  mut commands: Commands,
  query: Query<(Entity, &GridFootprint), (With<ChairState>, Without<CustomerZone>)>,
) {
  for (entity, footprint) in query.iter() {
    commands.entity(entity).insert(CustomerZone {
      cells: footprint.cells.clone(),
    });
  }
}
