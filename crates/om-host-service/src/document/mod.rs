//! Pure authoritative source records and frozen persistence plans; no CAS execution or disk IO.
/// Native editor/source admission and original-ID commit transitions.
pub mod commit;
/// Typed source batch validation and dependency invalidation.
pub mod coordinator;
/// Trusted native source/control bridge, separate from physical IO.
pub mod endpoint;
mod hashes;
/// Frozen source/patch previews; not a tool registration or durable write.
pub mod preview;
mod source;
/// Checked inverse-source merge; durable undo/permissions remain separate host responsibilities.
pub mod undo;
pub use hashes::{request_hash, snapshot_hash};
pub use source::{SourceDocument, validate_commit, validate_snapshot};
