//! Per-`.vox` 256-color palette → 16×16 sRGB `Image`.
//!
//! Mirrors `bevy_vox_scene`'s 16×16 atlas behaviour: each palette index `i`
//! (1..=255) lands at cell `(i % 16, i / 16)` of the image. Cell (0, 0) is
//! left opaque black for the (unused) empty palette slot.
//!
//! Crucially the 256 colors come from the **`.vox` file's own palette**
//! (`DotVoxData.palette`), not from `dot_vox::DEFAULT_PALETTE`. Most
//! MagicaVoxel models override the default palette at save time, so reading
//! the wrong source produces visibly wrong colors (the symptom: every voxel
//! renders as a default-palette hue instead of the model's actual colors).
//!
//! The image is built inside [`crate::vox::asset::VoxLoader::load`] and
//! registered via `LoadContext::add_labeled_asset` so it travels with the
//! `VoxelAsset`.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

const ATLAS: u32 = 16;

/// Build a 16×16 RGBA8 sRGB image from a `.vox` file's 256-color palette.
/// The palette slice must be 256 entries long (`.vox` files always carry
/// exactly 256 colors); shorter slices are padded with opaque black.
pub fn build_palette_image(palette: &[dot_vox::Color]) -> Image {
  let mut data = vec![0u8; (ATLAS * ATLAS * 4) as usize];
  for y in 0..ATLAS {
    for x in 0..ATLAS {
      let idx = (y * ATLAS + x) as usize;
      let (r, g, b, a) = if let Some(c) = palette.get(idx) {
        (c.r, c.g, c.b, c.a)
      } else {
        (0, 0, 0, 255)
      };
      let off = ((y * ATLAS + x) * 4) as usize;
      data[off] = r;
      data[off + 1] = g;
      data[off + 2] = b;
      data[off + 3] = a;
    }
  }
  Image::new(
    Extent3d {
      width: ATLAS,
      height: ATLAS,
      depth_or_array_layers: 1,
    },
    TextureDimension::D2,
    data,
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::RENDER_WORLD,
  )
}

/// Look up a UV (`Vec2`) for a given palette index (1..=255). Mirrors the
/// formula used by the mesher: `((idx % 16) + 0.5) / 16`.
pub fn uv_for_index(idx: u8) -> Vec2 {
  let u = ((idx % 16) as f32 + 0.5) / 16.0;
  let v = ((idx / 16) as f32 + 0.5) / 16.0;
  Vec2::new(u, v)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn fake_palette() -> Vec<dot_vox::Color> {
    (0..256)
      .map(|i| dot_vox::Color {
        r: i as u8,
        g: (i as u8).wrapping_mul(2),
        b: 0,
        a: 255,
      })
      .collect()
  }

  #[test]
  fn image_is_16x16_rgba() {
    let img = build_palette_image(&fake_palette());
    assert_eq!(img.texture_descriptor.size.width, 16);
    assert_eq!(img.texture_descriptor.size.height, 16);
    assert_eq!(img.texture_descriptor.format, TextureFormat::Rgba8UnormSrgb);
  }

  #[test]
  fn cell_0_0_uses_palette_index_0() {
    let img = build_palette_image(&fake_palette());
    let data = img.data.as_ref().expect("image has cpu-side data");
    let r = data[0];
    let g = data[1];
    let b = data[2];
    let a = data[3];
    assert_eq!((r, g, b, a), (0, 0, 0, 255), "cell (0,0) should be index 0");
  }

  #[test]
  fn cell_1_0_uses_palette_index_1() {
    let img = build_palette_image(&fake_palette());
    let data = img.data.as_ref().expect("image has cpu-side data");
    // cell (1, 0): offset 4 bytes
    let r = data[4];
    let g = data[5];
    let b = data[6];
    let a = data[7];
    assert_eq!((r, g, b, a), (1, 2, 0, 255), "cell (1,0) should be index 1");
  }

  #[test]
  fn uv_index_17() {
    let uv = uv_for_index(17);
    // idx 17 → idx%16=1, idx/16=1 → u = v = (1 + 0.5) / 16 = 0.09375
    assert!((uv.x - 0.093_75).abs() < 1e-6, "u = {}", uv.x);
    assert!((uv.y - 0.093_75).abs() < 1e-6, "v = {}", uv.y);
  }

  #[test]
  fn uv_index_0() {
    let uv = uv_for_index(0);
    assert!((uv.x - 0.031_25).abs() < 1e-6);
    assert!((uv.y - 0.031_25).abs() < 1e-6);
  }
}
