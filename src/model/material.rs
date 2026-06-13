//! Material registry: maps `ModelDef.materials` strings to `Handle<StandardMaterial>`.
//!
//! The registry is built during asset load; it owns the texture image
//! dimensions used by `build_mesh` for UV normalization.

use bevy::prelude::*;
use bevy_pbr::StandardMaterial;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn build_registry_with_injected_resolver() {
    let def = super::super::schema::ModelDef {
      name: "two".into(),
      materials: vec!["a".into(), "b".into()],
      faces: vec![],
    };
    let reg = MaterialRegistry::build(&def, |name| {
      let h = Handle::<StandardMaterial>::default();
      let size = if name == "a" { [32, 32] } else { [64, 64] };
      (h, size)
    });
    assert_eq!(reg.materials.len(), 2);
    assert_eq!(reg.image_sizes[0], [32, 32]);
    assert_eq!(reg.image_sizes[1], [64, 64]);
  }

  #[test]
  fn image_size_lookup() {
    let def = super::super::schema::ModelDef {
      name: "x".into(),
      materials: vec!["a".into(), "b".into()],
      faces: vec![],
    };
    let reg = MaterialRegistry::build(&def, |name| {
      let size = if name == "a" { [8, 8] } else { [16, 16] };
      (Handle::<StandardMaterial>::default(), size)
    });
    assert_eq!(reg.image_size(0), [8, 8]);
    assert_eq!(reg.image_size(1), [16, 16]);
  }
}

#[derive(Clone)]
pub struct MaterialRegistry {
  pub materials: Vec<Handle<StandardMaterial>>,
  pub image_sizes: Vec<[u32; 2]>,
}

impl MaterialRegistry {
  pub fn new() -> Self {
    Self {
      materials: Vec::new(),
      image_sizes: Vec::new(),
    }
  }

  /// Build a registry by calling `resolve_material(name)` once per entry in
  /// `def.materials`. The resolver returns a `(StandardMaterial, image
  /// width, image height)` tuple. The function is unit-testable: the
  /// `AssetLoader` passes a real resolver at load time, tests inject
  /// deterministic stubs.
  pub fn build<F>(def: &super::schema::ModelDef, mut resolve_material: F) -> Self
  where
    F: FnMut(&str) -> (Handle<StandardMaterial>, [u32; 2]),
  {
    let mut reg = Self::new();
    for name in &def.materials {
      let (mat, size) = resolve_material(name);
      reg.materials.push(mat);
      reg.image_sizes.push(size);
    }
    reg
  }

  /// Return the image dimensions for material index `idx`.
  pub fn image_size(&self, idx: u16) -> [u32; 2] {
    self.image_sizes[idx as usize]
  }

  /// Closure adapter for `mesher::build_mesh`.
  pub fn image_size_fn(&self) -> impl Fn(u16) -> [u32; 2] + '_ {
    move |idx| self.image_size(idx)
  }
}

impl Default for MaterialRegistry {
  fn default() -> Self {
    Self::new()
  }
}
