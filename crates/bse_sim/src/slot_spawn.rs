use crate::components::{
  ChairState, GridDirection, GridPosition, RegisterState, StoveState, TableState,
};
use crate::slots::{
  CustomerSitSlot, QueueSlot, StaffCheckoutSlot, StaffCookSlot, StaffDeliverSlot,
};
use crate::world::GridLayers;
use bevy::prelude::*;

/// Spawn one StaffDeliverSlot per free adjacent cardinal cell around a table.
pub fn spawn_table_slots(
  mut commands: Commands,
  grid: Res<GridLayers>,
  query: Query<(Entity, &GridPosition), (Added<GridPosition>, With<TableState>)>,
) {
  for (entity, pos) in query.iter() {
    commands.entity(entity).with_children(|parent| {
      for side in [
        GridDirection::PosZ,
        GridDirection::NegX,
        GridDirection::NegZ,
        GridDirection::PosX,
      ] {
        let (dx, dz) = side.facing_offset();
        let cx = pos.x + dx;
        let cz = pos.z + dz;
        if cx >= 0 && cx < grid.width && cz >= 0 && cz < grid.height {
          parent.spawn(StaffDeliverSlot { side });
        }
      }
    });
  }
}

/// Spawn one StaffCookSlot per stove.
pub fn spawn_stove_slots(
  mut commands: Commands,
  query: Query<Entity, (Added<GridPosition>, With<StoveState>)>,
) {
  for entity in query.iter() {
    commands.entity(entity).with_children(|parent| {
      parent.spawn(StaffCookSlot);
    });
  }
}

/// Spawn one StaffCheckoutSlot per register.
pub fn spawn_register_slots(
  mut commands: Commands,
  query: Query<Entity, (Added<GridPosition>, With<RegisterState>)>,
) {
  for entity in query.iter() {
    commands.entity(entity).with_children(|parent| {
      parent.spawn(StaffCheckoutSlot);
    });
  }
}

/// Spawn initial QueueSlot { index: 0 } on each register.
pub fn spawn_initial_queue_slots(
  mut commands: Commands,
  query: Query<Entity, (Added<GridPosition>, With<RegisterState>)>,
) {
  for entity in query.iter() {
    commands.entity(entity).with_children(|parent| {
      parent.spawn(QueueSlot { index: 0 });
    });
  }
}

/// Spawn one CustomerSitSlot per chair.
pub fn spawn_chair_slots(
  mut commands: Commands,
  query: Query<Entity, (Added<GridPosition>, With<ChairState>)>,
) {
  for entity in query.iter() {
    commands.entity(entity).with_children(|parent| {
      parent.spawn(CustomerSitSlot);
    });
  }
}
