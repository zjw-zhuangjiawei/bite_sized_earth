use bevy_ecs::message::Message;

use crate::components::GridRotation;

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
  pub rotation: GridRotation,
}

#[derive(Message, Debug)]
pub struct RequestPlaceChair {
  pub anchor: (i32, i32),
  pub rotation: GridRotation,
}

#[derive(Message, Debug)]
pub struct RequestPlaceRegister {
  pub anchor: (i32, i32),
  pub rotation: GridRotation,
}

#[derive(Message, Debug)]
pub struct RequestDemolishAppliance {
  pub click: (i32, i32),
}
