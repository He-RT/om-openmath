//! Lossless lexical context shared by completion and hover.
mod axes;
mod completion;
pub(crate) use axes::free as free_axes;
pub(crate) use completion::{items, signature};
use om_parse::{
    Dialect,
    lexer::{Token, TokenKind as K},
};
#[derive(Clone, Copy)]
pub(crate) struct Word {
    pub from: u32,
    pub to: u32,
    pub call: bool,
}
pub(crate) fn word(tokens: &[Token], source: &str, cursor: u32, dialect: Dialect) -> Option<Word> {
    let index = tokens
        .iter()
        .position(|t| {
            matches!(
                t.kind,
                K::Identifier | K::Let | K::Where | K::And | K::Or | K::Not
            ) && t.span.start <= cursor
                && cursor < t.span.end
        })
        .or_else(|| {
            tokens.iter().position(|t| {
                matches!(
                    t.kind,
                    K::Identifier | K::Let | K::Where | K::And | K::Or | K::Not
                ) && t.span.start < cursor
                    && t.span.end == cursor
            })
        })?;
    let t = tokens[index];
    let next = tokens[index + 1..]
        .iter()
        .find(|t| !matches!(t.kind, K::Comment | K::Newline));
    let call = next.is_some_and(|n| {
        if dialect == Dialect::Modern {
            n.kind == K::LParen && t.span.end == n.span.start
        } else {
            n.kind == K::LBracket && !source[n.span.end as usize..].starts_with('[')
        }
    });
    Some(Word {
        from: t.span.start,
        to: t.span.end,
        call,
    })
}
pub(crate) fn blocked(tokens: &[Token], cursor: u32) -> bool {
    tokens.iter().any(|t| {
        t.span.start <= cursor
            && cursor <= t.span.end
            && matches!(
                t.kind,
                K::String
                    | K::Comment
                    | K::Number
                    | K::Blank
                    | K::BlankSequence
                    | K::BlankNullSequence
                    | K::Slot
                    | K::Error
            )
    })
}
pub(crate) fn call_context(
    tokens: &[Token],
    source: &str,
    cursor: u32,
    dialect: Dialect,
    constants: om_parse::ConstantMode,
) -> Option<om_core::Symbol> {
    let mut stack: Vec<(K, Option<om_core::Symbol>)> = vec![];
    let mut previous: Option<Token> = None;
    for token in tokens.iter().copied().take_while(|t| t.span.end <= cursor) {
        match token.kind {
            K::Comment | K::Newline => continue,
            K::LParen | K::LBracket | K::LBrace => {
                let function = previous
                    .filter(|t| t.kind == K::Identifier)
                    .filter(|t| {
                        if dialect == Dialect::Modern {
                            token.kind == K::LParen && t.span.end == token.span.start
                        } else {
                            token.kind == K::LBracket
                                && !source[token.span.end as usize..].starts_with('[')
                        }
                    })
                    .and_then(|t| {
                        om_parse::identifier_symbol(
                            &source[t.span.start as usize..t.span.end as usize],
                            dialect,
                            constants,
                            true,
                        )
                    });
                stack.push((token.kind, function));
            }
            K::RParen | K::RBracket | K::RBrace => {
                let opening = match token.kind {
                    K::RParen => K::LParen,
                    K::RBracket => K::LBracket,
                    _ => K::LBrace,
                };
                if stack.last().is_some_and(|(k, _)| *k == opening) {
                    stack.pop();
                } else {
                    stack.clear();
                }
            }
            _ => {}
        }
        previous = Some(token);
    }
    stack.last().and_then(|(_, s)| *s)
}

pub(crate) fn tokens(source: &str, dialect: Dialect) -> Vec<Token> {
    let end = source.find(['\r', '\n']).unwrap_or(source.len());
    let mut offset = if matches!(source[..end].trim(), "%wl" | "%modern") {
        end
    } else {
        0
    };
    if offset == 0 {
        return om_parse::lexer::lex(source, dialect).tokens;
    }
    if source[offset..].starts_with('\r') {
        offset += 1;
    }
    if source[offset..].starts_with('\n') {
        offset += 1;
    }
    let mut tokens = om_parse::lexer::lex(&source[offset..], dialect).tokens;
    for token in &mut tokens {
        token.span.start += offset as u32;
        token.span.end += offset as u32;
    }
    tokens.insert(
        0,
        Token {
            kind: K::Comment,
            span: om_parse::Span {
                start: 0,
                end: offset as u32,
            },
            class: om_parse::TokenClass::Comment,
        },
    );
    tokens
}
