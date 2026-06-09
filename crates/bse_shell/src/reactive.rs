use bevy::prelude::*;
use bse_model::{ModelAsset, ModelAssetHandle};
use bse_sim::components::{
  ChairState, Customer, GridDirection, GridPosition, GridSize, MovementProgress, RegisterState,
  Staff, StoveState, TableState,
};

pub fn render_tables(
  mut commands: Commands,
  assets: Res<AssetServer>,
  query: Query<
    (Entity, &GridPosition, &GridSize, &GridDirection),
    (Added<GridSize>, With<TableState>),
  >,
) {
  for (entity, pos, _size, direction) in query.iter() {
    let scale = 1.0 / 16.0;
    commands.entity(entity).insert((
      ModelAssetHandle(assets.load("models/table.ron")),
      Transform::from_xyz(pos.y as f32, 0.0, pos.x as f32)
        .with_scale(Vec3::splat(scale))
        .with_rotation(direction.to_bevy_quat()),
    ));
  }
}

pub fn render_chairs(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  query: Query<
    (Entity, &GridPosition, &GridSize, &GridDirection),
    (Added<GridSize>, With<ChairState>, Without<Mesh3d>),
  >,
) {
  for (entity, pos, _size, direction) in query.iter() {
    commands.entity(entity).insert((
      Transform::from_xyz(pos.y as f32, 0.0, pos.x as f32).with_rotation(direction.to_bevy_quat()),
      Visibility::default(),
    ));

    commands.entity(entity).with_children(|parent| {
      // Lower step (wider base)
      parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.40, 0.25, 0.40))),
        MeshMaterial3d(materials.add(StandardMaterial {
          base_color: Color::Srgba(Srgba::new(0.35, 0.35, 0.35, 1.0)),
          ..default()
        })),
        Transform::from_xyz(0.0, 0.125, 0.0),
      ));
      // Upper step (narrower seat)
      parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.30, 0.15, 0.30))),
        MeshMaterial3d(materials.add(StandardMaterial {
          base_color: Color::Srgba(Srgba::new(0.25, 0.25, 0.25, 1.0)),
          ..default()
        })),
        Transform::from_xyz(0.0, 0.325, 0.0),
      ));
    });
  }
}

pub fn render_registers(
  mut commands: Commands,
  assets: Res<AssetServer>,
  query: Query<
    (Entity, &GridPosition, &GridSize, &GridDirection),
    (Added<GridSize>, With<RegisterState>, Without<Mesh3d>),
  >,
) {
  for (entity, pos, _size, direction) in query.iter() {
    let scale = 1.0 / 32.0;
    commands.entity(entity).insert((
      ModelAssetHandle(assets.load("models/register.ron")),
      Transform::from_xyz(pos.y as f32, 0.0, pos.x as f32)
        .with_scale(Vec3::splat(scale))
        .with_rotation(direction.to_bevy_quat()),
    ));
  }
}

pub fn render_stoves(
  mut commands: Commands,
  assets: Res<AssetServer>,
  query: Query<
    (Entity, &GridPosition, &GridSize, &GridDirection),
    (Added<GridSize>, With<StoveState>, Without<Mesh3d>),
  >,
) {
  for (entity, pos, _size, direction) in query.iter() {
    let scale = 1.0 / 32.0;
    commands.entity(entity).insert((
      ModelAssetHandle(assets.load("models/stove.ron")),
      Transform::from_xyz(pos.y as f32, 0.0, pos.x as f32)
        .with_scale(Vec3::splat(scale))
        .with_rotation(direction.to_bevy_quat()),
    ));
  }
}

/// For every entity with a [`ModelAssetHandle`] but no `Mesh3d` yet, look up
/// the loaded `ModelAsset` and insert the bevy mesh + material handles.
///
/// Mirrors `vox::attach_voxel_mesh`'s `Without<Mesh3d>` re-match pattern
/// because `AssetServer::load` is async — the first `Added` tick almost
/// always fires before the asset is ready.
pub fn attach_model_mesh(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  model_assets: Res<Assets<ModelAsset>>,
  query: Query<(Entity, &ModelAssetHandle), (With<ModelAssetHandle>, Without<Mesh3d>)>,
) {
  for (entity, handle) in &query {
    let Some(asset) = model_assets.get(&handle.0) else {
      continue;
    };
    let mesh_handle = meshes.add(asset.mesh.clone());
    let mat_handle = asset.materials.first().cloned().unwrap_or_default();
    commands
      .entity(entity)
      .insert((Mesh3d(mesh_handle), MeshMaterial3d(mat_handle)));
  }
}

// =========================================================================
// 演员渲染（Staff / Customer，保持不变）
// =========================================================================

pub fn render_new_staff(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  query: Query<(Entity, &GridPosition), (Added<Staff>, Without<Mesh3d>)>,
) {
  for (entity, pos) in query.iter() {
    commands.entity(entity).insert((
      Mesh3d(meshes.add(Capsule3d::new(0.25, 0.8))),
      MeshMaterial3d(materials.add(StandardMaterial {
        base_color: Color::Srgba(Srgba::new(0.2, 0.4, 0.8, 1.0)),
        ..default()
      })),
      Transform::from_xyz(pos.y as f32, 0.65, pos.x as f32),
    ));
  }
}

pub fn render_new_customers(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  query: Query<(Entity, &GridPosition), (Added<Customer>, Without<Mesh3d>)>,
) {
  for (entity, pos) in query.iter() {
    commands.entity(entity).insert((
      Mesh3d(meshes.add(Capsule3d::new(0.25, 0.8))),
      MeshMaterial3d(materials.add(StandardMaterial {
        base_color: Color::Srgba(Srgba::new(0.8, 0.2, 0.2, 1.0)),
        ..default()
      })),
      Transform::from_xyz(pos.y as f32, 0.65, pos.x as f32),
    ));
  }
}

pub fn sync_agent_transform(mut query: Query<(&GridPosition, &MovementProgress, &mut Transform)>) {
  for (_grid_pos, movement, mut transform) in query.iter_mut() {
    let f_grid_x =
      movement.from.0 as f32 + (movement.to.0 as f32 - movement.from.0 as f32) * movement.progress;
    let f_grid_y =
      movement.from.1 as f32 + (movement.to.1 as f32 - movement.from.1 as f32) * movement.progress;
    transform.translation.x = f_grid_y;
    transform.translation.z = f_grid_x;
  }
}
