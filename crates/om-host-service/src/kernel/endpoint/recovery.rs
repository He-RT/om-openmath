use super::*;
use crate::kernel::acceptance::KernelRecovery;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const MAX_METADATA: usize = 16 * 1024 * 1024;
const MAX_BLOB: usize = 64 * 1024 * 1024;
const MAGIC: &[u8] = b"OMKR\x01";
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    protocol_version: u32,
    plan: NativeKernelCommit,
    receipt: NativeKernelReceipt,
}

fn decode(
    bytes: Vec<u8>,
    context: AcceptanceContext,
    worker: Arc<KernelWorker>,
    cancel: Arc<AtomicBool>,
) -> Result<AcceptedKernelState, KernelWorkerError> {
    if bytes.len() < 9 || &bytes[..5] != MAGIC {
        return Err(KernelWorkerError::Invalid);
    }
    let length = u32::from_le_bytes(
        bytes[5..9]
            .try_into()
            .map_err(|_| KernelWorkerError::Invalid)?,
    ) as usize;
    if length == 0 || length > MAX_METADATA || length > bytes.len() - 9 {
        return Err(KernelWorkerError::Invalid);
    }
    let metadata: Metadata =
        serde_json::from_slice(&bytes[9..9 + length]).map_err(|_| KernelWorkerError::Invalid)?;
    let plan = &metadata.plan;
    let receipt = &metadata.receipt;
    if metadata.protocol_version != 1
        || plan.store_id != context.store_id
        || receipt.store_id != context.store_id
        || plan.source.document_id != context.source.document_id
        || receipt.document_id != context.source.document_id
        || plan.acceptance_source.document_id != context.source.document_id
        || plan.acceptance_source.revision > context.source.revision
        || plan.acceptance_source.execution_epoch > context.source.execution_epoch
    {
        return Err(KernelWorkerError::Invalid);
    }
    if bytes.len() - 9 - length > MAX_BLOB {
        return Err(KernelWorkerError::Limit);
    }
    let general =
        crate::document::endpoint::general_configuration(Some(&receipt.producer.calculation));
    let interrupt = Interrupt {
        flag: cancel.clone(),
        ..Interrupt::default()
    };
    interrupt.tick().map_err(|_| KernelWorkerError::Cancelled)?;
    let accepted = AcceptedKernelState::recover(
        KernelRecovery {
            plan,
            receipt,
            bytes: bytes[9 + length..].to_vec(),
            general,
            expected_build: KERNEL_BUILD_ID,
        },
        &worker,
        CheckpointLimits::default(),
        &interrupt,
    )?;
    if cancel.load(Ordering::Acquire) {
        let _ = worker.discard(accepted.registry_ref());
        return Err(KernelWorkerError::Cancelled);
    }
    Ok(accepted)
}
impl KernelEndpoint {
    pub(super) fn restore_start(
        &mut self,
        body: KernelRestoreStart,
        ctx: &AcceptanceContext,
        source: &mut DocumentCommitController,
    ) -> Result<NativeKernelHostReply, String> {
        if let Some(entry) = self.operations.get(&body.operation_id) {
            if !matches!(&entry.identity,Identity::Restore{source,hash,length} if source==&body.expected_source_hash && hash==&body.payload_hash && *length as u64==body.byte_length.get())
            {
                return Err("DUPLICATE_OPERATION".into());
            }
            return self.restore_reply(&body.operation_id, ctx, source);
        }
        if self.closing {
            return Err("HOST_CLOSING".into());
        }
        if self.active.is_some()
            || self.busy()
            || source.source_gate_busy()
            || source.operation(&body.operation_id).is_some()
        {
            return Err("OPERATION_IN_PROGRESS".into());
        }
        if self.operations.len() >= 4096
            || body.byte_length.get() < 9
            || body.byte_length.get() > (MAX_METADATA + MAX_BLOB + 9) as u64
        {
            return Err("BUDGET_EXCEEDED".into());
        }
        if body.expected_source_hash != ctx.source.snapshot_hash {
            return Err("STALE_SOURCE".into());
        }
        let length = usize::try_from(body.byte_length.get()).map_err(|_| "BUDGET_EXCEEDED")?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| "BUDGET_EXCEEDED")?;
        source
            .hold_kernel_gate(&body.operation_id)
            .map_err(|_| "OPERATION_IN_PROGRESS")?;
        self.operations.insert(
            body.operation_id.clone(),
            Entry {
                identity: Identity::Restore {
                    source: body.expected_source_hash,
                    hash: body.payload_hash,
                    length,
                },
                phase: NativeKernelHostReplyPhase::Blob,
                receiver: None,
                candidate: None,
                plan: None,
                receipt: None,
                error: None,
                cancel: Arc::new(AtomicBool::new(false)),
                seed_started: Arc::new(AtomicBool::new(false)),
                upload: Some(bytes),
            },
        );
        self.restore_reply(&body.operation_id, ctx, source)
    }
    fn restore_reply(
        &self,
        id: &str,
        ctx: &AcceptanceContext,
        source: &DocumentCommitController,
    ) -> Result<NativeKernelHostReply, String> {
        let mut reply = self.status(id, ctx, source)?;
        if let Some(upload) = self
            .operations
            .get(id)
            .and_then(|entry| entry.upload.as_ref())
        {
            reply.blob_offset = Nullable(Some(Serial::new(upload.len() as u64)?))
        }
        Ok(reply)
    }
    pub(super) fn restore_write(
        &mut self,
        body: KernelRestoreWrite,
        ctx: &AcceptanceContext,
        source: &DocumentCommitController,
    ) -> Result<NativeKernelHostReply, String> {
        let entry = self
            .operations
            .get_mut(&body.operation_id)
            .ok_or("INVALID_REFERENCE")?;
        let Identity::Restore {
            source: expected,
            length,
            ..
        } = &entry.identity
        else {
            return Err("INVALID_ARGUMENT".into());
        };
        if expected != &ctx.source.snapshot_hash {
            return Err("STALE_SOURCE".into());
        }
        let upload = entry.upload.as_mut().ok_or("NOT_AVAILABLE")?;
        let offset = usize::try_from(body.offset.get()).map_err(|_| "INVALID_ARGUMENT")?;
        if body.bytes.is_empty() || body.bytes.len() > 65536 {
            return Err("INVALID_ARGUMENT".into());
        }
        let chunk = body
            .bytes
            .iter()
            .copied()
            .map(u8::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "INVALID_ARGUMENT")?;
        if offset < upload.len() {
            if offset
                .checked_add(body.bytes.len())
                .is_none_or(|end| end > upload.len())
                || upload[offset..offset + body.bytes.len()] != chunk
            {
                return Err("DUPLICATE_OPERATION".into());
            }
        } else {
            if offset != upload.len() || body.bytes.len() > *length - upload.len() {
                return Err("INVALID_ARGUMENT".into());
            }
            upload.extend_from_slice(&chunk);
        }
        self.restore_reply(&body.operation_id, ctx, source)
    }
    pub(super) fn restore_finish(
        &mut self,
        body: KernelRestoreFinish,
        ctx: &AcceptanceContext,
        source: &DocumentCommitController,
    ) -> Result<NativeKernelHostReply, String> {
        let entry = self
            .operations
            .get_mut(&body.operation_id)
            .ok_or("INVALID_REFERENCE")?;
        let Identity::Restore {
            source: expected,
            hash,
            length,
        } = &entry.identity
        else {
            return Err("INVALID_ARGUMENT".into());
        };
        if terminal(&entry.phase) || entry.receiver.is_some() {
            return self.status(&body.operation_id, ctx, source);
        }
        if expected != &ctx.source.snapshot_hash {
            return Err("STALE_SOURCE".into());
        }
        let upload = entry.upload.as_ref().ok_or("NOT_AVAILABLE")?;
        if upload.len() != *length {
            return Err("INVALID_ARGUMENT".into());
        }
        // Hash/decode/import belongs to the new background owner, never the control lock.
        let bytes = entry.upload.take().ok_or("NOT_AVAILABLE")?;
        let expected_hash = hash.clone();
        let worker = self.worker.clone();
        let context = ctx.clone();
        let cancel = entry.cancel.clone();
        let started = entry.seed_started.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        let thread = match thread::Builder::new()
            .name("om-kernel-recovery".into())
            .spawn(move || {
                started.store(true, Ordering::Release);
                let outcome = if format!("{:x}", Sha256::digest(&bytes)) != expected_hash {
                    Err(KernelWorkerError::Invalid)
                } else {
                    decode(bytes, context, worker, cancel)
                };
                let _ = sender.send(outcome);
            }) {
            Ok(thread) => thread,
            Err(_) => {
                entry.phase = NativeKernelHostReplyPhase::Failed;
                entry.error = Some("INTERNAL_ERROR".into());
                return Err("INTERNAL_ERROR".into());
            }
        };
        entry.receiver = Some(Receiver::Restore(receiver));
        entry.phase = NativeKernelHostReplyPhase::Queued;
        self.seed_threads.push(thread);
        self.status(&body.operation_id, ctx, source)
    }
}
