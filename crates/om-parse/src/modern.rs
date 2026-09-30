//! Pratt binding powers implement the ordering in PLAN §7.2.

use crate::{
    Fix, Severity, Span,
    lexer::TokenKind as K,
    literals, names,
    parser::{Node, Parsed, Parser, text},
};
use om_core::{BUILTIN as B, Expr, Symbol};

#[path = "modern_calls.rs"]
mod calls;

impl Parser<'_> {
    pub(crate) fn expression(&mut self, min: u8, stop_bar: bool) -> Parsed {
        if self.depth >= 128 {
            return self.fail("E013", "表达式嵌套超出解析限制");
        }
        self.depth += 1;
        let result = self.expression_inner(min, stop_bar);
        self.depth -= 1;
        result
    }
    fn expression_inner(&mut self, min: u8, stop_bar: bool) -> Parsed {
        let mut left = self.prefix(stop_bar)?;
        while let Some(token) = self.current() {
            let kind = token.kind;
            if kind == K::Bar && stop_bar {
                break;
            }
            if kind == K::LParen && left.direct_name && left.span.end == token.span.start {
                if 120 < min {
                    break;
                }
                left = self.function_call(left)?;
                continue;
            }
            if kind == K::LBracket && left.direct_name && left.span.end == token.span.start {
                if 120 < min {
                    break;
                }
                let start = left.span.start;
                let (mut args, end) = self.arguments(K::LBracket, K::RBracket)?;
                args.insert(0, left);
                left = self.call(B::PART, args, Span { start, end })?;
                continue;
            }
            if kind == K::Bang {
                if 110 < min {
                    break;
                }
                self.bump();
                let span = Span {
                    start: left.span.start,
                    end: token.span.end,
                };
                left = self.call(B::FACTORIAL, vec![left], span)?;
                continue;
            }
            if kind == K::Superscript {
                if 100 < min {
                    break;
                }
                self.bump();
                let value = super_number(text(self.src, token)).ok_or_else(|| {
                    self.report(token.span, Severity::Error, "E011", "非法上标指数", None)
                })?;
                let span = Span {
                    start: left.span.start,
                    end: token.span.end,
                };
                left = self.call(
                    B::POWER,
                    vec![left, Node::atom(Expr::int(value), token.span)],
                    span,
                )?;
                continue;
            }
            if comparison(kind).is_some() {
                if 60 < min {
                    break;
                }
                left = self.comparisons(left, stop_bar)?;
                continue;
            }
            if matches!(kind, K::Where | K::ReplaceAll) {
                if 10 < min {
                    break;
                }
                self.bump();
                let right = if kind == K::Where {
                    self.where_rules(stop_bar)?
                } else {
                    self.expression(11, stop_bar)?
                };
                let span = Span {
                    start: left.span.start,
                    end: right.span.end,
                };
                left = self.call(B::REPLACE_ALL, vec![left, right], span)?;
                continue;
            }
            if let Some((bp, right_bp, head)) = infix(kind) {
                if bp < min {
                    break;
                }
                self.bump();
                let mut right = self.expression(right_bp, stop_bar)?;
                if kind == K::Minus {
                    let span = right.span;
                    right = self.call(
                        B::TIMES,
                        vec![Node::atom(Expr::int(-1), token.span), right],
                        span,
                    )?;
                } else if kind == K::Slash {
                    let span = right.span;
                    right = self.call(
                        B::POWER,
                        vec![right, Node::atom(Expr::int(-1), token.span)],
                        span,
                    )?;
                }
                left = if matches!(head, B::PLUS | B::TIMES | B::AND | B::OR) {
                    self.chain(head, left, right)?
                } else {
                    let span = Span {
                        start: left.span.start,
                        end: right.span.end,
                    };
                    self.call(head, vec![left, right], span)?
                };
                continue;
            }
            if starts_atom(kind) {
                if 80 < min {
                    break;
                }
                if kind == K::LParen
                    && left.direct_name
                    && self.src[left.span.end as usize..token.span.start as usize]
                        .chars()
                        .all(char::is_whitespace)
                {
                    let gap = Span {
                        start: left.span.end,
                        end: token.span.start,
                    };
                    self.report(
                        gap,
                        Severity::Hint,
                        "W001",
                        "函数名与括号间的空白被当作乘法；调用函数请去掉空白",
                        Some(Fix {
                            span: gap,
                            replacement: String::new(),
                            label: "去掉函数与括号間的空白".into(),
                        }),
                    );
                }
                let right = self.expression(81, stop_bar)?;
                left = self.chain(B::TIMES, left, right)?;
                continue;
            }
            break;
        }
        Ok(left)
    }
    fn prefix(&mut self, stop_bar: bool) -> Parsed {
        let Some(token) = self.bump() else {
            return self.fail("E022", "需要表达式");
        };
        match token.kind {
            K::Number => {
                let expr = literals::number(text(self.src, token)).map_err(|message| {
                    self.report(token.span, Severity::Error, "E011", message, None)
                })?;
                Ok(Node::atom(expr, token.span))
            }
            K::String => {
                let expr = literals::string(text(self.src, token)).map_err(|message| {
                    self.report(token.span, Severity::Error, "E006", message, None)
                })?;
                Ok(Node::atom(expr, token.span))
            }
            K::Identifier => {
                let name = text(self.src, token);
                let raw_symbol = Symbol::intern(name);
                let bound = self.bindings.contains(&raw_symbol);
                let function = if bound {
                    None
                } else {
                    names::modern(name, self.env.constants).filter(|s| names::is_function(*s))
                };
                let symbol = if bound {
                    raw_symbol
                } else if self
                    .current()
                    .is_some_and(|t| t.kind == K::LParen && token.span.end == t.span.start)
                {
                    names::modern(name, self.env.constants).unwrap_or(raw_symbol)
                } else {
                    names::atom(name, self.env.constants)
                };
                let mut node = Node::atom(Expr::sym(symbol), token.span);
                node.direct_name = true;
                if function.is_some()
                    && (name.chars().count() > 1 || name == function.map_or("", Symbol::name))
                    && self
                        .kind()
                        .is_some_and(|k| starts_atom(k) && !matches!(k, K::LParen | K::LBracket))
                {
                    let right = self.expression(61, stop_bar)?;
                    let span = Span {
                        start: token.span.start,
                        end: right.span.end,
                    };
                    let replacement = format!(
                        "{}({})",
                        name,
                        &self.src[right.span.start as usize..right.span.end as usize]
                    );
                    let message = format!("函数需要括号：{replacement}");
                    self.report(
                        span,
                        Severity::Error,
                        "E010",
                        &message,
                        Some(Fix {
                            span,
                            replacement,
                            label: "添加函数调用括号".into(),
                        }),
                    );
                    return self.call(function.ok_or(())?, vec![right], span);
                }
                Ok(node)
            }
            K::Minus | K::Plus | K::Not | K::Bang | K::Sqrt => {
                let arg = self.expression(
                    if matches!(token.kind, K::Not | K::Bang) {
                        50
                    } else {
                        90
                    },
                    stop_bar,
                )?;
                let span = Span {
                    start: token.span.start,
                    end: arg.span.end,
                };
                match token.kind {
                    K::Plus => Ok(Node {
                        span,
                        direct_name: false,
                        ..arg
                    }),
                    K::Minus if arg.expr.as_number().is_some() => Ok(Node::atom(
                        Expr::number(arg.expr.as_number().map(|n| n.neg()).ok_or(())?),
                        span,
                    )),
                    K::Minus => self.call(
                        B::TIMES,
                        vec![Node::atom(Expr::int(-1), token.span), arg],
                        span,
                    ),
                    K::Sqrt => self.call(B::SQRT, vec![arg], span),
                    _ => self.call(B::NOT, vec![arg], span),
                }
            }
            K::LParen | K::Bar => {
                self.groups += 1;
                let close = if token.kind == K::LParen {
                    K::RParen
                } else {
                    K::Bar
                };
                let result = (|| {
                    let mut inner = self.expression(0, close == K::Bar)?;
                    let end = self.expect(close)?.span.end;
                    let span = Span {
                        start: token.span.start,
                        end,
                    };
                    if close == K::Bar {
                        self.call(B::ABS, vec![inner], span)
                    } else {
                        inner.span = span;
                        inner.direct_name = false;
                        Ok(inner)
                    }
                })();
                self.groups -= 1;
                result
            }
            K::LBracket | K::LBrace => {
                self.pos -= 1;
                let close = if token.kind == K::LBracket {
                    K::RBracket
                } else {
                    K::RBrace
                };
                let (args, end) = self.arguments(token.kind, close)?;
                self.call(
                    B::LIST,
                    args,
                    Span {
                        start: token.span.start,
                        end,
                    },
                )
            }
            K::Out => {
                let mut args = vec![];
                let mut end = token.span.end;
                if let Some(next) = self.current()
                    && next.kind == K::Number
                    && next.span.start == end
                    && text(self.src, next).bytes().all(|c| c.is_ascii_digit())
                {
                    self.bump();
                    end = next.span.end;
                    args.push(Node::atom(
                        literals::number(text(self.src, next)).map_err(|message| {
                            self.report(next.span, Severity::Error, "E011", message, None)
                        })?,
                        next.span,
                    ));
                }
                self.call(
                    B::OUT,
                    args,
                    Span {
                        start: token.span.start,
                        end,
                    },
                )
            }
            K::Error => Err(()),
            _ => {
                self.report(token.span, Severity::Error, "E022", "需要表达式", None);
                Err(())
            }
        }
    }
    fn comparisons(&mut self, mut left: Node, stop_bar: bool) -> Parsed {
        let start = left.span.start;
        let mut comparisons = vec![];
        while let Some(token) = self.current() {
            let Some(head) = comparison(token.kind) else {
                break;
            };
            self.bump();
            let right = self.expression(61, stop_bar)?;
            let span = Span {
                start: left.span.start,
                end: right.span.end,
            };
            let next = Node {
                expr: right.expr.clone(),
                span: right.span,
                height: right.height,
                direct_name: right.direct_name,
            };
            comparisons.push(self.call(head, vec![left, right], span)?);
            left = next;
        }
        if comparisons.len() == 1 {
            comparisons.pop().ok_or(())
        } else {
            let end = left.span.end;
            self.call(B::AND, comparisons, Span { start, end })
        }
    }
    fn where_rules(&mut self, stop_bar: bool) -> Parsed {
        let start = self.at_span().start;
        let mut rules = vec![];
        loop {
            let left = self.expression(61, stop_bar)?;
            if !matches!(self.kind(), Some(K::Assign | K::Equal)) {
                return self.fail("E022", "where 需要变量 = 值");
            }
            self.bump();
            let right = self.expression(11, stop_bar)?;
            let span = Span {
                start: left.span.start,
                end: right.span.end,
            };
            rules.push(self.call(B::RULE, vec![left, right], span)?);
            if self.kind() != Some(K::Comma) {
                break;
            }
            self.bump();
        }
        let end = rules.last().map_or(start, |n| n.span.end);
        self.call(B::LIST, rules, Span { start, end })
    }
    pub(crate) fn arguments(&mut self, open: K, close: K) -> Result<(Vec<Node>, u32), ()> {
        self.expect(open)?;
        self.groups += 1;
        let result = (|| {
            let mut args = vec![];
            if self.current().is_some() && self.kind() != Some(close) {
                loop {
                    args.push(self.expression(0, false)?);
                    if self.kind() != Some(K::Comma) {
                        break;
                    }
                    self.bump();
                }
            }
            let end = self.expect(close)?.span.end;
            Ok((args, end))
        })();
        self.groups -= 1;
        result
    }
}

