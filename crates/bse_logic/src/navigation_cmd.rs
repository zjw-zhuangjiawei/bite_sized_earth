use std::collections::VecDeque;

use bevy::prelude::*;
use bse_core::components::{GridPosition, MovementProgress, PathQueue};
use bse_core::world::WorldGridMap;

use crate::pathfinding::compute_agent_path;

/// A discrete "fire-and-forget" navigation command for a single entity.
///
/// Reads the entity's [`GridPosition`] and the world's [`WorldGridMap`]
/// internally, computes an A* path to `target`, and injects the entity
/// with [`PathQueue`] + [`MovementProgress`] to drive it through the
/// unified movement system (`universal_agent_move_system`).
///
/// Identity-agnostic: any entity (Customer, Staff, …) can be navigated.
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

    let grid_map = entity.resource::<WorldGridMap>();

    let Some(path) = compute_agent_path(start, self.target, &grid_map, true) else {
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
