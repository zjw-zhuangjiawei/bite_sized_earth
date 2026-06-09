//! `ModelAsset` Bevy `Asset` type and the matching `AssetLoader`.

use bevy::asset::{AssetLoader, LoadContext, RenderAssetUsages, io::Reader};
use bevy::prelude::*;
use bevy_image::Image;
use bevy_mesh::{Indices, Mesh, PrimitiveTopology};
use bevy_pbr::StandardMaterial;
use thiserror::Error;

use crate::material::MaterialRegistry;
use crate::mesher::{MeshBuffers, build_mesh};
use crate::schema::ModelDef;

/// One loaded model: a `Mesh` plus its `StandardMaterial` handles and an
/// axis-aligned bounding box in voxel units.
#[derive(Asset, TypePath, Clone)]
pub struct ModelAsset {
  pub mesh: Mesh,
  pub materials: Vec<Handle<StandardMaterial>>,
  pub bounds: crate::schema::BBox,
}

#[derive(Debug, Error)]
pub enum ModelLoadError {
  #[error("I/O error reading .ron bytes: {0}")]
  Io(#[from] std::io::Error),
  #[error("UTF-8 error: {0}")]
  Utf8(#[from] std::str::Utf8Error),
  #[error("RON parse error: {0}")]
  Ron(#[from] ron::error::SpannedError),
  #[error("material index {0} out of bounds (have {1})")]
  MaterialIndex(u16, usize),
}

#[derive(Default, TypePath)]
pub struct ModelLoader;

impl AssetLoader for ModelLoader {
  type Asset = ModelAsset;
  type Settings = ();
  type Error = ModelLoadError;

  async fn load(
    &self,
    reader: &mut dyn Reader,
    _settings: &Self::Settings,
    ctx: &mut LoadContext<'_>,
  ) -> Result<ModelAsset, ModelLoadError> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).await?;
    let text = std::str::from_utf8(&bytes)?;
    let def: ModelDef = ron::from_str(text)?;

    for face in &def.faces {
      if (face.material as usize) >= def.materials.len() {
        return Err(ModelLoadError::MaterialIndex(
          face.material,
          def.materials.len(),
        ));
      }
    }

    // Build the registry: load each texture as a labeled sub-asset and
    // wrap it in a StandardMaterial. Image dimensions are unknown at
    // load time (the image is async); we default to 16×16 and let the
    // artist match `uv_size` to that scale. A future revision can read
    // the image dimensions from `Assets<Image>` after the first load.
    const DEFAULT_TEX_SIZE: [u32; 2] = [16, 16];
    let registry = MaterialRegistry::build(&def, |name| {
      let image: Handle<Image> = ctx.load(format!("textures/{name}.png"));
      let material = ctx.add_labeled_asset(
        format!("material:{name}"),
        StandardMaterial {
          base_color_texture: Some(image),
          ..default()
        },
      );
      (material, DEFAULT_TEX_SIZE)
    });

    let buffers: MeshBuffers = build_mesh(&def, &registry.image_size_fn());
    let mesh = mesh_from_buffers(&buffers);
    let bounds = def.bounds().unwrap_or(crate::schema::BBox::EMPTY);

    Ok(ModelAsset {
      mesh,
      materials: registry.materials,
      bounds,
    })
  }

  fn extensions(&self) -> &[&str] {
    &["ron"]
  }
}

fn mesh_from_buffers(b: &MeshBuffers) -> Mesh {
  let mut mesh = Mesh::new(
    PrimitiveTopology::TriangleList,
    RenderAssetUsages::RENDER_WORLD,
  );
  mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, b.positions.clone());
  mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, b.normals.clone());
  mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, b.uvs.clone());
  mesh.insert_indices(Indices::U32(b.indices.clone()));
  mesh
}
