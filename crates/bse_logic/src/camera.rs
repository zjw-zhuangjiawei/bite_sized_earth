use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bse_core::components::MainCamera;

#[derive(Resource)]
pub struct CameraControlConfig {
  pub move_speed: f32,
  pub zoom_speed: f32,
}

impl Default for CameraControlConfig {
  fn default() -> Self {
    Self {
      move_speed: 500.0,
      zoom_speed: 0.1,
    }
  }
}

pub fn camera_controller_system(
  time: Res<Time>,
  keyboard: Res<ButtonInput<KeyCode>>,
  mut scroll_events: MessageReader<MouseWheel>,
  config: Res<CameraControlConfig>,
  camera: Single<(&mut Projection, &mut Transform), With<MainCamera>>,
) {
  let (mut projection, mut transform) = camera.into_inner();

  let Projection::Orthographic(ortho) = &mut *projection else {
    return;
  };

  let mut input = Vec2::ZERO;
  if keyboard.pressed(KeyCode::ArrowUp) {
    input.y += 1.0;
  }
  if keyboard.pressed(KeyCode::ArrowDown) {
    input.y -= 1.0;
  }
  if keyboard.pressed(KeyCode::ArrowRight) {
    input.x += 1.0;
  }
  if keyboard.pressed(KeyCode::ArrowLeft) {
    input.x -= 1.0;
  }

  if input != Vec2::ZERO {
    input = input.normalize_or_zero();
    let rotation = Quat::from_rotation_y(45.0_f32.to_radians());
    let isometric_dir = rotation * Vec3::new(input.x, 0.0, -input.y);
    let delta = isometric_dir * config.move_speed * time.delta_secs() * ortho.scale;
    transform.translation += delta;
  }

  for event in scroll_events.read() {
    ortho.scale -= event.y * config.zoom_speed;
    ortho.scale = ortho.scale.clamp(0.02, 0.2);
  }
}

pub struct CameraControlPlugin;

impl Plugin for CameraControlPlugin {
  fn build(&self, app: &mut App) {
    app.insert_resource(CameraControlConfig::default());
    app.add_systems(Update, camera_controller_system);
  }
}
