use std::collections::VecDeque;

use super::components::{
  GridPosition, MovementProgress, NavState, NavTarget, PathQueue, SlotPosition,
};
use super::pathfinding::compute_agent_path;
use super::world::GridLayers;
use bevy::prelude::*;

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
    let start = (pos.x, pos.y);

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
      NavState::default(),
    ));
  }
}

/// Navigate entity toward a slot entity.
///
/// Reads the slot's [`GridPosition`] from the world, computes A* path,
/// and injects [`PathQueue`] + [`MovementProgress`] + [`NavTarget`].
pub struct NavigateToSlot {
  pub slot: Entity,
  pub speed: f32,
}

impl EntityCommand for NavigateToSlot {
  fn apply(self, mut entity: EntityWorldMut) {
    let pos = entity
      .get::<GridPosition>()
      .expect("NavigateToSlot requires GridPosition component");
    let start = (pos.x, pos.y);

    let Some(target_pos) = entity.world().get::<SlotPosition>(self.slot) else {
      warn!(
        "NavigateToSlot: target slot {:?} has no SlotPosition",
        self.slot
      );
      return;
    };
    let target = (target_pos.x, target_pos.y);

    let grid = entity.resource::<GridLayers>();

    let Some(path) = compute_agent_path(start, target, &grid) else {
      warn!(
        "NavigateToSlot: no path from ({},{}) to ({},{})",
        start.0, start.1, target.0, target.1
      );
      return;
    };

    if path.len() <= 1 {
      return;
    }

    let next = path[1];
    let remaining: VecDeque<(i32, i32)> = path.into_iter().skip(2).collect();

    entity.insert((
      NavTarget { slot: self.slot },
      NavState::default(),
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
