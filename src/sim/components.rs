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
  pub y: i32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct SlotOffset {
  pub dx: i32,
  pub dz: i32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct SlotPosition {
  pub x: i32,
  pub y: i32,
}

/// Which layer this entity belongs to (placed by construction handlers).
pub use super::world::GridLayer;

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
pub struct NavigationComplete {
  pub failed: bool,
}

/// Agent is navigating toward this slot entity.
/// The movement system reads the slot's GridPosition each tick.
#[derive(Component, Debug, Clone, Copy)]
pub struct NavTarget {
  pub slot: Entity,
}

/// Local navigation state machine.
#[derive(Component, Debug, Default)]
pub enum NavState {
  #[default]
  Cruising,
  Blocked {
    ticks: u32,
  },
  FallbackBFS,
}

/// Marker: agent needs A* fallback from current position to NavTarget.
#[derive(Component, Debug)]
pub struct ReplanRequest;

// =========================================================================
// 3. Interaction
// =========================================================================

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
  WalkingToRegister,
  WaitingForPayment,
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
  WalkingToRegister,
  CheckingOut(f32),
}

#[derive(Component, Debug, Clone, Copy)]
pub struct StaffTarget {
  pub target_table: Entity,
}

/// Agent is navigating to or occupying a slot entity.
#[derive(Component, Debug, Clone, Copy)]
pub struct SlotTarget {
  pub slot: Entity,
}

// =========================================================================
// 5. Appliance geometry + footprint
// =========================================================================

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridDirection {
  PosX,
  PosY,
  NegX,
  NegY,
}

impl GridDirection {
  pub fn rotate_cw(self) -> Self {
    match self {
      Self::PosX => Self::NegY,
      Self::NegY => Self::NegX,
      Self::NegX => Self::PosY,
      Self::PosY => Self::PosX,
    }
  }

  pub fn rotate_ccw(self) -> Self {
    match self {
      Self::PosX => Self::PosY,
      Self::PosY => Self::NegX,
      Self::NegX => Self::NegY,
      Self::NegY => Self::PosX,
    }
  }

  pub fn facing_offset(self) -> (i32, i32) {
    match self {
      Self::PosX => (1, 0),
      Self::PosY => (0, 1),
      Self::NegX => (-1, 0),
      Self::NegY => (0, -1),
    }
  }

  pub fn to_bevy_quat(self) -> Quat {
    match self {
      Self::PosX => Quat::from_rotation_y(0.0),
      Self::PosY => Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
      Self::NegX => Quat::from_rotation_y(std::f32::consts::PI),
      Self::NegY => Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
    }
  }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct GridSize {
  pub right: i32,
  pub forward: i32,
}

/// Compute all grid cells occupied by an appliance.
///
/// `anchor` is the left-back corner of the appliance (from the facing
/// direction's perspective).  The appliance extends `right` cells to the
/// right (perpendicular to facing) and `forward` cells along the facing
/// direction.
pub fn get_footprint(
  size: &GridSize,
  direction: GridDirection,
  anchor: (i32, i32),
) -> Vec<(i32, i32)> {
  let (sx, sy) = match direction {
    GridDirection::PosX | GridDirection::NegX => (size.forward, size.right),
    GridDirection::PosY | GridDirection::NegY => (size.right, size.forward),
  };
  let (ax, ay) = match direction {
    GridDirection::PosX | GridDirection::NegY => (anchor.0, anchor.1),
    GridDirection::NegX | GridDirection::PosY => (anchor.0 - sx + 1, anchor.1 - sy + 1),
  };
  let mut cells = Vec::with_capacity((sx * sy) as usize);
  for dx in 0..sx {
    for dy in 0..sy {
      cells.push((ax + dx, ay + dy));
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
  pub sit_slot: Entity,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub enum StoveState {
  #[default]
  Idle,
  Cooking(f32),
}

#[derive(Component, Debug, Clone, Copy)]
pub struct ProcessingCustomer {
  pub customer: Entity,
}

/// Per-register queue of customers waiting to pay.
/// Front of Vec = first in line (closest to being served).
#[derive(Component, Debug, Clone, Default)]
pub struct RegisterQueue {
  pub customers: Vec<Entity>,
}
