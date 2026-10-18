//! Host-owned Agent work reuses native planning and isolated working sessions.
use om_kernel::{
    CellExecutionScope, KernelConfig, Session,
    config::GeneralConfig,
    protocol::{CellInput, CellKind, CellStatus, Dialect, NotebookFile, Request},
};
use om_num::ctx::{Clock, Interrupt};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    cell::Cell,
    collections::BTreeSet,
    sync::{Arc, atomic::AtomicBool},
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Work {
    expected: NotebookFile,
    general: GeneralConfig,
    command: Command,
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    Catalog,
    SourcePreview {
        source: String,
        dialect: Dialect,
    },
    Preview {
        file: NotebookFile,
    },
    Plan {
        cell_ids: Vec<String>,
        scope: CellExecutionScope,
        allowed_cell_ids: Vec<String>,
    },
    Run {
        cell_id: String,
    },
    Scratch {
        source: String,
        dialect: Dialect,
        use_notebook_definitions: bool,
    },
}
fn state(session: &mut Session) -> Result<Value, String> {
    serde_json::to_value(session.handle(Request::GetNotebookState).0)
        .map_err(|_| "Cannot encode math state".into())
}
pub(super) fn execute(session: &mut Session, source: &str) -> Result<String, String> {
    let value = om_agent_contract::json::strict_json_bounded(source.as_bytes(), 4 * 1024 * 1024)
        .map_err(str::to_owned)?;
    let work: Work = serde_json::from_value(value).map_err(|_| "INVALID_ARGUMENT")?;
    let current = state(session)?;
    if current["state"]["file"]
        != serde_json::to_value(&work.expected).map_err(|_| "INVALID_ARGUMENT")?
        || serde_json::to_value(&session.config.general).map_err(|_| "INVALID_ARGUMENT")?
            != serde_json::to_value(&work.general).map_err(|_| "INVALID_ARGUMENT")?
    {
        return Err("STALE_SNAPSHOT".into());
    }
    let cancel = Arc::new(AtomicBool::new(false));
    let clock: Arc<dyn Clock> = Arc::new(super::BrowserClock);
    let ctx = Interrupt {
        flag: cancel.clone(),
        deadline_ms: Some(clock.now_ms() + 5000.0),
        clock: Some(clock),
        steps_left: Cell::new(100_000_000),
    };
    let result = match work.command {
        Command::Catalog => {
            let context = session.execution_context();
            json!({"type":"catalog","catalog":om_kernel::capabilities::function_documents(),"context":{"known_functions":context.known_functions,"owned":context.owned}})
        }
        Command::SourcePreview { source, dialect } => {
            if source.len() > 65536 {
                return Err("LIMIT_EXCEEDED".into());
            }
            let response = session
                .handle(Request::Preview {
                    source,
                    dialect,
                    cursor: None,
                })
                .0;
            let om_kernel::protocol::Response::Preview(preview) = response else {
                return Err("NOT_AVAILABLE".into());
            };
            json!({"type":"source_preview","diagnostics":preview.diagnostics})
        }
        Command::Preview { file } => {
            let context = session.execution_context();
            let plan = om_kernel::source::assess_source_change(
                &work.expected,
                &file,
                session.config.general.dialect,
                session.config.general.constants,
                &context.known_functions,
                &context.owned,
                false,
            )?;
            json!({"type":"preview","analysis":plan.analysis,"affected_cell_ids":plan.affected_cells})
        }
        Command::Plan {
            cell_ids,
            scope,
            allowed_cell_ids,
        } => {
            let plan = session.plan_cell_execution(
                &cell_ids,
                scope,
                &allowed_cell_ids.into_iter().collect::<BTreeSet<_>>(),
                &ctx,
            )?;
            json!({"type":"plan","plan":plan})
        }
        Command::Run { cell_id } => {
            let mut candidate = session
                .fork_working_session(cancel)
                .map_err(|_| "CONTEXT_NOT_READY")?;
            candidate.run_current_cell_with_readiness(&cell_id, true)?;
            let next = candidate.into_session();
            let cell = next
                .notebook
                .cells
                .iter()
                .find(|cell| cell.id == cell_id)
                .ok_or("INVALID_REFERENCE")?;
            let output = cell.output.clone();
            let status = cell.status;
            *session = next;
            json!({"type":"ran","cell_id":cell_id,"status":status,"output":output,"state":state(session)?["state"]})
        }
        Command::Scratch {
            source,
            dialect,
            use_notebook_definitions,
        } => {
            if source.len() > 65536 {
                return Err("LIMIT_EXCEEDED".into());
            }
            if use_notebook_definitions
                && session
                    .notebook
                    .cells
                    .iter()
                    .any(|cell| !cell.defines.is_empty() && cell.status != CellStatus::Done)
            {
                return Err("CONTEXT_NOT_READY".into());
            }
            let mut isolated = if use_notebook_definitions {
                session
                    .fork_working_session(cancel.clone())
                    .map_err(|_| "CONTEXT_NOT_READY")?
                    .into_session()
            } else {
                Session::with_cancel_token(
                    KernelConfig {
                        general: session.config.general.clone(),
                        ..Default::default()
                    },
                    Some(Arc::new(super::BrowserClock)),
                    cancel,
                )
            };
            isolated.config.general.reactive = false;
            isolated.config.general.auto_run_dependents = false;
            isolated.config.llm.enabled = false;
            isolated.config.llm.profiles.clear();
            let mut id = "agent-scratch-input".to_owned();
            while isolated.notebook.cells.iter().any(|cell| cell.id == id) {
                id.push('x');
            }
            isolated.append_isolated_input(CellInput {
                id: id.clone(),
                source,
                kind: CellKind::Math,
                dialect,
            })?;
            isolated.run_isolated_current_cell(&id, &ctx)?;
            let cell = isolated
                .notebook
                .cells
                .iter()
                .find(|cell| cell.id == id)
                .ok_or("INVALID_REFERENCE")?;
            json!({"type":"scratch","status":cell.status,"output":cell.output,"notebook_modified":false})
        }
    };
    serde_json::to_string(&result).map_err(|_| "Cannot encode math result".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn run(session: &mut Session, command: Value) -> Result<Value, String> {
        let work = json!({"expected":state(session)?["state"]["file"],"general":session.config.general,"command":command});
        serde_json::from_str(&execute(session, &work.to_string())?)
            .map_err(|error| error.to_string())
    }
    #[test]
    fn actual_dependency_plan_and_scratch_preserve_parent_state() {
        let mut session = Session::new(Default::default(), None);
        for (id, source) in [
            ("a", "let a=2"),
            ("b", "a+1"),
            ("unrelated", "let other=99"),
        ] {
            session.handle(Request::UpsertCell {
                cell: CellInput {
                    id: id.into(),
                    source: source.into(),
                    kind: CellKind::Math,
                    dialect: Dialect::Modern,
                },
            });
        }
        let plan = run(
            &mut session,
            json!({"type":"plan","cell_ids":["b"],"scope":"required","allowed_cell_ids":["a","b"]}),
        )
        .unwrap();
        assert_eq!(plan["plan"]["ordered_cell_ids"], json!(["a", "b"]));
        assert!(run(&mut session, json!({"type":"run","cell_id":"b"})).is_err());
        run(&mut session, json!({"type":"run","cell_id":"a"})).unwrap();
        let output = run(&mut session, json!({"type":"run","cell_id":"b"})).unwrap();
        assert_eq!(output["output"]["items"][0]["input_form"], "3");
        assert_eq!(output["state"]["cells"][2]["status"], "Stale");
        let before = state(&mut session).unwrap();
        let scratch=run(&mut session,json!({"type":"scratch","source":"let a=5; a+1","dialect":"Modern","use_notebook_definitions":false})).unwrap();
        assert_eq!(scratch["output"]["items"][0]["input_form"], "6");
        assert_eq!(state(&mut session).unwrap(), before);
        let output = session
            .handle(Request::Evaluate {
                cell_id: "verify".into(),
                source: "a+1".into(),
                dialect: Dialect::Modern,
            })
            .0;
        assert_eq!(
            serde_json::to_value(output).unwrap()["output"]["items"][0]["input_form"],
            "3"
        );
    }
}
