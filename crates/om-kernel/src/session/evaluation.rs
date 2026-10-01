//! Parse preflight and sequential evaluation preserve raw provenance and actual history.
use super::Session;
use crate::{
    config::{ConfigDialect, Constants},
    notebook::StatementRecord,
    protocol::*,
};
use om_core::{Abort, Interrupt};
use om_eval::EvalError;
use om_parse::{ParseEnv, ParseOutput};

struct Execution {
    output: CellOutput,
    records: Vec<StatementRecord>,
    exec_count: Option<u32>,
    status: CellStatus,
}

pub(super) struct CellRun {
    pub changed: std::collections::BTreeSet<om_core::Symbol>,
    pub rejected: bool,
}

impl Session {
    pub(super) fn parse_source(&self, source: &str, dialect: Dialect) -> ParseOutput {
        let env = ParseEnv {
            known_functions: self.eval.defs.known_functions(),
            constants: match self.config.general.constants {
                Constants::Math => om_parse::ConstantMode::Math,
                Constants::Strict => om_parse::ConstantMode::Strict,
            },
        };
        let effective = match (dialect, self.config.general.dialect) {
            (Dialect::Auto, ConfigDialect::Modern) => om_parse::Dialect::Modern,
            (Dialect::Auto, ConfigDialect::Wolfram) => om_parse::Dialect::Wolfram,
            _ => dialect.into(),
        };
        om_parse::parse_with(source, effective, &env)
    }

    pub(super) fn analyze_cell(&mut self, index: usize) {
        let cell = &self.notebook.cells[index];
        let (defines, uses) = if cell.kind == CellKind::Math {
            crate::dependency::analyze(&self.parse_source(&cell.source, cell.dialect))
        } else {
            Default::default()
        };
        let cell = &mut self.notebook.cells[index];
        cell.defines = defines;
        cell.uses = uses;
    }

    pub(super) fn run_cell(&mut self, index: usize) -> CellRun {
        let cell = &self.notebook.cells[index];
        let id = cell.id.clone();
        let parsed = self.parse_source(&cell.source, cell.dialect);
        let (defines, uses) = crate::dependency::analyze(&parsed);
        let mut changed = self.cell_symbols(index);
        changed.extend(defines.iter().copied());
        self.notebook.cells[index].defines = defines;
        self.notebook.cells[index].uses = uses;
        let valid = !parsed
            .diagnostics
            .iter()
            .any(|d| d.severity == om_parse::Severity::Error);
        if valid && self.config.general.reactive {
            let mut targets: Vec<_> = self.notebook.cells[index].defines.iter().copied().collect();
            targets.sort_by_key(|s| s.name());
            for symbol in &targets {
                if let Some(owner) = self.owners.get(symbol)
                    && owner != &id
                {
                    let number = self
                        .notebook
                        .cells
                        .iter()
                        .position(|c| &c.id == owner)
                        .map_or(0, |i| i + 1);
                    let text = self.localized(
                        &format!(
                            "err.multiple_definitions: \"{}\" 已在第 {number} 个 cell 中定义",
                            symbol.name()
                        ),
                        &format!(
                            "err.multiple_definitions: \"{}\" is already defined in cell {number}",
                            symbol.name()
                        ),
                    );
                    self.cell_error(index, "err.multiple_definitions", text);
                    return CellRun {
                        changed,
                        rejected: true,
                    };
                }
            }
            changed.extend(self.release_owned(&id));
        }
        self.apply_settings();
        let start = self.clock.as_ref().map(|clock| clock.now_ms());
        let ctx = Interrupt {
            flag: self.interrupt.clone(),
            clock: self.clock.clone(),
            deadline_ms: start
                .map(|ms| ms + self.config.general.eval_timeout_ms as f64)
                .filter(|ms| ms.is_finite()),
            ..Interrupt::default()
        };
        self.eval.defs.take_changed_symbols();
        let mut execution = self.execute(&parsed, &ctx);
        let mutations = self.eval.defs.take_changed_symbols();
        let live = self.eval.defs.defined_symbols();
        self.owners.retain(|symbol, _| live.contains(symbol));
        for &symbol in &mutations {
            if live.contains(&symbol) {
                self.owners.insert(symbol, id.clone());
            } else {
                self.owners.remove(&symbol);
            }
        }
        changed.extend(mutations);
        if let (Some(start), Some(clock)) = (start, &self.clock) {
            let elapsed = (clock.now_ms() - start).max(0.0);
            if elapsed.is_finite() {
                execution.output.timing_ms = elapsed;
            }
        }
        let cell = &mut self.notebook.cells[index];
        cell.output = Some(execution.output);
        cell.status = execution.status;
        cell.exec_count = execution.exec_count;
        cell.records = execution.records;
        CellRun {
            changed,
            rejected: false,
        }
    }

