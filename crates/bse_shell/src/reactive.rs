use bevy::prelude::*;
use bse_sim::components::{
  ApplianceGeometry, ChairState, Customer, GridDirection, GridPosition, MovementProgress,
  RegisterState, Staff, StoveState, TableState,
};

pub fn render_tables(
  mut commands: Commands,
  assets: Res<AssetServer>,
  query: Query<
    (Entity, &GridPosition, &ApplianceGeometry),
    (Added<ApplianceGeometry>, With<TableState>),
  >,
) {
  for (entity, pos, geo) in query.iter() {
    let rotation_y = match geo.direction {
      GridDirection::PosZ => 0.0,
      GridDirection::NegX => -std::f32::consts::FRAC_PI_2,
      GridDirection::NegZ => std::f32::consts::PI,
      GridDirection::PosX => std::f32::consts::FRAC_PI_2,
    };
    let scale = 1.0 / 16.0;
    commands.entity(entity).insert((
      SceneRoot(assets.load("table.vox")),
      Transform::from_xyz(pos.x as f32, 0.0, pos.z as f32)
        .with_scale(Vec3::splat(scale))
        .with_rotation(Quat::from_rotation_y(rotation_y)),
    ));
  }
}

pub fn render_chairs(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  query: Query<
    (Entity, &GridPosition, &ApplianceGeometry),
    (Added<ApplianceGeometry>, With<ChairState>, Without<Mesh3d>),
  >,
) {
  for (entity, pos, geo) in query.iter() {
    let rotation_y = match geo.direction {
      GridDirection::PosZ => 0.0,
      GridDirection::NegX => -std::f32::consts::FRAC_PI_2,
      GridDirection::NegZ => std::f32::consts::PI,
      GridDirection::PosX => std::f32::consts::FRAC_PI_2,
    };

    commands.entity(entity).insert((
      Transform::from_xyz(pos.x as f32, 0.0, pos.z as f32)
        .with_rotation(Quat::from_rotation_y(rotation_y)),
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
    (Entity, &GridPosition, &ApplianceGeometry),
    (
      Added<ApplianceGeometry>,
      With<RegisterState>,
      Without<Mesh3d>,
    ),
  >,
) {
  for (entity, pos, geo) in query.iter() {
    let rotation_y = match geo.direction {
      GridDirection::PosZ => 0.0,
      GridDirection::NegX => -std::f32::consts::FRAC_PI_2,
      GridDirection::NegZ => std::f32::consts::PI,
      GridDirection::PosX => std::f32::consts::FRAC_PI_2,
    };
    // 2x1 footprint: center vox over long axis. right=2 along appliance's
    // local "right" → world axis depends on direction.
    let (sx, sz) = match geo.direction {
      GridDirection::PosZ | GridDirection::NegZ => (geo.right, geo.forward),
      GridDirection::PosX | GridDirection::NegX => (geo.forward, geo.right),
    };
    let offset_sign = match geo.direction {
      GridDirection::PosZ | GridDirection::NegX => 1.0,
      GridDirection::NegZ | GridDirection::PosX => -1.0,
    };
    let world_x = pos.x as f32 + offset_sign * (sx as f32 - 1.0) / 2.0;
    let world_z = pos.z as f32 + offset_sign * (sz as f32 - 1.0) / 2.0;
    // register.vox is 64x32x32 (2:1:1) → fits 2x1 footprint uniformly.
    // bevy_vox_scene remap: bevy_x=vox_x (mirrored), bevy_y=vox_z (up), bevy_z=vox_y (depth).
    let scale = 1.0 / 32.0;
    commands.entity(entity).insert((
      SceneRoot(assets.load("register.vox")),
      Transform::from_xyz(world_x, 0.0, world_z)
        .with_scale(Vec3::splat(scale))
        .with_rotation(Quat::from_rotation_y(rotation_y)),
    ));
  }
}

pub fn render_stoves(
  mut commands: Commands,
  assets: Res<AssetServer>,
  query: Query<
    (Entity, &GridPosition, &ApplianceGeometry),
    (Added<ApplianceGeometry>, With<StoveState>, Without<Mesh3d>),
  >,
) {
  for (entity, pos, geo) in query.iter() {
    let rotation_y = match geo.direction {
      GridDirection::PosZ => 0.0,
      GridDirection::NegX => -std::f32::consts::FRAC_PI_2,
      GridDirection::NegZ => std::f32::consts::PI,
      GridDirection::PosX => std::f32::consts::FRAC_PI_2,
    };
    // 2x1 footprint: center vox over long axis.
    let (sx, sz) = match geo.direction {
      GridDirection::PosZ | GridDirection::NegZ => (geo.right, geo.forward),
      GridDirection::PosX | GridDirection::NegX => (geo.forward, geo.right),
    };
    let offset_sign = match geo.direction {
      GridDirection::PosZ | GridDirection::NegX => 1.0,
      GridDirection::NegZ | GridDirection::PosX => -1.0,
    };
    let world_x = pos.x as f32 + offset_sign * (sx as f32 - 1.0) / 2.0;
    let world_z = pos.z as f32 + offset_sign * (sz as f32 - 1.0) / 2.0;
    // stove.vox is 64x32x32 (2:1:1) → fits 2x1 footprint uniformly.
    let scale = 1.0 / 32.0;
    commands.entity(entity).insert((
      SceneRoot(assets.load("stove.vox")),
      Transform::from_xyz(world_x, 0.0, world_z)
        .with_scale(Vec3::splat(scale))
        .with_rotation(Quat::from_rotation_y(rotation_y)),
    ));
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
      Transform::from_xyz(pos.x as f32, 0.65, pos.z as f32),
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
      Transform::from_xyz(pos.x as f32, 0.65, pos.z as f32),
    ));
  }
}

/// Reads logic-layer position ([`GridPosition`] + [`MovementProgress`]) and
/// writes the visual [`Transform`].  Keeps visual interpolation out of the
/// logic crate.
pub fn sync_agent_transform(mut query: Query<(&GridPosition, &MovementProgress, &mut Transform)>) {
  for (_grid_pos, movement, mut transform) in query.iter_mut() {
    let fx =
      movement.from.0 as f32 + (movement.to.0 as f32 - movement.from.0 as f32) * movement.progress;
    let fz =
      movement.from.1 as f32 + (movement.to.1 as f32 - movement.from.1 as f32) * movement.progress;
    transform.translation.x = fx;
    transform.translation.z = fz;
  }
}
