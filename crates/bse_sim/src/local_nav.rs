use crate::world::{CellIntent, GridLayers, ReserveResult};
use bevy::prelude::*;

/// Result of a local navigation step.
pub enum StepResult {
  /// Move to this cell (reservation succeeded).
  Advance { to: (i32, i32) },
  /// All attempts blocked — wait.
  Wait,
  /// Blocked too long, trigger A* replan.
  Replan,
}

/// Greedy fan with BFS fallback.
///
/// 1. Compute ideal cell (manhattan step toward target).
/// 2. Try reserve in fan order: [ideal, left90, right90, stay].
/// 3. All blocked → BFS(max_depth=3) for reachable cell closer to target.
/// 4. BFS fails → Wait.
pub fn local_step(
  entity: Entity,
  current: (i32, i32),
  target: (i32, i32),
  speed: f32,
  grid: &mut GridLayers,
  tick_delta: f32,
) -> StepResult {
  let desired_dir = direction_toward(current, target);
  let ideal = (current.0 + desired_dir.0, current.1 + desired_dir.1);

  // If ideal IS the target, just advance — no reservation needed for final cell
  if ideal == target {
    return StepResult::Advance { to: ideal };
  }

  // 1. Try ideal cell (passing through)
  if let ReserveResult::Claimed | ReserveResult::Preempted(_) = grid.try_reserve(
    ideal.0,
    ideal.1,
    entity,
    CellIntent::Transient,
    speed,
    tick_delta,
  ) {
    return StepResult::Advance { to: ideal };
  }

  // 2. Fan: [left, right, reverse, stay]
  let left = (current.0 - desired_dir.1, current.1 + desired_dir.0);
  let right = (current.0 + desired_dir.1, current.1 - desired_dir.0);
  let reverse = (current.0 - desired_dir.0, current.1 - desired_dir.1);

  for &candidate in &[left, right, reverse, current] {
    if let ReserveResult::Claimed | ReserveResult::Preempted(_) = grid.try_reserve(
      candidate.0,
      candidate.1,
      entity,
      CellIntent::Transient,
      speed,
      tick_delta,
    ) {
      return StepResult::Advance { to: candidate };
    }
  }

  // 3. BFS fallback (max_depth=3) — finds closest geometrically reachable cell
  if let Some(bfs_cell) = bfs_closer(current, target, grid, 3, entity) {
    if bfs_cell == target {
      return StepResult::Advance { to: bfs_cell };
    }
    if let ReserveResult::Claimed | ReserveResult::Preempted(_) = grid.try_reserve(
      bfs_cell.0,
      bfs_cell.1,
      entity,
      CellIntent::Transient,
      speed,
      tick_delta,
    ) {
      return StepResult::Advance { to: bfs_cell };
    }
  }

  StepResult::Wait
}

/// Manhattan step from `from` toward `to`. Each component in {-1, 0, 1}.
/// Prefers horizontal movement when both axes differ.
fn direction_toward(from: (i32, i32), to: (i32, i32)) -> (i32, i32) {
  let dx = (to.0 - from.0).clamp(-1, 1);
  let dy = (to.1 - from.1).clamp(-1, 1);
  if dx != 0 && dy != 0 {
    if to.0.abs_diff(from.0) >= to.1.abs_diff(from.1) {
      (dx, 0)
    } else {
      (0, dy)
    }
  } else {
    (dx, dy)
  }
}

/// BFS from `start` up to `max_depth`, looking for a cell strictly closer
/// to `target` than `start`. Skips cells reserved by other agents —
/// `is_walkable_for` treats own reservations as walkable.
fn bfs_closer(
  start: (i32, i32),
  target: (i32, i32),
  grid: &GridLayers,
  max_depth: u32,
  entity: Entity,
) -> Option<(i32, i32)> {
  use std::collections::VecDeque;

  let start_dist = manhattan(start, target);
  let mut visited = vec![start];
  let mut queue = VecDeque::new();
  queue.push_back((start, 0u32));

  while let Some((cell, depth)) = queue.pop_front() {
    if depth >= max_depth {
      continue;
    }

    for &(dx, dy) in &[(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
      let next = (cell.0 + dx, cell.1 + dy);
      if visited.contains(&next) {
        continue;
      }
      visited.push(next);

      // Skip cells blocked by static obstacles or reserved by other agents
      if !grid.is_walkable_for(next.0, next.1, entity) {
        continue;
      }

      if manhattan(next, target) < start_dist {
        return Some(next);
      }

      queue.push_back((next, depth + 1));
    }
  }

  None
}

fn manhattan(a: (i32, i32), b: (i32, i32)) -> u32 {
  a.0.abs_diff(b.0) + a.1.abs_diff(b.1)
}
