use bevy::prelude::*;
use bite_sized_earth::{ShellPlugin, SimPlugin};

fn main() {
  App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(SimPlugin)
    .add_plugins(ShellPlugin)
    .run();
}
