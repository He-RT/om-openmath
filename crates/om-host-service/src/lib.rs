//! Framework-independent native host protocol and authoritative coordination services.
#![forbid(unsafe_code)]
/// Independent native single-turn model codec channel; no network or authority.
pub mod model;
pub mod protocol;

/// Independent native owners, queues and operation cancellation.
pub mod scheduler;

/// Authoritative source snapshots and typed frozen persistence plans.
pub mod document;
/// Temporary actual kernel checkpoints and scoped owned restore, independent of document IO.
pub mod kernel;

/// Private Agent transport and trusted task identity; no model/tool executor lives here.
pub mod agent;
/// Host-owned scoped immutable references and deadlines.
pub mod references;
/// Immutable actual result ownership, scope-checked references and read-only projections.
pub mod results;
