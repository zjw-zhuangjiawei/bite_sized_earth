//! Greedy mesher for voxel data.
//!
//! Replaces `block-mesh` + `ndshape` + `ndcopy` from `bevy_vox_scene`. The
//! algorithm is the classic *Greedy Meshing* by Mikola Lysenko (0fps article,
//! <https://0fps.net/2012/06/30/meshing-in-a-minecraft-game/>).
//!
//! Input: a flat `&[u8]` of palette indices (0 = empty) plus a [`Shape3`]
//! describing dimensions, both in Bevy-space coordinates. Output: per-vertex
//! `positions`, `normals`, `uvs`, and triangle `indices` for a `bevy::prelude::Mesh`.
//!
//! Visual output is intended to match `bevy_vox_scene 0.21`:
//! - UVs index a 16×16 palette atlas (`(idx % 16 + 0.5) / 16` etc.).
//! - Outer skin voxels are meshed (1-voxel zero-padded border).
//! - Front faces CCW, back faces CW (`quad_mesh_indices`).
//!
//! No `unsafe`. No `bevy` types in the inner data flow — only the free
//! function return values are framework-neutral `Vec`s. Caller wraps them
//! into a `Mesh`.

/// Axis-indexed 3D shape. Linearization: `i = (x * size[1] + y) * size[2] + z`.
#[derive(Clone, Copy, Debug)]
pub struct Shape3 {
  pub size: [u32; 3],
}

impl Shape3 {
  pub fn new(size: [u32; 3]) -> Self {
    Self { size }
  }

  /// Total voxel count.
  pub fn count(&self) -> u32 {
    self.size[0] * self.size[1] * self.size[2]
  }

  pub fn linearize(&self, [x, y, z]: [u32; 3]) -> u32 {
    (x * self.size[1] + y) * self.size[2] + z
  }

  pub fn delinearize(&self, mut i: u32) -> [u32; 3] {
    let z = i % self.size[2];
    i /= self.size[2];
    let y = i % self.size[1];
    let x = i / self.size[1];
    [x, y, z]
  }

  pub fn in_bounds(&self, [x, y, z]: [u32; 3]) -> bool {
    x < self.size[0] && y < self.size[1] && z < self.size[2]
  }
}

/// A merged face-quad in voxel space. `minimum` is the `[N, U, V]` corner;
/// `width` and `height` extend along the U and V axes respectively.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnorientedQuad {
  pub minimum: [u32; 3],
  pub width: u32,
  pub height: u32,
}

/// Six face directions, each carrying the per-vertex geometry expansion.
#[derive(Clone, Copy, Debug)]
pub struct OrientedFace {
  /// Axis perpendicular to the face: 0=X, 1=Y, 2=Z.
  n_axis: u32,
  /// 1 for `+N`, -1 for `-N`.
  n_sign: i32,
  /// First tangent axis (0..=2).
  u_axis: u32,
  /// Second tangent axis (0..=2).
  v_axis: u32,
}

impl OrientedFace {
  // Axis assignment rule: U × V must equal +N (right-handed) for positive
  // faces and -N for negative faces. That keeps triangle (0,1,2) wound CCW
  // from the +N viewpoint, matching the front-face winding the renderer
  // expects. POS_Y and NEG_Y are the special cases where the natural
  // (U=+X, V=+Z) ordering gives a left-handed frame; we swap to (U=+Z, V=+X)
  // so U × V = +Y for POS_Y and -Y for NEG_Y. (POS_X and POS_Z are happy
  // with the obvious (Y,Z) and (X,Y) pairings respectively.)
  pub const POS_X: Self = Self {
    n_axis: 0,
    n_sign: 1,
    u_axis: 1,
    v_axis: 2,
  };
  pub const NEG_X: Self = Self {
    n_axis: 0,
    n_sign: -1,
    u_axis: 1,
    v_axis: 2,
  };
  pub const POS_Y: Self = Self {
    n_axis: 1,
    n_sign: 1,
    u_axis: 2,
    v_axis: 0,
  };
  pub const NEG_Y: Self = Self {
    n_axis: 1,
    n_sign: -1,
    u_axis: 2,
    v_axis: 0,
  };
  pub const POS_Z: Self = Self {
    n_axis: 2,
    n_sign: 1,
    u_axis: 0,
    v_axis: 1,
  };
  pub const NEG_Z: Self = Self {
    n_axis: 2,
    n_sign: -1,
    u_axis: 0,
    v_axis: 1,
  };

