use bevy::prelude::*;
use bse_shell::ShellPlugin;
use bse_sim::SimPlugin;

fn main() {
  App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(SimPlugin)
    .add_plugins(ShellPlugin)
    .run();
}
