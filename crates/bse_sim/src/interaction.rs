use std::collections::HashSet;

use bevy::prelude::*;
use smallvec::SmallVec;

use crate::components::{
  ApplianceGeometry, GridFootprint, GridPosition, InteractionPoints, InteractionRule,
};
use crate::messages::GridChangedMessage;
use crate::world::GridLayers;

/// Candidate interaction cells in front of an appliance, one per width-column.
pub fn compute_front_cells(
  pos: &GridPosition,
  geo: &ApplianceGeometry,
  bounds: (i32, i32),
) -> SmallVec<[(i32, i32); 8]> {
  let mut out = SmallVec::new();
  let (bw, bh) = bounds;
  let offset = geo.direction.facing_offset();
  let width = geo.right;
  for w in 0..width {
    let (dx, dz) = match geo.direction {
      crate::components::GridDirection::PosZ | crate::components::GridDirection::NegZ => (w, 0),
      crate::components::GridDirection::PosX | crate::components::GridDirection::NegX => (0, w),
    };
    let cx = pos.x + dx + offset.0;
    let cz = pos.z + dz + offset.1;
    if cx >= 0 && cx < bw && cz >= 0 && cz < bh {
      out.push((cx, cz));
    }
  }
  out
}

/// Candidate interaction cells within `range` Manhattan distance of the
/// footprint, excluding the footprint cells themselves.
pub fn compute_adjacent_cells(
  footprint: &GridFootprint,
  range: u32,
  bounds: (i32, i32),
) -> SmallVec<[(i32, i32); 8]> {
  let mut out = SmallVec::new();
  let (bw, bh) = bounds;
  let footprint_set: HashSet<(i32, i32)> = footprint.cells.iter().copied().collect();

  let min_x = footprint.cells.iter().map(|c| c.0).min().unwrap_or(0);
  let max_x = footprint.cells.iter().map(|c| c.0).max().unwrap_or(0);
  let min_z = footprint.cells.iter().map(|c| c.1).min().unwrap_or(0);
  let max_z = footprint.cells.iter().map(|c| c.1).max().unwrap_or(0);

  let r = range as i32;
  for x in (min_x - r).max(0)..=(max_x + r).min(bw - 1) {
    for z in (min_z - r).max(0)..=(max_z + r).min(bh - 1) {
      if footprint_set.contains(&(x, z)) {
        continue;
      }
      let dist = footprint
        .cells
        .iter()
        .map(|&(fx, fz)| (x - fx).abs() + (z - fz).abs())
        .min()
        .unwrap_or(i32::MAX);
      if dist <= r {
        out.push((x, z));
      }
    }
  }
  out
}

/// Reads `GridChangedMessage` and recomputes `InteractionPoints` only for
/// appliances whose interaction cells may have been invalidated.
pub fn refresh_interaction_points_on_grid_change(
  mut reader: MessageReader<GridChangedMessage>,
  grid: Res<GridLayers>,
  mut query: Query<(
    Entity,
    &GridPosition,
    &ApplianceGeometry,
    &GridFootprint,
    &InteractionRule,
    &mut InteractionPoints,
  )>,
) {
  let mut affected: HashSet<Entity> = HashSet::new();
  for msg in reader.read() {
    for &(cx, cz) in &msg.cells {
      for neighbour in grid.floor_neighbors(cx, cz) {
        affected.insert(neighbour);
      }
      if let Some(e) = grid.floor_entity_at(cx, cz) {
        affected.insert(e);
      }
    }
  }

  if affected.is_empty() {
    return;
  }

  let bounds = (grid.width, grid.height);

  for (entity, pos, geo, footprint, rule, mut points) in query.iter_mut() {
    if !affected.contains(&entity) {
      continue;
    }
    // OnSite entities' interaction cells never stale—skip recompute.
    // (Filtered above by `affected`, but Rust still needs the match exhaustiveness check.)
    let candidates = match rule {
      InteractionRule::Front => compute_front_cells(pos, geo, bounds),
      InteractionRule::AllAdjacent { range } => compute_adjacent_cells(footprint, *range, bounds),
      InteractionRule::OnSite => continue,
    };
    points.cells = candidates
      .into_iter()
      .filter(|&(x, z)| grid.floor_entity_at(x, z).is_none())
      .collect();
  }
}
