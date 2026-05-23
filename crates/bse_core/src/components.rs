use std::collections::VecDeque;

use bevy::prelude::*;

/// World-grid position customers navigate to when leaving.
pub const EXIT_POSITION: (i32, i32) = (0, 0);

/// Marker component for the main isometric camera entity.
#[derive(Component)]
pub struct MainCamera;

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

  pub fn actual_dimensions(base_width: i32, base_depth: i32, dir: Self) -> (i32, i32) {
    match dir {
      Self::PosZ | Self::NegZ => (base_width, base_depth),
      Self::PosX | Self::NegX => (base_depth, base_width),
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
  pub base_width: i32,
  pub base_depth: i32,
  pub direction: GridDirection,
}

/// 基于 anchor（玩家点击的格子）+ geometry 计算出所有被占网格坐标。
///
/// 约定：N/E 向正方向 (+X,+Z) 展开，S/W 向负方向 (-X,-Z) 展开。
/// 这样 2×1 物体在 N/S 方向会占据不同的格子（向西 vs 向东延伸）。
pub fn get_footprint(geometry: &ApplianceGeometry, anchor: (i32, i32)) -> Vec<(i32, i32)> {
  let (w, d) = GridDirection::actual_dimensions(geometry.base_width, geometry.base_depth, geometry.direction);
  let (start_x, start_z) = match geometry.direction {
    GridDirection::PosZ | GridDirection::NegX => (anchor.0, anchor.1),
    GridDirection::NegZ | GridDirection::PosX => (anchor.0 - w + 1, anchor.1 - d + 1),
  };
  let mut cells = Vec::with_capacity((w * d) as usize);
  for dx in 0..w {
    for dz in 0..d {
      cells.push((start_x + dx, start_z + dz));
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
