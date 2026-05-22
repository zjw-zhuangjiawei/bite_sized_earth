use bevy::{camera::ScalingMode, prelude::*};
use bse_core::components::MainCamera;

pub fn setup_camera(mut commands: Commands) {
  let angle_y = std::f32::consts::FRAC_PI_4; // 45°
  let angle_x = -(1.0 / 3.0_f32.sqrt()).asin(); // 35.264°
  let isometric_rotation = Quat::from_rotation_y(angle_y) * Quat::from_rotation_x(angle_x);

  let grid_center = Vec3::new(15.5, 0.0, 15.5);
  let camera_back_dir = isometric_rotation * Vec3::Z;
  let camera_position = grid_center + camera_back_dir * 50.0;

  let camera_transform =
    Transform::from_translation(camera_position).with_rotation(isometric_rotation);

  commands.spawn((
    MainCamera,
    Camera3d::default(),
    Projection::Orthographic(OrthographicProjection {
      scale: 0.08,
      scaling_mode: ScalingMode::WindowSize,
      ..OrthographicProjection::default_3d()
    }),
    camera_transform,
  ));
}
