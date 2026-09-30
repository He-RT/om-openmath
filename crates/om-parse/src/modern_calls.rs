//! Calls, keyword options and explicit declarations in modern syntax.

use crate::{
    Fix, Severity, Span,
    lexer::TokenKind as K,
    names,
    parser::{Node, Parsed, Parser, text},
};
use om_core::{BUILTIN as B, Expr, Symbol};

impl Parser<'_> {
    pub(super) fn function_call(&mut self, left: Node) -> Parsed {
        let name = self.src[left.span.start as usize..left.span.end as usize].to_owned();
        let symbol = left.expr.as_symbol().ok_or(())?;
        let open = self.expect(K::LParen)?;
        self.groups += 1;
        let result = (|| {
            let mut args = vec![];
            let mut domain = None;
            if self.current().is_some() && self.kind() != Some(K::RParen) {
                loop {
                    if self.kind() == Some(K::Identifier)
                        && self
                            .tokens
                            .get(self.pos + 1)
                            .is_some_and(|t| t.kind == K::Colon)
                    {
                        let key = self.bump().ok_or(())?;
                        self.bump();
                        let value = self.expression(0, false)?;
                        if text(self.src, key).eq_ignore_ascii_case("domain") {
                            if domain.is_some() {
                                return self.fail("E012", "domain 参数重复");
                            }
                            domain = Some(value);
                        } else {
                            let span = Span {
                                start: key.span.start,
                                end: value.span.end,
                            };
                            let key =
                                Node::atom(Expr::sym(names::option(text(self.src, key))), key.span);
                            args.push(self.call(B::RULE, vec![key, value], span)?);
                        }
                    } else {
                        args.push(self.expression(0, false)?);
                    }
                    if self.kind() != Some(K::Comma) {
                        break;
                    }
                    self.bump();
                }
            }
            let end = self.expect(K::RParen)?.span.end;
            let span = Span {
                start: left.span.start,
                end,
            };
            if let Some(domain) = domain {
                self.domain_options(symbol, &mut args, domain)?;
            }
            if !names::is_function(symbol) && !self.env.known_functions.contains(&symbol) {
                self.report(
                    left.span,
                    Severity::Hint,
                    "W002",
                    "未知函数名；表示乘法时请添加 *",
                    Some(Fix {
                        span: Span {
                            start: open.span.start,
                            end: open.span.start,
                        },
                        replacement: "*".into(),
                        label: "改为显式乘法".into(),
                    }),
                );
            }
            match name.to_ascii_lowercase().as_str() {
                "atan2" if args.len() == 2 => {
                    args.swap(0, 1);
                    self.call(B::ARCTAN, args, span)
                }
                "log10" | "log2" if args.len() == 1 => {
                    let base = if name.eq_ignore_ascii_case("log10") {
                        10
                    } else {
                        2
                    };
                    args.insert(0, Node::atom(Expr::int(base), left.span));
                    self.call(B::LOG, args, span)
                }
                "root" if name != "Root" && args.len() == 2 => {
                    let degree = args.pop().ok_or(())?;
                    let reciprocal = self.call(
                        B::POWER,
                        vec![degree, Node::atom(Expr::int(-1), span)],
                        span,
                    )?;
                    let exponent = self.call(
                        B::TIMES,
                        vec![Node::atom(Expr::int(1), span), reciprocal],
                        span,
                    )?;
                    args.push(exponent);
                    self.call(B::POWER, args, span)
                }
                "root" if name == "Root" => self.call(symbol, args, span),
                "atan2" | "log10" | "log2" | "root" => self.fail("E012", "函数参数个数不正确"),
                _ => self.call(symbol, args, span),
            }
        })();
        self.groups -= 1;
        result.map(|mut node| {
            node.call_syntax = true;
            node
        })
    }

    fn domain_options(
        &mut self,
        function: Symbol,
        args: &mut Vec<Node>,
        domain: Node,
    ) -> Result<(), ()> {
        if !matches!(function, B::SOLVE | B::REDUCE) || args.len() < 2 {
            return self.fail("E012", "domain 需要 Solve/Reduce 的显式变量参数");
        }
        let Some(value) = domain.expr.as_symbol() else {
            return self.fail("E012", "未知定义域");
        };
        let positives = value.name().eq_ignore_ascii_case("positives");
        let symbol = match value {
            B::REALS | B::INTEGERS | B::COMPLEXES | B::RATIONALS => value,
            _ if positives => B::REALS,
            _ => return self.fail("E012", "未知定义域"),
        };
        if positives {
            let variables = if args[1].expr.is_head(B::LIST) {
                args[1].expr.args().to_vec()
            } else {
                vec![args[1].expr.clone()]
            };
            let mut conditions = vec![];
            for variable in variables {
                if variable.as_symbol().is_none() {
                    return self.fail("E012", "positives 需要符号变量");
                }
                conditions.push(self.call(
                    B::GREATER,
                    vec![
                        Node::atom(variable, args[1].span),
                        Node::atom(Expr::int(0), domain.span),
                    ],
                    args[1].span,
                )?);
            }
            let first = args.remove(0);
            let head = if first.expr.is_head(B::LIST) {
                B::LIST
            } else {
                B::AND
            };
            let mut combined = first;
            for condition in conditions {
                combined = self.chain(head, combined, condition)?;
            }
            args.insert(0, combined);
        }
        args.insert(2, Node::atom(Expr::sym(symbol), domain.span));
        Ok(())
    }

    pub(crate) fn declaration(&mut self) -> Parsed {
        let start = self.expect(K::Let)?.span.start;
        let token = self.expect(K::Identifier)?;
        let name = text(self.src, token);
        let function = self.kind() == Some(K::LParen);
        let symbol = if function {
            names::modern(name, self.env.constants).unwrap_or_else(|| Symbol::intern(name))
        } else {
            names::atom(name, self.env.constants)
        };
        let mut target = Node::atom(Expr::sym(symbol), token.span);
        if function {
            self.bump();
            self.groups += 1;
            let result = (|| {
                let mut parameters = vec![];
                let mut seen = std::collections::BTreeSet::new();
                if self.kind() != Some(K::RParen) {
                    loop {
                        let parameter = self.expect(K::Identifier)?;
                        let parameter_name = text(self.src, parameter);
                        if !seen.insert(parameter_name) {
                            return self.fail("E012", "函数形参重复");
                        }
                        let blank = self.call(B::BLANK, vec![], parameter.span)?;
                        parameters.push(self.call(
                            B::PATTERN,
                            vec![
                                Node::atom(Expr::symbol(parameter_name), parameter.span),
                                blank,
                            ],
                            parameter.span,
                        )?);
                        if self.kind() != Some(K::Comma) {
                            break;
                        }
                        self.bump();
                    }
                }
                let end = self.expect(K::RParen)?.span.end;
                self.call(
                    symbol,
                    parameters,
                    Span {
                        start: token.span.start,
                        end,
                    },
                )
            })();
            self.groups -= 1;
            target = result?;
        }
        self.expect(K::Assign)?;
        let previous_bindings = std::mem::take(&mut self.bindings);
        let already_known = self.env.known_functions.contains(&symbol);
        if function {
            self.env.known_functions.insert(symbol);
            for parameter in target.expr.args() {
                if let Some(name) = parameter.args().first().and_then(Expr::as_symbol) {
                    self.bindings.insert(name);
                }
            }
        }
        let value = self.expression(0, false);
        self.bindings = previous_bindings;
        if value.is_err() && !already_known {
            self.env.known_functions.remove(&symbol);
        }
        let value = value?;
        let span = Span {
            start,
            end: value.span.end,
        };
        self.call(
            if function { B::SET_DELAYED } else { B::SET },
            vec![target, value],
            span,
        )
    }
}
