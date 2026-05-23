use std::collections::VecDeque;

use bevy::prelude::*;

/// World-grid position customers navigate to when leaving.
pub const EXIT_POSITION: (i32, i32) = (0, 0);


// =========================================================================
// 1. 通用"肉体与运动"套件 — 所有人共有，不带任何身份倾向
// =========================================================================

#[derive(Component, Debug, Clone, Copy)]
pub struct GridPosition {
  pub x: i32,
  pub z: i32,
}

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

/// 正向到达标记：agent_movement_tick 在路径走完后插入，业务系统消费后手动移除。
/// 替换 Without<MovementProgress> 反模式。
#[derive(Component, Debug, Clone, Copy)]
pub struct NavigationComplete;

// =========================================================================
// 2. 身份 + 状态融合组件 — 类型系统保证 identity-state 共存
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
// 3. 新架构：物理形体 + 身份组件（无 ApplianceType 枚举）
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

/// 纯物理形体：只描述占据哪些网格，不描述"是什么"
#[derive(Component, Debug, Clone, Copy)]
pub struct ApplianceGeometry {
  /// Cells extending to the right of the facing direction (perpendicular axis)
  pub right: i32,
  /// Cells extending forward along the facing direction
  pub forward: i32,
  pub direction: GridDirection,
}

/// Compute all grid cells occupied by an appliance.
///
/// `anchor` is the left-back corner of the appliance (from the facing direction's perspective).
/// The appliance extends `right` cells to the right (perpendicular to facing) and
/// `forward` cells along the facing direction.
///
/// Mapping rules:
/// - PosZ: right=+X, forward=+Z
/// - NegZ: right=-X, forward=-Z
/// - PosX: right=-Z, forward=+X
/// - NegX: right=+Z, forward=-X
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

// 身份组件：挂载了哪个组件，ECS 世界就认为它是"什么"

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub enum TableState {
  #[default]
  Empty,
  Occupied,    // customer group seated
  Ordered,     // order placed, awaiting cooking
  Served,      // food delivered, customers eating
  Dirty,       // finished, awaiting cleanup
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

/// 顾客入座后挂载，存储其占用的椅子 Entity。消除所有位置匹配查找。
/// 通过椅子上的 BelongsToTable 间接找到桌子。
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
