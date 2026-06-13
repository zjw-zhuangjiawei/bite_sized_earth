use bevy::prelude::*;

pub fn setup_checkerboard(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
) {
  let tile_mesh = meshes.add(Cuboid::new(1.0, 0.2, 1.0));

  let color_a = materials.add(StandardMaterial::from(Color::Srgba(Srgba::new(
    0.5, 0.7, 0.3, 1.0,
  ))));
  let color_b = materials.add(StandardMaterial::from(Color::Srgba(Srgba::new(
    0.4, 0.6, 0.2, 1.0,
  ))));

  let grid_size = 32;

  for x in 0..grid_size {
    for z in 0..grid_size {
      let material = if (x + z) % 2 == 0 {
        color_a.clone()
      } else {
        color_b.clone()
      };

      commands.spawn((
        Mesh3d(tile_mesh.clone()),
        MeshMaterial3d(material),
        Transform::from_xyz(z as f32 + 0.5, -0.1, x as f32 + 0.5),
      ));
    }
  }
}

pub fn setup_lighting(mut commands: Commands) {
  commands.spawn((
    DirectionalLight {
      shadows_enabled: true,
      illuminance: 10000.0,
      ..default()
    },
    Transform::from_rotation(Quat::from_euler(
      EulerRot::XYZ,
      -std::f32::consts::FRAC_PI_4,
      std::f32::consts::FRAC_PI_4,
      0.0,
    )),
  ));
}
