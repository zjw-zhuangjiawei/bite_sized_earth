pub mod components;
pub mod messages;
pub mod world;
pub mod construction;  // define_appliances! macro, placement, and demolish handlers
pub mod movement;
pub mod pathfinding;
pub mod navigation_cmd;
pub mod customer_lifecycle;
pub mod kitchen_workflow;
#[cfg(feature = "dev")]
pub mod dev_systems;

use std::collections::VecDeque;
use bevy::prelude::*;
use world::WorldGridMap;

#[derive(Resource, Default)]
pub struct OrderQueue {
  pub pending: VecDeque<Entity>,
}

pub struct SimPlugin;

impl Plugin for SimPlugin {
  fn build(&self, app: &mut App) {
    messages::register_all(app);
    app.insert_resource(OrderQueue::default());

    // Startup
    app.add_systems(Startup, init_grid_map);

    // Construction: handle placement commands from any source (dev console, future UI)
    app.add_systems(
      Update,
      (
        construction::handle_place_table,
        construction::handle_place_chair,
        construction::handle_place_register,
        construction::handle_place_stove,
        construction::handle_demolish_appliance,
      ),
    );

    // Dev-only: debug spawn + chair binding (needs ApplyDeferred between placement and binding)
    #[cfg(feature = "dev")]
    app.add_systems(
      Update,
      (
        (
          dev_systems::handle_spawn_staff_requests,
          dev_systems::handle_spawn_customer_requests,
        ),
        bevy::ecs::schedule::ApplyDeferred,
        dev_systems::bind_new_chairs_system.run_if(dev_systems::has_unbound_chairs),
      )
        .chain(),
    );

    // Gameplay pipeline: Movement → flush → Logic → flush → Reactions
    app.add_systems(
      Update,
      (
        movement::agent_movement_tick,
        bevy::ecs::schedule::ApplyDeferred,
        (
          customer_lifecycle::customer_find_seat_system,
          customer_lifecycle::customer_arrive_at_seat_system,
          kitchen_workflow::staff_pickup_system,
          kitchen_workflow::staff_cooking_system,
        ),
        bevy::ecs::schedule::ApplyDeferred,
        (
          kitchen_workflow::staff_deliver_system,
          customer_lifecycle::customer_eating_system,
          customer_lifecycle::customer_exit_and_despawn_system,
          customer_lifecycle::cleanup_table_system,
        ),
      )
        .chain(),
    );
  }
}

fn init_grid_map(mut commands: Commands) {
  commands.insert_resource(WorldGridMap::new(32, 32));
}
