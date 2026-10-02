//! Output inspection evaluates a private readonly projection, never a notebook cell.
use super::Session;
use crate::protocol::*;
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt};
impl Session {
    pub(super) fn inspect_expression(&self, source: String, numeric: bool) -> Response {
        let invalid = || {
            self.error(
                "err.inspect",
                "无法查看该表达式",
                "Cannot inspect this expression",
            )
        };
        if source.len() > 65_536 {
            return invalid();
        }
        let Ok(expr) = om_parse::parse_expr(&source, om_parse::Dialect::Wolfram) else {
            return invalid();
        };
        let start = self.clock.as_ref().map(|c| c.now_ms());
        if start.is_some_and(|ms| !ms.is_finite()) {
            return invalid();
        }
        let ctx = Interrupt {
            clock: self.clock.clone(),
            steps_left: std::cell::Cell::new(1_048_576),
            deadline_ms: start
                .map(|ms| ms + self.config.general.eval_timeout_ms.min(5000) as f64)
                .filter(|ms| ms.is_finite()),
            ..Interrupt::default()
        };
        let mut work = vec![&expr];
        while let Some(e) = work.pop() {
            if ctx.tick().is_err() {
                return invalid();
            }
            if let ExprKind::Normal(n) = e.kind() {
                if n.head.as_symbol().is_some_and(|s| {
                    matches!(
                        s.name(),
                        "Set"
                            | "SetDelayed"
                            | "Unset"
                            | "Clear"
                            | "ClearAll"
                            | "CompoundExpression"
                    )
                }) {
                    return invalid();
                }
                work.push(&n.head);
                work.extend(n.args.iter());
            }
        }
        let expr = if numeric {
            Expr::call(B::N, [expr, Expr::int(20)])
        } else {
            expr
        };
        let mut readonly = self.eval.fork_readonly();
        readonly.settings.record_steps = false;
        match readonly.evaluate(&expr, &ctx) {
            Ok(value) => Response::Expression {
                value: crate::output::expression_view(&value),
            },
            Err(_) => invalid(),
        }
    }
}
