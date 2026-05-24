pub mod components;
pub mod construction;
pub mod customer_lifecycle;
#[cfg(feature = "dev")]
pub mod dev_systems;
pub mod kitchen_workflow;
pub mod messages;
pub mod movement;
pub mod navigation_cmd;
pub mod pathfinding;
pub mod register_workflow;
pub mod world;
pub mod zone_helpers;

use bevy::prelude::*;
use std::collections::VecDeque;
use world::GridLayers;

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
    app.add_systems(Startup, init_grid_layers);

    // ── Construction phase ──
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

    // ── Zone computation (once per new appliance) ──
    app.add_systems(
      Update,
      (
        register_workflow::update_register_zones,
        kitchen_workflow::update_table_zones,
        kitchen_workflow::update_stove_zones,
        dev_systems::update_chair_zones,
      ),
    );

    // ── Dev-only spawning ──
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

    // ── Gameplay pipeline ──
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
          register_workflow::customer_arrive_at_register_system,
          customer_lifecycle::customer_eating_system,
          register_workflow::staff_checkout_system,
          customer_lifecycle::customer_exit_and_despawn_system,
          customer_lifecycle::cleanup_table_system,
        ),
      )
        .chain(),
    );
  }
}

fn init_grid_layers(mut commands: Commands) {
  commands.insert_resource(GridLayers::new(32, 32));
}
