use bevy::prelude::*;
use bse_core::components::{
  ApplianceGeometry, BelongsToTable, ChairState, Customer, CustomerState,
  GridPosition, RegisterState, Staff, StaffState, StoveState, TableState, get_footprint,
};
use bse_core::messages::{
  DebugSpawnCustomerRequest, DebugSpawnStaffRequest, RequestDemolishAppliance,
  RequestPlaceChair, RequestPlaceRegister, RequestPlaceStove, RequestPlaceTable,
};
use bse_core::world::{GridOccupancy, WorldGridMap};

// =========================================================================
// 新架构：多网格放置系统
// =========================================================================

/// Run condition: skip bind_new_chairs_system when no unbound chairs exist.
pub fn has_unbound_chairs(chair_q: Query<&ChairState, Without<BelongsToTable>>) -> bool {
  !chair_q.is_empty()
}

fn try_place(
  commands: &mut Commands,
  grid_map: &mut WorldGridMap,
  anchor: (i32, i32),
  geometry: ApplianceGeometry,
  identity: impl Component,
) -> Option<Entity> {
  let footprint = get_footprint(&geometry, anchor);
  if !grid_map.is_area_empty(&footprint) {
    return None;
  }

  grid_map.fill_area(&footprint, GridOccupancy::Occupied);

  let entity = commands.spawn((
    GridPosition {
      x: anchor.0,
      z: anchor.1,
    },
    geometry,
    identity,
  )).id();

  Some(entity)
}

pub fn handle_place_table(
  mut commands: Commands,
  mut grid_map: ResMut<WorldGridMap>,
  mut reader: MessageReader<RequestPlaceTable>,
) {
  for req in reader.read() {
    let geometry = ApplianceGeometry {
      base_width: 2,
      base_depth: 1,
      direction: req.direction,
    };
    if let Some(_entity) = try_place(&mut commands, &mut grid_map, req.anchor, geometry, TableState::default()) {
      info!("Placed table at ({},{}), direction {:?}", req.anchor.0, req.anchor.1, req.direction);
    }
  }
}

pub fn handle_place_chair(
  mut commands: Commands,
  mut grid_map: ResMut<WorldGridMap>,
  mut reader: MessageReader<RequestPlaceChair>,
) {
  for req in reader.read() {
    let geometry = ApplianceGeometry {
      base_width: 1,
      base_depth: 1,
      direction: req.direction,
    };
    if let Some(_entity) = try_place(&mut commands, &mut grid_map, req.anchor, geometry, ChairState::default()) {
      info!("Placed chair at ({},{}), direction {:?}", req.anchor.0, req.anchor.1, req.direction);
    }
  }
}

pub fn handle_place_register(
  mut commands: Commands,
  mut grid_map: ResMut<WorldGridMap>,
  mut reader: MessageReader<RequestPlaceRegister>,
) {
  for req in reader.read() {
    let geometry = ApplianceGeometry {
      base_width: 2,
      base_depth: 1,
      direction: req.direction,
    };
    if let Some(_entity) = try_place(&mut commands, &mut grid_map, req.anchor, geometry, RegisterState::default()) {
      info!("Placed register at ({},{}), direction {:?}", req.anchor.0, req.anchor.1, req.direction);
    }
  }
}

pub fn handle_place_stove(
  mut commands: Commands,
  mut grid_map: ResMut<WorldGridMap>,
  mut reader: MessageReader<RequestPlaceStove>,
) {
  for req in reader.read() {
    let geometry = ApplianceGeometry {
      base_width: 2,
      base_depth: 1,
      direction: req.direction,
    };
    if let Some(_entity) = try_place(&mut commands, &mut grid_map, req.anchor, geometry, StoveState::default()) {
      info!("Placed stove at ({},{}), direction {:?}", req.anchor.0, req.anchor.1, req.direction);
    }
  }
}

pub fn handle_demolish_appliance(
  mut commands: Commands,
  mut grid_map: ResMut<WorldGridMap>,
  mut reader: MessageReader<RequestDemolishAppliance>,
  query: Query<(Entity, &GridPosition, &ApplianceGeometry)>,
) {
  for req in reader.read() {
    for (entity, pos, geometry) in query.iter() {
      let footprint = get_footprint(geometry, (pos.x, pos.z));
      if footprint.contains(&req.click) {
        grid_map.clear_area(&footprint);
        commands.entity(entity).despawn();
        info!("Demolished appliance at ({},{}), footprint {:?}", req.click.0, req.click.1, footprint);
        break;
      }
    }
  }
}

// =========================================================================
// 旧系统：员工 / 顾客生成（保持不变）
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
