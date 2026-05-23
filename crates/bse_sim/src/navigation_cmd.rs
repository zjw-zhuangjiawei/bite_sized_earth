use std::collections::VecDeque;

use bevy::prelude::*;
use crate::components::{GridPosition, MovementProgress, PathQueue};
use crate::pathfinding::compute_agent_path;
use crate::world::GridLayers;

/// A discrete "fire-and-forget" navigation command for a single entity.
///
/// Reads the entity's [`GridPosition`] and the world's [`GridLayers`],
/// computes an A* path to `target`, and injects [`PathQueue`] +
/// [`MovementProgress`].  Cell reservation is handled by `agent_movement_tick`.
pub struct NavigateTo {
    pub target: (i32, i32),
    pub speed: f32,
}

impl EntityCommand for NavigateTo {
    fn apply(self, mut entity: EntityWorldMut) {
        let pos = entity
            .get::<GridPosition>()
            .expect("NavigateTo requires GridPosition component");
        let start = (pos.x, pos.z);

        let grid = entity.resource::<GridLayers>();

        let Some(path) = compute_agent_path(start, self.target, &grid) else {
            return;
        };

        if path.len() <= 1 {
            return;
        }

        let next = path[1];
        let remaining: VecDeque<(i32, i32)> = path.into_iter().skip(2).collect();

        entity.insert((
            PathQueue { path: remaining },
            MovementProgress {
                from: start,
                to: next,
                progress: 0.0,
                speed: self.speed,
            },
        ));
    }
}
