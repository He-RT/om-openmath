//! Synchronous shared-kernel entry point with host-injected time and cancellation.
mod assistant;
mod editing;
mod editor;
mod evaluation;
mod graph;
mod llm;
pub use llm::LlmCancellation;
mod exploration;
mod inspection;
#[cfg(feature = "native")]
mod native;
mod plotting;
mod reactive;
mod recovery;
mod values;

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
    system_language: Language,
    owners: std::collections::BTreeMap<om_core::Symbol, CellId>,
    llm: llm::LlmState,
    output_serial: u64,
    #[cfg(feature = "native")]
    config_store: Option<crate::native::ConfigStore>,
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
            system_language: Language::ZhCn,
            owners: Default::default(),
            llm: Default::default(),
            output_serial: 0,
            #[cfg(feature = "native")]
            config_store: None,
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
            Request::InspectExploreExpression {
                context,
                values,
                source,
                numeric,
            } => {
                return (
                    self.inspect_explore_expression(context, values, source, numeric),
                    vec![],
                );
            }
            Request::SampleExplorePlot {
                context,
                values,
                request,
            } => return (self.sample_explore_plot(context, values, request), vec![]),
            Request::GetExploreContext { query } => return (self.explore_context(query), vec![]),
            Request::SampleExplore {
                query,
                values,
                revision,
            } => return (self.sample_explore(query, values, revision), vec![]),
            Request::RunExploreContext {
                context,
                values,
                revision,
            } => return (self.run_explore_context(context, values, revision), vec![]),
            Request::GetFunctionCatalog => {
                return (
                    Response::FunctionCatalog {
                        catalog: crate::capabilities::function_catalog(),
                    },
                    vec![],
                );
            }
            Request::GetCapabilities { platform } => {
                return (
                    Response::Capabilities {
                        capabilities: crate::capabilities::capabilities(platform),
                    },
                    vec![],
                );
            }
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
            Request::InspectExpression { source, numeric } => {
                return (self.inspect_expression(source, numeric), vec![]);
            }
            Request::InspectValue { query } => return (self.inspect_value(query), vec![]),
            Request::GetNotebookState => return (self.notebook_state(), vec![]),
            Request::GetVariables => return (self.variables(), vec![]),
            Request::RestoreDefinitions => return self.restore_definitions(),
            Request::RenameNotebook { title } => {
                self.notebook.title = title;
                return (Response::Ok, vec![]);
            }
            Request::SetSystemLanguage { language } => {
                self.system_language = language;
                return (Response::Ok, vec![]);
            }
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
            Request::LlmTranslate { .. }
            | Request::LlmExplain { .. }
            | Request::LlmComplete { .. }
            | Request::LlmChat { .. }
            | Request::LlmFixError { .. }
            | Request::LlmTestProfile { .. }
            | Request::LlmCancel { .. }
            | Request::LlmHttpChunk { .. }
            | Request::LlmHttpEnd { .. } => return self.handle_llm(req),
            #[cfg(feature = "native")]
            Request::GetConfig => return (self.stored_config(), vec![]),
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
                    #[cfg(feature = "native")]
                    if let Some(store) = &self.config_store {
                        config = match store.submit(&config, &self.config) {
                            Ok(config) => config,
                            Err(error) => return (self.config_error(&error), vec![]),
                        };
                    }
                    config.merge_redacted_keys(&self.config);
                    self.config = config;
                    self.apply_settings();
                    Response::Ok
                }
            }
            Request::LoadNotebook { file } => match Notebook::from_file(file) {
                Ok(notebook) => {
                    let events = self.cancel_all_llm();
                    self.notebook = notebook;
                    self.eval = Evaluator::new();
                    self.owners.clear();
                    self.apply_settings();
                    for index in 0..self.notebook.cells.len() {
                        self.analyze_cell(index);
                    }
                    return (Response::Ok, events);
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
        if self.effective_language() == Language::En {
            en.into()
        } else {
            zh.into()
        }
    }
    /// User language or the injected host locale when the preference remains Auto.
    pub fn effective_language(&self) -> Language {
        match self.config.general.language {
            Language::Auto => self.system_language,
            language => language,
        }
    }

    fn error(&self, key: &str, zh: &str, en: &str) -> Response {
        Response::Error {
            message: format!("{key}: {}", self.localized(zh, en)),
        }
    }
}
