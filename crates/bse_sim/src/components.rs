use std::collections::VecDeque;

use bevy::prelude::*;
use smallvec::SmallVec;

/// World-grid position customers navigate to when leaving.
pub const EXIT_POSITION: (i32, i32) = (0, 0);

// =========================================================================
// 1. Spatial / layer components
// =========================================================================

#[derive(Component, Debug, Clone, Copy)]
pub struct GridPosition {
  pub x: i32,
  pub z: i32,
}

/// Which layer this entity belongs to (placed by construction handlers).
pub use crate::world::GridLayer;

/// The set of grid cells an entity occupies.  Written once at spawn, read at
/// demolish time and by the interaction-point refresh system.
#[derive(Component, Debug, Clone)]
pub struct GridFootprint {
  pub cells: SmallVec<[(i32, i32); 8]>,
}

// =========================================================================
// 2. Movement suite
// =========================================================================

#[derive(Component, Default)]
pub struct PathQueue {
  pub path: VecDeque<(i32, i32)>,
}

#[derive(Component)]
pub struct MovementProgress {
  pub from: (i32, i32),
  pub to: (i32, i32),
  pub progress: f32,
  pub speed: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct NavigationComplete;

// =========================================================================
// 3. Interaction
// =========================================================================

/// Immutable rule describing how interaction cells are derived for an appliance.
/// Written once at spawn, never changed.
#[derive(Component, Debug, Clone)]
pub enum InteractionRule {
  /// Cells in front of the appliance along its facing direction.
  Front,
  /// All cells within `range` Manhattan distance of the footprint.
  AllAdjacent { range: u32 },
  /// The entity's own footprint cells (e.g. chair — agent sits on the chair).
  OnSite,
}

/// Mutable set of currently-available interaction cells.  Computed at spawn
/// and refreshed on-demand when a neighbour changes.
#[derive(Component, Debug, Clone)]
pub struct InteractionPoints {
  pub cells: SmallVec<[(i32, i32); 8]>,
}

// =========================================================================
// 4. Actor identity + state
// =========================================================================

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Customer {
  pub state: CustomerState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CustomerState {
  Entering,
  WalkingToSeat,
  WaitingForFood,
  Eating(f32),
  Leaving,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Staff {
  pub state: StaffState,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum StaffState {
  #[default]
  Idle,
  WalkingToKitchen,
  Cooking(f32),
  Delivering,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct StaffTarget {
  pub target_table: Entity,
}

// =========================================================================
// 5. Appliance geometry + footprint
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridDirection {
  PosZ,
  NegX,
  NegZ,
  PosX,
}

impl GridDirection {
  pub fn rotate_cw(self) -> Self {
    match self {
      Self::PosZ => Self::NegX,
      Self::NegX => Self::NegZ,
      Self::NegZ => Self::PosX,
      Self::PosX => Self::PosZ,
    }
  }

  pub fn facing_offset(self) -> (i32, i32) {
    match self {
      Self::PosZ => (0, 1),
      Self::NegZ => (0, -1),
      Self::PosX => (1, 0),
      Self::NegX => (-1, 0),
    }
  }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct ApplianceGeometry {
  pub right: i32,
  pub forward: i32,
  pub direction: GridDirection,
}

/// Compute all grid cells occupied by an appliance.
///
/// `anchor` is the left-back corner of the appliance (from the facing
/// direction's perspective).  The appliance extends `right` cells to the
/// right (perpendicular to facing) and `forward` cells along the facing
/// direction.
pub fn get_footprint(geometry: &ApplianceGeometry, anchor: (i32, i32)) -> Vec<(i32, i32)> {
  let (sx, sz) = match geometry.direction {
    GridDirection::PosZ | GridDirection::NegZ => (geometry.right, geometry.forward),
    GridDirection::PosX | GridDirection::NegX => (geometry.forward, geometry.right),
  };
  let (ax, az) = match geometry.direction {
    GridDirection::PosZ | GridDirection::NegX => (anchor.0, anchor.1),
    GridDirection::NegZ | GridDirection::PosX => (anchor.0 - sx + 1, anchor.1 - sz + 1),
  };
  let mut cells = Vec::with_capacity((sx * sz) as usize);
  for dx in 0..sx {
    for dz in 0..sz {
      cells.push((ax + dx, az + dz));
    }
  }
  cells
}

// =========================================================================
// 6. Appliance identity components
// =========================================================================

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub enum TableState {
  #[default]
  Empty,
  Occupied,
  Ordered,
  Served,
  Dirty,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub enum ChairState {
  #[default]
  Available,
  Reserved,
  Occupied,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub enum RegisterState {
  #[default]
  Idle,
  Checkout,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct BelongsToTable {
  pub table: Entity,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct SeatedAt {
  pub chair: Entity,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub enum StoveState {
  #[default]
  Idle,
  Cooking(f32),
}
