use super::components::{GridDirection, GridSize};
use bevy::prelude::*;

// === Slot identity markers ===

#[derive(Component)]
pub struct StaffCookSlot;

#[derive(Component)]
pub struct StaffDeliverSlot {
  pub side: GridDirection,
}

#[derive(Component)]
pub struct StaffCheckoutSlot;

#[derive(Component)]
pub struct CustomerSitSlot;

#[derive(Component)]
pub struct QueueSlot {
  pub index: usize,
}

/// A fixed-position slot at the exit point.  Customers navigate here when leaving.
#[derive(Component)]
pub struct ExitSlot;

// === Slot state ===

/// Slot is currently occupied by an agent. Absent = free.
#[derive(Component)]
pub struct Occupied {
  pub by: Entity,
}

// === Position helpers ===

/// Staff cook position: front-left cell of stove.
pub fn staff_cook_cell(
  parent_pos: (i32, i32),
  size: &GridSize,
  direction: GridDirection,
) -> (i32, i32) {
  let right = size.right as i32;
  let (dx, dy) = match direction {
    GridDirection::PosY => (right - 1, 1),
    GridDirection::NegY => (0, -1),
    GridDirection::PosX => (1, 0),
    GridDirection::NegX => (-1, right - 1),
  };
  (parent_pos.0 + dx, parent_pos.1 + dy)
}

/// Staff deliver position: one cell in the given direction from table position.
pub fn staff_deliver_cell(table_pos: (i32, i32), side: GridDirection) -> (i32, i32) {
  let (dx, dy) = side.facing_offset();
  (table_pos.0 + dx, table_pos.1 + dy)
}

/// Staff checkout position: behind register center.
pub fn staff_checkout_cell(
  reg_pos: (i32, i32),
  size: &GridSize,
  direction: GridDirection,
) -> (i32, i32) {
  let (dx, dy) = match direction {
    GridDirection::PosY => (size.right / 2, -1),
    GridDirection::NegY => (size.right / 2, 1),
    GridDirection::PosX => (-1, size.right / 2),
    GridDirection::NegX => (1, size.right / 2),
  };
  (reg_pos.0 + dx, reg_pos.1 + dy)
}

/// Customer sit position: chair's own cell.
pub fn customer_sit_cell(chair_pos: (i32, i32)) -> (i32, i32) {
  (chair_pos.0, chair_pos.1)
}

/// Queue cell: 1-wide line in front of register, aligned laterally with the
/// staff checkout cell. Extends forward from the front of the register.
pub fn queue_cell(
  reg_pos: (i32, i32),
  size: &GridSize,
  direction: GridDirection,
  index: usize,
) -> (i32, i32) {
  let (start_dx, start_dy) = match direction {
    GridDirection::PosY => (size.right / 2, 1),
    GridDirection::NegY => (size.right / 2, -1),
    GridDirection::PosX => (1, size.right / 2),
    GridDirection::NegX => (-1, size.right / 2),
  };
  let (fdx, fdy) = direction.facing_offset();
  let start_x = reg_pos.0 + start_dx;
  let start_y = reg_pos.1 + start_dy;
  let idx = index as i32;
  (start_x + fdx * idx, start_y + fdy * idx)
}
