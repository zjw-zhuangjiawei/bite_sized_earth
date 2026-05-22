use bevy::prelude::*;
use bse_core::components::{GridPosition, MovementProgress, PathQueue};

/// Identity-agnostic movement driver.
///
/// Queries every entity that carries the three movement components
/// (`GridPosition`, `MovementProgress`, `PathQueue`) — whether it is a
/// Customer, Staff, or any future agent.  When the path is exhausted the
/// movement components are automatically removed so the entity returns to
/// an idle / waiting state.
pub fn universal_agent_move_system(
  mut commands: Commands,
  time: Res<Time>,
  mut query: Query<(
    Entity,
    &mut Transform,
    &mut GridPosition,
    &mut MovementProgress,
    &mut PathQueue,
  )>,
) {
  let delta = time.delta_secs();
  for (entity, mut transform, mut grid_pos, mut movement, mut path_queue) in query.iter_mut() {
    movement.progress += delta * movement.speed;

    if movement.progress >= 1.0 {
      grid_pos.x = movement.to.0;
      grid_pos.z = movement.to.1;

      transform.translation.x = grid_pos.x as f32;
      transform.translation.z = grid_pos.z as f32;

      debug!("Move: arrived at ({},{})", grid_pos.x, grid_pos.z);

      if let Some(next_grid) = path_queue.path.pop_front() {
        debug!(
          "Move: next segment ({},{}) -> ({},{}), remaining {}",
          grid_pos.x,
          grid_pos.z,
          next_grid.0,
          next_grid.1,
          path_queue.path.len()
        );
        movement.from = (grid_pos.x, grid_pos.z);
        movement.to = next_grid;
        movement.progress = 0.0;
      } else {
        debug!("Move: path complete, removing movement components");
        commands
          .entity(entity)
          .remove::<(MovementProgress, PathQueue)>();
      }
    } else {
      // Frame-independent linear interpolation — fixes the visual "teleport"
      // bug where entities stood still until progress hit 1.0.
      let from_x = movement.from.0 as f32;
      let from_z = movement.from.1 as f32;
      let to_x = movement.to.0 as f32;
      let to_z = movement.to.1 as f32;
      transform.translation.x = from_x + (to_x - from_x) * movement.progress;
      transform.translation.z = from_z + (to_z - from_z) * movement.progress;
    }
  }
}
