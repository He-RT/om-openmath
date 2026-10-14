//! Inspect actual retained output on the independent result worker, without source evaluation.
mod geometry;
mod view;
use super::scratch::{ScratchEvaluation, ScratchResultStore};
use crate::{
    agent::ingress::ToolIntent,
    protocol::generated::{AgentToolName, ResultBindingAcceptance, ResultBindingFreshness},
    references::{ReferenceError, ReferenceKind, ReferenceRegistry, ReferenceScope},
    results::{ResolvedResult, ResultError, ResultStore},
};
use om_kernel::{
    protocol::{ValueKind, ValueQuery},
    retained_results::RetainedStepEntry,
};
use om_num::ctx::{Abort, Interrupt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use view::InspectionView;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Part {
    #[default]
    Summary,
    Value,
    Diagnostics,
    Steps,
    Geometry,
    Preview,
}
fn default_limit() -> u32 {
    20
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    result_ref: String,
    #[serde(default)]
    part: Part,
    cursor: Option<String>,
    #[serde(default = "default_limit")]
    limit: u32,
}
/// Native-only frozen original intent/arguments. No model can supply a scope or result producer.
pub struct FrozenInspection {
    intent: ToolIntent,
    arguments: String,
}
impl FrozenInspection {
    /// Verify the original canonical argument hash before queueing it; SourceEndpoint obtains this
    /// intent exclusively from the acknowledged proposal, never from caller-supplied raw arguments.
    pub fn original(intent: &ToolIntent, arguments: &str) -> Result<Self, String> {
        if intent.name != AgentToolName::InspectResult
            || arguments.len() > 65536
            || format!("{:x}", Sha256::digest(arguments.as_bytes())) != intent.arguments_hash
        {
            return Err("IPC_CALL_SCOPE_MISMATCH".into());
        }
        Ok(Self {
            intent: intent.clone(),
            arguments: arguments.into(),
        })
    }
    /// Native original request identity, shared with cancellation/status/durable result readback.
    pub fn id(&self) -> &str {
        &self.intent.operation_id
    }
    pub(crate) fn identity(&self) -> Value {
        json!({"kind":"agent_inspect_result","operation":self.intent.operation_id,"turn":self.intent.turn_id,"call":self.intent.call_id,"proposal":self.intent.proposal_event,"arguments_hash":self.intent.arguments_hash,"arguments":self.arguments})
    }
}
#[derive(Clone, Serialize)]
struct ValueRoute {
    path: Vec<u32>,
    row: u32,
    column: u32,
}
#[derive(Clone, Serialize)]
struct StepRoute {
    parent: Vec<u32>,
    offset: u32,
}
#[derive(Clone, Serialize)]
enum Position {
    Offset(u32),
    Values(Vec<ValueRoute>),
    Steps(Vec<StepRoute>),
    Geometry { route: usize, offset: u32 },
}
#[derive(Clone, Serialize)]
struct Cursor {
    result: String,
    part: Part,
    limit: u32,
    position: Position,
}
/// Runtime-local opaque cursors: exact result/query/scope, 128 entries/32MiB/ten-minute maximum.
/// The values remain in the existing immutable ResultStore, not copied or reexecuted here.
pub struct ResultInspectionService {
    cursors: ReferenceRegistry<Cursor>,
    scratch: ScratchResultStore,
}
/// Actual result owner/current native task facts, supplied by the result worker only.
pub struct InspectionContext<'a> {
    /// Existing retained-result owner; references retain their original history/expiry semantics.
    pub store: &'a ResultStore,
    /// Actual native runtime/document/task/grants binding.
    pub scope: &'a ReferenceScope,
    /// Native monotonic milliseconds.
    pub now: u64,
    /// Real cancellation/deadline/step budget.
    pub interrupt: &'a Interrupt,
}
fn tick(ctx: &Interrupt) -> Result<(), &'static str> {
    ctx.tick().map_err(|e| match e {
        Abort::Interrupted => "CANCELLED",
        Abort::Timeout => "TIMEOUT",
        Abort::Budget => "LIMIT_EXCEEDED",
    })
}
fn projection(error: ResultError) -> &'static str {
    match error {
        ResultError::Reference(ReferenceError::PermissionDenied) => "PERMISSION_DENIED",
        ResultError::Reference(_) => "INVALID_REFERENCE",
        ResultError::Budget => "LIMIT_EXCEEDED",
        ResultError::Projection(s) if s.contains("BUDGET") || s.contains("LIMIT") => {
            "LIMIT_EXCEEDED"
        }
        _ => "NOT_AVAILABLE",
    }
}
pub(crate) fn failure(id: &str, scope: &ReferenceScope, code: &str) -> Value {
    let code = match code {
        "PERMISSION_DENIED" => "PERMISSION_DENIED",
        "INVALID_ARGUMENT" => "INVALID_ARGUMENT",
        "INVALID_REFERENCE" | "STALE_SNAPSHOT" => "INVALID_REFERENCE",
        "CANCELLED" => "CANCELLED",
        "TIMEOUT" => "TIMEOUT",
        "CONTEXT_NOT_READY" => "CONTEXT_NOT_READY",
        "DEPENDENCY_NOT_READY" => "DEPENDENCY_NOT_READY",
        "EXECUTION_FAILED" => "EXECUTION_FAILED",
        "NOT_SUPPORTED" => "NOT_SUPPORTED",
        "LIMIT_EXCEEDED" | "BUDGET_EXCEEDED" | "RESULT_PAGE_TOO_LARGE" => "LIMIT_EXCEEDED",
        _ => "NOT_AVAILABLE",
    };
    json!({"ok":false,"status":"failed","request_ref":id,"operation_ref":null,
        "document":{"id":scope.document,"generation":scope.generation,"revision":scope.revision},
        "data":null,"error":{"code":code,"message":code,"recovery":if code=="LIMIT_EXCEEDED" {"reduce_request"} else {"retry_read"},"details":{}},
        "pagination":{"truncated":false,"next_cursor":null}})
}
impl ResultInspectionService {
    /// Fresh native entropy; deterministic keys are used only in owned unit fixtures.
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            cursors: ReferenceRegistry::new(key),
            scratch: ScratchResultStore::new(
                Sha256::digest([key.as_slice(), b"isolated-scratch-results"].concat()).into(),
            ),
        }
    }
    pub(crate) fn complete_scratch(
        &mut self,
        packet: ScratchEvaluation,
        scope: &ReferenceScope,
        now: u64,
        ctx: &Interrupt,
    ) -> Value {
        let id = packet.request.clone();
        match self
            .scratch
            .register(packet, scope, now, ctx)
            .and_then(|reference| self.scratch.result(&reference, scope, now))
        {
            Ok(value) => value,
            Err(code) => failure(&id, scope, &code),
        }
    }
    /// Complete closed inspection result, including truthful failures. No evaluation, state
    /// acceptance, external IO, renderer invocation or mathematical success is inferred here.
    pub fn execute(&mut self, request: &FrozenInspection, source: InspectionContext<'_>) -> Value {
        let result = self.inspect(request, &source);
        match result {
            Ok(value) => value,
            Err(code) => failure(
                request.id(),
                source.scope,
                tick(source.interrupt).err().unwrap_or(code),
            ),
        }
    }
    fn inspect(
        &mut self,
        request: &FrozenInspection,
        source: &InspectionContext<'_>,
    ) -> Result<Value, &'static str> {
        tick(source.interrupt)?;
        source.scope.validate().map_err(|_| "INVALID_REFERENCE")?;
        if !source.scope.can_read {
            return Err("PERMISSION_DENIED");
        }
        let value =
            om_agent_contract::json::strict_json_bounded(request.arguments.as_bytes(), 65536)
                .map_err(|_| "INVALID_ARGUMENT")?;
        let value =
            super::checked_data("inspect_result_args", value).map_err(|_| "INVALID_ARGUMENT")?;
        let args: Args = serde_json::from_value(value).map_err(|_| "INVALID_ARGUMENT")?;
        self.cursors.retain_scope(source.scope);
        let position = if let Some(cursor) = &args.cursor {
            let c = self
                .cursors
                .resolve(cursor, ReferenceKind::Cursor, source.scope, source.now)
                .map_err(|_| "INVALID_REFERENCE")?;
            if c.result != args.result_ref || c.part != args.part || c.limit != args.limit {
                return Err("INVALID_REFERENCE");
            }
            c.position.clone()
        } else {
            match args.part {
                Part::Value => Position::Values(vec![ValueRoute {
                    path: vec![],
                    row: 0,
                    column: 0,
                }]),
                Part::Steps => Position::Steps(vec![StepRoute {
                    parent: vec![],
                    offset: 0,
                }]),
                Part::Geometry => Position::Geometry {
                    route: 0,
                    offset: 0,
                },
                _ => Position::Offset(0),
            }
        };
        let scratch = self
            .scratch
            .resolve(&args.result_ref, source.scope, source.now)
            .ok();
        let notebook = if scratch.is_none() {
            Some(
                source
                    .store
                    .resolve(&args.result_ref, source.scope, source.now)
                    .map_err(projection)?,
            )
        } else {
            None
        };
        let result = if let Some(scratch) = &scratch {
            InspectionView::scratch(scratch)
        } else {
            InspectionView::notebook(notebook.as_ref().expect("selected"))?
        };
        let (payload, next, render) =
            page(&result, args.part, args.limit, position, source.interrupt)?;
        tick(source.interrupt)?;
        // Validate the bounded payload before retaining a continuation, so a failed large reply
        // cannot exhaust the cursor registry with continuations the caller never received.
        if serde_json::to_vec(&payload)
            .map_err(|_| "NOT_AVAILABLE")?
            .len()
            > 240 * 1024
        {
            return Err("LIMIT_EXCEEDED");
        }
        let next = next
            .map(|position| {
                self.cursors.issue(
                    ReferenceKind::Cursor,
                    source.scope.clone(),
                    Cursor {
                        result: args.result_ref,
                        part: args.part,
                        limit: args.limit,
                        position,
                    },
                    source.now,
                    600_000,
                )
            })
            .transpose()
            .map_err(|_| "LIMIT_EXCEEDED")?;
        let envelope = json!({"ok":true,"status":"completed","request_ref":request.id(),"operation_ref":null,
            "document":{"id":source.scope.document,"generation":source.scope.generation,"revision":source.scope.revision},
            "data":{"origin":result.origin,"is_current":result.current,"part":args.part,"payload":payload,"render_state":render,"media_ref":null},
            "error":null,"pagination":{"truncated":next.is_some(),"next_cursor":next}});
        super::checked_data("inspect_result_result", envelope).map_err(|_| "LIMIT_EXCEEDED")
    }
}
fn page(
    result: &InspectionView<'_>,
    part: Part,
    limit: u32,
    position: Position,
    ctx: &Interrupt,
) -> Result<(Value, Option<Position>, &'static str), &'static str> {
    let binding = result.binding();
    match (part, position) {
        (Part::Summary, Position::Offset(offset)) => {
            let s = result.summary();
            let statement = s.statements.iter().find(|s| {
                s.out_index == result.out_index
                    && Some(s.view_id.as_str()) == result.view_id.as_deref()
            });
            let presentation = result
                .presentation(offset, limit.min(32), ctx)
                .map_err(projection)?;
            let p = &presentation["presentation"];
            let total = p["total_solutions"]
                .as_u64()
                .unwrap_or(0)
                .max(p["total_intervals"].as_u64().unwrap_or(0));
            let end = u64::from(offset) + u64::from(limit.min(32));
            let next = (end < total).then_some(Position::Offset(
                u32::try_from(end).map_err(|_| "LIMIT_EXCEEDED")?,
            ));
            Ok((
                json!({"binding":binding,"cell_id":s.cell_id,"cell_kind":s.cell_kind,"status":s.status,"source_byte_length":s.source_byte_length,"statement_count":s.statements.len(),"statement":statement,"presentation":presentation}),
                next,
                "not_applicable",
            ))
        }
        (Part::Diagnostics, Position::Offset(offset)) => {
            let messages = result.messages();
            let end = (offset as usize)
                .checked_add(limit as usize)
                .ok_or("LIMIT_EXCEEDED")?
                .min(messages.len());
            let page = messages
                .get(offset as usize..end)
                .ok_or("INVALID_REFERENCE")?;
            Ok((
                json!({"binding":binding,"status":result.summary().status,"total":messages.len(),"offset":offset,"messages":page}),
                (end < messages.len()).then_some(Position::Offset(end as u32)),
                "not_applicable",
            ))
        }
        (Part::Value, Position::Values(mut pending)) => {
            let route = pending.pop().ok_or("INVALID_REFERENCE")?;
            let page = result
                .page(
                    &ValueQuery {
                        cell_id: result.cell_id.clone(),
                        out_index: result.out_index,
                        view_id: result.view_id.clone().ok_or("NOT_AVAILABLE")?,
                        path: route.path.clone(),
                        offset: route.row,
                        limit,
                        column_offset: route.column,
                        column_limit: 32,
                        include_source: false,
                    },
                    ctx,
                )
                .map_err(projection)?;
            let column = route
                .column
                .checked_add(page.columns.len() as u32)
                .ok_or("LIMIT_EXCEEDED")?;
            let row = route
                .row
                .checked_add(page.rows.len() as u32)
                .ok_or("LIMIT_EXCEEDED")?;
            if column < page.column_count && column > route.column {
                pending.push(ValueRoute {
                    column,
                    ..route.clone()
                });
            } else if row < page.row_count && row > route.row {
                pending.push(ValueRoute {
                    row,
                    column: 0,
                    ..route.clone()
                });
            }
            // The closed tool has no model-supplied path argument. Its opaque traversal therefore
            // visits actual nested containers after this window, including every matrix column.
            for row in page.rows.iter().rev() {
                for entry in row.cells.iter().rev() {
                    tick(ctx)?;
                    if entry.count > 0
                        && matches!(
                            entry.kind,
                            ValueKind::List
                                | ValueKind::Matrix
                                | ValueKind::Record
                                | ValueKind::Table
                        )
                    {
                        if entry.path.len() > 32 || pending.len() >= 4096 {
                            return Err("LIMIT_EXCEEDED");
                        }
                        pending.push(ValueRoute {
                            path: entry.path.clone(),
                            row: 0,
                            column: 0,
                        });
                    }
                }
            }
            Ok((
                json!({"binding":binding,"page":page}),
                (!pending.is_empty()).then_some(Position::Values(pending)),
                "not_applicable",
            ))
        }
        (Part::Steps, Position::Steps(mut pending)) => {
            let mut entries = Vec::<RetainedStepEntry>::new();
            let mut recorded = true;
            while entries.len() < limit as usize {
                let Some(route) = pending.pop() else { break };
                tick(ctx)?;
                let p = result
                    .steps(&route.parent, route.offset, 1, ctx)
                    .map_err(projection)?;
                recorded &= p.recorded;
                if let Some(entry) = p.entries.into_iter().next() {
                    if route.offset + 1 < p.total {
                        pending.push(StepRoute {
                            offset: route.offset + 1,
                            ..route
                        });
                    }
                    if entry.child_count > 0 {
                        if entry.path.len() > 128 || pending.len() >= 129 {
                            return Err("LIMIT_EXCEEDED");
                        }
                        pending.push(StepRoute {
                            parent: entry.path.clone(),
                            offset: 0,
                        });
                    }
                    entries.push(entry);
                }
            }
            Ok((
                json!({"binding":binding,"recorded":recorded,"entries":entries}),
                (!pending.is_empty()).then_some(Position::Steps(pending)),
                "not_applicable",
            ))
        }
        (Part::Geometry, Position::Geometry { route, offset }) => {
            geometry::page(result, route, offset, limit, ctx)
        }
        (Part::Preview, Position::Offset(0)) => {
            let summary = result.geometry(ctx).map_err(projection)?;
            let render = if summary.available {
                "data_only"
            } else {
                "unavailable"
            };
            Ok((
                json!({"binding":binding,"geometry":summary,"preview_available":false,"reason":"No retained rendered media is registered for this result"}),
                None,
                render,
            ))
        }
        _ => Err("INVALID_REFERENCE"),
    }
}
