//! Wolfram atoms, patterns and grouped input.

use crate::{
    Severity, Span,
    lexer::TokenKind as K,
    literals,
    parser::{Node, Parsed, Parser, text},
};
use om_core::{BUILTIN as B, Expr};

impl Parser<'_> {
    pub(super) fn wl_atom(&mut self) -> Parsed {
        let Some(token) = self.bump() else {
            return self.fail("E022", "需要表达式");
        };
        match token.kind {
            K::Number => {
                let expr = literals::wolfram_number(text(self.src, token)).map_err(|message| {
                    self.report(token.span, Severity::Error, "E011", message, None)
                })?;
                Ok(Node::atom(expr, token.span))
            }
            K::String => {
                let expr = literals::wolfram_string(text(self.src, token)).map_err(|message| {
                    self.report(token.span, Severity::Error, "E006", message, None)
                })?;
                Ok(Node::atom(expr, token.span))
            }
            K::Identifier => {
                let symbol = crate::names::wolfram(text(self.src, token)).map_err(|message| {
                    self.report(token.span, Severity::Error, "E005", message, None)
                })?;
                let mut node = Node::atom(Expr::sym(symbol), token.span);
                node.direct_name = true;
                Ok(node)
            }
            K::Blank | K::BlankSequence | K::BlankNullSequence => {
                self.pos -= 1;
                self.wl_blank()
            }
            K::Slot => {
                let suffix = &text(self.src, token)[1..];
                let value = if suffix.is_empty() {
                    Expr::int(1)
                } else {
                    literals::number(suffix).map_err(|message| {
                        self.report(token.span, Severity::Error, "E011", message, None)
                    })?
                };
                self.call(B::SLOT, vec![Node::atom(value, token.span)], token.span)
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
                    let value = literals::number(text(self.src, next)).map_err(|message| {
                        self.report(next.span, Severity::Error, "E011", message, None)
                    })?;
                    args.push(Node::atom(value, next.span));
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
            K::Minus | K::Plus | K::Bang | K::Not | K::Sqrt => {
                let arg = self.wl_expression(if matches!(token.kind, K::Bang | K::Not) {
                    60
                } else {
                    100
                })?;
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
            K::LParen => {
                self.groups += 1;
                let result = (|| {
                    let mut node = self.wl_expression(0)?;
                    let end = self.expect(K::RParen)?.span.end;
                    node.span = Span {
                        start: token.span.start,
                        end,
                    };
                    node.direct_name = false;
                    Ok(node)
                })();
                self.groups -= 1;
                result
            }
            K::LBrace => {
                self.pos -= 1;
                let (args, end) = self.wl_delimited(K::LBrace, K::RBrace)?;
                self.call(
                    B::LIST,
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
    pub(super) fn wl_blank(&mut self) -> Parsed {
        let token = self.bump().ok_or(())?;
        let head = match token.kind {
            K::Blank => B::BLANK,
            K::BlankSequence => B::BLANK_SEQUENCE,
            _ => B::BLANK_NULL_SEQUENCE,
        };
        let mut args = vec![];
        let mut end = token.span.end;
        if let Some(next) = self.current()
            && next.kind == K::Identifier
            && next.span.start == end
        {
            self.bump();
            end = next.span.end;
            let symbol = crate::names::wolfram(text(self.src, next)).map_err(|message| {
                self.report(next.span, Severity::Error, "E005", message, None)
            })?;
            args.push(Node::atom(Expr::sym(symbol), next.span));
        }
        self.call(
            head,
            args,
            Span {
                start: token.span.start,
                end,
            },
        )
    }
}
