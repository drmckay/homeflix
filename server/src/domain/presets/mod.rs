//! Preset Collections Module
//!
//! Contains curated franchise collections that mix movies and TV series
//! with proper timeline ordering.

mod franchises;

pub use franchises::{PresetCollection, PresetCollectionItem, get_all_presets};