  pub const ALL: [Self; 6] = [
    Self::POS_X,
    Self::NEG_X,
    Self::POS_Y,
    Self::NEG_Y,
    Self::POS_Z,
    Self::NEG_Z,
  ];

  fn normal(&self) -> [f32; 3] {
    let mut n = [0.0, 0.0, 0.0];
    n[self.n_axis as usize] = self.n_sign as f32;
    n
  }

  /// Four corner positions of `quad` in voxel-space. `voxel_size` is applied
  /// uniformly. Corners follow the diagram:
  ///
  /// ```text
  ///         2 ----> 3
  ///           ^
  ///     ^       \
  ///     |         \
  ///  +V |   0 ----> 1
  ///     |
  ///      -------->
  ///        +U
  /// (+N pointing out of the screen)
  /// ```
  pub fn quad_mesh_positions(&self, quad: &UnorientedQuad, voxel_size: f32) -> [[f32; 3]; 4] {
    let [mut nx, mut ny, mut nz] = quad.minimum;
    // The world-space offset along the normal: the face lies at the
    // +N boundary of the voxel (i.e., at min[N] + n_sign * 1) for +N faces,
    // and at min[N] (no offset) for -N faces.
    if self.n_sign > 0 {
      match self.n_axis {
        0 => nx += 1,
        1 => ny += 1,
        _ => nz += 1,
      }
    }
    let base = [nx as f32, ny as f32, nz as f32];
    let a = base;
    let mut b = base;
    let mut c = base;
    let mut d = base;
    // Corner 1: +U by `width`.
    b[self.u_axis as usize] = base[self.u_axis as usize] + quad.width as f32;
    // Corner 2: +V by `height`.
    c[self.v_axis as usize] = base[self.v_axis as usize] + quad.height as f32;
    // Corner 3: +U and +V.
    d[self.u_axis as usize] = b[self.u_axis as usize];
    d[self.v_axis as usize] = c[self.v_axis as usize];

    let scale = |v: [f32; 3]| [v[0] * voxel_size, v[1] * voxel_size, v[2] * voxel_size];
    [scale(a), scale(b), scale(c), scale(d)]
  }

  pub fn quad_mesh_normals(&self) -> [[f32; 3]; 4] {
    let n = self.normal();
    [n, n, n, n]
  }

  /// Six vertex indices forming two triangles. Front faces (positive normal)
  /// are wound CCW, back faces CW.
  pub fn quad_mesh_indices(&self, start: u32) -> [u32; 6] {
    if self.n_sign > 0 {
      [start, start + 1, start + 2, start + 1, start + 3, start + 2]
    } else {
      [start, start + 2, start + 1, start + 1, start + 2, start + 3]
    }
  }
}

/// Output of [`greedy_mesh`]: per-vertex attributes and triangle indices.
pub struct MeshBuffers {
  pub positions: Vec<[f32; 3]>,
  pub normals: Vec<[f32; 3]>,
  pub uvs: Vec<[f32; 2]>,
  pub indices: Vec<u32>,
}

#[derive(Clone, Copy)]
struct FaceQuad {
  face: OrientedFace,
  quad: UnorientedQuad,
  palette_index: u8,
}

