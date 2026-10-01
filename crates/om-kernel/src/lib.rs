//! Shared sessions and notebook kernel protocol.
#![forbid(unsafe_code)]

/// Shared configuration and key masking.
pub mod config;
/// Shared JSON transport types.
pub mod protocol;

mod dependency;
mod notebook;
mod session;
mod views;
mod wire;

pub use config::KernelConfig;
pub use notebook::{Cell, Notebook};
pub use session::Session;
