use bevy::prelude::*;
use bse_core::components::{
  ApplianceGeometry, ChairState, Customer, CustomerState, GridPosition, RegisterState, Staff,
  StaffState, TableState, get_footprint,
};
use bse_core::messages::{
  DebugSpawnCustomerRequest, DebugSpawnStaffRequest, RequestDemolishAppliance, RequestPlaceChair,
  RequestPlaceRegister, RequestPlaceTable,
};
use bse_core::world::{GridOccupancy, WorldGridMap};

// =========================================================================
// 新架构：多网格放置系统
// =========================================================================

fn try_place(
  commands: &mut Commands,
  grid_map: &mut WorldGridMap,
  anchor: (i32, i32),
  geometry: ApplianceGeometry,
  identity: impl Component,
) -> bool {
  let footprint = get_footprint(&geometry, anchor);
  if !grid_map.is_area_empty(&footprint) {
    return false;
  }

  grid_map.fill_area(&footprint, GridOccupancy::Occupied);

  commands.spawn((
    GridPosition {
      x: anchor.0,
      z: anchor.1,
    },
    geometry,
    identity,
  ));

  true
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
      rotation: req.rotation,
    };
    if try_place(&mut commands, &mut grid_map, req.anchor, geometry, TableState::default()) {
      info!("Placed table at ({},{}), rotation {:?}", req.anchor.0, req.anchor.1, req.rotation);
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
      rotation: req.rotation,
    };
    if try_place(&mut commands, &mut grid_map, req.anchor, geometry, ChairState::default()) {
      info!("Placed chair at ({},{}), rotation {:?}", req.anchor.0, req.anchor.1, req.rotation);
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
      rotation: req.rotation,
    };
    if try_place(&mut commands, &mut grid_map, req.anchor, geometry, RegisterState::default()) {
      info!("Placed register at ({},{}), rotation {:?}", req.anchor.0, req.anchor.1, req.rotation);
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
      Transform::from_xyz(req.grid_x as f32, 0.65, req.grid_z as f32),
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
      Transform::from_xyz(req.grid_x as f32, 0.65, req.grid_z as f32),
    ));
  }
}
