use bevy::prelude::*;

use super::asset::ModelLoader;

/// Bevy plugin: registers `ModelAsset` + `ModelLoader` so any `.ron` file
/// under an asset path can be loaded via `asset_server.load("models/x.ron")`.
pub struct ModelLoaderPlugin;

impl Plugin for ModelLoaderPlugin {
  fn build(&self, app: &mut App) {
    app
      .init_asset::<super::asset::ModelAsset>()
      .init_asset_loader::<ModelLoader>();
  }
}

/// Component marker: spawn the entity with the `.ron` referenced by the
/// inner handle. The `attach_model_mesh` system (in `shell::reactive`)
/// inserts `Mesh3d` + `MeshMaterial3d` once the asset is ready.
#[derive(Component, Deref)]
pub struct ModelAssetHandle(pub Handle<super::asset::ModelAsset>);
