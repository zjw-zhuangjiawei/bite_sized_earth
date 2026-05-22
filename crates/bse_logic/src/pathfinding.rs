use bevy::prelude::*;
use bse_core::world::WorldGridMap;
use pathfinding::prelude::astar;

pub fn compute_agent_path(
    start: (i32, i32),
    goal: (i32, i32),
    grid_map: &WorldGridMap,
    walkable_goal: bool,
) -> Option<Vec<(i32, i32)>> {
    let result = astar(
        &start,
        |&(x, z)| {
            [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)]
                .into_iter()
                .map(move |(dx, dz)| (x + dx, z + dz))
                .filter(|&(nx, nz)| {
                    grid_map.is_walkable(nx, nz) || (walkable_goal && (nx, nz) == goal)
                })
                .map(|p| (p, 1))
        },
        |&(x, z)| (goal.0.abs_diff(x) + goal.1.abs_diff(z)) as i32,
        |&p| p == goal,
    );
    let path = result.map(|(path, _cost)| path);
    debug!(
        "A* from ({},{}) to ({},{}): {:?}",
        start.0, start.1, goal.0, goal.1, path
    );
    path
}