    fn execute(&mut self, parsed: &ParseOutput, ctx: &Interrupt) -> Execution {
        self.eval.messages.take();
        self.eval.last_steps = None;
        let mut result = Execution {
            output: CellOutput {
                items: vec![],
                messages: vec![],
                timing_ms: 0.0,
            },
            records: vec![],
            exec_count: None,
            status: CellStatus::Done,
        };
        for diagnostic in &parsed.diagnostics {
            let level = match diagnostic.severity {
                om_parse::Severity::Error => MsgLevel::Error,
                om_parse::Severity::Warning => MsgLevel::Warning,
                om_parse::Severity::Hint => MsgLevel::Info,
            };
            result.output.messages.push(Message {
                symbol: "Parse".into(),
                tag: diagnostic.code.into(),
                text: diagnostic.message.clone(),
                level,
            });
            if level == MsgLevel::Error {
                result.status = CellStatus::Error;
                result.output.items.push(OutputItem::Error {
                    message: format!("{}: {}", diagnostic.code, diagnostic.message),
                    span: Some(diagnostic.span.into()),
                });
            }
        }
        if result.status == CellStatus::Error {
            return result;
        }
        for statement in &parsed.statements {
            let index = u32::try_from(self.eval.history.len())
                .ok()
                .and_then(|n| n.checked_add(1));
            let Some(out_index) = index else {
                result.status = CellStatus::Error;
                result.output.items.push(OutputItem::Error {
                    message: self.localized(
                        "err.history_limit: 输出历史已超过编号上限",
                        "err.history_limit: Output history exceeds its index limit",
                    ),
                    span: Some(statement.span.into()),
                });
                break;
            };
            let outcome = ctx
                .tick()
                .map_err(EvalError::from)
                .and_then(|()| self.eval.evaluate_statement(&statement.expr, ctx));
            result
                .output
                .messages
                .extend(self.eval.messages.take().iter().map(Message::from));
            let steps = self.eval.last_steps.take();
            match outcome {
                Ok(value) => {
                    let record = StatementRecord {
                        input: statement.expr.clone(),
                        value,
                        steps,
                        suppress_output: statement.suppress_output,
                        out_index,
                        solver: self.eval.take_solver_result(),
                    };
                    if !record.suppress_output {
                        match crate::output::pack(&record, &self.eval, ctx) {
                            Ok(item) => result.output.items.push(item),
                            Err(error) => {
                                let (tag, text) = self.plot_error(&error);
                                result.output.items.push(OutputItem::Error {
                                    message: text.clone(),
                                    span: Some(statement.span.into()),
                                });
                                result.output.messages.push(Message {
                                    symbol: "Plot".into(),
                                    tag: tag.into(),
                                    text,
                                    level: MsgLevel::Error,
                                });
                                result.status = CellStatus::Error;
                            }
                        }
                    }
                    result.records.push(record);
                    result.exec_count = Some(out_index);
                    if result.status == CellStatus::Error {
                        break;
                    }
                }
                Err(error) => {
                    let (tag, text) = self.execution_error(&error);
                    result.output.messages.push(Message {
                        symbol: "Kernel".into(),
                        tag: tag.into(),
                        text: text.clone(),
                        level: MsgLevel::Error,
                    });
                    result.output.items.push(OutputItem::Error {
                        message: text,
                        span: Some(statement.span.into()),
                    });
                    result.status = CellStatus::Error;
                    break;
                }
            }
        }
        if result
            .output
            .messages
            .iter()
            .any(|message| message.level == MsgLevel::Error)
        {
            result.status = CellStatus::Error;
        }
        result
    }

    pub(super) fn execution_error(&self, error: &EvalError) -> (&'static str, String) {
        match error {
            EvalError::Abort(Abort::Interrupted) => (
                "interrupted",
                self.localized("$Aborted: 计算已中断", "$Aborted: interrupted"),
            ),
            EvalError::Abort(Abort::Timeout) => (
                "timeout",
                self.localized("$Aborted: 已超过计算时限", "$Aborted: time limit exceeded"),
            ),
            EvalError::Abort(Abort::Budget) => (
                "budget",
                self.localized(
                    "$Aborted: 已超过计算步骤预算",
                    "$Aborted: step budget exceeded",
                ),
            ),
            _ => ("evaluation", error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests;
