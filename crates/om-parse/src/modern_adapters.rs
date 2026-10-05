//! Modern mode/positional adapters only select actually implemented callbacks.
use crate::{
    Span,
    lexer::Token,
    names,
    parser::{Node, Parsed, Parser, text},
};
use om_core::{BUILTIN as B, Expr, ExprKind, Symbol};
pub(crate) struct Keyword {
    pub key: Token,
    pub value: Node,
}
impl Parser<'_> {
    fn take_keyword(&self, keywords: &mut Vec<Keyword>, name: &str) -> Option<Node> {
        keywords
            .iter()
            .position(|k| text(self.src, k.key).eq_ignore_ascii_case(name))
            .map(|i| keywords.remove(i).value)
    }
    fn choice(
        &mut self,
        node: Option<Node>,
        default: &str,
        allowed: &[&str],
    ) -> Result<String, ()> {
        let Some(node) = node else {
            return Ok(default.into());
        };
        let value = match node.expr.kind() {
            ExprKind::String(s) => s.as_ref(),
            ExprKind::Symbol(s) => s.name(),
            _ => return self.fail("E027", "模式参数需要字面枚举，不能求值后猜测"),
        };
        if !allowed.contains(&value) {
            return self.fail("E027", "不支持的模式或输出类型");
        }
        Ok(value.into())
    }
    pub(crate) fn adapt_call(
        &mut self,
        mut symbol: Symbol,
        args: &mut Vec<Node>,
        keywords: &mut Vec<Keyword>,
        span: Span,
    ) -> Result<Symbol, ()> {
        if symbol == B::SOLVE {
            let mode = self.take_keyword(keywords, "mode");
            let output = self.take_keyword(keywords, "output");
            let mode = self.choice(mode, "exact", &["exact", "numeric"])?;
            let output = self.choice(output, "rules", &["rules", "values"])?;
            symbol = Symbol::intern(match (mode.as_str(), output.as_str()) {
                ("numeric", "values") => "NSolveValues",
                ("numeric", _) => "NSolve",
                (_, "values") => "SolveValues",
                _ => "Solve",
            });
        }
        if symbol == B::SIMPLIFY {
            let level = self.take_keyword(keywords, "level");
            if self.choice(level, "basic", &["basic", "deep"])? == "deep" {
                symbol = B::FULL_SIMPLIFY;
            }
        }
        if symbol == B::PLOT {
            let view = self.take_keyword(keywords, "view");
            if self.choice(view, "line", &["line", "contour"])? == "contour" {
                symbol = B::CONTOUR_PLOT;
            }
        }
        if symbol == B::D
            && let Some(order) = self.take_keyword(keywords, "order")
        {
            if args.len() != 2 {
                return self.fail("E027", "order 需要唯一显式求导变量");
            }
            self.named_option(symbol, "order", &order.expr, order.span)?;
            let variable = args.pop().ok_or(())?;
            args.push(self.call(B::LIST, vec![variable, order], span)?);
        }
        if symbol == B::FIND_ROOT {
            let initial = self.take_keyword(keywords, "initial");
            let bracket = self.take_keyword(keywords, "bracket");
            if initial.is_some() && bracket.is_some() {
                return self.fail("E027", "initial 与 bracket 互斥");
            }
            if initial.is_some() || bracket.is_some() {
                if args.len() != 2 || args[1].expr.as_symbol().is_none() {
                    return self.fail(
                        "E027",
                        "initial/bracket 需要显式裸变量，不能与旧起点列表混用",
                    );
                }
                let variable = args.pop().ok_or(())?;
                let starts = if let Some(value) = initial {
                    vec![variable, value]
                } else {
                    let value = bracket.ok_or(())?;
                    if !(value.expr.is_head(B::SPAN) || value.expr.is_head(B::LIST))
                        || value.expr.args().len() != 2
                    {
                        return self.fail("E027", "bracket 需要两端点区间");
                    }
                    vec![
                        variable,
                        Node::atom(value.expr.args()[0].clone(), value.span),
                        Node::atom(value.expr.args()[1].clone(), value.span),
                    ]
                };
                args.push(self.call(B::LIST, starts, span)?);
            }
        }
        let positional = match symbol.name() {
            "N" => Some(("precision", 1)),
            "Coefficient" => Some(("order", 2)),
            "Collect" => Some(("coefficient_fn", 2)),
            "Floor" | "Ceiling" | "Round" => Some(("step", 1)),
            "Range" => Some(("step", 2)),
            _ => None,
        };
        if let Some((name, position)) = positional
            && let Some(value) = self.take_keyword(keywords, name)
        {
            self.named_option(symbol, name, &value.expr, value.span)?;
            if args.len() != position {
                return self.fail("E027", "命名位置参数与已有参数冲突或缺少前置参数");
            }
            if !(symbol == B::N
                && (matches!(value.expr.kind(),ExprKind::String(s) if s.as_ref()=="machine")
                    || value
                        .expr
                        .as_symbol()
                        .is_some_and(|s| s.name() == "machine")))
            {
                args.push(value);
            }
        }
        if matches!(symbol, B::PLOT | B::CONTOUR_PLOT)
            || matches!(symbol.name(), "Integrate" | "NIntegrate")
        {
            let mut remaining = vec![];
            for keyword in std::mem::take(keywords) {
                let key = text(self.src, keyword.key);
                let option = om_core::catalog::by_runtime(symbol.name()).is_some_and(|f| {
                    f.options.iter().any(|p| {
                        p.name.eq_ignore_ascii_case(key)
                            || p.runtime_name.as_deref() == Some(names::option(key).name())
                    })
                });
                if !option
                    && keyword.value.expr.is_head(B::SPAN)
                    && keyword.value.expr.args().len() == 2
                {
                    let axis = Symbol::intern(key);
                    if names::atom(key, self.env.constants) != axis
                        || om_core::builtins::names().contains(&axis.name())
                    {
                        return self.fail(
                            "E027",
                            "数学范围坐标需要自由符号；常量别名可使用严格常量模式",
                        );
                    }
                    args.push(self.call(
                        B::LIST,
                        vec![
                            Node::atom(Expr::sym(axis), keyword.key.span),
                            Node::atom(keyword.value.expr.args()[0].clone(), keyword.value.span),
                            Node::atom(keyword.value.expr.args()[1].clone(), keyword.value.span),
                        ],
                        span,
                    )?);
                } else {
                    remaining.push(keyword);
                }
            }
            *keywords = remaining;
        }
        Ok(symbol)
    }
    pub(crate) fn keyword_rule(&mut self, symbol: Symbol, keyword: Keyword) -> Parsed {
        let key = text(self.src, keyword.key);
        let option = self.named_option(symbol, key, &keyword.value.expr, keyword.key.span)?;
        let span = Span {
            start: keyword.key.span.start,
            end: keyword.value.span.end,
        };
        let mut value = keyword.value;
        if option == B::WORKING_PRECISION
            && matches!(value.expr.kind(),ExprKind::String(s) if s.as_ref()=="machine")
        {
            value.expr = Expr::symbol("MachinePrecision");
        }
        if option.name() == "PlotRange" && value.expr.is_head(B::SPAN) {
            value.expr = Expr::call(B::LIST, value.expr.args().iter().cloned());
        }
        self.call(
            B::RULE,
            vec![Node::atom(Expr::sym(option), keyword.key.span), value],
            span,
        )
    }
}
