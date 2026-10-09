//! Framework-independent native host protocol and authoritative coordination services.
#![forbid(unsafe_code)]
pub mod protocol;

/// Independent native owners, queues and operation cancellation.
pub mod scheduler;
