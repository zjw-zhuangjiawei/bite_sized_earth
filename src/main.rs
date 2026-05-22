use bevy::prelude::*;
use bse_logic::LogicPlugin;
use bse_render::RenderPlugin;

fn main() {
  App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(LogicPlugin)
    .add_plugins(RenderPlugin)
    .run();
}
