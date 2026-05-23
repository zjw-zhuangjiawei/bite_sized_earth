use bevy::prelude::*;

use crate::components::GridDirection;

#[derive(Message, Debug)]
pub struct DebugSpawnStaffRequest {
  pub grid_x: i32,
  pub grid_z: i32,
}

#[derive(Message, Debug)]
pub struct DebugSpawnCustomerRequest {
  pub grid_x: i32,
  pub grid_z: i32,
}

// ===== 新消息：多网格建造系统 =====

#[derive(Message, Debug)]
pub struct RequestPlaceTable {
  pub anchor: (i32, i32),
  pub direction: GridDirection,
}

#[derive(Message, Debug)]
pub struct RequestPlaceChair {
  pub anchor: (i32, i32),
  pub direction: GridDirection,
}

#[derive(Message, Debug)]
pub struct RequestPlaceRegister {
  pub anchor: (i32, i32),
  pub direction: GridDirection,
}

#[derive(Message, Debug)]
pub struct RequestDemolishAppliance {
  pub click: (i32, i32),
}

#[derive(Message, Debug)]
pub struct RequestPlaceStove {
  pub anchor: (i32, i32),
  pub direction: GridDirection,
}

pub fn register_all(app: &mut App) {
  app.add_message::<DebugSpawnStaffRequest>();
  app.add_message::<DebugSpawnCustomerRequest>();
  app.add_message::<RequestPlaceTable>();
  app.add_message::<RequestPlaceChair>();
  app.add_message::<RequestPlaceRegister>();
  app.add_message::<RequestDemolishAppliance>();
  app.add_message::<RequestPlaceStove>();
}
