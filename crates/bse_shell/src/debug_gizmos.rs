use bevy::prelude::*;
use bse_sim::components::{ApplianceGeometry, GridPosition, MovementProgress, PathQueue};
use crate::dev_console::DevConsoleState;

/// 在 DevConsoleState 的 anchor 位置画一个半透明绿色方块（XZ 平面）
pub fn draw_spawn_position_highlight_system(
    state: Res<DevConsoleState>,
    mut gizmos: Gizmos,
) {
    let x = state.anchor_x as f32;
    let z = state.anchor_z as f32;
    let half = 0.5;
    let corners = [
        Vec3::new(x - half, 0.15, z - half),
        Vec3::new(x + half, 0.15, z - half),
        Vec3::new(x + half, 0.15, z + half),
        Vec3::new(x - half, 0.15, z + half),
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
            Vec3::new(mov.to.0 as f32, 0.15, mov.to.1 as f32)
        } else {
            Vec3::new(grid_pos.x as f32, 0.15, grid_pos.z as f32)
        };
        points.push(start);

        for &(x, z) in path_queue.path.iter() {
            points.push(Vec3::new(x as f32, 0.15, z as f32));
        }

        if points.len() >= 2 {
            gizmos.linestrip(points, Color::srgba(0.0, 0.7, 1.0, 0.8));

            if let Some(&last_point) = path_queue.path.back() {
                let iso = Isometry3d::new(
                    Vec3::new(last_point.0 as f32, 0.15, last_point.1 as f32),
                    Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                );
                gizmos.circle(iso, 0.2, Color::srgba(1.0, 0.8, 0.0, 0.9));
            }
        }
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
  query: Query<(&GridPosition, &ApplianceGeometry)>,
  mut gizmos: Gizmos,
) {
  for (pos, geo) in query.iter() {
    let (dx, dz) = geo.direction.facing_offset();
    let origin = Vec3::new(pos.x as f32, 0.2, pos.z as f32);
    let tip = Vec3::new((pos.x + dx) as f32, 0.2, (pos.z + dz) as f32);
    gizmos.arrow(origin, tip, Color::srgba(0.0, 1.0, 1.0, 0.5));
  }
}
