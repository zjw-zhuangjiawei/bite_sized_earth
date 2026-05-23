use bevy::prelude::*;
use crate::components::{
  ApplianceGeometry, BelongsToTable, ChairState, Customer, CustomerState,
  GridPosition, Staff, StaffState, TableState, get_footprint,
};
use crate::messages::{
  DebugSpawnCustomerRequest, DebugSpawnStaffRequest,
};
use crate::world::WorldGridMap;

// =========================================================================
// 员工 / 顾客生成
// =========================================================================

pub fn handle_spawn_staff_requests(
  mut commands: Commands,
  grid_map: Res<WorldGridMap>,
  mut message_reader: MessageReader<DebugSpawnStaffRequest>,
) {
  for req in message_reader.read() {
    if !grid_map.is_walkable(req.grid_x, req.grid_z) {
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
  mut message_reader: MessageReader<DebugSpawnCustomerRequest>,
) {
  for req in message_reader.read() {
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
// 椅子-桌子绑定（独立系统，等命令刷新后运行）
// =========================================================================

/// Run condition: skip bind_new_chairs_system when no unbound chairs exist.
pub fn has_unbound_chairs(chair_q: Query<&ChairState, Without<BelongsToTable>>) -> bool {
  !chair_q.is_empty()
}

/// 为没有 BelongsToTable 的新椅子查找朝向的桌子并绑定。
///
/// 独立于 handle_place_chair 运行，因为同帧放置桌子的 entity
/// 在命令刷新前不可见。每帧查询椅子 + 桌子，跳过已绑定的椅子。
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
        commands
          .entity(chair_entity)
          .insert(BelongsToTable { table: table_entity });
        info!(
          "Chair at ({},{}) bound to table {:?}",
          chair_pos.x, chair_pos.z, table_entity
        );
        break;
      }
    }
  }
}
