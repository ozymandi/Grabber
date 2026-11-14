//! Texture atlas (OpenSpec change add-texture-atlas): workspace model, storage,
//! automatic placement and export rendering. Tauri-free so it can be unit-tested.

pub mod model;
pub mod store;
pub mod place;
pub mod render;
pub mod files;
pub mod edit;
pub mod history;
