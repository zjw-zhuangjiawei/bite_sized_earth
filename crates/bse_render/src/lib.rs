pub mod camera;
pub mod debug_gizmos;
pub mod dev_console;
pub mod environment;
pub mod reactive;

use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use bse_core::messages::{
  RequestDemolishAppliance, RequestPlaceChair, RequestPlaceRegister, RequestPlaceTable,
};
use dev_console::DevConsoleState;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
  fn build(&self, app: &mut App) {
    app.add_plugins(EguiPlugin::default());
    app.insert_resource(DevConsoleState::default());
    app.add_systems(
      Startup,
      (
        camera::setup_camera,
        environment::setup_checkerboard,
        environment::setup_lighting,
      ),
    );
    app.add_systems(
      Update,
      (
        reactive::render_tables,
        reactive::render_chairs,
        reactive::render_registers,
        reactive::render_new_staff,
        reactive::render_new_customers,
        debug_gizmos::draw_spawn_position_highlight_system,
        debug_gizmos::draw_agent_path_preview_system,
      ),
    );
    app.add_systems(
      EguiPrimaryContextPass,
      dev_console::render_egui_console,
    );
    // 注册消息（渲染层发送，逻辑层消费）
    app.add_message::<RequestPlaceTable>();
    app.add_message::<RequestPlaceChair>();
    app.add_message::<RequestPlaceRegister>();
    app.add_message::<RequestDemolishAppliance>();
  }
}
