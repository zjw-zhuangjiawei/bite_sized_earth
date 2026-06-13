use super::dev_console::DevConsoleState;
use crate::sim::components::{GridDirection, GridPosition, GridSize, MovementProgress, PathQueue};
use crate::sim::slots::{
  CustomerSitSlot, Occupied, QueueSlot, StaffCheckoutSlot, StaffCookSlot, StaffDeliverSlot,
  customer_sit_cell, queue_cell, staff_checkout_cell, staff_cook_cell, staff_deliver_cell,
};
use bevy::prelude::*;

/// 在 DevConsoleState 的 anchor 位置画一个半透明绿色方块（XZ 平面）
pub fn draw_spawn_position_highlight_system(state: Res<DevConsoleState>, mut gizmos: Gizmos) {
  let cx = state.anchor_x as f32 + 0.5;
  let cy = state.anchor_y as f32 + 0.5;
  let half = 0.5;
  let corners = [
    Vec3::new(cy - half, 0.15, cx - half),
    Vec3::new(cy + half, 0.15, cx - half),
    Vec3::new(cy + half, 0.15, cx + half),
    Vec3::new(cy - half, 0.15, cx + half),
  ];
  gizmos.lineloop(corners, Color::srgba(0.0, 1.0, 0.0, 0.4));
}

/// 遍历全场 PathQueue，画冷光蓝路径折线 + 终点黄圈
pub fn draw_agent_path_preview_system(
  query: Query<(&GridPosition, &PathQueue, Option<&MovementProgress>)>,
  mut gizmos: Gizmos,
) {
  for (grid_pos, path_queue, movement_opt) in query.iter() {
    let mut points = Vec::new();

    let start = if let Some(mov) = movement_opt {
      Vec3::new(mov.to.1 as f32 + 0.5, 0.15, mov.to.0 as f32 + 0.5)
    } else {
      Vec3::new(grid_pos.y as f32 + 0.5, 0.15, grid_pos.x as f32 + 0.5)
    };
    points.push(start);

    for &(x, y) in path_queue.path.iter() {
      points.push(Vec3::new(y as f32 + 0.5, 0.15, x as f32 + 0.5));
    }

    if points.len() >= 2 {
      gizmos.linestrip(points, Color::srgba(0.0, 0.7, 1.0, 0.8));

      if let Some(&last_point) = path_queue.path.back() {
        let iso = Isometry3d::new(
          Vec3::new(last_point.1 as f32 + 0.5, 0.15, last_point.0 as f32 + 0.5),
          Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        );
        gizmos.circle(iso, 0.2, Color::srgba(1.0, 0.8, 0.0, 0.9));
      }
    }
  }
}

/// Faint world grid overlay at y=0.01. Lines at every integer X and Z.
/// Red emphasis line marks bevy X axis (grid.y = 0); green emphasis line marks
/// bevy Z axis (grid.x = 0).
pub fn draw_world_grid_overlay(mut gizmos: Gizmos) {
  const GRID_SIZE: i32 = 32;
  const Y: f32 = 0.01;
  let faint = Color::srgba(0.5, 0.5, 0.5, 0.25);
  let bevy_x_color = Color::srgba(1.0, 0.3, 0.3, 0.6);
  let bevy_z_color = Color::srgba(0.3, 1.0, 0.3, 0.6);

  for i in 0..=GRID_SIZE {
    let fi = i as f32;
    let color_x = if i == 0 { bevy_x_color } else { faint };
    gizmos.line(
      Vec3::new(fi, Y, 0.0),
      Vec3::new(fi, Y, GRID_SIZE as f32),
      color_x,
    );
    let color_z = if i == 0 { bevy_z_color } else { faint };
    gizmos.line(
      Vec3::new(0.0, Y, fi),
      Vec3::new(GRID_SIZE as f32, Y, fi),
      color_z,
    );
  }
}

/// Red (X) and blue (Z) axis arrows at the origin.
pub fn draw_world_axes(mut gizmos: Gizmos) {
  gizmos.arrow(
    Vec3::new(0.0, 0.1, 0.0),
    Vec3::new(2.0, 0.1, 0.0),
    Color::srgba(1.0, 0.0, 0.0, 0.8),
  );
  gizmos.arrow(
    Vec3::new(0.0, 0.1, 0.0),
    Vec3::new(0.0, 0.1, 2.0),
    Color::srgba(0.0, 0.0, 1.0, 0.8),
  );
}

/// Small cyan arrows showing each appliance's facing direction.
pub fn draw_appliance_direction_gizmos(
  query: Query<(&GridPosition, &GridSize, &GridDirection)>,
  mut gizmos: Gizmos,
) {
  for (pos, _size, direction) in query.iter() {
    let (dx, dy) = direction.facing_offset();
    let origin = Vec3::new(pos.y as f32 + 0.5, 0.2, pos.x as f32 + 0.5);
    let tip = Vec3::new((pos.y + dy) as f32 + 0.5, 0.2, (pos.x + dx) as f32 + 0.5);
    gizmos.arrow(origin, tip, Color::srgba(0.0, 1.0, 1.0, 0.5));
  }
}

/// Render slot cells with green (free) / red (occupied) color coding.
pub fn draw_slot_gizmos(
  slots: Query<(
    Option<&StaffCookSlot>,
    Option<&StaffDeliverSlot>,
    Option<&StaffCheckoutSlot>,
    Option<&CustomerSitSlot>,
    Option<&QueueSlot>,
    Option<&Occupied>,
    &ChildOf,
  )>,
  appliances: Query<(&GridPosition, &GridSize, &GridDirection)>,
  mut gizmos: Gizmos,
) {
  for (cook, deliver, checkout, sit, queue, occupied, child_of) in slots.iter() {
    let parent = child_of.parent();
    let cell: Option<(i32, i32)> = if let Ok((pos, size, direction)) = appliances.get(parent) {
      if cook.is_some() {
        Some(staff_cook_cell((pos.x, pos.y), size, *direction))
      } else if deliver.is_some() {
        Some(staff_deliver_cell((pos.x, pos.y), deliver.unwrap().side))
      } else if checkout.is_some() {
        Some(staff_checkout_cell((pos.x, pos.y), size, *direction))
      } else if sit.is_some() {
        Some(customer_sit_cell((pos.x, pos.y)))
      } else if let Some(qslot) = queue {
        Some(queue_cell((pos.x, pos.y), size, *direction, qslot.index))
      } else {
        None
      }
    } else {
      None
    };

    if let Some((cx, cy)) = cell {
      let half = 0.35;
      let cx = cx as f32 + 0.5;
      let cy = cy as f32 + 0.5;
      let corners = [
        Vec3::new(cy - half, 0.13, cx - half),
        Vec3::new(cy + half, 0.13, cx - half),
        Vec3::new(cy + half, 0.13, cx + half),
        Vec3::new(cy - half, 0.13, cx + half),
      ];
      let color = if occupied.is_some() {
        Color::srgba(1.0, 0.2, 0.2, 0.6)
      } else {
        Color::srgba(0.2, 1.0, 0.4, 0.6)
      };
      gizmos.lineloop(corners, color);
    }
  }
}
