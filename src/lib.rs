//! Single-crate workspace for Bite-Sized Earth.
//!
//! Two-layer architecture enforced by convention + CI grep lint:
//! - [`sim`]  — game logic (components, systems, world grid)
//! - [`shell`] — rendering and input (depends on `sim`)
//! - [`model`] — voxel model loader (used by `shell`)
//!
//! Cross-layer direction: `shell` → `sim` (never the reverse).

pub mod model;
pub mod shell;
pub mod sim;

pub use shell::ShellPlugin;
pub use sim::SimPlugin;