/// Run the greedy mesher.
///
/// `voxels` must be `shape.count()` bytes long, palette indices 1..=255 are
/// solid, 0 is empty. Caller is responsible for any padding (a 1-voxel
/// zero-padded border is required if outer faces should be meshed).
pub fn greedy_mesh(voxels: &[u8], shape: &Shape3, voxel_size: f32) -> MeshBuffers {
  assert_eq!(
    voxels.len(),
    shape.count() as usize,
    "voxels len ≠ shape count"
  );

  let mut out: Vec<FaceQuad> = Vec::new();

  for &face in &OrientedFace::ALL {
    mesh_one_face(voxels, shape, face, &mut out);
  }

  // Expand quads into per-vertex attributes.
  let mut positions = Vec::with_capacity(out.len() * 4);
  let mut normals = Vec::with_capacity(out.len() * 4);
  let mut uvs = Vec::with_capacity(out.len() * 4);
  let mut indices = Vec::with_capacity(out.len() * 6);

  for fq in &out {
    let pos = fq.face.quad_mesh_positions(&fq.quad, voxel_size);
    let nrm = fq.face.quad_mesh_normals();
    let u = ((fq.palette_index % 16) as f32 + 0.5) / 16.0;
    let v = ((fq.palette_index / 16) as f32 + 0.5) / 16.0;
    let uv = [[u, v]; 4];
    let base = positions.len() as u32;
    positions.extend_from_slice(&pos);
    normals.extend_from_slice(&nrm);
    uvs.extend_from_slice(&uv);
    indices.extend_from_slice(&fq.face.quad_mesh_indices(base));
  }

  MeshBuffers {
    positions,
    normals,
    uvs,
    indices,
  }
}

