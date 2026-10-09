//! Pure authoritative source records and frozen persistence plans; no CAS execution or disk IO.
/// Typed source batch validation and dependency invalidation.
pub mod coordinator;
mod hashes;
/// Frozen source/patch previews; not a tool registration or durable write.
pub mod preview;
mod source;
pub use hashes::{request_hash, snapshot_hash};
pub use source::{SourceDocument, validate_commit, validate_snapshot};
