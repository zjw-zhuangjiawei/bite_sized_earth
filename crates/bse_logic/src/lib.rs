pub mod customer_lifecycle;
pub mod dev_systems;
pub mod kitchen_workflow;
pub mod movement;
pub mod navigation_cmd;
pub mod pathfinding;

use bevy::ecs::schedule::ApplyDeferred;
use bevy::prelude::*;

pub struct LogicPlugin;

impl Plugin for LogicPlugin {
  fn build(&self, app: &mut App) {
    // =========================================================================
    // Phase 0: Dev placement → flush → bind chairs (only when unbound)
    // =========================================================================
    app.add_systems(
      Update,
      (
        (
          dev_systems::handle_place_table,
          dev_systems::handle_place_chair,
          dev_systems::handle_place_register,
          dev_systems::handle_place_stove,
          dev_systems::handle_demolish_appliance,
          dev_systems::handle_spawn_staff_requests,
          dev_systems::handle_spawn_customer_requests,
        ),
        ApplyDeferred,
        dev_systems::bind_new_chairs_system.run_if(dev_systems::has_unbound_chairs),
      )
        .chain(),
    );

    // =========================================================================
    // Gameplay pipeline: Movement → flush → Logic → flush → Reactions
    // =========================================================================
    app.add_systems(
      Update,
      (
        // Phase 1: Advance movement, insert NavigationComplete on arrival
        movement::agent_movement_tick,
        ApplyDeferred,
        // Phase 2: Process NavigationComplete + state machines
        (
          customer_lifecycle::customer_find_seat_system,
          customer_lifecycle::customer_arrive_at_seat_system,
          kitchen_workflow::staff_pickup_system,
          kitchen_workflow::staff_cooking_system,
        ),
        ApplyDeferred,
        // Phase 3: Serve, eat, exit, cleanup (react to Phase 2 state changes)
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
