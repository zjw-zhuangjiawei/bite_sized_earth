//! Self-hosted MagicaVoxel `.vox` asset loader + greedy mesher.
//!
//! Replaces `bevy_vox_scene` (and its transitive `block-mesh` / `ndshape` /
//! `ndcopy` deps) with a single-crate implementation built on top of
//! `dot-vox` for file parsing.
//!
//! ## Why this exists
//!
//! The project uses a tiny subset of `bevy_vox_scene`:
//! - single-model `.vox` files (opaque, default palette, no animation)
//! - render output as a `bevy::prelude::Mesh` with `StandardMaterial`
//!
//! `bevy_vox_scene 0.21` pulls in ~3 transitive crates to do that. This
//! module is ~600 LOC of self-contained code that does only what we need.
//!
//! ## Pipeline
//!
//! ```text
//! .vox bytes  →  dot_vox::load_bytes
//!             →  remap.rs (MV→Bevy axis swap, x mirror, palette shift)
//!             →  mesh.rs::greedy_mesh (0fps greedy meshing)
//!             →  bevy::prelude::Mesh
//!             →  VoxelAsset
//! ```
//!
//! ## AssetLoader limitation
//!
//! Bevy 0.18's `AssetLoader::load` does not expose `ResMut<Assets<Mesh>>`, so
//! the loader cannot mint a `Handle<Mesh>`. The asset carries the built
//! `Mesh` inline; the [`attach_voxel_mesh`] system mints the per-entity
//! `Handle<Mesh>` and `Handle<StandardMaterial>` at spawn time. See
//! `asset.rs` for details.
//!
//! ## Color path
//!
//! The 256-color default MagicaVoxel palette is packed into a 16×16 RGBA8
//! `Image` (see [`palette`]). The mesh's per-vertex UVs index into this
//! atlas; all `StandardMaterial`s share the same `Handle<Image>`, built
//! once at `Startup` and cached as [`VoxPaletteImage`].

pub mod asset;
pub mod mesh;
pub mod palette;
pub mod remap;

use bevy::prelude::*;

pub use asset::{VoxLoadError, VoxLoader, VoxelAsset};
pub use remap::RemapError;

/// Bevy component marker: spawn the entity with the `.vox` referenced by the
/// inner handle. The [`attach_voxel_mesh`] system inserts the `Mesh3d` +
/// `MeshMaterial3d` for you once the asset is loaded.
#[derive(Component, Deref)]
pub struct VoxelAssetHandle(pub Handle<VoxelAsset>);

/// Plugin entry point. Add this to your `App` to register the `VoxLoader`.
pub struct VoxLoaderPlugin;

impl Plugin for VoxLoaderPlugin {
  fn build(&self, app: &mut App) {
    app
      .init_asset::<VoxelAsset>()
      .init_asset_loader::<VoxLoader>()
      .add_systems(Update, attach_voxel_mesh);
  }
}

/// For every entity carrying a [`VoxelAssetHandle`] but not yet a [`Mesh3d`],
/// look up the loaded asset, mint per-entity mesh + material handles, and
/// insert them.
///
/// ## Why `Without<Mesh3d>` instead of `Added<VoxelAssetHandle>`
///
/// `Added<T>` fires only on the frame the component is added. The handle
/// returned by `AssetServer::load` is `Strong` but the underlying asset is
/// loaded asynchronously on Bevy's task pool. The first `Added` tick almost
/// always sees an empty `Assets<VoxelAsset>` and the entity is skipped
/// silently — after which `Added` never fires again and the entity is never
/// revisited. Result: invisible entities.
///
/// `Without<Mesh3d>` instead makes the query re-match the entity every frame
/// until `Mesh3d` is inserted. The first frame the asset is ready the work
/// happens; once `Mesh3d` is in place the entity drops out of the query.
pub fn attach_voxel_mesh(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  voxel_assets: Res<Assets<VoxelAsset>>,
  query: Query<(Entity, &VoxelAssetHandle), (With<VoxelAssetHandle>, Without<Mesh3d>)>,
) {
  for (entity, h) in &query {
    let Some(asset) = voxel_assets.get(&h.0) else {
      // Asset not yet ready — leave the entity in the query for next frame.
      continue;
    };
    let mesh_handle = meshes.add(asset.mesh.clone());
    let mat_handle = materials.add(StandardMaterial {
      base_color_texture: Some(asset.palette.clone()),
      unlit: false,
      ..default()
    });
    commands
      .entity(entity)
      .insert((Mesh3d(mesh_handle), MeshMaterial3d(mat_handle)));
  }
}
