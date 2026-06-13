use serde::{Deserialize, Serialize};

/// One of six face planes in axis-aligned voxel space. The normal direction is
/// the sign (Pos/Neg); the magnitude is the face's `depth` along the axis.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Plane {
  PosX,
  NegX,
  PosY,
  NegY,
  PosZ,
  NegZ,
}

/// A single quad (axis-aligned rectangle on a plane).
///
/// `geo_*` is in voxel units (model-local); `uv_*` is in texture pixels.
/// `material` is the index into `ModelDef::materials`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Face {
  pub plane: Plane,
  pub depth: i32,
  pub geo_min: [u32; 2],
  pub geo_size: [u32; 2],
  pub uv_min: [u32; 2],
  pub uv_size: [u32; 2],
  pub material: u16,
}

/// A face-list model: name, material name table, and a list of quads.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelDef {
  pub name: String,
  pub materials: Vec<String>,
  pub faces: Vec<Face>,
}

/// Axis-aligned bounding box in voxel units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BBox {
  pub min: [i32; 3],
  pub max: [i32; 3],
}

impl BBox {
  pub const EMPTY: Self = Self {
    min: [i32::MAX; 3],
    max: [i32::MIN; 3],
  };

  pub fn expand(&mut self, point: [i32; 3]) {
    for i in 0..3 {
      self.min[i] = self.min[i].min(point[i]);
      self.max[i] = self.max[i].max(point[i]);
    }
  }

  pub fn size(&self) -> [u32; 3] {
    [
      (self.max[0] - self.min[0]) as u32,
      (self.max[1] - self.min[1]) as u32,
      (self.max[2] - self.min[2]) as u32,
    ]
  }
}

/// Return the (n_axis, u_axis, v_axis) tuple for a plane, in [0, 3).
pub fn plane_axes(plane: Plane) -> (usize, usize, usize) {
  match plane {
    Plane::PosX | Plane::NegX => (0, 1, 2),
    Plane::PosY | Plane::NegY => (1, 2, 0),
    Plane::PosZ | Plane::NegZ => (2, 0, 1),
  }
}

impl ModelDef {
  /// Compute the world-space AABB of the model from its face list.
  /// Returns `None` if there are no faces.
  pub fn bounds(&self) -> Option<BBox> {
    let mut bbox = BBox::EMPTY;
    let mut any = false;
    for face in &self.faces {
      let (n_axis, u_axis, v_axis) = plane_axes(face.plane);
      let n = face.depth;
      let u0 = face.geo_min[0] as i32;
      let v0 = face.geo_min[1] as i32;
      let u1 = u0 + face.geo_size[0] as i32;
      let v1 = v0 + face.geo_size[1] as i32;
      for &(u, v) in &[(u0, v0), (u1, v0), (u0, v1), (u1, v1)] {
        let mut p = [0i32; 3];
        p[n_axis] = n;
        p[u_axis] = u;
        p[v_axis] = v;
        bbox.expand(p);
        any = true;
      }
    }
    if any { Some(bbox) } else { None }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ron_roundtrip() {
    let original = ModelDef {
      name: "chair".into(),
      materials: vec!["wood/oak".into(), "fabric/red".into()],
      faces: vec![Face {
        plane: Plane::PosX,
        depth: 33,
        geo_min: [0, 0],
        geo_size: [8, 16],
        uv_min: [0, 0],
        uv_size: [8, 16],
        material: 0,
      }],
    };
    let s = ron::to_string(&original).unwrap();
    let parsed: ModelDef = ron::from_str(&s).unwrap();
    assert_eq!(parsed.name, "chair");
    assert_eq!(parsed.faces.len(), 1);
    assert_eq!(parsed.faces[0].plane, Plane::PosX);
    assert_eq!(parsed.materials, vec!["wood/oak", "fabric/red"]);
  }

  #[test]
  fn bounds_of_empty_model() {
    let def = ModelDef {
      name: "empty".into(),
      materials: vec![],
      faces: vec![],
    };
    assert_eq!(def.bounds(), None);
  }

  #[test]
  fn bounds_of_cube() {
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
    let bb = def.bounds().unwrap();
    assert_eq!(bb.min, [1, 0, 0]);
    assert_eq!(bb.max, [1, 1, 1]);
  }
}
