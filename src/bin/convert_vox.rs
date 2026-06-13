//! `convert-vox`: dev CLI. Reads a MagicaVoxel `.vox` file, emits a
//! face-list `.ron` for the runtime.
//!
//! Usage: `convert-vox <in.vox> [out.ron]`
//!
//! Material names are written as `__mat_N` placeholders. The artist renames
//! them in the .ron, and the corresponding texture lives at
//! `assets/textures/<name>.png`.

use std::path::PathBuf;

use bite_sized_earth::model::dev::{convert, vox_import};

fn main() {
  let args: Vec<String> = std::env::args().collect();
  if args.len() < 2 {
    eprintln!("usage: convert-vox <in.vox> [out.ron]");
    std::process::exit(1);
  }
  let in_path = PathBuf::from(&args[1]);
  let out_path = if args.len() >= 3 {
    PathBuf::from(&args[2])
  } else {
    in_path.with_extension("ron")
  };

  let name = in_path
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or("model")
    .to_string();

  let grid = match vox_import::load(&in_path) {
    Ok(g) => g,
    Err(e) => {
      eprintln!("failed to load {}: {e}", in_path.display());
      std::process::exit(1);
    }
  };
  let def = convert::vox_to_model_def(&name, &grid);
  let pretty = ron::ser::PrettyConfig::default()
    .depth_limit(4)
    .separate_tuple_members(true);
  let text = match ron::ser::to_string_pretty(&def, pretty) {
    Ok(s) => s,
    Err(e) => {
      eprintln!("failed to serialize RON: {e}");
      std::process::exit(1);
    }
  };
  if let Err(e) = std::fs::write(&out_path, text) {
    eprintln!("failed to write {}: {e}", out_path.display());
    std::process::exit(1);
  }
  println!(
    "wrote {} ({} faces, {} materials)",
    out_path.display(),
    def.faces.len(),
    def.materials.len()
  );
}
