//! Geometry: face list → bevy `Mesh` buffers.
//!
//! Voxel space: every `Face.geo_*` is in integer voxel units; the world AABB
//! is computed by `ModelDef::bounds`. The mesher scales by `voxel_size` only
//! at materialization time (asset.rs), keeping the geometry module pure.

use crate::schema::{Face, Plane, plane_axes};

/// Return the unit normal vector for `plane`.
pub fn plane_normal(plane: Plane) -> [i8; 3] {
  match plane {
    Plane::PosX => [1, 0, 0],
    Plane::NegX => [-1, 0, 0],
    Plane::PosY => [0, 1, 0],
    Plane::NegY => [0, -1, 0],
    Plane::PosZ => [0, 0, 1],
    Plane::NegZ => [0, 0, -1],
  }
}

/// Compute the world-space corner of a face quad.
pub fn corner_in_world(plane: Plane, depth: i32, uv_min: [u32; 2]) -> [i32; 3] {
  let (n_axis, u_axis, v_axis) = plane_axes(plane);
  let mut p = [0i32; 3];
  p[n_axis] = depth;
  p[u_axis] = uv_min[0] as i32;
  p[v_axis] = uv_min[1] as i32;
  p
}

/// One expanded quad: 4 corner positions, normals, and pixel-space UVs.
#[derive(Debug, Clone, PartialEq)]
pub struct QuadVerts {
  pub positions: [[f32; 3]; 4],
  pub normals: [[f32; 3]; 4],
  pub uvs: [[f32; 2]; 4],
}

/// Expand one face into 4 vertex positions, normals, and pixel-space UVs.
///
/// Vertex order:
/// - 0: (U=0, V=0)
/// - 1: (U=size[0], V=0)
/// - 2: (U=0, V=size[1])
/// - 3: (U=size[0], V=size[1])
pub fn expand_quad(face: &Face) -> QuadVerts {
  let n = plane_normal(face.plane);
  let n_f = [n[0] as f32, n[1] as f32, n[2] as f32];
  let (_n_axis, u_axis, v_axis) = plane_axes(face.plane);
  let u_len = face.geo_size[0] as f32;
  let v_len = face.geo_size[1] as f32;

  let p0 = corner_in_world(face.plane, face.depth, face.geo_min);
  let p0f = [p0[0] as f32, p0[1] as f32, p0[2] as f32];

  let p1f = axis_add(p0f, u_axis, u_len);
  let p2f = axis_add(p0f, v_axis, v_len);
  let p3f = axis_add(p1f, v_axis, v_len);

  let u_max = face.uv_size[0] as f32;
  let v_max = face.uv_size[1] as f32;
  let uvs = [
    [face.uv_min[0] as f32, face.uv_min[1] as f32],
    [face.uv_min[0] as f32 + u_max, face.uv_min[1] as f32],
    [face.uv_min[0] as f32, face.uv_min[1] as f32 + v_max],
    [face.uv_min[0] as f32 + u_max, face.uv_min[1] as f32 + v_max],
  ];

  let positions = [permute(p0f), permute(p1f), permute(p2f), permute(p3f)];
  let normals = [permute(n_f), permute(n_f), permute(n_f), permute(n_f)];

  QuadVerts {
    positions,
    normals,
    uvs,
  }
}

fn axis_add(p: [f32; 3], axis: usize, len: f32) -> [f32; 3] {
  let mut r = p;
  r[axis] += len;
  r
}

fn permute(p: [f32; 3]) -> [f32; 3] {
  [p[1], p[2], p[0]]
}

/// Normalize pixel-space UVs to [0, 1] using the texture's pixel dimensions.
pub fn normalize_uv(uv_min: [u32; 2], uv_size: [u32; 2], img: [u32; 2]) -> [[f32; 2]; 4] {
  let (w, h) = (img[0] as f32, img[1] as f32);
  let u0 = uv_min[0] as f32 / w;
  let v0 = uv_min[1] as f32 / h;
  let u1 = (uv_min[0] + uv_size[0]) as f32 / w;
  let v1 = (uv_min[1] + uv_size[1]) as f32 / h;
  [[u0, v0], [u1, v0], [u0, v1], [u1, v1]]
}

