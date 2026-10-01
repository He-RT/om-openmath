//! Parse preflight and sequential evaluation preserve raw provenance and actual history.
use super::Session;
use crate::{
    Cell,
    config::{ConfigDialect, Constants},
    notebook::StatementRecord,
    protocol::*,
};
use om_core::{Abort, Interrupt};
use om_eval::EvalError;
use om_parse::{ParseEnv, ParseOutput};
use std::sync::atomic::Ordering;

struct Execution {
    output: CellOutput,
    records: Vec<StatementRecord>,
    exec_count: Option<u32>,
    status: CellStatus,
}

impl Session {
    pub(super) fn evaluate_cell(
        &mut self,
        cell_id: CellId,
        source: String,
        dialect: Dialect,
    ) -> Response {
        if cell_id.is_empty() {
            return self.error(
                "err.empty_cell_id",
                "单元格 ID 不能为空",
                "Cell IDs must not be empty",
            );
        }
        let index = if let Some(index) = self
            .notebook
            .cells
            .iter()
            .position(|cell| cell.id == cell_id)
        {
            if self.notebook.cells[index].kind != CellKind::Math {
                return self.error(
                    "err.non_math_cell",
                    "只有数学单元格可以执行 CAS 求值",
                    "Only Math cells can run CAS evaluation",
                );
            }
            index
        } else {
            self.notebook.cells.push(Cell::from_input(CellInput {
                id: cell_id.clone(),
                kind: CellKind::Math,
                source: String::new(),
                dialect,
            }));
            self.notebook.cells.len() - 1
        };
        self.notebook.cells[index].source = source.clone();
        self.notebook.cells[index].dialect = dialect;
        self.notebook.cells[index].status = CellStatus::Running;
        self.interrupt.store(false, Ordering::Relaxed);
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
        let parsed = om_parse::parse_with(&source, effective, &env);
        let mut execution = self.execute(&parsed, &ctx);
        if let (Some(start), Some(clock)) = (start, &self.clock) {
            let elapsed = (clock.now_ms() - start).max(0.0);
            if elapsed.is_finite() {
                execution.output.timing_ms = elapsed;
            }
        }
        // The index cannot change during this exclusive Session borrow.
        let cell = &mut self.notebook.cells[index];
        cell.output = Some(execution.output.clone());
        cell.status = execution.status;
        cell.exec_count = execution.exec_count;
        cell.records = execution.records;
        Response::Evaluated {
            cell_id,
            output: execution.output,
            reran: vec![],
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
                    };
                    if !record.suppress_output {
                        result.output.items.push(OutputItem::Expr {
                            out_index: record.out_index,
                            input_form: om_format::input_form(&record.value),
                            modern_form: om_format::modern_form(&record.value),
                            latex: om_format::latex(&record.value),
                        });
                    }
                    result.records.push(record);
                    result.exec_count = Some(out_index);
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

    fn execution_error(&self, error: &EvalError) -> (&'static str, String) {
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
