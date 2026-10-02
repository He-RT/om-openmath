//! Strict actual CAS tools execute against a readonly snapshot with independent limits.
use super::{LlmCancellation, Session};
use crate::{
    assistant::LIMIT,
    protocol::{Message, Suggestion},
};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt};
use om_llm::ToolCall;
use serde::Deserialize;
use serde_json::json;
pub(super) struct ToolOutput {
    pub(super) content: String,
    pub(super) suggestion: Option<Suggestion>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Evaluate {
    code: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Solve {
    equations: Vec<String>,
    variables: Vec<String>,
    #[serde(default)]
    domain: Domain,
}
#[derive(Deserialize, Default)]
enum Domain {
    #[default]
    Complexes,
    Reals,
    Integers,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Propose {
    code: String,
    dialect: String,
}
impl Session {
    pub(super) fn llm_tool(&self, call: &ToolCall, cancel: &LlmCancellation) -> ToolOutput {
        match self.execute_llm_tool(call, cancel) {
            Ok(output) if output.content.len() <= LIMIT => output,
            Ok(_) => ToolOutput {
                content: json!({"error":"Tool result exceeds source limit"}).to_string(),
                suggestion: None,
            },
            Err(error) => ToolOutput {
                content: json!({"error":error}).to_string(),
                suggestion: None,
            },
        }
    }
    fn execute_llm_tool(
        &self,
        call: &ToolCall,
        cancel: &LlmCancellation,
    ) -> Result<ToolOutput, String> {
        if call.arguments.len() > LIMIT {
            return Err("Tool arguments exceed source limit".into());
        }
        if call.name == "propose_cell" {
            let p: Propose =
                serde_json::from_str(&call.arguments).map_err(|_| "Invalid propose_cell object")?;
            let dialect = match p.dialect.as_str() {
                "modern" => om_parse::Dialect::Modern,
                "wolfram" => om_parse::Dialect::Wolfram,
                _ => return Err("Invalid proposal dialect".into()),
            };
            let expr = self.parse_tool_source(&p.code, dialect)?;
            let suggestion = Suggestion {
                wolfram: om_format::input_form(&expr),
                modern: om_format::modern_form(&expr),
                latex: om_format::latex(&expr),
                explanation: self.localized("建议代码，尚未执行", "Proposed source, not executed"),
            };
            return Ok(ToolOutput {content:json!({"proposed":true,"wolfram":suggestion.wolfram,"modern":suggestion.modern,"executed":false}).to_string(),suggestion:Some(suggestion)});
        }
        if !matches!(call.name.as_str(), "evaluate" | "solve") {
            return Err("Unknown CAS tool".into());
        }
        let clock = self
            .clock
            .clone()
            .ok_or("CAS tools require a host clock for the five-second deadline")?;
        let now = clock.now_ms();
        if !now.is_finite() || !(now + 5000.0).is_finite() {
            return Err("Invalid host clock".into());
        }
        let ctx = Interrupt {
            flag: cancel.flag.clone(),
            deadline_ms: Some(now + 5000.0),
            clock: Some(clock),
            ..Interrupt::default()
        };
        let expr = if call.name == "evaluate" {
            let p: Evaluate =
                serde_json::from_str(&call.arguments).map_err(|_| "Invalid evaluate object")?;
            self.parse_tool_source(&p.code, om_parse::Dialect::Wolfram)?
        } else {
            let p: Solve =
                serde_json::from_str(&call.arguments).map_err(|_| "Invalid solve object")?;
            if p.equations.is_empty()
                || p.variables.is_empty()
                || p.equations.len() > 64
                || p.variables.len() > 64
            {
                return Err("Solve requires 1..64 equations and variables".into());
            }
            let equations = p
                .equations
                .iter()
                .map(|e| self.parse_tool_source(e, om_parse::Dialect::Wolfram))
                .collect::<Result<Vec<_>, _>>()?;
            let variables = p
                .variables
                .iter()
                .map(|e| self.parse_tool_source(e, om_parse::Dialect::Wolfram))
                .collect::<Result<Vec<_>, _>>()?;
            let mut seen = std::collections::BTreeSet::new();
            if variables
                .iter()
                .any(|v| v.as_symbol().is_none_or(|s| !seen.insert(s)))
            {
                return Err("Variables must be distinct symbols".into());
            }
            Expr::call(
                B::SOLVE,
                [
                    Expr::call(B::LIST, equations),
                    Expr::call(B::LIST, variables),
                    Expr::sym(match p.domain {
                        Domain::Complexes => B::COMPLEXES,
                        Domain::Reals => B::REALS,
                        Domain::Integers => B::INTEGERS,
                    }),
                ],
            )
        };
        let mut work = vec![&expr];
        while let Some(e) = work.pop() {
            ctx.tick().map_err(|e| e.to_string())?;
            if let ExprKind::Normal(n) = e.kind() {
                if matches!(n.head.as_symbol(), Some(B::SET | B::SET_DELAYED | B::CLEAR))
                    || n.head
                        .as_symbol()
                        .is_some_and(|s| matches!(s.name(), "Unset" | "ClearAll"))
                {
                    return Err("CAS tools cannot change definitions".into());
                }
                work.push(&n.head);
                work.extend(n.args.iter());
            }
        }
        let mut readonly = self.eval.fork_readonly();
        readonly.settings.record_steps = true;
        let value = readonly.evaluate(&expr, &ctx).map_err(|e| e.to_string())?;
        let messages = readonly
            .messages
            .take()
            .iter()
            .map(Message::from)
            .collect::<Vec<_>>();
        if call.name == "evaluate" {
            return Ok(ToolOutput {content:json!({"input_form":om_format::input_form(&value),"latex":om_format::latex(&value),"messages":messages}).to_string(),suggestion:None});
        }
        let supported = readonly.take_solver_result().is_some();
        let steps = readonly
            .last_steps
            .as_ref()
            .map(crate::output::render_steps);
        let mut summary = vec![];
        if let Some(steps) = &steps {
            let mut work = steps.root.iter().rev().collect::<Vec<_>>();
            while let Some(step) = work.pop() {
                summary.push(step.title_key.clone());
                work.extend(step.children.iter().rev());
            }
        }
        Ok(ToolOutput {content:json!({"solutions":om_format::input_form(&value),"steps_summary":summary,"messages":messages,"supported":supported}).to_string(),suggestion:None})
    }
    fn parse_tool_source(&self, source: &str, dialect: om_parse::Dialect) -> Result<Expr, String> {
        if source.len() > LIMIT {
            return Err("Tool source exceeds limit".into());
        }
        let parsed = self.parse_source(source, dialect.into());
        let errors = parsed
            .diagnostics
            .iter()
            .filter(|d| d.severity == om_parse::Severity::Error)
            .map(|d| format!("{}: {}", d.code, d.message))
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
        if parsed.statements.len() != 1 || parsed.statements[0].suppress_output {
            return Err("Expected one nonempty expression".into());
        }
        Ok(parsed
            .statements
            .into_iter()
            .next()
            .ok_or("Missing expression")?
            .expr)
    }
}
