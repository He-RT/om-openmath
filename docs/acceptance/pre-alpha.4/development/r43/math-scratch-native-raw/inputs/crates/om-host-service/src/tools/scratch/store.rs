use super::*;
use crate::references::{ReferenceKind, ReferenceRegistry};
use om_kernel::{
    protocol::{CellStatus, MsgLevel},
    retained_results::{RetainedResult, RetainedResultSummary},
};
use std::collections::BTreeMap;
#[derive(Serialize)]
struct Lease {
    request: String,
}
struct StoredScratch {
    request: String,
    scope: ReferenceScope,
    source_hash: String,
    parent: Option<ScratchParent>,
    definitions: bool,
    value: RetainedResult,
    reserved: usize,
    expires: u64,
}
/// Separate result origin/retention owner. No main Kernel receipt, accepted head or write API.
pub struct ScratchResultStore {
    refs: ReferenceRegistry<Lease>,
    records: BTreeMap<String, Arc<StoredScratch>>,
    retired: Vec<Arc<StoredScratch>>,
    bytes: usize,
}
/// Actual immutable scratch result; it never resolves as a main StoredResult/NativeKernelReceipt.
pub struct ResolvedScratch {
    record: Arc<StoredScratch>,
    reference: String,
    current: bool,
}
impl ResolvedScratch {
    /// Complete exact source of this original retained occurrence, never a main reevaluation.
    pub fn source(
        &self,
        path: &[u32],
        format: om_kernel::retained_results::ResultSourceFormat,
        ctx: &Interrupt,
    ) -> Result<String, String> {
        let summary = self.summary();
        let tail = summary.statements.last().ok_or("NOT_AVAILABLE")?;
        self.record
            .value
            .value_source(tail.out_index, &tail.view_id, path, format, ctx)
    }
    /// Real original scratch status/types/statement count.
    pub fn summary(&self) -> RetainedResultSummary {
        self.record.value.summary()
    }
    /// Explicit isolated origin and actual parent provenance, without a main acceptance claim.
    pub fn binding(&self) -> Value {
        let summary = self.summary();
        let tail = summary.statements.last();
        json!({"record_type":"scratch_result_binding","origin":"scratch","result_ref":self.reference,"request_ref":self.record.request,
            "document_id":self.record.scope.document,"document_generation":self.record.scope.generation,"source_revision":self.record.scope.revision,
            "execution_epoch":self.record.scope.execution_epoch,"config_revision":self.record.scope.config_revision,"definition_revision":self.record.scope.definition_revision,
            "snapshot_hash":self.record.scope.snapshot_hash,"source_hash":self.record.source_hash,"parent_checkpoint":self.record.parent,
            "use_notebook_definitions":self.record.definitions,"acceptance":"isolated","is_current":self.current,"cell_id":summary.cell_id,
            "out_index":tail.map_or(0,|s|s.out_index),"view_id":tail.map(|s|s.view_id.as_str()),"status":summary.status})
    }
    pub(crate) fn retained(&self) -> &RetainedResult {
        &self.record.value
    }
    pub(crate) fn is_current(&self) -> bool {
        self.current
    }
}
impl ScratchResultStore {
    /// New host entropy; deterministic only for owned unit fixtures.
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            refs: ReferenceRegistry::new(key),
            records: BTreeMap::new(),
            retired: vec![],
            bytes: 0,
        }
    }
    fn reap(&mut self, now: u64) {
        let expired = self
            .records
            .iter()
            .filter(|(_, r)| r.expires <= now)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in expired {
            self.retired
                .push(self.records.remove(&id).expect("selected"));
        }
        self.retired.retain(|r| {
            if Arc::strong_count(r) == 1 {
                self.bytes -= r.reserved;
                false
            } else {
                true
            }
        });
    }
    /// Decode only the actual scratch worker packet on this result owner; the wire descriptor has
    /// an independent scratch document/build and cannot be accepted by the main Kernel port.
    pub fn register(
        &mut self,
        packet: ScratchEvaluation,
        scope: &ReferenceScope,
        now: u64,
        ctx: &Interrupt,
    ) -> Result<String, String> {
        if packet.scope != *scope || !scope.can_read {
            return Err("PERMISSION_DENIED".into());
        }
        tick(ctx)?;
        self.reap(now);
        if self.records.contains_key(&packet.request) || packet.reserved > 32 * 1024 * 1024 {
            return Err("LIMIT_EXCEEDED".into());
        }
        while self.records.len() + self.retired.len() >= 32
            || self
                .bytes
                .checked_add(packet.reserved)
                .is_none_or(|b| b > 32 * 1024 * 1024)
        {
            let id = self
                .records
                .iter()
                .filter(|(_, r)| Arc::strong_count(r) == 1)
                .min_by_key(|(_, r)| r.expires)
                .map(|(id, _)| id.clone())
                .ok_or("LIMIT_EXCEEDED")?;
            self.retired
                .push(self.records.remove(&id).expect("selected"));
            self.reap(now);
        }
        let session = Session::decode_checkpoint(
            &packet.bytes,
            CheckpointRestore {
                binding: &packet.binding,
                source: &packet.source,
                general: &packet.general,
                clock: ctx.clock.clone(),
                cancel: ctx.flag.clone(),
            },
            CheckpointLimits {
                max_bytes: 16 * 1024 * 1024,
                ..Default::default()
            },
            ctx,
        )
        .map_err(|_| "NOT_AVAILABLE")?
        .into_session();
        let value = session.retain_result(&packet.binding.document_id)?;
        tick(ctx)?;
        let token = self
            .refs
            .issue(
                ReferenceKind::Result,
                scope.clone(),
                Lease {
                    request: packet.request.clone(),
                },
                now,
                600_000,
            )
            .map_err(|_| "LIMIT_EXCEEDED")?;
        let expires = now
            .checked_add(600_000)
            .filter(|&n| n <= crate::protocol::MAX_SERIAL)
            .ok_or("LIMIT_EXCEEDED")?;
        self.bytes += packet.reserved;
        self.records.insert(
            packet.request.clone(),
            Arc::new(StoredScratch {
                request: packet.request,
                scope: packet.scope,
                source_hash: packet.source_hash,
                parent: packet.parent,
                definitions: packet.definitions,
                value,
                reserved: packet.reserved,
                expires,
            }),
        );
        Ok(token)
    }
    /// Original readonly result history across source changes; runtime/task/grants/TTL stay exact.
    pub fn resolve(
        &self,
        reference: &str,
        scope: &ReferenceScope,
        now: u64,
    ) -> Result<ResolvedScratch, String> {
        let lease = self
            .refs
            .resolve_result_history(reference, scope, now)
            .map_err(|_| "INVALID_REFERENCE")?;
        let record = self
            .records
            .get(&lease.request)
            .cloned()
            .ok_or("INVALID_REFERENCE")?;
        Ok(ResolvedScratch {
            current: record.scope == *scope,
            record,
            reference: reference.into(),
        })
    }
    /// Closed real scratch result. A math error can retain its successful prefix/diagnostics under
    /// an isolated ref, but never becomes tool/math success or an accepted main checkpoint.
    pub fn result(
        &self,
        reference: &str,
        scope: &ReferenceScope,
        now: u64,
    ) -> Result<Value, String> {
        let resolved = self.resolve(reference, scope, now)?;
        let summary = resolved.summary();
        let messages = resolved.retained().messages();
        if messages.len() > 100 {
            return Err("LIMIT_EXCEEDED".into());
        }
        let diagnostics=messages.iter().map(|m|json!({"code":m.tag,"message":m.text,"severity":match m.level {MsgLevel::Error=>"error",MsgLevel::Warning=>"warning",MsgLevel::Info=>"hint"},"cell_id":summary.cell_id,"start_utf8":null,"end_utf8":null})).collect::<Vec<_>>();
        let done = summary.status == CellStatus::Done;
        let envelope = json!({"ok":done,"status":if done {"completed"} else {"failed"},"request_ref":resolved.record.request,"operation_ref":null,
            "document":{"id":scope.document,"generation":scope.generation,"revision":scope.revision},
            "data":{"result_ref":reference,"evaluation_status":if done {"done"} else {"error"},"output_summary":{"binding":resolved.binding(),"summary":summary},"diagnostics":diagnostics,"notebook_modified":false},
            "error":if done {Value::Null} else {json!({"code":"EXECUTION_FAILED","message":"Isolated scratch evaluation reported an error; original successful prefix is retained","recovery":"none","details":{}})},
            "pagination":{"truncated":false,"next_cursor":null}});
        super::super::checked_data("evaluate_scratch_result", envelope)
            .map_err(|_| "LIMIT_EXCEEDED".into())
    }
}
