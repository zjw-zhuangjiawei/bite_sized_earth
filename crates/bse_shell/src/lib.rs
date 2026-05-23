pub mod debug_gizmos;
pub mod dev_console;
pub mod environment;
pub mod input;
pub mod reactive;

use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use bevy_enhanced_input::EnhancedInputPlugin;
use dev_console::DevConsoleState;
use input::camera::CameraControlPlugin;

pub struct ShellPlugin;

impl Plugin for ShellPlugin {
  fn build(&self, app: &mut App) {
    app.add_plugins(EguiPlugin::default());
    app.add_plugins(EnhancedInputPlugin);
    app.add_plugins(CameraControlPlugin);
    app.insert_resource(DevConsoleState::default());
    app.add_systems(
      Startup,
      (
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
        reactive::render_stoves,
        reactive::render_new_staff,
        reactive::render_new_customers,
        reactive::sync_agent_transform,
        debug_gizmos::draw_spawn_position_highlight_system,
        debug_gizmos::draw_agent_path_preview_system,
        debug_gizmos::draw_world_axes,
        debug_gizmos::draw_appliance_direction_gizmos,
      ),
    );
    app.add_systems(
      EguiPrimaryContextPass,
      dev_console::render_egui_console,
    );
  }
}
