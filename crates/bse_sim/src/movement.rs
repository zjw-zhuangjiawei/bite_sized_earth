use bevy::prelude::*;
use crate::components::{GridPosition, MovementProgress, NavigationComplete, PathQueue};
use crate::world::GridLayers;

/// Identity-agnostic movement tick.
///
/// Advances [`MovementProgress`], updates [`GridPosition`] on waypoint
/// arrival, and manages dynamic cell reservation for collision avoidance.
pub fn agent_movement_tick(
    mut commands: Commands,
    time: Res<Time>,
    mut grid: ResMut<GridLayers>,
    mut query: Query<(
        Entity,
        &mut GridPosition,
        &mut MovementProgress,
        &mut PathQueue,
    )>,
) {
    let delta = time.delta_secs();
    for (entity, mut grid_pos, mut movement, mut path_queue) in query.iter_mut() {
        movement.progress += delta * movement.speed;

        if movement.progress >= 1.0 {
            // Release the cell we are leaving
            grid.release_cell(movement.from.0, movement.from.1, entity);

            // Arrive at the next cell
            grid_pos.x = movement.to.0;
            grid_pos.z = movement.to.1;

            debug!("Move: arrived at ({},{})", grid_pos.x, grid_pos.z);

            if let Some(next_grid) = path_queue.path.pop_front() {
                // Reserve the upcoming cell
                grid.reserve_cell(next_grid.0, next_grid.1, entity);

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
                debug!("Move: path complete, inserting NavigationComplete");
                commands
                    .entity(entity)
                    .remove::<(MovementProgress, PathQueue)>()
                    .insert(NavigationComplete);
            }
        }
    }
}
