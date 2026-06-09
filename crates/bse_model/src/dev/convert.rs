//! Convert a `VoxGrid` to a `ModelDef` by greedy-merging same-material
//! quads on each of the 6 axis-aligned planes.

use crate::schema::{Face, ModelDef, Plane};

use super::vox_import::VoxGrid;

/// Build a `ModelDef` from a voxel grid. Material names are placeholder
/// strings (`__mat_0`, `__mat_1`, ...) the artist renames in the .ron.
pub fn vox_to_model_def(name: &str, grid: &VoxGrid) -> ModelDef {
  let materials: Vec<String> = (0..grid.used_count).map(|i| format!("__mat_{i}")).collect();
  let faces = extract_faces(grid);
  ModelDef {
    name: name.into(),
    materials,
    faces,
  }
}

fn extract_faces(grid: &VoxGrid) -> Vec<Face> {
  let mut faces = Vec::new();

  // (plane, n_axis, u_axis, v_axis, n_sign) — depth adjustment is in the loop
  // because +N faces sit at n_idx+1 while -N faces sit at n_idx.
  let planes = [
    (Plane::PosX, 0usize, 1usize, 2usize, 1i32),
    (Plane::NegX, 0usize, 1usize, 2usize, -1i32),
    (Plane::PosY, 1usize, 2usize, 0usize, 1i32),
    (Plane::NegY, 1usize, 2usize, 0usize, -1i32),
    (Plane::PosZ, 2usize, 0usize, 1usize, 1i32),
    (Plane::NegZ, 2usize, 0usize, 1usize, -1i32),
  ];

  for (plane, n_axis, u_axis, v_axis, n_sign) in planes {
    let n_size = grid.size[n_axis];
    let u_size = grid.size[u_axis];
    let v_size = grid.size[v_axis];

    for n_idx in 0..n_size {
      let mut mask: Vec<Option<u8>> = vec![None; (u_size * v_size) as usize];
      for v in 0..v_size {
        for u in 0..u_size {
          let mut coord = [0i32; 3];
          coord[n_axis] = n_idx as i32;
          coord[u_axis] = u as i32;
          coord[v_axis] = v as i32;
          let this_voxel =
            grid.voxels[grid.linearize(coord[0] as u32, coord[1] as u32, coord[2] as u32)];
          if this_voxel == 0 {
            continue;
          }
          let mut n_coord = coord;
          n_coord[n_axis] += n_sign;
          let neighbor_empty = !grid.in_bounds(n_coord[0], n_coord[1], n_coord[2])
            || grid.voxels[grid.linearize(n_coord[0] as u32, n_coord[1] as u32, n_coord[2] as u32)]
              == 0;
          if neighbor_empty {
            mask[(u + v * u_size) as usize] = Some(this_voxel);
          }
        }
      }

      // Greedy merge same-material rectangles.
      for v in 0..v_size {
        let mut u = 0;
        while u < u_size {
          let Some(target) = mask[(u + v * u_size) as usize] else {
            u += 1;
            continue;
          };
          let mut w = 1;
          while u + w < u_size && mask[((u + w) + v * u_size) as usize] == Some(target) {
            w += 1;
          }
          let mut h = 1;
          'extend_v: while v + h < v_size {
            for du in 0..w {
              if mask[((u + du) + (v + h) * u_size) as usize] != Some(target) {
                break 'extend_v;
              }
            }
            h += 1;
          }
          let compact = grid.palette_remap[target as usize];
          let depth = n_idx as i32 + (if n_sign > 0 { 1 } else { 0 });
          faces.push(Face {
            plane,
            depth,
            geo_min: [u, v],
            geo_size: [w, h],
            uv_min: [0, 0],
            uv_size: [w * 16, h * 16], // default 16 texels/voxel
            material: compact as u16,
          });
          for dv in 0..h {
            for du in 0..w {
              mask[((u + du) + (v + dv) * u_size) as usize] = None;
            }
          }
          u += w;
        }
      }
    }
  }

  faces
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_grid(voxels: Vec<u8>, size: [u32; 3]) -> VoxGrid {
    let mut used: Vec<u8> = voxels.iter().copied().filter(|&i| i != 0).collect();
    used.sort_unstable();
    used.dedup();
    let mut palette_remap = [0u8; 256];
    for (compact, &original) in used.iter().enumerate() {
      palette_remap[original as usize] = compact as u8;
    }
    let used_count = used.len() as u8;
    VoxGrid {
      size,
      voxels,
      palette_remap,
      used_count,
    }
  }

  #[test]
  fn single_voxel_yields_six_faces() {
    let grid = make_grid(vec![1], [1, 1, 1]);
    let def = vox_to_model_def("test", &grid);
    assert_eq!(def.faces.len(), 6);
    // Each face: 1x1 voxel, 1 material
    for f in &def.faces {
      assert_eq!(f.geo_size, [1, 1]);
      assert_eq!(f.material, 0);
    }
  }

  #[test]
  fn two_voxels_merge_shared_internal_face() {
    // 2 voxels in a row along X: (0,0,0) and (1,0,0)
    let grid = make_grid(vec![1, 1], [2, 1, 1]);
    let def = vox_to_model_def("test", &grid);
    // -X face: 1 (left voxel). +X face: 1 (right voxel).
    // +Y, -Y, +Z, -Z: each 1 face covering the whole 2x1 strip = 4 faces.
    // Total: 6 (the internal face between the two voxels is hidden).
    assert_eq!(def.faces.len(), 6);
  }

  #[test]
  fn material_remap_compacts_palette() {
    // Voxels use MV indices 5 and 200. Compact to 0 and 1.
    let grid = make_grid(vec![5, 200], [1, 1, 2]);
    let def = vox_to_model_def("test", &grid);
    assert_eq!(def.materials.len(), 2);
    // The two faces should have materials 0 and 1.
    let mut mats: Vec<u16> = def.faces.iter().map(|f| f.material).collect();
    mats.sort_unstable();
    mats.dedup();
    assert_eq!(mats, vec![0, 1]);
  }

  #[test]
  fn placeholder_material_names() {
    let grid = make_grid(vec![1, 2, 3], [1, 1, 3]);
    let def = vox_to_model_def("chair", &grid);
    assert_eq!(def.materials, vec!["__mat_0", "__mat_1", "__mat_2"]);
  }
}
