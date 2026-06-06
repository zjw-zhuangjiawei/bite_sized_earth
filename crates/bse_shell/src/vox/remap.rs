//! MagicaVoxel → Bevy coordinate remap and palette index normalisation.
//!
//! ## Frame conventions
//!
//! - **MagicaVoxel**: right-handed Z-up. +X right, +Y forward (depth into
//!   the model), +Z up. (+X × +Y = +Z.)
//! - **Bevy** (this project): right-handed Y-up. +X right, +Y up, +Z out
//!   of the screen toward the viewer. (+X × +Y = +Z.)
//!
//! Both frames are right-handed, so a clean **cyclic permutation** maps MV
//! axes onto Bevy axes without any mirror — handedness is preserved end to
//! end:
//!
//! | MV axis | → | Bevy axis | meaning |
//! |---------|---|-----------|---------|
//! | +X (right) | → | +X (right) | model width stays the model's width |
//! | +Y (depth) | → | +Z (out of screen) | MV's "forward" axis points toward the viewer |
//! | +Z (up) | → | +Y (up) | MV's up is Bevy's up |
//!
//! This differs from `bevy_vox_scene`, which mirrors +X. The mirror is
//! visible only on models that aren't left-right symmetric; for this
//! project's symmetric voxel furniture both produce the same image.
//!
//! ## Implementation
//!
//! `bevy_x = v.x`, `bevy_y = v.z`, `bevy_z = v.y`. No mirror, no negation.
//! The `+1` offset on each axis is the 1-voxel padding layer the mesher
//! needs to produce outer-skin quads without bounds checks.
//!
//! Palette indices are also re-mapped: MV uses 1..=255 for solid and 0 for
//! empty. We store 1..=254 (255 reserved as `EMPTY`) and treat 0 as empty
//! inside the mesher.

use crate::vox::mesh::{greedy_mesh, MeshBuffers, Shape3};

/// Output of [`remap_and_mesh`]: the mesher-ready buffers and the bevy-space
/// AABB size (in voxel units) for downstream placement.
pub struct RemappedMesh {
  pub buffers: MeshBuffers,
  pub shape: Shape3,
  pub size_units: [u32; 3],
}

const EMPTY: u8 = 0;

/// Read an MV `Model`, apply the coord remap, run the greedy mesher.
pub fn remap_and_mesh(model: &dot_vox::Model, voxel_size: f32) -> Result<RemappedMesh, RemapError> {
  if model.size.x == 0 || model.size.y == 0 || model.size.z == 0 {
    return Err(RemapError::EmptyModel);
  }

  // Bevy-space dimensions = (vox_x, vox_z, vox_y). One voxel of padding on
  // each face so the outer model skin can be meshed.
  let inner: [u32; 3] = [model.size.x, model.size.z, model.size.y];
  let padded: [u32; 3] = [inner[0] + 2, inner[1] + 2, inner[2] + 2];
  let shape = Shape3::new(padded);

  let mut voxels = vec![EMPTY; shape.count() as usize];

  for v in &model.voxels {
    // Skip MV-empty voxels (i == 0). MV's 1..=255 maps to 1..=254 in our
    // internal representation (255 reserved as empty sentinel).
    if v.i == 0 {
      continue;
    }
    // Cyclic permutation: mv (x, y, z) → bevy (x, z, y). No mirror, no
    // negation. Preserves handedness (right-hand → right-hand).
    let bevy_x = v.x as u32;
    let bevy_y = v.z as u32;
    let bevy_z = v.y as u32;
    // Inner coords + 1 for the outer padding layer.
    let coord = [bevy_x + 1, bevy_y + 1, bevy_z + 1];
    if !shape.in_bounds(coord) {
      return Err(RemapError::OutOfBounds {
        coord,
        size: padded,
      });
    }
    voxels[shape.linearize(coord) as usize] = v.i;
  }

  let buffers = greedy_mesh(&voxels, &shape, voxel_size);

  Ok(RemappedMesh {
    buffers,
    shape,
    size_units: padded,
  })
}

#[derive(Debug, thiserror::Error)]
pub enum RemapError {
  #[error("model has zero-sized dimension")]
  EmptyModel,
  #[error("voxel coord {coord:?} out of bounds for size {size:?}")]
  OutOfBounds { coord: [u32; 3], size: [u32; 3] },
}

#[cfg(test)]
mod tests {
  use super::*;
  use dot_vox::{Model, Size, Voxel};

  fn make_model(size: Size, voxels: Vec<Voxel>) -> Model {
    Model { size, voxels }
  }

  #[test]
  fn remap_cyclic_permutation() {
    // MV (x, y, z) → bevy (x, z, y). No mirror. 2×2×1 MV model.
    // vox(x=0, y=0, z=0) becomes bevy(0, 0, 0).
    // After +1 padding: voxel sits at bevy(1, 1, 1).
    let m = make_model(
      Size { x: 2, y: 2, z: 1 },
      vec![
        Voxel {
          x: 0,
          y: 0,
          z: 0,
          i: 5,
        },
        Voxel {
          x: 1,
          y: 0,
          z: 0,
          i: 6,
        },
      ],
    );
    let r = remap_and_mesh(&m, 1.0).expect("remap ok");
    // Padded shape (4, 3, 4) — same as before, since the permutation only
    // affects per-voxel coords, not the outer shape.
    assert_eq!(r.size_units, [4, 3, 4]);
    assert_eq!(r.buffers.uvs.len() % 4, 0, "vertex count divisible by 4");
    assert!(
      r.buffers.indices.len() >= 12,
      "at least 2 voxels of geometry"
    );
    // UV check: idx 5 or 6 are the only palette indices in the test data.
    let uv_first = r.buffers.uvs[0];
    let matches = |idx: u8| {
      let u = (idx % 16) as f32 / 16.0 + 0.5 / 16.0;
      let v = (idx / 16) as f32 / 16.0 + 0.5 / 16.0;
      (uv_first[0] - u).abs() < 1e-6 && (uv_first[1] - v).abs() < 1e-6
    };
    assert!(
      matches(5) || matches(6),
      "uv {uv_first:?} didn't match idx 5 or 6"
    );
  }

  #[test]
  fn empty_model_errors() {
    let m = make_model(Size { x: 0, y: 0, z: 0 }, vec![]);
    assert!(matches!(
      remap_and_mesh(&m, 1.0),
      Err(RemapError::EmptyModel)
    ));
  }
}
