use crate::components::{
  ApplianceGeometry, ChairState, GridDirection, GridPosition, RegisterState, SlotOffset,
  SlotPosition, StoveState, TableState, EXIT_POSITION,
};
use crate::slots::{
  CustomerSitSlot, ExitSlot, Occupied, QueueSlot, StaffCheckoutSlot, StaffCookSlot,
  StaffDeliverSlot,
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

// =============================================================================
// Insert systems: compute SlotOffset + SlotPosition for each slot type
// =============================================================================

/// Insert SlotOffset + SlotPosition on StaffCookSlot children.
///
/// Math mirrors [`staff_cook_cell`] in slots.rs: offset is relative to the
/// stove's left-back anchor.
pub fn insert_cook_offset(
  mut commands: Commands,
  slots: Query<(Entity, &ChildOf), (Added<StaffCookSlot>, Without<SlotOffset>)>,
  parents: Query<(&GridPosition, &ApplianceGeometry)>,
) {
  for (entity, parent_handle) in slots.iter() {
    let Ok((parent_pos, geo)) = parents.get(parent_handle.parent()) else {
      continue;
    };
    let right = geo.right;
    let (dx, dz) = match geo.direction {
      GridDirection::PosZ => (right - 1, 1),
      GridDirection::NegZ => (0, -1),
      GridDirection::PosX => (1, 0),
      GridDirection::NegX => (-1, right - 1),
    };
    commands.entity(entity).insert((
      SlotOffset { dx, dz },
      SlotPosition {
        x: parent_pos.x + dx,
        z: parent_pos.z + dz,
      },
    ));
  }
}

/// Insert SlotOffset + SlotPosition on StaffDeliverSlot children.
///
/// Offset is one cell in the slot's facing direction from the table.
pub fn insert_deliver_offset(
  mut commands: Commands,
  slots: Query<
    (Entity, &StaffDeliverSlot, &ChildOf),
    (Added<StaffDeliverSlot>, Without<SlotOffset>),
  >,
  parents: Query<&GridPosition>,
) {
  for (entity, slot, parent_handle) in slots.iter() {
    let Ok(parent_pos) = parents.get(parent_handle.parent()) else {
      continue;
    };
    let (dx, dz) = slot.side.facing_offset();
    commands.entity(entity).insert((
      SlotOffset { dx, dz },
      SlotPosition {
        x: parent_pos.x + dx,
        z: parent_pos.z + dz,
      },
    ));
  }
}

/// Insert SlotOffset + SlotPosition on StaffCheckoutSlot children.
///
/// Math mirrors [`staff_checkout_cell`] in slots.rs.
pub fn insert_checkout_offset(
  mut commands: Commands,
  slots: Query<(Entity, &ChildOf), (Added<StaffCheckoutSlot>, Without<SlotOffset>)>,
  parents: Query<(&GridPosition, &ApplianceGeometry)>,
) {
  for (entity, parent_handle) in slots.iter() {
    let Ok((parent_pos, geo)) = parents.get(parent_handle.parent()) else {
      continue;
    };
    let (dx, dz) = match geo.direction {
      GridDirection::PosZ => (geo.right / 2, -1),
      GridDirection::NegZ => (geo.right / 2, 1),
      GridDirection::PosX => (-1, geo.right / 2),
      GridDirection::NegX => (1, geo.right / 2),
    };
    commands.entity(entity).insert((
      SlotOffset { dx, dz },
      SlotPosition {
        x: parent_pos.x + dx,
        z: parent_pos.z + dz,
      },
    ));
  }
}

/// Insert SlotOffset + SlotPosition on CustomerSitSlot children.
///
/// Customer sits on the chair cell itself, so relative offset is (0, 0).
pub fn insert_sit_offset(
  mut commands: Commands,
  slots: Query<(Entity, &ChildOf), (Added<CustomerSitSlot>, Without<SlotOffset>)>,
  parents: Query<&GridPosition>,
) {
  for (entity, parent_handle) in slots.iter() {
    let Ok(parent_pos) = parents.get(parent_handle.parent()) else {
      continue;
    };
    commands.entity(entity).insert((
      SlotOffset { dx: 0, dz: 0 },
      SlotPosition {
        x: parent_pos.x,
        z: parent_pos.z,
      },
    ));
  }
}

/// Insert SlotOffset + SlotPosition on newly added QueueSlot children.
///
/// Math mirrors [`queue_cell`] in slots.rs: 1-wide line in front of the register.
pub fn insert_queue_offset(
  mut commands: Commands,
  slots: Query<(Entity, &QueueSlot, &ChildOf), (Added<QueueSlot>, Without<SlotOffset>)>,
  parents: Query<(&GridPosition, &ApplianceGeometry)>,
) {
  for (entity, slot, parent_handle) in slots.iter() {
    let Ok((parent_pos, geo)) = parents.get(parent_handle.parent()) else {
      continue;
    };
    let (start_dx, start_dz) = match geo.direction {
      GridDirection::PosZ => (geo.right / 2, 1),
      GridDirection::NegZ => (geo.right / 2, -1),
      GridDirection::PosX => (1, geo.right / 2),
      GridDirection::NegX => (-1, geo.right / 2),
    };
    let (fdx, fdz) = match geo.direction {
      GridDirection::PosZ => (0, 1),
      GridDirection::NegZ => (0, -1),
      GridDirection::PosX => (1, 0),
      GridDirection::NegX => (-1, 0),
    };
    let start_x = parent_pos.x + start_dx;
    let start_z = parent_pos.z + start_dz;
    let idx = slot.index as i32;
    let px = start_x + fdx * idx;
    let pz = start_z + fdz * idx;
    commands.entity(entity).insert((
      SlotOffset {
        dx: px - parent_pos.x,
        dz: pz - parent_pos.z,
      },
      SlotPosition { x: px, z: pz },
    ));
  }
}

/// Recompute SlotOffset + SlotPosition when a QueueSlot's `index` changes.
///
/// Unlike the insert systems, this does NOT filter on `Without<SlotOffset>` so
/// it fires on every index change.
pub fn reindex_queue_slots(
  mut commands: Commands,
  slots: Query<(Entity, &QueueSlot, &ChildOf), Changed<QueueSlot>>,
  parents: Query<(&GridPosition, &ApplianceGeometry)>,
) {
  for (entity, slot, parent_handle) in slots.iter() {
    let Ok((parent_pos, geo)) = parents.get(parent_handle.parent()) else {
      continue;
    };
    let (start_dx, start_dz) = match geo.direction {
      GridDirection::PosZ => (geo.right / 2, 1),
      GridDirection::NegZ => (geo.right / 2, -1),
      GridDirection::PosX => (1, geo.right / 2),
      GridDirection::NegX => (-1, geo.right / 2),
    };
    let (fdx, fdz) = match geo.direction {
      GridDirection::PosZ => (0, 1),
      GridDirection::NegZ => (0, -1),
      GridDirection::PosX => (1, 0),
      GridDirection::NegX => (-1, 0),
    };
    let start_x = parent_pos.x + start_dx;
    let start_z = parent_pos.z + start_dz;
    let idx = slot.index as i32;
    let px = start_x + fdx * idx;
    let pz = start_z + fdz * idx;
    commands.entity(entity).insert((
      SlotOffset {
        dx: px - parent_pos.x,
        dz: pz - parent_pos.z,
      },
      SlotPosition { x: px, z: pz },
    ));
  }
}

/// Spawn the singleton exit slot at `EXIT_POSITION`.
pub fn spawn_exit_slot(mut commands: Commands) {
  commands.spawn((
    ExitSlot,
    SlotPosition {
      x: EXIT_POSITION.0,
      z: EXIT_POSITION.1,
    },
  ));
}

/// Maintain queue length: for each register, if the highest-index occupied
/// QueueSlot is at index N, ensure QueueSlot { index: N+1 } exists.
pub fn ensure_next_queue_slot(
  mut commands: Commands,
  registers: Query<Entity, With<RegisterState>>,
  all_slots: Query<(Entity, &ChildOf, &QueueSlot)>,
  occupied_set: Query<Entity, (With<QueueSlot>, With<Occupied>)>,
) {
  use std::collections::HashSet;

  let occupied: HashSet<Entity> = occupied_set.iter().collect();

  for reg_entity in registers.iter() {
    let mut max_occupied_idx: Option<usize> = None;
    let mut existing_idx: HashSet<usize> = HashSet::new();

    for (slot_entity, parent, qslot) in all_slots.iter() {
      if parent.parent() != reg_entity {
        continue;
      }
      existing_idx.insert(qslot.index);
      if occupied.contains(&slot_entity) {
        max_occupied_idx = Some(match max_occupied_idx {
          Some(m) => m.max(qslot.index),
          None => qslot.index,
        });
      }
    }

    if let Some(max_idx) = max_occupied_idx {
      let next_idx = max_idx + 1;
      if !existing_idx.contains(&next_idx) {
        commands.entity(reg_entity).with_children(|parent| {
          parent.spawn(QueueSlot { index: next_idx });
        });
      }
    }
  }
}

/// Shrink queue: maintain invariant `slots 0..=max_occupied` exist (or just
/// slot 0 if no one is queued). Despawn trailing empty slots.
pub fn shrink_queue_slots(
  mut commands: Commands,
  registers: Query<Entity, With<RegisterState>>,
  all_slots: Query<(Entity, &ChildOf, &QueueSlot)>,
  occupied_set: Query<Entity, (With<QueueSlot>, With<Occupied>)>,
) {
  use std::collections::HashSet;

  let occupied: HashSet<Entity> = occupied_set.iter().collect();

  for reg_entity in registers.iter() {
    let mut max_occupied: Option<usize> = None;
    for (slot_entity, parent, qslot) in all_slots.iter() {
      if parent.parent() != reg_entity {
        continue;
      }
      if occupied.contains(&slot_entity) {
        max_occupied = Some(match max_occupied {
          Some(m) => m.max(qslot.index),
          None => qslot.index,
        });
      }
    }
    let target_count = max_occupied.map_or(1, |m| m + 2);

    let to_remove: Vec<Entity> = all_slots
      .iter()
      .filter_map(|(entity, parent, qslot)| {
        if parent.parent() == reg_entity && qslot.index >= target_count {
          Some(entity)
        } else {
          None
        }
      })
      .collect();

    for entity in to_remove {
      commands.entity(entity).despawn();
    }
  }
}