fn mesh_one_face(voxels: &[u8], shape: &Shape3, face: OrientedFace, out: &mut Vec<FaceQuad>) {
  let n_size = shape.size[face.n_axis as usize];
  let u_size = shape.size[face.u_axis as usize];
  let v_size = shape.size[face.v_axis as usize];

  // For each slice along N: build a 2D mask on (U, V).
  let mut mask: Vec<Option<u8>> = vec![None; (u_size * v_size) as usize];

  for k in 0..n_size {
    // Reset mask for this slice.
    for m in mask.iter_mut() {
      *m = None;
    }

    for v in 0..v_size {
      for u in 0..u_size {
        let coord = assemble_coord(face.n_axis, k, face.u_axis, u, face.v_axis, v);
        let this_idx = shape.linearize(coord) as usize;
        let this_voxel = voxels[this_idx];
        if this_voxel == 0 {
          continue;
        }
        // Neighbor on the +N side: only visible if neighbor is empty or OOB.
        // We use wrapping arithmetic on purpose: when the neighbor is below 0
        // (e.g. the `-N` face of a voxel at axis 0), `saturating_sub` would
        // clamp to 0 and we'd accidentally read the voxel itself instead of
        // failing the bounds check. Wrapping produces a huge u32 that fails
        // `in_bounds` immediately.
        let n_coord = if face.n_sign > 0 {
          let mut c = coord;
          c[face.n_axis as usize] += 1;
          c
        } else {
          let mut c = coord;
          c[face.n_axis as usize] = c[face.n_axis as usize].wrapping_sub(1);
          c
        };
        let neighbor_empty =
          !shape.in_bounds(n_coord) || voxels[shape.linearize(n_coord) as usize] == 0;
        if neighbor_empty {
          mask[(u + v * u_size) as usize] = Some(this_voxel);
        }
      }
    }

    // Greedy expand mask into largest rectangles of same palette index.
    for v in 0..v_size {
      let mut u = 0;
      while u < u_size {
        let Some(target) = mask[(u + v * u_size) as usize] else {
          u += 1;
          continue;
        };
        // Extend along U.
        let mut w = 1;
        while u + w < u_size && mask[((u + w) + v * u_size) as usize] == Some(target) {
          w += 1;
        }
        // Extend along V.
        let mut h = 1;
        'extend_v: while v + h < v_size {
          for du in 0..w {
            if mask[((u + du) + (v + h) * u_size) as usize] != Some(target) {
              break 'extend_v;
            }
          }
          h += 1;
        }
        // Emit quad.
        let coord = assemble_coord(face.n_axis, k, face.u_axis, u, face.v_axis, v);
        out.push(FaceQuad {
          face,
          quad: UnorientedQuad {
            minimum: coord,
            width: w,
            height: h,
          },
          palette_index: target,
        });
        // Clear covered cells.
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

fn assemble_coord(n_axis: u32, n: u32, u_axis: u32, u: u32, v_axis: u32, v: u32) -> [u32; 3] {
  let mut c = [0u32; 3];
  c[n_axis as usize] = n;
  c[u_axis as usize] = u;
  c[v_axis as usize] = v;
  c
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn linearize_roundtrip() {
    let s = Shape3::new([5, 7, 11]);
    for x in 0..s.size[0] {
      for y in 0..s.size[1] {
        for z in 0..s.size[2] {
          let i = s.linearize([x, y, z]);
          assert_eq!(
            s.delinearize(i),
            [x, y, z],
            "roundtrip failed for ({x},{y},{z})"
          );
        }
      }
    }
  }

  /// One solid voxel at (1,1,1) inside a 3×3×3 zero-padded shape. Expect 6
  /// quads (one per face), 24 vertices, 36 indices.
  #[test]
  fn single_voxel_yields_six_quads() {
    let shape = Shape3::new([3, 3, 3]);
    let mut voxels = vec![0u8; shape.count() as usize];
    voxels[shape.linearize([1, 1, 1]) as usize] = 5; // arbitrary palette idx
    let m = greedy_mesh(&voxels, &shape, 1.0);
    assert_eq!(m.positions.len(), 24, "6 quads × 4 vertices");
    assert_eq!(m.normals.len(), 24);
    assert_eq!(m.uvs.len(), 24);
    assert_eq!(m.indices.len(), 36, "6 quads × 6 indices");
  }

  /// 2×2×1 plane (z=0, x∈{0,1}, y∈{0,1}) padded to 3×3×3. All same color.
  /// Expect the four exposed +Z faces to merge into 1 quad, and the four
  /// exposed -Z faces to merge into 1 quad. The sides of the 2×2 plane each
  /// contribute their own. Total quads: 1 (+Z merged) + 1 (-Z merged) +
  /// 4 sides (2 along x, 2 along y) = 6 quads.
  #[test]
  fn coplanar_same_color_merges() {
    let shape = Shape3::new([3, 3, 3]);
    let mut voxels = vec![0u8; shape.count() as usize];
    for x in 0..2 {
      for y in 0..2 {
        voxels[shape.linearize([x + 1, y + 1, 0]) as usize] = 7;
      }
    }
    let m = greedy_mesh(&voxels, &shape, 1.0);
    let quad_count = m.indices.len() / 6;
    assert_eq!(quad_count, 6, "expected 6 quads, got {quad_count}");
  }

  /// 1×2 line of 2 voxels with 2 different palette indices in a 4×3×3 padded
  /// shape. The shared internal face is hidden (both neighbors solid).
  /// Total quads: 6×2 minus the 1 hidden shared face (saving 2 triangles) =
  /// 10. This verifies that adjacent cells with different palette indices do
  /// NOT merge across the color boundary.
  #[test]
  fn line_two_colors_no_merge() {
    let shape = Shape3::new([4, 3, 3]);
    let mut voxels = vec![0u8; shape.count() as usize];
    voxels[shape.linearize([1, 1, 1]) as usize] = 1;
    voxels[shape.linearize([2, 1, 1]) as usize] = 2;
    let m = greedy_mesh(&voxels, &shape, 1.0);
    let quad_count = m.indices.len() / 6;
    assert_eq!(quad_count, 10, "expected 10 quads, got {quad_count}");
  }

  /// 3×3×3 fully solid block inside a 5×5×5 zero-padded shape. Inner voxels
  /// hidden: only the 6 outer 3×3 faces of the inner block should be meshed.
  /// Each face is 3×3=9 voxels but same color, so each face = 1 quad = 6 quads
  /// total.
  #[test]
  fn inner_voxel_hidden() {
    let shape = Shape3::new([5, 5, 5]);
    let mut voxels = vec![0u8; shape.count() as usize];
    for x in 1..=3 {
      for y in 1..=3 {
        for z in 1..=3 {
          voxels[shape.linearize([x, y, z]) as usize] = 9;
        }
      }
    }
    let m = greedy_mesh(&voxels, &shape, 1.0);
    let quad_count = m.indices.len() / 6;
    assert_eq!(
      quad_count, 6,
      "expected 6 outer-face quads, got {quad_count}"
    );
  }

  #[test]
  fn quad_mesh_indices_winding() {
    // POS_X: CCW when looking from +X.
    let p = OrientedFace::POS_X.quad_mesh_indices(10);
    assert_eq!(p, [10, 11, 12, 11, 13, 12]);
    // NEG_X: CW when looking from -X (which is CCW from +X).
    let n = OrientedFace::NEG_X.quad_mesh_indices(10);
    assert_eq!(n, [10, 12, 11, 11, 12, 13]);
  }

  #[test]
  fn quad_mesh_positions_pos_x() {
    // Quad at slice x=4, U=y=2 (width=3), V=z=5 (height=2), voxel_size=1.0.
    let q = UnorientedQuad {
      minimum: [4, 2, 5],
      width: 3,
      height: 2,
    };
    let p = OrientedFace::POS_X.quad_mesh_positions(&q, 1.0);
    // +N = +X, so face sits at x = 4 + 1 = 5.
    assert_eq!(p[0], [5.0, 2.0, 5.0]);
    assert_eq!(p[1], [5.0, 5.0, 5.0]); // +U by 3
    assert_eq!(p[2], [5.0, 2.0, 7.0]); // +V by 2
    assert_eq!(p[3], [5.0, 5.0, 7.0]);
  }

  #[test]
  fn uv_atlas_indices() {
    // Build a single-voxel mesh with palette idx 17 to verify UV mapping.
    let shape = Shape3::new([3, 3, 3]);
    let mut voxels = vec![0u8; shape.count() as usize];
    voxels[shape.linearize([1, 1, 1]) as usize] = 17;
    let m = greedy_mesh(&voxels, &shape, 1.0);
    let expected_u = ((17 % 16) as f32 + 0.5) / 16.0; // 0.09375
    let expected_v = ((17 / 16) as f32 + 0.5) / 16.0; // 0.03125
    for uv in &m.uvs {
      assert!((uv[0] - expected_u).abs() < 1e-6, "u mismatch: {}", uv[0]);
      assert!((uv[1] - expected_v).abs() < 1e-6, "v mismatch: {}", uv[1]);
    }
  }

  /// Regression test: every emitted face must have its expected outward
  /// normal. Catches the case where a face's (U, V) assignment is
  /// left-handed (U × V = -N), which would flip the triangle winding and
  /// get the face backface-culled. Previously the POS_Y / NEG_Y faces were
  /// exactly this kind of bug.
  #[test]
  fn all_six_faces_have_correct_outward_normals() {
    let shape = Shape3::new([3, 3, 3]);
    let mut voxels = vec![0u8; shape.count() as usize];
    voxels[shape.linearize([1, 1, 1]) as usize] = 1;
    let m = greedy_mesh(&voxels, &shape, 1.0);

    // One unique normal per quad (all 4 vertices of a quad share its normal).
    let mut normals: Vec<[i8; 3]> = m
      .normals
      .iter()
      .step_by(4)
      .map(|n| [n[0].round() as i8, n[1].round() as i8, n[2].round() as i8])
      .collect();
    normals.sort();
    let expected = [
      [-1, 0, 0],
      [0, -1, 0],
      [0, 0, -1],
      [0, 0, 1],
      [0, 1, 0],
      [1, 0, 0],
    ];
    assert_eq!(normals, expected, "each face must point along its ±N axis");
  }

  /// Sanity: a quad's geometric normal (from its triangle winding) matches
  /// the per-vertex `normal` attribute.
  #[test]
  fn pos_y_face_normal_is_up() {
    let shape = Shape3::new([3, 3, 3]);
    let mut voxels = vec![0u8; shape.count() as usize];
    voxels[shape.linearize([1, 1, 1]) as usize] = 1;
    let m = greedy_mesh(&voxels, &shape, 1.0);
    // Find the quad whose normal is +Y.
    let quad_idx = m
      .normals
      .iter()
      .step_by(4)
      .position(|n| n[1] > 0.5)
      .expect("POS_Y quad exists");
    let p = &m.positions[quad_idx * 4..quad_idx * 4 + 4];
    // Triangle (0, 1, 2) must give a +Y cross product.
    let e1 = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
    let e2 = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
    let cross_y = e1[2] * e2[0] - e1[0] * e2[2]; // j-component of e1×e2
    assert!(
      cross_y > 0.0,
      "POS_Y triangle must wind CCW (j>0), got {cross_y}"
    );
  }
}
