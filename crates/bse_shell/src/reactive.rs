use bevy::prelude::*;
use bse_core::components::{
  ApplianceGeometry, ChairState, Customer, GridDirection, GridPosition, MovementProgress,
  RegisterState, Staff, StoveState, TableState,
};

fn spawn_appliance_mesh(
  commands: &mut Commands,
  meshes: &mut ResMut<Assets<Mesh>>,
  materials: &mut ResMut<Assets<StandardMaterial>>,
  entity: Entity,
  pos: &GridPosition,
  geo: &ApplianceGeometry,
  shape: Cuboid,
  color: Srgba,
  y_offset: f32,
) {
  let rotation_y = match geo.direction {
    GridDirection::PosZ => 0.0,
    GridDirection::NegX => -std::f32::consts::FRAC_PI_2,
    GridDirection::NegZ => std::f32::consts::PI,
    GridDirection::PosX => std::f32::consts::FRAC_PI_2,
  };
  let (actual_w, actual_d) =
    GridDirection::actual_dimensions(geo.base_width, geo.base_depth, geo.direction);
  let offset_sign = match geo.direction {
    GridDirection::PosZ | GridDirection::NegX => 1.0,
    GridDirection::NegZ | GridDirection::PosX => -1.0,
  };
  let world_x = pos.x as f32 + offset_sign * (actual_w as f32 - 1.0) / 2.0;
  let world_z = pos.z as f32 + offset_sign * (actual_d as f32 - 1.0) / 2.0;

  commands.entity(entity).insert((
    Mesh3d(meshes.add(shape)),
    MeshMaterial3d(materials.add(StandardMaterial {
      base_color: Color::Srgba(color),
      ..default()
    })),
    Transform::from_xyz(world_x, y_offset, world_z)
      .with_rotation(Quat::from_rotation_y(rotation_y)),
  ));
}

pub fn render_tables(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  query: Query<
    (Entity, &GridPosition, &ApplianceGeometry),
    (Added<ApplianceGeometry>, With<TableState>, Without<Mesh3d>),
  >,
) {
  for (entity, pos, geo) in query.iter() {
    spawn_appliance_mesh(
      &mut commands,
      &mut meshes,
      &mut materials,
      entity,
      pos,
      geo,
      Cuboid::new(geo.base_width as f32, 0.6, geo.base_depth as f32),
      Srgba::new(0.8, 0.6, 0.4, 1.0),
      0.3,
    );
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
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  query: Query<
    (Entity, &GridPosition, &ApplianceGeometry),
    (Added<ApplianceGeometry>, With<RegisterState>, Without<Mesh3d>),
  >,
) {
  for (entity, pos, geo) in query.iter() {
    spawn_appliance_mesh(
      &mut commands,
      &mut meshes,
      &mut materials,
      entity,
      pos,
      geo,
      Cuboid::new(geo.base_width as f32, 0.8, geo.base_depth as f32),
      Srgba::new(0.4, 0.2, 0.1, 1.0),
      0.4,
    );
  }
}

pub fn render_stoves(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  query: Query<
    (Entity, &GridPosition, &ApplianceGeometry),
    (Added<ApplianceGeometry>, With<StoveState>, Without<Mesh3d>),
  >,
) {
  for (entity, pos, geo) in query.iter() {
    spawn_appliance_mesh(
      &mut commands,
      &mut meshes,
      &mut materials,
      entity,
      pos,
      geo,
      Cuboid::new(geo.base_width as f32, 0.4, geo.base_depth as f32),
      Srgba::new(0.15, 0.15, 0.15, 1.0),
      0.2,
    );
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
pub fn sync_agent_transform(
  mut query: Query<(&GridPosition, &MovementProgress, &mut Transform)>,
) {
  for (_grid_pos, movement, mut transform) in query.iter_mut() {
    let fx = movement.from.0 as f32
      + (movement.to.0 as f32 - movement.from.0 as f32) * movement.progress;
    let fz = movement.from.1 as f32
      + (movement.to.1 as f32 - movement.from.1 as f32) * movement.progress;
    transform.translation.x = fx;
    transform.translation.z = fz;
  }
}
