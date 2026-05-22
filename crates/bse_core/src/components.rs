use std::collections::VecDeque;

use bevy_ecs::prelude::*;

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

// =========================================================================
// 3. 新架构：物理形体 + 身份组件（无 ApplianceType 枚举）
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridRotation {
  North,
  East,
  South,
  West,
}

impl GridRotation {
  /// 90° 顺时针旋转
  pub fn rotate_cw(self) -> Self {
    match self {
      Self::North => Self::East,
      Self::East => Self::South,
      Self::South => Self::West,
      Self::West => Self::North,
    }
  }

  /// 返回旋转后的实际宽深（宽深交换逻辑）
  pub fn actual_dimensions(base_width: i32, base_depth: i32, rotation: Self) -> (i32, i32) {
    match rotation {
      Self::North | Self::South => (base_width, base_depth),
      Self::East | Self::West => (base_depth, base_width),
    }
  }
}

/// 纯物理形体：只描述占据哪些网格，不描述"是什么"
#[derive(Component, Debug, Clone, Copy)]
pub struct ApplianceGeometry {
  pub base_width: i32,
  pub base_depth: i32,
  pub rotation: GridRotation,
}

/// 基于 anchor（玩家点击的格子）+ geometry 计算出所有被占网格坐标。
///
/// 约定：N/E 向正方向 (+X,+Z) 展开，S/W 向负方向 (-X,-Z) 展开。
/// 这样 2×1 物体在 N/S 方向会占据不同的格子（向西 vs 向东延伸）。
pub fn get_footprint(geometry: &ApplianceGeometry, anchor: (i32, i32)) -> Vec<(i32, i32)> {
  let (w, d) = GridRotation::actual_dimensions(geometry.base_width, geometry.base_depth, geometry.rotation);
  let (start_x, start_z) = match geometry.rotation {
    GridRotation::North | GridRotation::East => (anchor.0, anchor.1),
    GridRotation::South | GridRotation::West => (anchor.0 - w + 1, anchor.1 - d + 1),
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
