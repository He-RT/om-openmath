//! Synchronous shared-kernel entry point with host-injected time and cancellation.
mod editing;
mod editor;
mod evaluation;
mod graph;
mod plotting;
mod reactive;

use crate::{KernelConfig, Notebook, config::Language, notebook::FileError, protocol::*};
use om_eval::Evaluator;
use om_num::ctx::Clock;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// One isolated evaluator, notebook, configuration and cancellation scope.
pub struct Session {
    eval: Evaluator,
    /// Notebook sources and execution state.
    pub notebook: Notebook,
    /// Current configuration; frontend serialization masks API keys.
    pub config: KernelConfig,
    interrupt: Arc<AtomicBool>,
    clock: Option<Arc<dyn Clock>>,
    owners: std::collections::BTreeMap<om_core::Symbol, CellId>,
}

impl Session {
    /// Create an independent session; hosts supply a clock for deadlines and timing.
    pub fn new(config: KernelConfig, clock: Option<Arc<dyn Clock>>) -> Self {
        let mut session = Self {
            eval: Evaluator::new(),
            notebook: Notebook::default(),
            config,
            interrupt: Arc::new(AtomicBool::new(false)),
            clock,
            owners: Default::default(),
        };
        session.apply_settings();
        session
    }

    /// Shared flag that a host may set while synchronous evaluation is running.
    pub fn interrupt_handle(&self) -> Arc<AtomicBool> {
        self.interrupt.clone()
    }

    /// Handle one client request and return its reply plus any asynchronous events.
    pub fn handle(&mut self, req: Request) -> (Response, Vec<Event>) {
        match req {
            Request::Evaluate {
                cell_id,
                source,
                dialect,
            } => return self.evaluate_cell(cell_id, source, dialect),
            Request::UpsertCell { cell } => return self.upsert_cell(cell),
            Request::DeleteCell { cell_id } => return self.delete_cell(cell_id),
            Request::MoveCell { cell_id, to_index } => {
                return (self.move_cell(cell_id, to_index), vec![]);
            }
            Request::RunAll => return self.run_all(),
            Request::SamplePlot { request } => return (self.sample_plot(request), vec![]),
            Request::Preview {
                source,
                dialect,
                cursor,
            } => return (self.preview(source, dialect, cursor), vec![]),
            Request::Complete {
                source,
                dialect,
                cursor,
            } => return (self.complete(source, dialect, cursor), vec![]),
            Request::Hover {
                source,
                dialect,
                cursor,
            } => return (self.hover(source, dialect, cursor), vec![]),
            _ => {}
        }
        let response = match req {
            Request::GetConfig => Response::Config {
                config: self.config.clone(),
            },
            Request::SetConfig { mut config } => {
                let mut names = std::collections::BTreeSet::new();
                if config
                    .llm
                    .profiles
                    .iter()
                    .any(|profile| profile.name.is_empty() || !names.insert(&profile.name))
                {
                    self.error(
                        "err.invalid_profiles",
                        "配置名称必须非空且唯一",
                        "Profile names must be nonempty and unique",
                    )
                } else {
                    config.merge_redacted_keys(&self.config);
                    self.config = config;
                    self.apply_settings();
                    Response::Ok
                }
            }
            Request::LoadNotebook { file } => match Notebook::from_file(file) {
                Ok(notebook) => {
                    self.notebook = notebook;
                    self.eval = Evaluator::new();
                    self.owners.clear();
                    self.apply_settings();
                    for index in 0..self.notebook.cells.len() {
                        self.analyze_cell(index);
                    }
                    Response::Ok
                }
                Err(FileError::Version(version)) => self.error(
                    "err.notebook_version",
                    &format!("不支持的笔记本版本：{version}"),
                    &format!("Unsupported notebook version: {version}"),
                ),
                Err(FileError::EmptyId) => self.error(
                    "err.empty_cell_id",
                    "单元格 ID 不能为空",
                    "Cell IDs must not be empty",
                ),
                Err(FileError::DuplicateId(id)) => self.error(
                    "err.duplicate_cell_id",
                    &format!("重复的单元格 ID：{id}"),
                    &format!("Duplicate cell ID: {id}"),
                ),
            },
            Request::SaveNotebook => Response::Notebook {
                file: self.notebook.to_file(),
            },
            Request::Interrupt => {
                self.interrupt.store(true, Ordering::Relaxed);
                Response::Ok
            }
            _ => self.error(
                "err.not_implemented",
                "此请求尚未实现",
                "This request is not implemented yet",
            ),
        };
        (response, vec![])
    }

    fn apply_settings(&mut self) {
        self.eval.settings.record_steps = self.config.general.show_steps;
    }

    fn localized(&self, zh: &str, en: &str) -> String {
        if self.config.general.language == Language::En {
            en.into()
        } else {
            zh.into()
        }
    }

    fn error(&self, key: &str, zh: &str, en: &str) -> Response {
        Response::Error {
            message: format!("{key}: {}", self.localized(zh, en)),
        }
    }
}
