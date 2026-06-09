//! `.vox` parser wrapper.
//!
//! Reads a MagicaVoxel `.vox` file and returns a dense `VoxGrid` in the
//! model's local 3D space (right-handed Z-up; MV axes passed through
//! unchanged). The Bevy-side remap happens later in the mesher; this
//! module does NOT swap axes.
//!
//! Plus a palette remap table (original MV idx → compact material idx
//! 0..N).

use std::path::Path;

/// A 3D voxel grid in model-local space (right-handed Z-up).
/// `size[0] = X` (model width), `size[1] = Y` (model depth),
/// `size[2] = Z` (model height, up).
/// Linearization: `i = (x * size[1] + y) * size[2] + z`.
#[derive(Debug, Clone)]
pub struct VoxGrid {
  /// `[X, Y, Z]` in model-local voxel units.
  pub size: [u32; 3],
  /// `size[0] * size[1] * size[2]` bytes; `0` = empty, `1..=255` = original
  /// MV palette index (pre-remap).
  pub voxels: Vec<u8>,
  /// `palette_remap[original_mv_idx] = compact_idx` (0..N) for indices
  /// actually used; unused indices map to 0 (a placeholder that callers
  /// must check via `used_count` before dereferencing).
  pub palette_remap: [u8; 256],
  /// Number of unique non-zero palette indices seen.
  pub used_count: u8,
}

impl VoxGrid {
  pub fn linearize(&self, x: u32, y: u32, z: u32) -> usize {
    (x * self.size[1] + y) as usize * self.size[2] as usize + z as usize
  }

  pub fn get(&self, x: u32, y: u32, z: u32) -> u8 {
    self.voxels[self.linearize(x, y, z)]
  }

  pub fn in_bounds(&self, x: i32, y: i32, z: i32) -> bool {
    x >= 0
      && y >= 0
      && z >= 0
      && (x as u32) < self.size[0]
      && (y as u32) < self.size[1]
      && (z as u32) < self.size[2]
  }
}

pub fn load(path: &Path) -> Result<VoxGrid, Box<dyn std::error::Error>> {
  let bytes = std::fs::read(path)?;
  let data = dot_vox::load_bytes(&bytes)?;

  let model = data.models.first().ok_or("vox file has no models")?;
  // MV axes pass through unchanged — the VoxGrid lives in model-local
  // space (Z up); the bevy-side permute is the mesher's job.
  let size = [model.size.x, model.size.y, model.size.z];

  let mut voxels = vec![0u8; (size[0] * size[1] * size[2]) as usize];
  for v in &model.voxels {
    let x = v.x as u32;
    let y = v.y as u32;
    let z = v.z as u32;
    let idx = (x * size[1] + y) as usize * size[2] as usize + z as usize;
    voxels[idx] = v.i;
  }

  // Collect unique non-zero palette indices actually used.
  let mut used: Vec<u8> = voxels.iter().copied().filter(|&i| i != 0).collect();
  used.sort_unstable();
  used.dedup();

  let mut palette_remap = [0u8; 256];
  for (compact, &original) in used.iter().enumerate() {
    palette_remap[original as usize] = compact as u8;
  }
  let used_count = used.len() as u8;

  Ok(VoxGrid {
    size,
    voxels,
    palette_remap,
    used_count,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn single_voxel_round_trip() {
    // Hand-craft a VoxGrid and check the helpers.
    let mut grid = VoxGrid {
      size: [1, 1, 1],
      voxels: vec![5],
      palette_remap: [0u8; 256],
      used_count: 1,
    };
    grid.palette_remap[5] = 0;
    assert_eq!(grid.get(0, 0, 0), 5);
    assert!(grid.in_bounds(0, 0, 0));
    assert!(!grid.in_bounds(-1, 0, 0));
    assert!(!grid.in_bounds(1, 0, 0));
  }
}
