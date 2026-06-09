//! bse_model: voxel model format (RON) and Bevy integration.

pub mod mesher;
pub mod schema;

#[cfg(feature = "bevy")]
pub mod asset;
#[cfg(feature = "bevy")]
pub mod material;
#[cfg(feature = "bevy")]
pub mod plugin;

#[cfg(feature = "vox-dev")]
pub mod dev;

#[cfg(feature = "bevy")]
pub use asset::{ModelAsset, ModelLoadError, ModelLoader};
#[cfg(feature = "bevy")]
pub use plugin::{ModelAssetHandle, ModelLoaderPlugin};
