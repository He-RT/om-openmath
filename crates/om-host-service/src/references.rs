//! Runtime-local opaque references, immutable values, strict scope and bounded retention.
use hmac::{Hmac, KeyInit, Mac};
use serde::Serialize;
use sha2_ref::Sha256;
use std::{collections::BTreeMap, sync::Arc};

/// Only trusted host bindings can construct this value; it is not an argument exposed to a model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReferenceScope {
    /// Trusted current process/runtime identity.
    pub runtime: String,
    /// Trusted current document identity.
    pub document: String,
    /// Open generation.
    pub generation: u64,
    /// Committed source revision.
    pub revision: u64,
    /// Accepted source/Math execution epoch.
    pub execution_epoch: u64,
    /// Source snapshot provenance.
    pub snapshot_hash: String,
    /// Current task identity, if any.
    pub task: Option<String>,
    /// Current task lifetime.
    pub task_generation: u64,
    /// Permission/grant revision.
    pub grant_revision: u64,
    /// Current mathematical settings revision.
    pub config_revision: u64,
    /// Accepted mathematical definition-state revision.
    pub definition_revision: u64,
    /// Real function metadata revision.
    pub metadata_revision: u64,
    /// Native editor barrier/draft provenance; empty means no edit is currently in progress.
    pub editor_state_hash: String,
    /// Actual task is in execution mode; discussion may still inspect a patch.
    pub execution_mode: bool,
    /// Host grants allow current source reading.
    pub can_read: bool,
    /// Host grants allow temporary source inspection.
    pub can_preview: bool,
    /// Host grants allow committing source; separate from inspecting it.
    pub can_write: bool,
}
impl ReferenceScope {
    /// Verify counters and actual provenance formats, without granting any permission.
    pub fn validate(&self) -> Result<(), ReferenceError> {
        let id = |s: &str| {
            !s.is_empty()
                && s.len() <= 256
                && s.bytes()
                    .enumerate()
                    .all(|(i, c)| c.is_ascii_alphanumeric() || (i > 0 && b"._:-".contains(&c)))
        };
        let hash = |s: &str| {
            s.len() == 64
                && s.bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        };
        if !id(&self.runtime)
            || !id(&self.document)
            || self.task.as_ref().is_some_and(|s| !id(s))
            || !hash(&self.snapshot_hash)
            || (!self.editor_state_hash.is_empty() && !hash(&self.editor_state_hash))
            || self.generation == 0
            || [
                self.generation,
                self.revision,
                self.execution_epoch,
                self.task_generation,
                self.grant_revision,
                self.config_revision,
                self.definition_revision,
                self.metadata_revision,
            ]
            .iter()
            .any(|&n| n > crate::protocol::MAX_SERIAL)
        {
            return Err(ReferenceError::Invalid);
        }
        Ok(())
    }
}
/// Reference purpose is checked independently from shape and shared document scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ReferenceKind {
    /// Source snapshot, up to 10 minutes.
    Snapshot,
    /// Frozen patch, up to 5 minutes.
    Preview,
}
impl ReferenceKind {
    fn prefix(self) -> &'static str {
        match self {
            Self::Snapshot => "snapshot",
            Self::Preview => "preview",
        }
    }
    fn maximum_ms(self) -> u64 {
        match self {
            Self::Snapshot => 600_000,
            Self::Preview => 300_000,
        }
    }
}
/// Known reference failures never authorize replay under a fresh operation ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReferenceError {
    /// Wrong shape, signature, issuer or missing identity.
    Invalid,
    /// Purpose mismatch.
    WrongKind,
    /// Required current owner binding differs.
    StaleScope,
    /// Passed the exact monotonic deadline.
    Expired,
    /// Revoked by task/document/lifecycle owner.
    Revoked,
    /// No supplied grant allows this operation.
    PermissionDenied,
    /// Count/byte/counter/time budget exceeded.
    Budget,
}
struct Entry<T> {
    kind: ReferenceKind,
    scope: ReferenceScope,
    issued: u64,
    expires: u64,
    value: Option<Arc<T>>,
    bytes: usize,
    revoked: bool,
}
/// Immutable values keyed by HMAC-authenticated opaque identities. The 32-byte key comes only
/// from host secure entropy per runtime; deterministic keys are permitted solely in own tests.
pub struct ReferenceRegistry<T> {
    key: [u8; 32],
    counter: u64,
    entries: BTreeMap<String, Entry<T>>,
    bytes: usize,
}
impl<T: Serialize> ReferenceRegistry<T> {
    /// Create a fresh runtime-local issuer. Never serialize or log the key to Pi/model context.
    pub fn new(host_key: [u8; 32]) -> Self {
        Self {
            key: host_key,
            counter: 0,
            entries: BTreeMap::new(),
            bytes: 0,
        }
    }
    /// Derive a unique trusted ID for a cell/operation/transaction, never from model client_key.
    pub fn allocate_id(&mut self, purpose: &str) -> Result<String, ReferenceError> {
        if purpose.is_empty()
            || purpose.len() > 32
            || !purpose.bytes().all(|c| c.is_ascii_alphanumeric())
        {
            return Err(ReferenceError::Invalid);
        }
        self.counter = self
            .counter
            .checked_add(1)
            .filter(|&n| n <= crate::protocol::MAX_SERIAL)
            .ok_or(ReferenceError::Budget)?;
        let mut mac =
            Hmac::<Sha256>::new_from_slice(&self.key).map_err(|_| ReferenceError::Invalid)?;
        mac.update(b"openmath-native-id-v1\0");
        mac.update(purpose.as_bytes());
        mac.update(&self.counter.to_be_bytes());
        let tag = mac.finalize().into_bytes();
        Ok(format!(
            "{purpose}-{:x}-{}",
            self.counter,
            tag.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ))
    }
    /// Freeze a validated scope/value; source and preview TTL limits cannot be expanded by a caller.
    pub fn issue(
        &mut self,
        kind: ReferenceKind,
        scope: ReferenceScope,
        value: T,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<String, ReferenceError> {
        scope.validate()?;
        if now_ms > crate::protocol::MAX_SERIAL || ttl_ms == 0 || ttl_ms > kind.maximum_ms() {
            return Err(ReferenceError::Budget);
        }
        self.prune(now_ms);
        if self.entries.len() >= 128 {
            return Err(ReferenceError::Budget);
        }
        let bytes = serde_json::to_vec(&value)
            .map_err(|_| ReferenceError::Invalid)?
            .len();
        if self
            .bytes
            .checked_add(bytes)
            .is_none_or(|n| n > 32 * 1024 * 1024)
        {
            return Err(ReferenceError::Budget);
        }
        let expires = now_ms
            .checked_add(ttl_ms)
            .filter(|&n| n <= crate::protocol::MAX_SERIAL)
            .ok_or(ReferenceError::Budget)?;
        let token = self.allocate_id(kind.prefix())?;
        self.bytes += bytes;
        self.entries.insert(
            token.clone(),
            Entry {
                kind,
                scope,
                issued: now_ms,
                expires,
                value: Some(Arc::new(value)),
                bytes,
                revoked: false,
            },
        );
        Ok(token)
    }
    /// Resolve the original immutable plan, never a reconstructed latest draft.
    pub fn resolve(
        &self,
        token: &str,
        kind: ReferenceKind,
        current: &ReferenceScope,
        now_ms: u64,
    ) -> Result<Arc<T>, ReferenceError> {
        current.validate()?;
        self.verify(token)?;
        let entry = self.entries.get(token).ok_or(ReferenceError::Invalid)?;
        if entry.kind != kind {
            return Err(ReferenceError::WrongKind);
        }
        if entry.revoked {
            return Err(ReferenceError::Revoked);
        }
        if now_ms < entry.issued || now_ms > crate::protocol::MAX_SERIAL {
            return Err(ReferenceError::Invalid);
        }
        if now_ms >= entry.expires {
            return Err(ReferenceError::Expired);
        }
        if entry.scope != *current {
            return Err(ReferenceError::StaleScope);
        }
        entry.value.clone().ok_or(ReferenceError::Expired)
    }
    /// Revoking releases the heavy value immediately but keeps an explicit bounded tombstone.
    pub fn revoke(&mut self, token: &str) -> Result<(), ReferenceError> {
        self.verify(token)?;
        let entry = self.entries.get_mut(token).ok_or(ReferenceError::Invalid)?;
        entry.revoked = true;
        entry.value = None;
        self.bytes -= entry.bytes;
        entry.bytes = 0;
        Ok(())
    }
    /// Release unusable temporary values when trusted scope changes, keeping the issuer counter.
    /// Durable operation identity and receipt retention belong to the document operation owner.
    pub fn retain_scope(&mut self, current: &ReferenceScope) {
        self.entries.retain(|_, entry| {
            let keep = entry.scope == *current;
            if !keep {
                self.bytes -= entry.bytes;
            }
            keep
        });
    }
    fn verify(&self, token: &str) -> Result<(), ReferenceError> {
        let mut parts = token.split('-');
        let purpose = parts.next().ok_or(ReferenceError::Invalid)?;
        let counter = u64::from_str_radix(parts.next().ok_or(ReferenceError::Invalid)?, 16)
            .map_err(|_| ReferenceError::Invalid)?;
        let raw = parts
            .next()
            .filter(|s| {
                s.len() == 64
                    && s.bytes()
                        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            })
            .ok_or(ReferenceError::Invalid)?;
        if parts.next().is_some() || counter == 0 {
            return Err(ReferenceError::Invalid);
        }
        let mut tag = [0; 32];
        for (i, slot) in tag.iter_mut().enumerate() {
            *slot = u8::from_str_radix(&raw[i * 2..i * 2 + 2], 16)
                .map_err(|_| ReferenceError::Invalid)?;
        }
        let mut mac =
            Hmac::<Sha256>::new_from_slice(&self.key).map_err(|_| ReferenceError::Invalid)?;
        mac.update(b"openmath-native-id-v1\0");
        mac.update(purpose.as_bytes());
        mac.update(&counter.to_be_bytes());
        mac.verify_slice(&tag).map_err(|_| ReferenceError::Invalid)
    }
    fn prune(&mut self, now: u64) {
        // Expired refs may be discarded: only durable ledger IDs must survive forever.
        self.entries.retain(|_, entry| {
            let keep = !entry.revoked && now < entry.expires;
            if !keep {
                self.bytes -= entry.bytes;
            }
            keep
        });
    }
}
