//! Core of ripper, kept free of Tauri so it can be unit- and integration-tested quickly.

pub mod atlas;
pub mod clipboard;
pub mod export;
pub mod geom;
pub mod launch;
pub mod mask;
pub mod mcp;
pub mod settings;
pub mod shape;
pub mod source;
pub mod update_policy;
pub mod warp;
