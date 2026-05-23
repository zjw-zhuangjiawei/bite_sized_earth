use bevy::prelude::*;
use bse_sim::SimPlugin;
use bse_shell::ShellPlugin;

fn main() {
  App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(SimPlugin)
    .add_plugins(ShellPlugin)
    .run();
}
