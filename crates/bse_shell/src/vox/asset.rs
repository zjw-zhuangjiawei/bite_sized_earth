//! `VoxelAsset` Bevy `Asset` type and the matching `AssetLoader`.
//!
//! ## Why the asset carries a `Mesh` inline
//!
//! Bevy 0.18's `AssetLoader::load` is `async (&self, &mut Reader, &Settings,
//! &mut LoadContext) -> Result<Asset, Error>`. It has no `ResMut<Assets<Mesh>>`
//! access, so the loader cannot mint a `Handle<Mesh>` itself.
//!
//! Two workarounds exist:
//!
//! 1. Mint the handle via `ctx.add_loaded_asset` *after* registering the mesh
//!    in a parallel way. This still requires a `Mesh` instance, which the
//!    loader must build (we do).
//! 2. Defer `Mesh` registration to a system that has full `World` access.
//!
//! We pick option 1 with one twist: the asset carries the fully-built
//! `Mesh` (clonable) so the attachment system can mint its own
//! `Handle<Mesh>` (and per-entity `Handle<StandardMaterial>`) without going
//! back through the asset system. This decouples the load step from
//! render-time spawn cost.
//!
//! ## Error type
//!
//! All non-fatal load failures are surfaced as `VoxLoadError`. Bevy logs
//! the error and the asset stays in a `Failed` state. Reactive systems
//! skip `Failed` handles (their `Handle<VoxelAsset>::get` returns `None`).

use bevy::asset::{AssetLoader, LoadContext, RenderAssetUsages, io::Reader};
use bevy::image::Image;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use thiserror::Error;

use crate::vox::palette::build_palette_image;
use crate::vox::remap::{RemapError, remap_and_mesh};

/// Built voxel mesh plus the palette `Image` and bevy-space AABB dimensions.
///
/// The `palette` handle is per-asset (one per `.vox` file). Sharing it across
/// assets would be wrong if two files override the MagicaVoxel default
/// palette differently — the meshes' UVs would point at the wrong colors.
#[derive(Asset, TypePath, Clone)]
pub struct VoxelAsset {
  pub mesh: Mesh,
  pub palette: Handle<Image>,
  /// Bevy-space dimensions, including the 1-voxel outer padding. The
  /// caller can use `shape_units[i] - 2` to recover the inner model size.
  pub shape_units: [u32; 3],
}

#[derive(Debug, Error)]
pub enum VoxLoadError {
  #[error("I/O error reading .vox bytes: {0}")]
  Io(#[from] std::io::Error),
  #[error("dot-vox parse error: {0}")]
  Parse(String),
  #[error("remap error: {0}")]
  Remap(#[from] RemapError),
}

/// Reads a MagicaVoxel `.vox` file from `reader` and produces a [`VoxelAsset`].
#[derive(Default, TypePath)]
pub struct VoxLoader;

impl AssetLoader for VoxLoader {
  type Asset = VoxelAsset;
  type Settings = ();
  type Error = VoxLoadError;

  async fn load(
    &self,
    reader: &mut dyn Reader,
    _settings: &Self::Settings,
    ctx: &mut LoadContext<'_>,
  ) -> Result<VoxelAsset, VoxLoadError> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).await?;
    let parsed = dot_vox::load_bytes(&bytes).map_err(|e| VoxLoadError::Parse(e.to_string()))?;
    let model = parsed
      .models
      .into_iter()
      .next()
      .ok_or(RemapError::EmptyModel)?;

    // Register this .vox file's own 256-color palette as a labeled asset so
    // it lives alongside the VoxelAsset in the asset server's storage. The
    // handle is then stored on the VoxelAsset and consumed by
    // `attach_voxel_mesh` when building the StandardMaterial.
    let palette_image = build_palette_image(&parsed.palette);
    let palette: Handle<Image> = ctx.add_labeled_asset("palette".to_string(), palette_image);

    let mut remapped = remap_and_mesh(&model, 1.0)?;

    // Offset the mesh so the inner model sits on the entity's origin in the
    // natural way for furniture. `bevy_vox_scene` exposes this as
    // `UnitOffset::CENTER_BASE = (0.5, 0.0, 0.5)`:
    //
    //   - X / Z: full centre — model is centred in the grid cell.
    //   - Y: base — model's bottom face sits at the entity's Y. Without
    //     this, a centre-aligned model placed at ground level (Y = 0) would
    //     have half its voxels below the floor, visible only as the top
    //     half. The grid spawn point in the reactive layer is `Y = 0`, so
    //     furniture (table / register / stove) needs the base offset.
    //
    // Math: `position_offset = inner_size * mesh_offset + leading_padding`,
    // where `leading_padding = 1` (half of the 2-voxel outer padding).
    //
    // For a 64×32×32 stove the inner bevy size is (64, 32, 32) and the
    // padded shape is (66, 34, 34). With `mesh_offset = (0.5, 0, 0.5)` we
    // get (33, 1, 17). The mesh's inner model spans y ∈ [1, 33] in padded
    // coords, so after offset subtraction it spans y ∈ [0, 32] — sitting
    // on the ground.
    let cx = remapped.size_units[0] as f32 * 0.5; // 33.0 for a 66-wide model
    let cz = remapped.size_units[2] as f32 * 0.5; // 17.0 for a 34-deep model
    let cy = 1.0_f32; // leading_padding: places inner-model base at y = 0
    for p in &mut remapped.buffers.positions {
      p[0] -= cx;
      p[1] -= cy;
      p[2] -= cz;
    }

    let mut mesh = Mesh::new(
      PrimitiveTopology::TriangleList,
      RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(
      Mesh::ATTRIBUTE_POSITION,
      VertexAttributeValues::Float32x3(remapped.buffers.positions),
    );
    mesh.insert_attribute(
      Mesh::ATTRIBUTE_NORMAL,
      VertexAttributeValues::Float32x3(remapped.buffers.normals),
    );
    mesh.insert_attribute(
      Mesh::ATTRIBUTE_UV_0,
      VertexAttributeValues::Float32x2(remapped.buffers.uvs),
    );
    mesh.insert_indices(Indices::U32(remapped.buffers.indices));
    Ok(VoxelAsset {
      mesh,
      palette,
      shape_units: remapped.size_units,
    })
  }

  fn extensions(&self) -> &[&str] {
    &["vox"]
  }
}
