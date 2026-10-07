//! Shared sessions and notebook kernel protocol.
#![forbid(unsafe_code)]

/// Pure LLM proposal verification and completion post-processing at the CAS host layer.
pub mod assistant;

/// Executable function descriptions and host capabilities.
pub mod capabilities;
/// Shared configuration and key masking.
pub mod config;
/// Explicit host-native configuration IO and credential resolution.
#[cfg(feature = "native")]
pub mod native;
/// Shared JSON transport types.
pub mod protocol;

/// Pure vector/raster and structured data artifact generation.
pub mod artifact;
mod artifact_views;
mod dependency;
mod editor;
mod explore;
mod explore_views;
mod notebook;
mod output;
mod plot;
mod plot_views;
mod scene3d;
mod scene_graph;
mod scene_views;
mod session;
mod value_views;
mod views;
mod wire;

pub use config::KernelConfig;
pub use notebook::{Cell, Notebook};
pub use session::{LlmCancellation, Session};
