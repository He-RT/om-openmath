//! Shared sessions and notebook kernel protocol.
#![forbid(unsafe_code)]

/// Shared configuration and key masking.
pub mod config;
/// Shared JSON transport types.
pub mod protocol;

mod views;
mod wire;
