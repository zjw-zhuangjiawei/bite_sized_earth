pub mod components;
pub mod messages;
pub mod world;

use std::collections::VecDeque;

use bevy::prelude::*;
use world::WorldGridMap;

#[derive(Resource, Default)]
pub struct OrderQueue {
  pub pending: VecDeque<Entity>,
}

pub struct CorePlugin;

impl Plugin for CorePlugin {
  fn build(&self, app: &mut App) {
    messages::register_all(app);
    app.insert_resource(OrderQueue::default());
    app.add_systems(Startup, init_grid_map);
  }
}

fn init_grid_map(mut commands: Commands) {
  commands.insert_resource(WorldGridMap::new(32, 32));
}
