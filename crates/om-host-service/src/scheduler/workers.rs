//! Bounded math/editor owners; all computation stays off the control coordinator.
use super::{Control, operations::Operations};
use crate::protocol::generated::{
    AnalyzeEditorAnalysisKind, Dialect, HostRequestBody, RequestEnvelope,
};
use om_kernel::{
    KernelConfig, Session,
    protocol::{Dialect as KernelDialect, HostPlatform, Request},
};
use om_num::ctx::Clock;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    sync::{Arc, atomic::AtomicBool, mpsc},
    thread::{self, JoinHandle},
    time::Instant,
};

struct MonotonicClock(Instant);
impl Clock for MonotonicClock {
    fn now_ms(&self) -> f64 {
        self.0.elapsed().as_secs_f64() * 1000.0
    }
}
pub(crate) struct Job {
    pub request: RequestEnvelope,
    pub token: Arc<AtomicBool>,
}
pub(crate) struct Worker {
    pub sender: mpsc::SyncSender<Job>,
    pub thread: JoinHandle<()>,
}
fn dialect(value: &Dialect) -> KernelDialect {
    match value {
        Dialect::Modern => KernelDialect::Modern,
        Dialect::Wolfram => KernelDialect::Wolfram,
        Dialect::Auto => KernelDialect::Auto,
    }
}
impl Worker {
    pub fn new(
        name: &str,
        control: mpsc::SyncSender<Control>,
        operations: Arc<Operations>,
    ) -> Result<Self, String> {
        let (sender, receiver) = mpsc::sync_channel::<Job>(2);
        let thread = thread::Builder::new()
            .name(name.into())
            .spawn(move || {
                while let Ok(job) = receiver.recv() {
                    let operation = job
                        .request
                        .operation_id
                        .0
                        .clone()
                        .unwrap_or_else(|| job.request.request_ref.clone());
                    if !operations.start(&operation) {
                        let _ = control.send(Control::Finished {
                            request: job.request.request_ref,
                            operation,
                            result: Err("CANCELLED".into()),
                        });
                        continue;
                    }
                    let result =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute(&job)))
                            .unwrap_or_else(|_| Err("WORKER_PANIC".into()));
                    if control
                        .send(Control::Finished {
                            request: job.request.request_ref,
                            operation,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .map_err(|_| "Cannot start native worker".to_owned())?;
        Ok(Self { sender, thread })
    }
}
fn execute(job: &Job) -> Result<Value, String> {
    // Catalog construction/encoding stays off the short control owner.
    match &job.request.body {
        HostRequestBody::ReadHostCapabilities(_) => {
            return bounded(json!({
                "kernel":om_kernel::capabilities::capabilities(HostPlatform::Desktop),
                "language_feature_version":1,"document_writes":false,
                "native_renderers_ready":false
            }));
        }
        HostRequestBody::ReadFunctionCatalog(body) => {
            let mut catalog = om_kernel::capabilities::function_catalog();
            let total = catalog.functions.len();
            let offset = body.offset.unwrap_or(0) as usize;
            let limit = body.limit.unwrap_or(32) as usize;
            if offset > total {
                return Err("INVALID_ARGUMENT".into());
            }
            catalog.functions = catalog
                .functions
                .into_iter()
                .skip(offset)
                .take(limit)
                .collect();
            let next = (offset + catalog.functions.len() < total)
                .then_some(offset + catalog.functions.len());
            return bounded(
                json!({"catalog":catalog,"offset":offset,"total":total,"next_offset":next}),
            );
        }
        _ => {}
    }
    let mut config = KernelConfig::default();
    config.general.reactive = false;
    let timeout = match &job.request.body {
        HostRequestBody::EvaluateScratch(body) => body.timeout_ms,
        _ => 1000,
    };
    config.general.eval_timeout_ms = timeout.into();
    let mut session = Session::with_cancel_token(
        config,
        Some(Arc::new(MonotonicClock(Instant::now()))),
        job.token.clone(),
    );
    session.handle(Request::SetHostPlatform {
        platform: HostPlatform::Desktop,
    });
    let body = match &job.request.body {
        HostRequestBody::EvaluateScratch(body) => {
            if body.use_notebook_definitions || body.definition_snapshot_ref.0.is_some() {
                return Err("CONTEXT_NOT_READY".into());
            }
            Request::Evaluate {
                cell_id: "scratch".into(),
                source: body.source.clone(),
                dialect: dialect(&body.dialect),
            }
        }
        HostRequestBody::AnalyzeEditor(body) => {
            if body.key.runtime_instance_id != job.request.runtime_instance_id {
                return Err("STALE_RUNTIME".into());
            }
            let actual_hash = format!("{:x}", Sha256::digest(body.source.as_bytes()));
            if actual_hash != body.key.source_hash {
                return Err("STALE_SOURCE".into());
            }
            // Editor requests are non-evaluating and have an independent owner/session.
            let cursor = body.key.cursor_utf8.0;
            if cursor.is_some_and(|c| !body.source.is_char_boundary(c as usize)) {
                return Err("INVALID_ARGUMENT".into());
            }
            match body.analysis_kind {
                AnalyzeEditorAnalysisKind::Preview => Request::Preview {
                    source: body.source.clone(),
                    dialect: dialect(&body.key.requested_dialect),
                    cursor,
                },
                AnalyzeEditorAnalysisKind::Complete => Request::Complete {
                    source: body.source.clone(),
                    dialect: dialect(&body.key.requested_dialect),
                    cursor: cursor.ok_or("INVALID_ARGUMENT")?,
                },
                AnalyzeEditorAnalysisKind::Hover => Request::Hover {
                    source: body.source.clone(),
                    dialect: dialect(&body.key.requested_dialect),
                    cursor: cursor.ok_or("INVALID_ARGUMENT")?,
                },
            }
        }
        _ => return Err("NOT_AVAILABLE".into()),
    };
    let (response, _events) = session.handle(body);
    let outcome = match &response {
        om_kernel::protocol::Response::Error { .. } => "failed",
        om_kernel::protocol::Response::Evaluated { output, .. }
            if output
                .messages
                .iter()
                .any(|m| matches!(m.level, om_kernel::protocol::MsgLevel::Error)) =>
        {
            "failed"
        }
        _ => "completed",
    };
    let value = serde_json::to_value(response).map_err(|_| "RESULT_ENCODING_FAILED")?;
    bounded(
        json!({"origin":"isolated","response":value,"run_outcome":outcome,"effect_committed":false}),
    )
}
fn bounded(value: Value) -> Result<Value, String> {
    if serde_json::to_vec(&value)
        .map_err(|_| "RESULT_ENCODING_FAILED")?
        .len()
        > 128 * 1024
    {
        return Err("BUDGET_EXCEEDED".into());
    }
    Ok(value)
}
