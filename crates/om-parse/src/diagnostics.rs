//! Delimiter diagnostics derived only from actual lexical bracket tokens.

use crate::{
    Diagnostic, Dialect, Fix, Severity, Span,
    lexer::{Token, TokenKind as K},
};

pub(crate) fn brackets(src: &str, tokens: &[Token], dialect: Dialect) -> Vec<Diagnostic> {
    let mut stack: Vec<Token> = vec![];
    let mut diagnostics = vec![];
    let mut operand = true;
    let mut bars = 0;
    for token in tokens {
        let closing_bar =
            token.kind == K::Bar && dialect == Dialect::Modern && !operand && bars > 0;
        match token.kind {
            K::LParen | K::LBracket | K::LBrace => {
                stack.push(*token);
                operand = true;
            }
            K::Bar if dialect == Dialect::Modern && !closing_bar => {
                stack.push(*token);
                bars += 1;
                operand = true;
            }
            K::RParen | K::RBracket | K::RBrace | K::Bar if token.kind != K::Bar || closing_bar => {
                let expected = stack.pop().map(|open| {
                    if open.kind == K::Bar {
                        bars -= 1;
                    }
                    close(open.kind)
                });
                operand = false;
                let actual = close(token.kind);
                if expected == Some(actual) {
                    continue;
                }
                let replacement = expected.map_or_else(String::new, |c| c.to_string());
                let message = expected.map_or_else(
                    || "多余的结束括号".into(),
                    |c| format!("括号类型不匹配：需要 {c}"),
                );
                let label = if expected.is_some() {
                    "更正结束括号"
                } else {
                    "删除多余的结束括号"
                };
                diagnostics.push(Diagnostic {
                    span: token.span,
                    severity: Severity::Error,
                    code: "E024",
                    message,
                    fix: Some(Fix {
                        span: token.span,
                        replacement,
                        label: label.into(),
                    }),
                });
            }
            K::Comment | K::Newline | K::Error => {}
            K::Number
            | K::Identifier
            | K::String
            | K::Slot
            | K::Out
            | K::Superscript
            | K::Prime => operand = false,
            K::Bang => {}
            _ => operand = true,
        }
    }
    if let Some(inner) = stack.last() {
        let closers: String = stack.iter().rev().map(|open| close(open.kind)).collect();
        let line_comment = tokens.last().is_some_and(|t| {
            t.kind == K::Comment
                && t.span.end as usize == src.len()
                && src[t.span.start as usize..].starts_with('#')
        });
        let replacement = if line_comment {
            format!("\n{closers}")
        } else {
            closers.clone()
        };
        diagnostics.push(Diagnostic {
            span: inner.span,
            severity: Severity::Error,
            code: "E023",
            message: format!("未闭合的括号：需要 {closers}"),
            fix: Some(Fix {
                span: Span {
                    start: src.len() as u32,
                    end: src.len() as u32,
                },
                replacement,
                label: "补全结束括号".into(),
            }),
        });
    }
    diagnostics
}

fn close(kind: K) -> char {
    match kind {
        K::Bar => '|',
        K::LParen | K::RParen => ')',
        K::LBracket | K::RBracket => ']',
        _ => '}',
    }
}
