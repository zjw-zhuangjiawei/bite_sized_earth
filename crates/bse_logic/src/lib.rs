pub mod camera;
pub mod customer_lifecycle;
pub mod dev_systems;
pub mod movement;
pub mod navigation_cmd;
pub mod pathfinding;

use bevy::prelude::*;
use bse_core::messages::{
  DebugSpawnCustomerRequest, DebugSpawnStaffRequest, RequestDemolishAppliance, RequestPlaceChair,
  RequestPlaceRegister, RequestPlaceTable,
};
use bse_core::world::WorldGridMap;

use camera::CameraControlPlugin;

fn init_grid_map(mut commands: Commands) {
  commands.insert_resource(WorldGridMap::new(32, 32));
}

pub struct LogicPlugin;

impl Plugin for LogicPlugin {
  fn build(&self, app: &mut App) {
    // 新架构消息
    app.add_message::<RequestPlaceTable>();
    app.add_message::<RequestPlaceChair>();
    app.add_message::<RequestPlaceRegister>();
    app.add_message::<RequestDemolishAppliance>();
    // 旧消息（保留 — Staff/Customer 仍然使用）
    app.add_message::<DebugSpawnStaffRequest>();
    app.add_message::<DebugSpawnCustomerRequest>();
    app.add_plugins(CameraControlPlugin);
    app.add_systems(Startup, init_grid_map);
    app.add_systems(
      Update,
      (
        dev_systems::handle_place_table,
        dev_systems::handle_place_chair,
        dev_systems::handle_place_register,
        dev_systems::handle_demolish_appliance,
        dev_systems::handle_spawn_staff_requests,
        dev_systems::handle_spawn_customer_requests,
        customer_lifecycle::customer_find_seat_system,
        customer_lifecycle::customer_arrive_at_seat_system,
        customer_lifecycle::customer_eating_system,
        customer_lifecycle::customer_exit_and_despawn_system,
        movement::universal_agent_move_system,
      ),
    );
  }
}