fn starts_atom(kind: K) -> bool {
    matches!(
        kind,
        K::Number
            | K::Identifier
            | K::String
            | K::LParen
            | K::LBracket
            | K::LBrace
            | K::Sqrt
            | K::Out
            | K::Bar
    )
}
fn infix(kind: K) -> Option<(u8, u8, Symbol)> {
    Some(match kind {
        K::Rule => (20, 20, B::RULE),
        K::Or => (30, 31, B::OR),
        K::And => (40, 41, B::AND),
        K::Plus | K::Minus => (70, 71, B::PLUS),
        K::Star | K::Slash => (80, 81, B::TIMES),
        K::Power => (100, 100, B::POWER),
        _ => return None,
    })
}
fn comparison(kind: K) -> Option<Symbol> {
    Some(match kind {
        K::Assign | K::Equal => B::EQUAL,
        K::Unequal => B::UNEQUAL,
        K::Less => B::LESS,
        K::LessEqual => B::LESS_EQUAL,
        K::Greater => B::GREATER,
        K::GreaterEqual => B::GREATER_EQUAL,
        K::Element => B::ELEMENT,
        _ => return None,
    })
}
fn super_number(text: &str) -> Option<i64> {
    let digits: String = text
        .chars()
        .map(|c| match c {
            '⁰' => '0',
            '¹' => '1',
            '²' => '2',
            '³' => '3',
            '⁴' => '4',
            '⁵' => '5',
            '⁶' => '6',
            '⁷' => '7',
            '⁸' => '8',
            '⁹' => '9',
            '⁻' => '-',
            _ => c,
        })
        .collect();
    digits.parse().ok()
}
