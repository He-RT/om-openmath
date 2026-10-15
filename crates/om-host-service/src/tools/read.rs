//! Bounded source reads from the confirmed owner and immutable accepted checkpoint facts.
//! This service issues real preview snapshots; a source fragment never grants whole-cell exposure.
use crate::{
    document::{commit::DocumentCommitController, coordinator::SourceCoordinator},
    kernel::acceptance::AcceptedKernelState,
    protocol::{generated::*, validation},
    references::{ReferenceKind, ReferenceRegistry, ReferenceScope},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const PAGE_BYTES: usize = 32 * 1024;
const RETAIN_BYTES: usize = 32 * 1024 * 1024;
const TTL: u64 = 600_000;

/// A readonly failure never produces a source mutation or mathematical work item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadError {
    /// Invalid closed arguments, view selection or range.
    InvalidArgument,
    /// Unknown, expired, cross-task or changed-authority reference/cursor.
    InvalidReference,
    /// Reading is not granted by the current native owner.
    PermissionDenied,
    /// Trusted current source binding differs from the owner.
    StaleSnapshot,
    /// One of the selected stable cell identities does not exist in this snapshot.
    NotFound,
    /// Reference count, retained bytes or monotonic clock budget exceeded.
    LimitExceeded,
    /// Actual result references must come from the native retained-result producer, still unbound.
    NotAvailable,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum View {
    #[default]
    Outline,
    Cells,
    Symbols,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    view: Option<View>,
    cell_ids: Option<Vec<String>>,
    snapshot_ref: Option<String>,
    cursor: Option<String>,
    include_outputs: Option<bool>,
    limit: Option<usize>,
}
#[derive(Clone, Serialize)]
struct Cursor {
    snapshot: String,
    view: View,
    ids: Option<Vec<String>>,
    outputs: bool,
    limit: usize,
    index: usize,
    byte_offset: usize,
}
#[derive(Clone, Serialize)]
struct Snapshot {
    scope: ReferenceScope,
    source: NativeSourceSnapshot,
    coordinator: SourceCoordinator,
    statuses: BTreeMap<String, Value>,
    symbols: Vec<Value>,
    recent_transaction_refs: Vec<String>,
    complete: BTreeSet<String>,
    issued: u64,
    expires: u64,
}
struct Retained {
    snapshot: Snapshot,
    bytes: usize,
}
/// A successful page is data, not an admission or execution receipt.
#[derive(Debug, Serialize)]
pub struct ReadPage {
    /// Original frozen source revision, for the native outer result envelope only.
    #[serde(skip)]
    pub source_revision: u64,
    /// The original snapshot's accepted-state fence; no later random draw may replace its output.
    #[serde(skip)]
    pub kernel_state_revision: u64,
    /// Actual requested output enrichment is performed on the independent result owner.
    #[serde(skip)]
    pub include_outputs: bool,
    /// Closed read_notebook_data shape.
    pub data: Value,
    /// Explicit truncation and an opaque cursor bound to this exact query.
    pub pagination: Value,
}
/// Runtime-local readonly retention. The native document owner supplies scope and source authority.
/// Never restore this object or its grants from a transcript, provider response or persisted JSON.
pub struct ReadNotebookService {
    snapshots: BTreeMap<String, Retained>,
    bytes: usize,
    cursors: ReferenceRegistry<Cursor>,
    history: Option<(ReferenceScope, Vec<String>)>,
}
fn decode(bytes: &[u8]) -> Result<Args, ReadError> {
    let root = super::schema();
    let value = om_agent_contract::json::strict_json_bounded(bytes, 65536)
        .map_err(|_| ReadError::InvalidArgument)?;
    validation::validate(&value, &root["$defs"]["read_notebook_args"], root)
        .map_err(|_| ReadError::InvalidArgument)?;
    serde_json::from_value(value).map_err(|_| ReadError::InvalidArgument)
}
fn same_authority(old: &ReferenceScope, current: &ReferenceScope) -> bool {
    old.can_read
        && current.can_read
        && old.runtime == current.runtime
        && old.document == current.document
        && old.generation == current.generation
        && old.task == current.task
        && old.task_generation == current.task_generation
        && old.grant_revision == current.grant_revision
        && old.metadata_revision == current.metadata_revision
}
fn capture(
    controller: &DocumentCommitController,
    coordinator: &SourceCoordinator,
    accepted: Option<&AcceptedKernelState>,
    scope: &ReferenceScope,
    now: u64,
) -> Result<Snapshot, ReadError> {
    let source = controller.owner().snapshot().clone();
    let mut statuses = BTreeMap::new();
    let mut symbols = vec![];
    if let Some(accepted) = accepted {
        let state = accepted.state();
        let producer = state.binding();
        if producer.document_id != scope.document
            || producer.document_generation != scope.generation
        {
            return Err(ReadError::StaleSnapshot);
        }
        // Title/prose changes preserve actual math state. Keep original producer provenance,
        // but test freshness using the real Math sequence/content and its execution/config epoch.
        let fresh = !om_kernel::source::math_source_changed(
            state.source(),
            &crate::document::coordinator::kernel_file(&source.file),
        ) && producer.execution_epoch == source.execution_epoch.get()
            && accepted.receipt().producer.config_revision.get() == scope.config_revision;
        if fresh {
            for (id, status) in state.cell_states() {
                statuses.insert(id.clone(), json!(status));
            }
        }
        for owner in &state.execution_context().owned {
            symbols.push(json!({"name":owner.symbol,"cell_id":owner.cell_id,"callable":state.execution_context().known_functions.contains(&owner.symbol),"is_current":fresh,"producer":{"source_revision":producer.source_revision,"execution_epoch":producer.execution_epoch,"kernel_state_revision":producer.kernel_state_revision,"snapshot_hash":producer.source_snapshot_hash}}));
        }
        symbols.sort_by(|a, b| {
            (a["name"].as_str(), a["cell_id"].as_str())
                .cmp(&(b["name"].as_str(), b["cell_id"].as_str()))
        });
    }
    Ok(Snapshot {
        scope: scope.clone(),
        source,
        coordinator: coordinator.clone(),
        statuses,
        symbols,
        recent_transaction_refs: controller.recent_transaction_refs(),
        complete: BTreeSet::new(),
        issued: now,
        expires: now
            .checked_add(TTL)
            .filter(|n| *n <= crate::protocol::MAX_SERIAL)
            .ok_or(ReadError::LimitExceeded)?,
    })
}
impl ReadNotebookService {
    /// Resolve the actual read-issued token for an explicit math operation. Historical snapshots
    /// remain readable as history but cannot choose a current execution checkpoint or source.
    /// This never issues a replacement preview token or upgrades full-cell source exposure.
    pub fn current_math_snapshot(
        &mut self,
        reference: &str,
        scope: &ReferenceScope,
        now: u64,
    ) -> Result<NativeSourceSnapshot, ReadError> {
        scope.validate().map_err(|_| ReadError::StaleSnapshot)?;
        if !scope.can_read {
            return Err(ReadError::PermissionDenied);
        }
        self.prune(scope, now);
        let retained = self
            .snapshots
            .get(reference)
            .ok_or(ReadError::InvalidReference)?;
        if now < retained.snapshot.issued {
            return Err(ReadError::InvalidReference);
        }
        if retained.snapshot.scope != *scope {
            return Err(ReadError::StaleSnapshot);
        }
        Ok(retained.snapshot.source.clone())
    }
    /// Undo authority is current read-issued source plus the exact exposed durable history set.
    pub(crate) fn undo_transactions(
        &mut self,
        reference: &str,
        transactions: &[String],
        scope: &ReferenceScope,
        now: u64,
    ) -> Result<NativeSourceSnapshot, ReadError> {
        let source = self.current_math_snapshot(reference, scope, now)?;
        let original = &self
            .snapshots
            .get(reference)
            .ok_or(ReadError::InvalidReference)?
            .snapshot;
        if transactions.is_empty()
            || transactions.len() > 32
            || transactions.iter().collect::<BTreeSet<_>>().len() != transactions.len()
        {
            return Err(ReadError::InvalidArgument);
        }
        if transactions
            .iter()
            .any(|id| !original.recent_transaction_refs.contains(id))
        {
            return Err(ReadError::PermissionDenied);
        }
        Ok(source)
    }
    /// Fresh entropy is supplied by the trusted native host (deterministic only in own fixtures).
    pub fn new(host_key: [u8; 32]) -> Self {
        Self {
            snapshots: BTreeMap::new(),
            bytes: 0,
            cursors: ReferenceRegistry::new(host_key),
            history: None,
        }
    }
    /// Trusted native IO callback after independent durable history/head readback. Refs are data,
    /// never transaction write/undo grants; the exact current snapshot must still match at read.
    pub fn bind_history(
        &mut self,
        scope: &ReferenceScope,
        refs: Vec<String>,
    ) -> Result<(), ReadError> {
        scope.validate().map_err(|_| ReadError::StaleSnapshot)?;
        if !scope.can_read {
            return Err(ReadError::PermissionDenied);
        }
        if refs.len() > 32
            || refs.iter().collect::<BTreeSet<_>>().len() != refs.len()
            || refs.iter().any(|s| {
                s.is_empty()
                    || s.len() > 256
                    || !s
                        .bytes()
                        .enumerate()
                        .all(|(i, c)| c.is_ascii_alphanumeric() || (i > 0 && b"._:-".contains(&c)))
            })
        {
            return Err(ReadError::InvalidArgument);
        }
        self.history = Some((scope.clone(), refs));
        Ok(())
    }
    fn prune(&mut self, scope: &ReferenceScope, now: u64) {
        self.snapshots.retain(|_, entry| {
            let keep = now < entry.snapshot.expires && same_authority(&entry.snapshot.scope, scope);
            if !keep {
                self.bytes -= entry.bytes;
            }
            keep
        });
        self.cursors.retain_scope(scope);
    }
    fn retain(&mut self, reference: String, snapshot: Snapshot, bytes: usize) {
        self.bytes += bytes;
        self.snapshots
            .insert(reference, Retained { snapshot, bytes });
    }
    /// Read only confirmed source. No evaluator/Session request, IO, source update or random draw.
    /// Current complete cells receive a NEW real preview snapshot; old excerpts remain excerpts.
    pub fn read(
        &mut self,
        bytes: &[u8],
        controller: &mut DocumentCommitController,
        coordinator: &SourceCoordinator,
        accepted: Option<&AcceptedKernelState>,
        scope: &ReferenceScope,
        now: u64,
    ) -> Result<ReadPage, ReadError> {
        let args = decode(bytes)?;
        scope.validate().map_err(|_| ReadError::StaleSnapshot)?;
        if !scope.can_read {
            return Err(ReadError::PermissionDenied);
        }
        let owner = controller.owner().snapshot();
        if scope.document != owner.document_id
            || scope.generation != controller.owner().generation().get()
            || scope.revision != owner.revision.get()
            || scope.execution_epoch != owner.execution_epoch.get()
            || scope.snapshot_hash != owner.snapshot_hash
            || scope.metadata_revision
                != u64::from(om_kernel::capabilities::function_catalog().metadata_version)
        {
            return Err(ReadError::StaleSnapshot);
        }
        self.prune(scope, now);
        let query = if let Some(token) = &args.cursor {
            let original = self
                .cursors
                .resolve(token, ReferenceKind::Cursor, scope, now)
                .map_err(|_| ReadError::InvalidReference)?;
            if args
                .snapshot_ref
                .as_ref()
                .is_some_and(|s| s != &original.snapshot)
                || args.view.is_some_and(|v| v != original.view)
                || args
                    .cell_ids
                    .as_ref()
                    .is_some_and(|ids| Some(ids) != original.ids.as_ref())
                || args.limit.is_some_and(|n| n != original.limit)
                || args.include_outputs.is_some_and(|b| b != original.outputs)
            {
                return Err(ReadError::InvalidArgument);
            }
            (*original).clone()
        } else {
            let view = args.view.unwrap_or_default();
            if (view == View::Cells) != args.cell_ids.is_some() {
                return Err(ReadError::InvalidArgument);
            }
            Cursor {
                snapshot: args.snapshot_ref.unwrap_or_default(),
                view,
                ids: args.cell_ids,
                outputs: args.include_outputs.unwrap_or(false),
                limit: args.limit.unwrap_or(20),
                index: 0,
                byte_offset: 0,
            }
        };
        let mut snapshot = if query.snapshot.is_empty() {
            let mut snapshot = capture(controller, coordinator, accepted, scope, now)?;
            if let Some((bound, refs)) = &self.history
                && bound == scope
            {
                for reference in refs {
                    if !snapshot.recent_transaction_refs.contains(reference) {
                        snapshot.recent_transaction_refs.push(reference.clone());
                    }
                }
                snapshot.recent_transaction_refs.truncate(32);
            }
            snapshot
        } else {
            let retained = self
                .snapshots
                .get(&query.snapshot)
                .ok_or(ReadError::InvalidReference)?;
            if now < retained.snapshot.issued {
                return Err(ReadError::InvalidReference);
            }
            retained.snapshot.clone()
        };
        let is_current = snapshot.scope == *scope;
        let mut cells = vec![];
        let mut symbols = vec![];
        let mut complete = BTreeSet::new();
        let mut next = None;
        let mut source_continues = false;
        if query.view == View::Symbols {
            let end = (query.index + query.limit).min(snapshot.symbols.len());
            symbols.extend_from_slice(&snapshot.symbols[query.index..end]);
            if end < snapshot.symbols.len() {
                next = Some(Cursor {
                    index: end,
                    ..query.clone()
                });
            }
        } else {
            let selected = if let Some(ids) = &query.ids {
                ids.iter()
                    .map(|id| {
                        snapshot
                            .source
                            .file
                            .cells
                            .iter()
                            .find(|c| &c.id == id)
                            .ok_or(ReadError::NotFound)
                    })
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                snapshot.source.file.cells.iter().collect()
            };
            let mut remaining = PAGE_BYTES;
            for (index, cell) in selected
                .iter()
                .enumerate()
                .skip(query.index)
                .take(query.limit)
            {
                let start = if index == query.index {
                    query.byte_offset
                } else {
                    0
                };
                let mut end = start;
                let source = if query.view == View::Cells {
                    end = (start + remaining).min(cell.source.len());
                    while !cell.source.is_char_boundary(end) {
                        end -= 1;
                    }
                    remaining -= end - start;
                    Some(&cell.source[start..end])
                } else {
                    None
                };
                let full = source.is_some() && start == 0 && end == cell.source.len();
                if full {
                    complete.insert(cell.id.clone());
                }
                let more_source = source.is_some() && end < cell.source.len();
                source_continues = more_source;
                if more_source {
                    next = Some(Cursor {
                        index,
                        byte_offset: end,
                        ..query.clone()
                    });
                } else if index + 1 < selected.len()
                    && (cells.len() + 1 == query.limit || remaining < 4)
                {
                    next = Some(Cursor {
                        index: index + 1,
                        byte_offset: 0,
                        ..query.clone()
                    });
                }
                let revision = snapshot
                    .source
                    .cell_revisions
                    .iter()
                    .find(|r| r.cell_id == cell.id)
                    .ok_or(ReadError::StaleSnapshot)?
                    .revision;
                cells.push(json!({"cell_id":cell.id,"kind":cell.kind,"dialect":cell.dialect,"source_hash":format!("{:x}",Sha256::digest(cell.source.as_bytes())),"revision":revision,"status":snapshot.statuses.get(&cell.id).cloned().unwrap_or(json!("Stale")),"source":source,"source_complete":full,"source_cursor":null,"result_ref":null}));
                if next.is_some() {
                    break;
                }
            }
        }
        // Never upgrade an earlier outline/fragment token. Snapshot IDs used by preview_source
        // stay immutable, and a historical read cannot issue a current write capability.
        let needs_snapshot =
            query.snapshot.is_empty() || (is_current && !complete.is_subset(&snapshot.complete));
        let reference = if needs_snapshot {
            snapshot.complete.extend(complete);
            let retained_bytes = serde_json::to_vec(&snapshot)
                .map_err(|_| ReadError::LimitExceeded)?
                .len();
            if self.snapshots.len() >= 128
                || self
                    .bytes
                    .checked_add(retained_bytes)
                    .is_none_or(|n| n > RETAIN_BYTES)
            {
                return Err(ReadError::LimitExceeded);
            }
            let reference = controller
                .snapshot(
                    snapshot.coordinator.clone(),
                    snapshot.complete.clone(),
                    scope,
                    now,
                )
                .map_err(|_| ReadError::LimitExceeded)?;
            self.retain(reference.clone(), snapshot.clone(), retained_bytes);
            reference
        } else {
            query.snapshot.clone()
        };
        let cursor = if let Some(mut value) = next {
            value.snapshot = reference.clone();
            Some(
                self.cursors
                    .issue(ReferenceKind::Cursor, scope.clone(), value, now, TTL)
                    .map_err(|_| ReadError::LimitExceeded)?,
            )
        } else {
            None
        };
        if source_continues && let Some(last) = cells.last_mut() {
            last["source_cursor"] = json!(cursor);
        }
        let data = super::checked_data("read_notebook_data",json!({"snapshot_ref":reference,"is_current":is_current,"title":snapshot.source.file.title,"cells":cells,"symbols":symbols,"recent_transaction_refs":snapshot.recent_transaction_refs})).map_err(|_| ReadError::LimitExceeded)?;
        Ok(ReadPage {
            source_revision: snapshot.source.revision.get(),
            kernel_state_revision: snapshot.scope.definition_revision,
            include_outputs: query.outputs,
            data,
            pagination: json!({"truncated":cursor.is_some(),"next_cursor":cursor}),
        })
    }
}
