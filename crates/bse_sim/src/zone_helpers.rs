use crate::components::{ApplianceGeometry, GridDirection, GridFootprint};
use smallvec::SmallVec;

/// Generate `length` cells along a direction from a start point.
pub fn line(start: (i32, i32), dir: GridDirection, length: u32) -> SmallVec<[(i32, i32); 8]> {
  let mut cells = SmallVec::new();
  let offset = dir.facing_offset();
  for i in 1..=length {
    cells.push((start.0 + offset.0 * i as i32, start.1 + offset.1 * i as i32));
  }
  cells
}

/// All cells within `range` Manhattan distance of footprint (geometric only).
pub fn adjacent_cells(footprint: &GridFootprint, range: u32) -> SmallVec<[(i32, i32); 8]> {
  let mut cells = SmallVec::new();
  let r = range as i32;
  let min_x = footprint.cells.iter().map(|c| c.0).min().unwrap_or(0);
  let max_x = footprint.cells.iter().map(|c| c.0).max().unwrap_or(0);
  let min_z = footprint.cells.iter().map(|c| c.1).min().unwrap_or(0);
  let max_z = footprint.cells.iter().map(|c| c.1).max().unwrap_or(0);
  for x in (min_x - r)..=(max_x + r) {
    for z in (min_z - r)..=(max_z + r) {
      if footprint.cells.contains(&(x, z)) {
        continue;
      }
      let dist = footprint
        .cells
        .iter()
        .map(|&(fx, fz)| (x - fx).abs() + (z - fz).abs())
        .min()
        .unwrap_or(i32::MAX);
      if dist <= r {
        cells.push((x, z));
      }
    }
  }
  cells
}

/// Cells in front of appliance, `depth` rows deep, one cell per width-column.
pub fn front_cells(
  anchor: (i32, i32),
  geo: &ApplianceGeometry,
  depth: u32,
) -> SmallVec<[(i32, i32); 8]> {
  let mut cells = SmallVec::new();
  let offset = geo.direction.facing_offset();
  for w in 0..geo.right {
    let (dx, dz) = match geo.direction {
      GridDirection::PosZ | GridDirection::NegZ => (w, 0),
      GridDirection::PosX | GridDirection::NegX => (0, w),
    };
    for d in 1..=depth {
      cells.push((
        anchor.0 + dx + offset.0 * d as i32,
        anchor.1 + dz + offset.1 * d as i32,
      ));
    }
  }
  cells
}

/// Clamp cell list to grid bounds.
pub fn clamp(cells: &[(i32, i32)], bounds: (i32, i32)) -> SmallVec<[(i32, i32); 8]> {
  cells
    .iter()
    .copied()
    .filter(|&(x, z)| x >= 0 && x < bounds.0 && z >= 0 && z < bounds.1)
    .collect()
}
