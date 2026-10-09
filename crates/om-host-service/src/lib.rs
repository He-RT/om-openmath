//! Framework-independent native host protocol and authoritative coordination services.
#![forbid(unsafe_code)]
pub mod protocol;

/// Independent native owners, queues and operation cancellation.
pub mod scheduler;

/// Authoritative source snapshots and typed frozen persistence plans.
pub mod document;

/// Host-owned scoped immutable references and deadlines.
pub mod references;
