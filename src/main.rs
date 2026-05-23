use bevy::prelude::*;
use bse_core::CorePlugin;
use bse_logic::LogicPlugin;
use bse_shell::ShellPlugin;

fn main() {
  App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(CorePlugin)
    .add_plugins(LogicPlugin)
    .add_plugins(ShellPlugin)
    .run();
}
