//! Kernel state capture/read/restore. Registering temporary bytes never certifies durable acceptance.
/// Frozen actual candidates, final cancellation barrier and trusted physical receipt verification.
pub mod acceptance;
/// Main mathematics worker; it freezes candidates but never decides durable acceptance.
pub mod worker;
use om_kernel::{
    Session, WorkingSession,
    checkpoint::{CheckpointBinding, CheckpointError, CheckpointLimits, CheckpointRestore},
    config::GeneralConfig,
    protocol::{NotebookFile, Request, Response},
};
use om_num::ctx::Interrupt;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc};

/// Immutable actual captured bytes and their trusted provenance/source/config.
pub struct KernelState {
    binding: CheckpointBinding,
    source: NotebookFile,
    general: GeneralConfig,
    bytes: Arc<[u8]>,
    hash: String,
    reserved: usize,
}
impl KernelState {
    /// Import verified original bytes without executing source. Full codec validation occurs before
    /// any state is returned; trusted caller must additionally verify durable receipt/current scope.
    pub fn import(
        bytes: Vec<u8>,
        binding: CheckpointBinding,
        source: NotebookFile,
        general: GeneralConfig,
        limits: CheckpointLimits,
        ctx: &Interrupt,
    ) -> Result<Self, KernelStateError> {
        let _validated = Session::decode_checkpoint(
            &bytes,
            CheckpointRestore {
                binding: &binding,
                source: &source,
                general: &general,
                clock: None,
                cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            },
            limits,
            ctx,
        )?;
        let reserved = bytes
            .len()
            .checked_add(source.title.len())
            .and_then(|n| {
                source.cells.iter().try_fold(n, |n, c| {
                    n.checked_add(c.id.len())
                        .and_then(|n| n.checked_add(c.source.len()))
                })
            })
            .and_then(|n| n.checked_add(1024))
            .ok_or(KernelStateError::Limit)?;
        Ok(Self {
            binding,
            source,
            general,
            hash: digest(&bytes),
            bytes: bytes.into(),
            reserved,
        })
    }
    /// Freeze an actual idle session off the registry lock. This alone grants no active status.
    pub fn capture(
        session: &mut Session,
        binding: CheckpointBinding,
        limits: CheckpointLimits,
        ctx: &Interrupt,
    ) -> Result<Self, KernelStateError> {
        ctx.tick()?;
        let Response::NotebookState { state } = session.handle(Request::GetNotebookState).0 else {
            return Err(KernelStateError::Invalid);
        };
        let bytes = session.encode_checkpoint(&binding, limits, ctx)?;
        let reserved = bytes
            .len()
            .checked_add(state.file.title.len())
            .and_then(|n| {
                state.file.cells.iter().try_fold(n, |n, c| {
                    n.checked_add(c.id.len())
                        .and_then(|n| n.checked_add(c.source.len()))
                })
            })
            .and_then(|n| n.checked_add(1024))
            .ok_or(KernelStateError::Limit)?;
        Ok(Self {
            binding,
            source: state.file,
            general: session.config.general.clone(),
            hash: digest(&bytes),
            bytes: bytes.into(),
            reserved,
        })
    }
    /// Actual captured source; it may precede the current committed document revision.
    pub fn source(&self) -> &NotebookFile {
        &self.source
    }
    /// Actual captured calculation/display configuration, with no credential profiles.
    pub fn general(&self) -> &GeneralConfig {
        &self.general
    }
    /// Actual producer/source/math state binding, not a model request.
    pub fn binding(&self) -> &CheckpointBinding {
        &self.binding
    }
    /// Actual encoded bytes for the trusted host Blob writer. Borrowing does not publish them.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// SHA256 of those exact bytes, checked again on restoration or Blob import.
    pub fn hash(&self) -> &str {
        &self.hash
    }
    /// Restore a new owned work candidate from the original immutable bytes, no source replay.
    pub fn restore(
        &self,
        restore: CheckpointRestore<'_>,
        limits: CheckpointLimits,
        ctx: &Interrupt,
    ) -> Result<WorkingSession, KernelStateError> {
        if restore.binding != &self.binding
            || serde_json::to_value(restore.source).map_err(|_| KernelStateError::Invalid)?
                != serde_json::to_value(&self.source).map_err(|_| KernelStateError::Invalid)?
            || serde_json::to_value(restore.general).map_err(|_| KernelStateError::Invalid)?
                != serde_json::to_value(&self.general).map_err(|_| KernelStateError::Invalid)?
            || digest(&self.bytes) != self.hash
        {
            return Err(KernelStateError::Invalid);
        }
        Ok(Session::decode_checkpoint(
            &self.bytes,
            restore,
            limits,
            ctx,
        )?)
    }
}
/// No partial/unscoped imported state; no capacity failure is reported as an accepted checkpoint.
#[derive(Debug)]
pub enum KernelStateError {
    /// Wrong scope/source/config/hash or missing/revoked state.
    Invalid,
    /// Bytes/states/lifetime budget exceeded.
    Limit,
    /// Actual underlying checkpoint decode/interrupt failure.
    Checkpoint(CheckpointError),
    /// Actual host cancellation/budget.
    Abort(om_num::ctx::Abort),
}
impl From<CheckpointError> for KernelStateError {
    fn from(error: CheckpointError) -> Self {
        Self::Checkpoint(error)
    }
}
impl From<om_num::ctx::Abort> for KernelStateError {
    fn from(error: om_num::ctx::Abort) -> Self {
        Self::Abort(error)
    }
}
impl std::fmt::Display for KernelStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid => f.write_str("invalid kernel state"),
            Self::Limit => f.write_str("kernel state pool is full"),
            Self::Checkpoint(e) => e.fmt(f),
            Self::Abort(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for KernelStateError {}
/// Bounded temporary checkpoints. Retired but externally pinned payloads keep their reservation.
pub struct KernelStatePool {
    entries: BTreeMap<String, Arc<KernelState>>,
    retired: Vec<Arc<KernelState>>,
    bytes: usize,
    max_bytes: usize,
    max_states: usize,
    created: u32,
}
impl KernelStatePool {
    /// Host-owned pool; normal budgets are 64MiB and 32 simultaneous/reserved states.
    pub fn new(max_bytes: usize, max_states: usize) -> Result<Self, KernelStateError> {
        if max_bytes == 0 || max_bytes > 64 * 1024 * 1024 || max_states == 0 || max_states > 32 {
            return Err(KernelStateError::Limit);
        }
        Ok(Self {
            entries: BTreeMap::new(),
            retired: Vec::new(),
            bytes: 0,
            max_bytes,
            max_states,
            created: 0,
        })
    }
    /// Freeze the real idle Session, assign a host-only opaque ID, and retain immutable source scope.
    pub fn capture(
        &mut self,
        session: &mut Session,
        binding: CheckpointBinding,
        limits: CheckpointLimits,
        ctx: &Interrupt,
    ) -> Result<String, KernelStateError> {
        ctx.tick()?;
        let state = KernelState::capture(session, binding, limits, ctx)?;
        self.register(state)
    }
    /// Insert already frozen bytes. Encoding/math never takes place while a shared pool is locked.
    /// The ID denotes a temporary state; coordinator/physical persistence must still accept it.
    pub fn register(&mut self, state: KernelState) -> Result<String, KernelStateError> {
        self.reap();
        if self.entries.len() + self.retired.len() >= self.max_states || self.created >= 4096 {
            return Err(KernelStateError::Limit);
        }
        if self
            .bytes
            .checked_add(state.reserved)
            .is_none_or(|n| n > self.max_bytes)
        {
            return Err(KernelStateError::Limit);
        }
        let mut id = [0; 16];
        getrandom::fill(&mut id).map_err(|_| KernelStateError::Invalid)?;
        let id = format!(
            "kernel-state-{}",
            id.iter().map(|b| format!("{b:02x}")).collect::<String>()
        );
        if self.entries.contains_key(&id) {
            return Err(KernelStateError::Invalid);
        }
        self.bytes += state.reserved;
        self.created += 1;
        self.entries.insert(id.clone(), Arc::new(state));
        Ok(id)
    }
    /// Read original trusted bytes; a guessed ID cannot resolve a new object or create a write grant.
    pub fn state(&self, id: &str) -> Result<Arc<KernelState>, KernelStateError> {
        self.entries
            .get(id)
            .cloned()
            .ok_or(KernelStateError::Invalid)
    }
    /// Revoke lookup now; outstanding caller pins retain bytes and pool charge until released.
    pub fn revoke(&mut self, id: &str) -> Result<(), KernelStateError> {
        let state = self.entries.remove(id).ok_or(KernelStateError::Invalid)?;
        self.retired.push(state);
        self.reap();
        Ok(())
    }
    /// Release actual no-longer-pinned reservations; this is not physical Blob/database GC.
    pub fn reap(&mut self) {
        self.retired.retain(|state| {
            let keep = Arc::strong_count(state) > 1;
            if !keep {
                self.bytes -= state.reserved;
            }
            keep
        });
    }
    /// Actual currently reserved bytes, including revoked caller-pinned states.
    pub fn reserved_bytes(&self) -> usize {
        self.bytes
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
