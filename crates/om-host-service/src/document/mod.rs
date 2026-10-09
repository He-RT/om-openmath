//! Pure authoritative source records and frozen persistence plans; no CAS execution or disk IO.
mod hashes;
mod source;
pub use hashes::{request_hash, snapshot_hash};
pub use source::{SourceDocument, validate_commit, validate_snapshot};
