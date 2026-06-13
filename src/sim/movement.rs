use super::components::{
  GridPosition, MovementProgress, NavState, NavTarget, NavigationComplete, PathQueue,
  ReplanRequest, SlotPosition,
};
use super::local_nav::{StepResult, local_step};
use super::world::{CellIntent, GridLayers, ReserveResult};
use bevy::prelude::*;

/// Movement tick with TTL reservation + local adjustment.
///
/// 1. Decay all reservation TTLs
/// 2. For each agent, advance progress
/// 3. On waypoint arrival: release cell, try reserve next
/// 4. If blocked: local_step (fan → BFS → wait/replan)
pub fn agent_movement_tick(
  mut commands: Commands,
  time: Res<Time>,
  mut grid: ResMut<GridLayers>,
  slot_positions: Query<&SlotPosition>,
  mut query: Query<(
    Entity,
    &mut GridPosition,
    &mut MovementProgress,
    &mut PathQueue,
    &NavTarget,
    &mut NavState,
  )>,
) {
  grid.tick_reservations();
  let delta = time.delta_secs();

  for (entity, mut grid_pos, mut movement, mut path_queue, nav_target, mut nav_state) in
    query.iter_mut()
  {
    movement.progress += delta * movement.speed;

    if movement.progress < 1.0 {
      continue;
    }

    // Arrived at a cell — release previous and update position
    grid.release_cell(movement.from.0, movement.from.1, entity);
    grid_pos.x = movement.to.0;
    grid_pos.y = movement.to.1;
    debug!("Move: arrived at ({},{})", grid_pos.x, grid_pos.y);

    // Read target slot's grid position
    let target_pos = slot_positions.get(nav_target.slot).ok().map(|p| (p.x, p.y));

    // If slot is gone, trigger replan
    let Some(target) = target_pos else {
      commands.entity(entity).insert(ReplanRequest);
      continue;
    };

    // Check if we've reached the target slot
    if (grid_pos.x, grid_pos.y) == target {
      debug!("Move: reached target slot {:?}", nav_target.slot);
      grid.make_permanent(grid_pos.x, grid_pos.y, entity);
      commands
        .entity(entity)
        .remove::<(MovementProgress, PathQueue, NavState)>()
        .insert(NavigationComplete { failed: false });
      continue;
    }

    // Try next waypoint from queue
    if let Some(next) = path_queue.path.pop_front() {
      match grid.try_reserve(
        next.0,
        next.1,
        entity,
        CellIntent::Transient,
        movement.speed,
        delta,
      ) {
        ReserveResult::Claimed | ReserveResult::Preempted(_) => {
          // Normal advance
          movement.from = (grid_pos.x, grid_pos.y);
          movement.to = next;
          movement.progress = 0.0;
          *nav_state = NavState::Cruising;
          debug!(
            "Move: next segment -> ({},{}), remaining {}",
            next.0,
            next.1,
            path_queue.path.len()
          );
        }
        ReserveResult::Blocked => {
          // Push back and try local adjustment
          path_queue.path.push_front(next);
          match local_step(
            entity,
            (grid_pos.x, grid_pos.y),
            target,
            movement.speed,
            &mut grid,
            delta,
          ) {
            StepResult::Advance { to } => {
              movement.from = (grid_pos.x, grid_pos.y);
              movement.to = to;
              movement.progress = 0.0;
              *nav_state = NavState::Cruising;
              debug!(
                "Move: local adjust -> ({},{}), path_queue backup",
                to.0, to.1
              );
            }
            StepResult::Wait => {
              *nav_state = match *nav_state {
                NavState::Blocked { ticks } => NavState::Blocked { ticks: ticks + 1 },
                _ => NavState::Blocked { ticks: 1 },
              };
              debug!(
                "Move: blocked, waiting tick {}",
                match *nav_state {
                  NavState::Blocked { ticks } => ticks,
                  _ => 0,
                }
              );
            }
            StepResult::Replan => {
              commands.entity(entity).insert(ReplanRequest);
              debug!("Move: blocked too long, requesting replan");
            }
          }
        }
      }
    } else if (grid_pos.x, grid_pos.y) == target {
      // Path consumed and at target
      grid.make_permanent(grid_pos.x, grid_pos.y, entity);
      commands
        .entity(entity)
        .remove::<(MovementProgress, PathQueue, NavState)>()
        .insert(NavigationComplete { failed: false });
    } else {
      // Path consumed but not at target — trigger replan
      commands.entity(entity).insert(ReplanRequest);
      debug!("Move: path empty but not at target, replanning");
    }
  }
}

/// A* fallback for stuck agents.
///
/// Reads ReplanRequest, reads slot's GridPosition, computes fresh A* path.
/// On success: replaces PathQueue + MovementProgress, clears ReplanRequest.
/// On failure: inserts NavigationComplete { failed: true }.
pub fn agent_replan(
  mut commands: Commands,
  grid: Res<GridLayers>,
  slot_positions: Query<&SlotPosition>,
  mut query: Query<
    (
      Entity,
      &GridPosition,
      &NavTarget,
      &mut MovementProgress,
      &mut PathQueue,
    ),
    With<ReplanRequest>,
  >,
) {
  for (entity, pos, nav_target, mut movement, mut path_queue) in query.iter_mut() {
    let Some(target_pos) = slot_positions.get(nav_target.slot).ok().map(|p| (p.x, p.y)) else {
      // Slot is gone — give up
      commands.entity(entity).remove::<ReplanRequest>();
      commands
        .entity(entity)
        .remove::<(MovementProgress, PathQueue)>()
        .insert(NavigationComplete { failed: true });
      continue;
    };

    let start = (pos.x, pos.y);
    let path = super::pathfinding::compute_agent_path(start, target_pos, &grid);

    let Some(path) = path else {
      warn!(
        "Replan: no path from ({},{}) to ({},{})",
        start.0, start.1, target_pos.0, target_pos.1
      );
      commands.entity(entity).remove::<ReplanRequest>();
      commands
        .entity(entity)
        .remove::<(MovementProgress, PathQueue)>()
        .insert(NavigationComplete { failed: true });
      continue;
    };

    if path.len() <= 1 {
      commands.entity(entity).remove::<ReplanRequest>();
      commands
        .entity(entity)
        .remove::<(MovementProgress, PathQueue)>()
        .insert(NavigationComplete { failed: false });
      continue;
    }

    // Rebuild movement from current position
    let next = path[1];
    let remaining: std::collections::VecDeque<(i32, i32)> = path.into_iter().skip(2).collect();

    *movement = MovementProgress {
      from: start,
      to: next,
      progress: 0.0,
      speed: movement.speed,
    };
    *path_queue = PathQueue { path: remaining };
    commands.entity(entity).remove::<ReplanRequest>();
    debug!(
      "Replan: new path from ({},{}) with {} remaining",
      start.0,
      start.1,
      path_queue.path.len()
    );
  }
}
