use super::*;
impl KernelEndpoint {
    /// Short trusted native control command; all restore/evaluate/encode work remains off owner.
    pub fn command(
        &mut self,
        command: NativeKernelHostCommand,
        ctx: &AcceptanceContext,
        source: &mut DocumentCommitController,
    ) -> Result<NativeKernelHostReply, String> {
        self.advance(ctx, source)?;
        match command {
            NativeKernelHostCommand::KernelState(_) => Ok(self.base_reply(ctx, source)),
            NativeKernelHostCommand::KernelBootstrap(body) => {
                if source.operation(&body.operation_id).is_some() {
                    return Err("DUPLICATE_OPERATION".into());
                }
                if let Some(entry) = self.operations.get(&body.operation_id) {
                    if !matches!(&entry.identity,Identity::Bootstrap {source} if source==&body.expected_source_hash)
                    {
                        return Err("DUPLICATE_OPERATION".into());
                    }
                    return self.status(&body.operation_id, ctx, source);
                }
                if self.closing {
                    return Err("HOST_CLOSING".into());
                }
                if self.active.is_some() || self.busy() || source.source_gate_busy() {
                    return Err("OPERATION_IN_PROGRESS".into());
                }
                if self.operations.len() >= 4096 {
                    return Err("BUDGET_EXCEEDED".into());
                }
                if body.expected_source_hash != ctx.source.snapshot_hash {
                    return Err("STALE_SOURCE".into());
                }
                source
                    .hold_kernel_gate(&body.operation_id)
                    .map_err(|_| "OPERATION_IN_PROGRESS")?;
                let cancel = Arc::new(AtomicBool::new(false));
                let started = Arc::new(AtomicBool::new(false));
                let (sender, receiver) = mpsc::sync_channel(1);
                let work = self.worker.clone();
                let context = ctx.clone();
                let id = body.operation_id.clone();
                let token = cancel.clone();
                let running = started.clone();
                let spawned = thread::Builder::new()
                    .name("openmath-main-bootstrap".into())
                    .spawn(move || {
                        running.store(true, Ordering::Release);
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            let interrupt = Interrupt {
                                flag: token,
                                ..Interrupt::default()
                            };
                            FrozenKernelPlan::bootstrap(
                                &context,
                                &id,
                                &work,
                                CheckpointLimits::default(),
                                &interrupt,
                            )
                        }))
                        .unwrap_or(Err(KernelWorkerError::Internal));
                        if let Err(mpsc::SendError(Ok(plan))) = sender.send(result) {
                            let _ = plan.discard(&work);
                        }
                    });
                let thread = match spawned {
                    Ok(thread) => thread,
                    Err(_) => {
                        source
                            .release_kernel_gate(&body.operation_id)
                            .map_err(|_| "INTERNAL_ERROR")?;
                        return Err("INTERNAL_ERROR".into());
                    }
                };
                self.seed_threads.push(thread);
                self.operations.insert(
                    body.operation_id.clone(),
                    Entry {
                        identity: Identity::Bootstrap {
                            source: body.expected_source_hash,
                        },
                        phase: NativeKernelHostReplyPhase::Queued,
                        receiver: Some(Receiver::Bootstrap(receiver)),
                        candidate: None,
                        plan: None,
                        receipt: None,
                        error: None,
                        cancel,
                        seed_started: started,
                    },
                );
                self.status(&body.operation_id, ctx, source)
            }
            NativeKernelHostCommand::KernelRunCell(body) => {
                if source.operation(&body.operation_id).is_some() {
                    return Err("DUPLICATE_OPERATION".into());
                }
                if let Some(entry) = self.operations.get(&body.operation_id) {
                    if !matches!(&entry.identity,Identity::Cell {source,cell} if source==&body.expected_source_hash && cell==&body.cell_id)
                    {
                        return Err("DUPLICATE_OPERATION".into());
                    }
                    return self.status(&body.operation_id, ctx, source);
                }
                if self.closing {
                    return Err("HOST_CLOSING".into());
                }
                if self.busy() || source.source_gate_busy() {
                    return Err("OPERATION_IN_PROGRESS".into());
                }
                if self.operations.len() >= 4096 {
                    return Err("BUDGET_EXCEEDED".into());
                }
                if body.expected_source_hash != ctx.source.snapshot_hash {
                    return Err("STALE_SOURCE".into());
                }
                let active = self.active.as_ref().ok_or("CONTEXT_NOT_READY")?;
                let cancel = Arc::new(AtomicBool::new(false));
                let receiver = self
                    .worker
                    .submit(KernelJob {
                        runtime_instance_id: ctx.runtime_instance_id.clone(),
                        document_generation: ctx.document_generation,
                        operation_id: body.operation_id.clone(),
                        parent_checkpoint_ref: active.registry_ref().into(),
                        source: ctx.source.clone(),
                        general: ctx.general.clone(),
                        config_revision: ctx.config_revision,
                        cell_id: body.cell_id.clone(),
                        cancel: cancel.clone(),
                    })
                    .map_err(error)?;
                self.operations.insert(
                    body.operation_id.clone(),
                    Entry {
                        identity: Identity::Cell {
                            source: body.expected_source_hash,
                            cell: body.cell_id,
                        },
                        phase: NativeKernelHostReplyPhase::Queued,
                        receiver: Some(Receiver::Cell(receiver)),
                        candidate: None,
                        plan: None,
                        receipt: None,
                        error: None,
                        cancel,
                        seed_started: Arc::new(AtomicBool::new(false)),
                    },
                );
                self.status(&body.operation_id, ctx, source)
            }
            NativeKernelHostCommand::KernelStatus(body) => {
                self.status(&body.operation_id, ctx, source)
            }
            NativeKernelHostCommand::KernelReadBlob(body) => {
                let entry = self
                    .operations
                    .get(&body.operation_id)
                    .ok_or("INVALID_REFERENCE")?;
                let plan = entry.plan.as_ref().ok_or("NOT_AVAILABLE")?;
                let bytes = plan.state().bytes();
                let offset = usize::try_from(body.offset.get()).map_err(|_| "INVALID_ARGUMENT")?;
                if body.count == 0 || body.count > 65536 || offset > bytes.len() {
                    return Err("INVALID_ARGUMENT".into());
                }
                let end = offset
                    .checked_add(body.count as usize)
                    .ok_or("BUDGET_EXCEEDED")?
                    .min(bytes.len());
                let mut reply = self.status(&body.operation_id, ctx, source)?;
                reply.plan = Nullable(None);
                reply.phase = NativeKernelHostReplyPhase::Blob;
                reply.blob_offset = Nullable(Some(body.offset));
                reply.blob_bytes = bytes[offset..end].iter().map(|b| u32::from(*b)).collect();
                Ok(reply)
            }
            NativeKernelHostCommand::KernelBarrier(body) => {
                if self.closing {
                    return Err("HOST_CLOSING".into());
                }
                if source.kernel_gate() != Some(body.operation_id.as_str()) {
                    return Err("OPERATION_IN_PROGRESS".into());
                }
                let entry = self
                    .operations
                    .get_mut(&body.operation_id)
                    .ok_or("INVALID_REFERENCE")?;
                let plan = entry.plan.as_ref().ok_or("NOT_AVAILABLE")?;
                plan.enter_commit(ctx, self.active.as_ref())
                    .map_err(error)?;
                entry.phase = NativeKernelHostReplyPhase::Committing;
                self.status(&body.operation_id, ctx, source)
            }
            NativeKernelHostCommand::KernelUnknown(body) => {
                let entry = self
                    .operations
                    .get_mut(&body.operation_id)
                    .ok_or("INVALID_REFERENCE")?;
                entry
                    .plan
                    .as_ref()
                    .ok_or("NOT_AVAILABLE")?
                    .mark_unknown()
                    .map_err(error)?;
                entry.phase = NativeKernelHostReplyPhase::Unknown;
                self.status(&body.operation_id, ctx, source)
            }
            NativeKernelHostCommand::KernelCompleted(body) => {
                let entry = self
                    .operations
                    .get_mut(&body.operation_id)
                    .ok_or("INVALID_REFERENCE")?;
                if let Some(receipt) = &entry.receipt {
                    if serde_json::to_value(receipt).map_err(|_| "INTERNAL_ERROR")?
                        != serde_json::to_value(&body.receipt).map_err(|_| "INVALID_ARGUMENT")?
                    {
                        return Err("INVALID_ARGUMENT".into());
                    }
                    return self.status(&body.operation_id, ctx, source);
                }
                let plan = entry.plan.as_ref().ok_or("NOT_AVAILABLE")?;
                let accepted = plan.accept_receipt(&body.receipt).map_err(error)?;
                let old = self.active.replace(accepted);
                entry.receipt = Some(body.receipt);
                entry.phase = NativeKernelHostReplyPhase::Completed;
                entry.plan = None;
                if let Some(old) = old {
                    let _ = self.worker.discard(old.registry_ref());
                }
                source
                    .release_kernel_gate(&body.operation_id)
                    .map_err(|_| "INTERNAL_ERROR")?;
                self.status(&body.operation_id, ctx, source)
            }
            NativeKernelHostCommand::KernelDiscard(body) => {
                let entry = self
                    .operations
                    .get_mut(&body.operation_id)
                    .ok_or("INVALID_REFERENCE")?;
                entry
                    .plan
                    .as_ref()
                    .ok_or("NOT_AVAILABLE")?
                    .discard(&self.worker)
                    .map_err(error)?;
                entry.plan = None;
                entry.phase = NativeKernelHostReplyPhase::Discarded;
                source
                    .release_kernel_gate(&body.operation_id)
                    .map_err(|_| "INTERNAL_ERROR")?;
                self.status(&body.operation_id, ctx, source)
            }
            NativeKernelHostCommand::KernelSettled(body) => {
                let entry = self
                    .operations
                    .get_mut(&body.operation_id)
                    .ok_or("INVALID_REFERENCE")?;
                entry
                    .plan
                    .as_ref()
                    .ok_or("NOT_AVAILABLE")?
                    .confirmed_no_commit(&body.proof, &self.worker)
                    .map_err(error)?;
                entry.plan = None;
                entry.phase = NativeKernelHostReplyPhase::Discarded;
                source
                    .release_kernel_gate(&body.operation_id)
                    .map_err(|_| "INTERNAL_ERROR")?;
                self.status(&body.operation_id, ctx, source)
            }
        }
    }
}
