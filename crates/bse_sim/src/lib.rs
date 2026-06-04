pub mod components;
pub mod construction;
pub mod customer_ai;
#[cfg(feature = "dev")]
pub mod dev_systems;
pub mod local_nav;
pub mod messages;
pub mod movement;
pub mod navigation_cmd;
pub mod pathfinding;
pub mod slot_spawn;
pub mod slots;
pub mod staff_ai;
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
    app.add_systems(Startup, (init_grid_layers, slot_spawn::spawn_exit_slot));

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

    // ── Slot spawning + offset insertion ──
    app.add_systems(
      Update,
      (
        (
          slot_spawn::spawn_table_slots,
          slot_spawn::spawn_stove_slots,
          slot_spawn::spawn_register_slots,
          slot_spawn::spawn_chair_slots,
          slot_spawn::spawn_initial_queue_slots,
        ),
        bevy::ecs::schedule::ApplyDeferred,
        (
          slot_spawn::insert_cook_offset,
          slot_spawn::insert_deliver_offset,
          slot_spawn::insert_checkout_offset,
          slot_spawn::insert_sit_offset,
          slot_spawn::insert_queue_offset,
        ),
        slot_spawn::reindex_queue_slots,
      )
        .chain(),
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
        movement::agent_replan,
        movement::agent_movement_tick,
        bevy::ecs::schedule::ApplyDeferred,
        (
          customer_ai::customer_find_seat,
          customer_ai::customer_arrive_at_seat,
          staff_ai::staff_pickup,
          staff_ai::staff_arrive_at_stove,
        ),
        bevy::ecs::schedule::ApplyDeferred,
        (
          staff_ai::staff_cooking,
          staff_ai::staff_deliver,
          customer_ai::customer_arrive_at_queue,
          slot_spawn::shrink_queue_slots,
          slot_spawn::ensure_next_queue_slot,
          customer_ai::customer_eating,
          staff_ai::staff_checkout_start,
          staff_ai::staff_arrive_at_checkout,
          staff_ai::staff_checkout_tick,
          customer_ai::customer_exit,
          customer_ai::cleanup_tables,
        ),
      )
        .chain(),
    );
  }
}

fn init_grid_layers(mut commands: Commands) {
  commands.insert_resource(GridLayers::new(32, 32));
}