/// Output of `build_mesh`: per-vertex attributes, triangle indices, and
/// per-material sub-ranges.
#[derive(Debug, Clone)]
pub struct MeshBuffers {
  pub positions: Vec<[f32; 3]>,
  pub normals: Vec<[f32; 3]>,
  pub uvs: Vec<[f32; 2]>,
  pub indices: Vec<u32>,
  /// `(material_index, vertex_start, vertex_end)`.
  pub material_groups: Vec<(u16, u32, u32)>,
}

impl MeshBuffers {
  pub fn empty() -> Self {
    Self {
      positions: Vec::new(),
      normals: Vec::new(),
      uvs: Vec::new(),
      indices: Vec::new(),
      material_groups: Vec::new(),
    }
  }
}

/// Build the full mesh from a `ModelDef`.
///
/// `image_size_for` returns the texture dimensions for a given material
/// index; used to normalize UVs. The registry itself is not consulted here
/// so this function stays pure and unit-testable.
pub fn build_mesh(
  def: &crate::schema::ModelDef,
  image_size_for: &dyn Fn(u16) -> [u32; 2],
) -> MeshBuffers {
  let mut positions = Vec::new();
  let mut normals = Vec::new();
  let mut uvs = Vec::new();
  let mut indices = Vec::new();
  let mut material_groups = Vec::new();

  for face in &def.faces {
    let start = positions.len() as u32;
    let verts = expand_quad(face);
    let img = image_size_for(face.material);
    let uv = normalize_uv(face.uv_min, face.uv_size, img);

    positions.extend_from_slice(&verts.positions);
    normals.extend_from_slice(&verts.normals);
    uvs.extend_from_slice(&uv);

    let end = positions.len() as u32;
    let base = start;

    // Neg* planes share their (u, v) frame with the corresponding Pos*
    // plane, so the (p1-p0) × (p2-p0) cross product always points in
    // the +u × +v direction (i.e. +N). For Pos* this matches the
    // intended normal; for Neg* it's opposite. Reverse the triangle
    // order on Neg* so the front face ends up on the -N side.
    let (t0, t1) = match face.plane {
      Plane::NegX | Plane::NegY | Plane::NegZ => {
        ([base + 2, base + 1, base], [base + 2, base + 3, base + 1])
      }
      _ => ([base, base + 1, base + 2], [base + 1, base + 3, base + 2]),
    };
    indices.extend_from_slice(&t0);
    indices.extend_from_slice(&t1);

    material_groups.push((face.material, start, end));
  }

  MeshBuffers {
    positions,
    normals,
    uvs,
    indices,
    material_groups,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::schema::{Face, ModelDef, Plane};

  #[test]
  fn plane_normal_pos_x() {
    assert_eq!(plane_normal(Plane::PosX), [1, 0, 0]);
  }
  #[test]
  fn plane_normal_neg_y() {
    assert_eq!(plane_normal(Plane::NegY), [0, -1, 0]);
  }
  #[test]
  fn plane_normal_pos_z() {
    assert_eq!(plane_normal(Plane::PosZ), [0, 0, 1]);
  }
  #[test]
  fn plane_axes_posx_uses_yz() {
    assert_eq!(plane_axes(Plane::PosX), (0, 1, 2));
  }
  #[test]
  fn plane_axes_posy_uses_zx() {
    assert_eq!(plane_axes(Plane::PosY), (1, 2, 0));
  }
  #[test]
  fn corner_world_posx() {
    let p = corner_in_world(Plane::PosX, 33, [0, 0]);
    assert_eq!(p, [33, 0, 0]);
  }
  #[test]
  fn corner_world_posy_with_offset() {
    // PosY convention: (U=+Z, V=+X). uv_min[0]=4 → Z=4, uv_min[1]=8 → X=8.
    let p = corner_in_world(Plane::PosY, 16, [4, 8]);
    assert_eq!(p, [8, 16, 4]);
  }

  #[test]
  fn expand_quad_posx_corners() {
    let face = Face {
      plane: Plane::PosX,
      depth: 33,
      geo_min: [0, 0],
      geo_size: [4, 8],
      uv_min: [0, 0],
      uv_size: [4, 8],
      material: 0,
    };
    let v = expand_quad(&face);
    assert_eq!(v.positions[0], [0.0, 0.0, 33.0]);
    assert_eq!(v.positions[1], [4.0, 0.0, 33.0]);
    assert_eq!(v.positions[2], [0.0, 8.0, 33.0]);
    assert_eq!(v.positions[3], [4.0, 8.0, 33.0]);
    for n in &v.normals {
      assert_eq!(*n, [0.0, 0.0, 1.0]);
    }
    assert_eq!(v.uvs[0], [0.0, 0.0]);
    assert_eq!(v.uvs[3], [4.0, 8.0]);
  }

  #[test]
  fn expand_quad_negz_winding() {
    let face = Face {
      plane: Plane::NegZ,
      depth: 0,
      geo_min: [0, 0],
      geo_size: [2, 2],
      uv_min: [0, 0],
      uv_size: [2, 2],
      material: 0,
    };
    let v = expand_quad(&face);
    assert_eq!(v.normals[0], [0.0, -1.0, 0.0]);
    assert_eq!(v.positions[0], [0.0, 0.0, 0.0]);
    assert_eq!(v.positions[1], [0.0, 0.0, 2.0]);
    assert_eq!(v.positions[2], [2.0, 0.0, 0.0]);
    assert_eq!(v.positions[3], [2.0, 0.0, 2.0]);
  }

  #[test]
  fn expand_quad_posy_z_then_x() {
    // PosY uses (U=+Z, V=+X) per schema convention.
    let face = Face {
      plane: Plane::PosY,
      depth: 5,
      geo_min: [0, 0],
      geo_size: [2, 3],
      uv_min: [0, 0],
      uv_size: [2, 3],
      material: 0,
    };
    let v = expand_quad(&face);
    // p0 = (0, 5, 0); +U=+Z by 2; +V=+X by 3
    assert_eq!(v.positions[0], [5.0, 0.0, 0.0]);
    assert_eq!(v.positions[1], [5.0, 2.0, 0.0]);
    assert_eq!(v.positions[2], [5.0, 0.0, 3.0]);
    assert_eq!(v.positions[3], [5.0, 2.0, 3.0]);
  }

  #[test]
  fn normalize_uv_pixel_to_atlas() {
    let img_size = [32, 32];
    let uv = normalize_uv([4, 8], [8, 8], img_size);
    assert!((uv[0][0] - 4.0 / 32.0).abs() < 1e-6);
    assert!((uv[0][1] - 8.0 / 32.0).abs() < 1e-6);
    assert!((uv[3][0] - 12.0 / 32.0).abs() < 1e-6);
    assert!((uv[3][1] - 16.0 / 32.0).abs() < 1e-6);
  }

  fn dummy_size(_idx: u16) -> [u32; 2] {
    [16, 16]
  }

  #[test]
  fn build_mesh_single_face_one_group() {
    let def = ModelDef {
      name: "cube".into(),
      materials: vec!["m".into()],
      faces: vec![Face {
        plane: Plane::PosX,
        depth: 1,
        geo_min: [0, 0],
        geo_size: [1, 1],
        uv_min: [0, 0],
        uv_size: [1, 1],
        material: 0,
      }],
    };
    let bufs = build_mesh(&def, &dummy_size);
    assert_eq!(bufs.positions.len(), 4);
    assert_eq!(bufs.indices.len(), 6);
    assert_eq!(bufs.material_groups.len(), 1);
    assert_eq!(bufs.material_groups[0], (0, 0, 4));
  }

  #[test]
  fn build_mesh_two_faces_split_groups() {
    let def = ModelDef {
      name: "two".into(),
      materials: vec!["a".into(), "b".into()],
      faces: vec![
        Face {
          plane: Plane::PosX,
          depth: 1,
          geo_min: [0, 0],
          geo_size: [1, 1],
          uv_min: [0, 0],
          uv_size: [1, 1],
          material: 0,
        },
        Face {
          plane: Plane::NegX,
          depth: 0,
          geo_min: [0, 0],
          geo_size: [1, 1],
          uv_min: [0, 0],
          uv_size: [1, 1],
          material: 1,
        },
      ],
    };
    let bufs = build_mesh(&def, &dummy_size);
    assert_eq!(bufs.material_groups.len(), 2);
    assert_eq!(bufs.material_groups[0], (0, 0, 4));
    assert_eq!(bufs.material_groups[1], (1, 4, 8));
  }

  #[test]
  fn build_mesh_uvs_normalized() {
    let def = ModelDef {
      name: "u".into(),
      materials: vec!["m".into()],
      faces: vec![Face {
        plane: Plane::PosX,
        depth: 1,
        geo_min: [0, 0],
        geo_size: [4, 4],
        uv_min: [4, 4],
        uv_size: [8, 8],
        material: 0,
      }],
    };
    let bufs = build_mesh(&def, &|_| [16, 16]);
    // uv at corner (4,4) of a 16x16 image = 0.25
    assert!((bufs.uvs[0][0] - 0.25).abs() < 1e-6);
    // (12, 12) → 0.75
    assert!((bufs.uvs[3][0] - 0.75).abs() < 1e-6);
  }

  /// Regression: every face's cross product (in bevy frame, after the
  /// [a, b, c] → [b, c, a] permutation) must agree with the permuted
  /// intended normal. Otherwise back-face culling removes the visible
  /// side of negative-plane faces.
  #[test]
  fn build_mesh_winding_matches_normal_all_planes() {
    for plane in [
      Plane::PosX,
      Plane::NegX,
      Plane::PosY,
      Plane::NegY,
      Plane::PosZ,
      Plane::NegZ,
    ] {
      let def = ModelDef {
        name: "w".into(),
        materials: vec!["m".into()],
        faces: vec![Face {
          plane,
          depth: 4,
          geo_min: [0, 0],
          geo_size: [4, 4],
          uv_min: [0, 0],
          uv_size: [16, 16],
          material: 0,
        }],
      };
      let bufs = build_mesh(&def, &|_| [16, 16]);
      // Compute cross product using the ACTUAL triangle indices
      // emitted by build_mesh (which may have been reversed for
      // negative planes).
      let i0 = bufs.indices[0] as usize;
      let i1 = bufs.indices[1] as usize;
      let i2 = bufs.indices[2] as usize;
      let p0 = bufs.positions[i0];
      let p1 = bufs.positions[i1];
      let p2 = bufs.positions[i2];
      let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
      let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
      let cross = [
        e1[1] * e2[2] - e1[2] * e2[1],
        e1[2] * e2[0] - e1[0] * e2[2],
        e1[0] * e2[1] - e1[1] * e2[0],
      ];
      let intended_grid = plane_normal(plane);
      let intended_bevy = [
        intended_grid[1] as f32,
        intended_grid[2] as f32,
        intended_grid[0] as f32,
      ];
      let dot =
        cross[0] * intended_bevy[0] + cross[1] * intended_bevy[1] + cross[2] * intended_bevy[2];
      assert!(
        dot > 0.0,
        "winding inverted for {plane:?}: cross={cross:?}, intended_bevy={intended_bevy:?}, dot={dot}, indices=({i0},{i1},{i2})"
      );
    }
  }
}
