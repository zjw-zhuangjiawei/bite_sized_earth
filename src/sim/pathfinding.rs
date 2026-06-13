use super::world::GridLayers;
use bevy::prelude::*;
use pathfinding::prelude::astar;

pub fn compute_agent_path(
  start: (i32, i32),
  goal: (i32, i32),
  grid: &GridLayers,
) -> Option<Vec<(i32, i32)>> {
  let result = astar(
    &start,
    |&(x, y)| {
      [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .map(move |(dx, dy)| (x + dx, y + dy))
        .filter(|&(nx, ny)| grid.is_walkable(nx, ny) || (nx, ny) == goal)
        .map(|p| (p, 1))
    },
    |&(x, y)| (goal.0.abs_diff(x) + goal.1.abs_diff(y)) as i32,
    |&p| p == goal,
  );
  let path = result.map(|(path, _cost)| path);
  debug!(
    "A* from ({},{}) to ({},{}): {:?}",
    start.0, start.1, goal.0, goal.1, path
  );
  path
}
