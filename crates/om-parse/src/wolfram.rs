//! Supported Wolfram operators in the official relative precedence order.
// See https://reference.wolfram.com/language/tutorial/OperatorInputForms.html.

use crate::{
    Span,
    lexer::TokenKind as K,
    parser::{Node, Parsed, Parser, text},
};
use om_core::{BUILTIN as B, Expr, Symbol};

#[path = "wolfram_atoms.rs"]
mod atoms;

impl Parser<'_> {
    pub(crate) fn wl_expression(&mut self, min: u8) -> Parsed {
        if self.depth >= 128 {
            return self.fail("E013", "表达式嵌套超出解析限制");
        }
        self.depth += 1;
        let result = self.wl_inner(min);
        self.depth -= 1;
        result
    }
    fn wl_inner(&mut self, min: u8) -> Parsed {
        let mut left = self.wl_atom()?;
        while let Some(token) = self.current() {
            if token.kind == K::LBracket {
                if 150 < min {
                    break;
                }
                left = self.wl_call(left)?;
                continue;
            }
            if matches!(
                token.kind,
                K::Blank | K::BlankSequence | K::BlankNullSequence
            ) && left.direct_name
                && left.span.end == token.span.start
            {
                if 160 < min {
                    break;
                }
                let blank = self.wl_blank()?;
                let span = Span {
                    start: left.span.start,
                    end: blank.span.end,
                };
                left = self.call(B::PATTERN, vec![left, blank], span)?;
                continue;
            }
            if token.kind == K::Prime {
                if 120 < min {
                    break;
                }
                let mut count = 0;
                let mut end = token.span.end;
                while self.kind() == Some(K::Prime) {
                    count += 1;
                    if let Some(t) = self.bump() {
                        end = t.span.end;
                    }
                }
                let span = Span {
                    start: left.span.start,
                    end,
                };
                let derivative = self.call(
                    Symbol::intern("Derivative"),
                    vec![Node::atom(Expr::int(count), token.span)],
                    span,
                )?;
                left = self.apply(derivative, vec![left], span)?;
                continue;
            }
            if matches!(token.kind, K::Bang | K::Function | K::Unset) {
                let (bp, head) = match token.kind {
                    K::Bang => (120, B::FACTORIAL),
                    K::Function => (15, B::FUNCTION),
                    _ => (10, Symbol::intern("Unset")),
                };
                if bp < min {
                    break;
                }
                self.bump();
                let span = Span {
                    start: left.span.start,
                    end: token.span.end,
                };
                left = self.call(head, vec![left], span)?;
                continue;
            }
            if comparison(token.kind).is_some() {
                if 70 < min {
                    break;
                }
                left = self.wl_comparisons(left)?;
                continue;
            }
            if token.kind == K::Semicolon {
                if 5 < min {
                    break;
                }
                self.bump();
                let right = if matches!(
                    self.kind(),
                    None | Some(
                        K::RParen | K::RBracket | K::RBrace | K::Comma | K::Newline | K::Semicolon
                    )
                ) {
                    Node::atom(Expr::sym(B::NULL), token.span)
                } else {
                    self.wl_expression(6)?
                };
                left = self.chain(B::COMPOUND_EXPRESSION, left, right)?;
                continue;
            }
            if token.kind == K::Power && text(self.src, token) == "**" {
                return self.fail("E020", "Wolfram 子集不支持 ** 运算符");
            }
            if let Some((bp, right_bp, head)) = infix(token.kind) {
                if bp < min {
                    break;
                }
                self.bump();
                let mut right = self.wl_expression(right_bp)?;
                let span = Span {
                    start: left.span.start,
                    end: right.span.end,
                };
                match token.kind {
                    K::PrefixApply => {
                        left = self.apply(left, vec![right], span)?;
                    }
                    K::PostfixApply => {
                        left = self.apply(right, vec![left], span)?;
                    }
                    K::Colon => {
                        if left.expr.as_symbol().is_none() {
                            return self.fail("E022", "Pattern 名称需要符号");
                        }
                        left = self.call(B::PATTERN, vec![left, right], span)?;
                    }
                    K::Minus => {
                        let right_span = right.span;
                        right = self.call(
                            B::TIMES,
                            vec![Node::atom(Expr::int(-1), token.span), right],
                            right_span,
                        )?;
                        left = self.chain(B::PLUS, left, right)?;
                    }
                    K::Slash => {
                        let right_span = right.span;
                        right = self.call(
                            B::POWER,
                            vec![right, Node::atom(Expr::int(-1), token.span)],
                            right_span,
                        )?;
                        left = self.chain(B::TIMES, left, right)?;
                    }
                    _ if matches!(head, B::PLUS | B::TIMES | B::AND | B::OR)
                        || token.kind == K::SameQ =>
                    {
                        left = self.chain(head, left, right)?;
                    }
                    _ => {
                        left = self.call(head, vec![left, right], span)?;
                    }
                }
                continue;
            }
            if starts_atom(token.kind) {
                if 90 < min {
                    break;
                }
                let right = self.wl_expression(91)?;
                left = self.chain(B::TIMES, left, right)?;
                continue;
            }
            break;
        }
        Ok(left)
    }
    fn wl_comparisons(&mut self, left: Node) -> Parsed {
        let start = left.span.start;
        let mut values = vec![left];
        let mut heads = vec![];
        while let Some(token) = self.current() {
            let Some(head) = comparison(token.kind) else {
                break;
            };
            self.bump();
            heads.push(head);
            values.push(self.wl_expression(71)?);
        }
        let end = values.last().map_or(start, |v| v.span.end);
        if heads.iter().all(|h| *h == heads[0]) {
            self.call(heads[0], values, Span { start, end })
        } else {
            let mut args = vec![];
            for (index, value) in values.into_iter().enumerate() {
                if index > 0 {
                    args.push(Node::atom(Expr::sym(heads[index - 1]), value.span));
                }
                args.push(value);
            }
            self.call(B::INEQUALITY, args, Span { start, end })
        }
    }
    fn wl_call(&mut self, left: Node) -> Parsed {
        let start = left.span.start;
        self.expect(K::LBracket)?;
        self.groups += 1;
        let result = (|| {
            let part = self.kind() == Some(K::LBracket);
            let (mut args, end) = if part {
                let (args, _) = self.wl_delimited(K::LBracket, K::RBracket)?;
                let end = self.expect(K::RBracket)?.span.end;
                (args, end)
            } else {
                self.wl_arguments_after_open(K::RBracket)?
            };
            let span = Span { start, end };
            if part {
                args.insert(0, left);
                self.call(B::PART, args, span)
            } else {
                self.apply(left, args, span)
            }
        })();
        self.groups -= 1;
        result
    }
    pub(super) fn wl_delimited(&mut self, open: K, close: K) -> Result<(Vec<Node>, u32), ()> {
        self.expect(open)?;
        self.wl_arguments_after_open(close)
    }
    fn wl_arguments_after_open(&mut self, close: K) -> Result<(Vec<Node>, u32), ()> {
        self.groups += 1;
        let result = (|| {
            let mut args = vec![];
            if self.current().is_some() && self.kind() != Some(close) {
                loop {
                    args.push(self.wl_expression(0)?);
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
            | K::LBrace
            | K::Slot
            | K::Out
            | K::Blank
            | K::BlankSequence
            | K::BlankNullSequence
            | K::Sqrt
    )
}
fn comparison(kind: K) -> Option<Symbol> {
    Some(match kind {
        K::Equal => B::EQUAL,
        K::Unequal => B::UNEQUAL,
        K::Less => B::LESS,
        K::LessEqual => B::LESS_EQUAL,
        K::Greater => B::GREATER,
        K::GreaterEqual => B::GREATER_EQUAL,
        _ => return None,
    })
}
fn infix(kind: K) -> Option<(u8, u8, Symbol)> {
    Some(match kind {
        K::PrefixApply => (140, 140, B::APPLY),
        K::Apply => (130, 130, B::APPLY),
        K::Map => (130, 130, B::MAP),
        K::Power => (110, 110, B::POWER),
        K::Slash => (95, 96, B::TIMES),
        K::Star => (90, 91, B::TIMES),
        K::Plus | K::Minus => (80, 81, B::PLUS),
        K::SameQ => (65, 66, Symbol::intern("SameQ")),
        K::Element => (65, 66, B::ELEMENT),
        K::And => (50, 51, B::AND),
        K::Or => (40, 41, B::OR),
        K::Colon => (30, 31, B::PATTERN),
        K::Condition => (25, 26, B::CONDITION),
        K::Rule => (20, 20, B::RULE),
        K::RuleDelayed => (20, 20, B::RULE_DELAYED),
        K::ReplaceAll => (18, 19, B::REPLACE_ALL),
        K::ReplaceRepeated => (18, 19, Symbol::intern("ReplaceRepeated")),
        K::PostfixApply => (12, 13, B::APPLY),
        K::Assign => (10, 10, B::SET),
        K::SetDelayed => (10, 10, B::SET_DELAYED),
        _ => return None,
    })
}
