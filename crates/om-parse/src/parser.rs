//! Public entry points, source coordinates and shared parser bookkeeping.

use crate::{
    Diagnostic, Dialect, Fix, ParseEnv, ParseOutput, Severity, Span, Stmt, TokenClass,
    lexer::{Token, TokenKind as K, lex},
    names,
};
use om_core::{Expr, Symbol};

/// Resolve a dialect from a leading marker or distinct Wolfram syntax cues.
pub fn detect_dialect(src: &str) -> Dialect {
    if let Some((dialect, _)) = marker(src) {
        return dialect;
    }
    let out = lex(src, Dialect::Modern);
    // Explicit modern composition wins over the ambiguous f[index] heuristic.
    // Otherwise let/lambda programs containing p[1] are misclassified as Wolfram.
    if out.tokens.iter().enumerate().any(|(i, t)| {
        matches!(t.kind, K::Pipe | K::LambdaArrow | K::Interval)
            || t.kind == K::Let
                && out
                    .tokens
                    .get(i + 1)
                    .is_some_and(|next| next.kind == K::Identifier)
    }) {
        return Dialect::Modern;
    }
    let mut evidence = 0u8;
    for (i, token) in out.tokens.iter().enumerate() {
        match token.kind {
            K::Equal => evidence |= 1,
            K::Rule => evidence |= 2,
            K::SetDelayed => evidence |= 4,
            K::ReplaceAll => evidence |= 8,
            K::Comment if text(src, *token).starts_with("(*") => evidence |= 16,
            K::Identifier => {
                let name = text(src, *token);
                if name.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
                    && name.bytes().all(|c| c.is_ascii_alphanumeric())
                    && !names::known_lowercase(name)
                    && out.tokens.get(i + 1).is_some_and(|next| {
                        next.kind == K::LBracket && token.span.end == next.span.start
                    })
                {
                    return Dialect::Wolfram;
                }
            }
            _ => {}
        }
    }
    if evidence.count_ones() >= 2 {
        Dialect::Wolfram
    } else {
        Dialect::Modern
    }
}

fn marker(src: &str) -> Option<(Dialect, usize)> {
    let line_end = src.find(['\r', '\n']).unwrap_or(src.len());
    let dialect = match src[..line_end].trim() {
        "%wl" => Dialect::Wolfram,
        "%modern" => Dialect::Modern,
        _ => return None,
    };
    let mut offset = line_end;
    if src[offset..].starts_with('\r') {
        offset += 1;
    }
    if src[offset..].starts_with('\n') {
        offset += 1;
    }
    Some((dialect, offset))
}

/// Parse a cell using the default mathematical constant and function environment.
pub fn parse(src: &str, dialect: Dialect) -> ParseOutput {
    parse_with(src, dialect, &ParseEnv::default())
}

/// Parse with session function names and the configured constant interpretation.
pub fn parse_with(src: &str, dialect: Dialect, env: &ParseEnv) -> ParseOutput {
    parse_mode(src, dialect, env, false)
}
fn parse_mode(
    src: &str,
    dialect: Dialect,
    env: &ParseEnv,
    serialized_symbols: bool,
) -> ParseOutput {
    if u32::try_from(src.len()).is_err() {
        let dialect = if dialect == Dialect::Auto {
            Dialect::Modern
        } else {
            dialect
        };
        return ParseOutput {
            statements: vec![],
            diagnostics: lex(src, dialect).diagnostics,
            dialect,
            tokens: vec![],
        };
    }
    let dialect = if dialect == Dialect::Auto {
        detect_dialect(src)
    } else {
        dialect
    };
    let offset = marker(src).map_or(0, |(_, offset)| offset);
    let mut out = if serialized_symbols {
        crate::lexer::lex_input_form(&src[offset..])
    } else {
        lex(&src[offset..], dialect)
    };
    for token in &mut out.tokens {
        token.span.start += offset as u32;
        token.span.end += offset as u32;
    }
    for d in &mut out.diagnostics {
        d.span.start += offset as u32;
        d.span.end += offset as u32;
    }
    out.diagnostics
        .extend(crate::diagnostics::brackets(src, &out.tokens, dialect));
    let mut highlights = Vec::new();
    if offset > 0 {
        highlights.push((
            Span {
                start: 0,
                end: offset as u32,
            },
            TokenClass::Comment,
        ));
    }
    for (index, token) in out.tokens.iter().enumerate() {
        if token.kind != K::Identifier {
            highlights.push((token.span, token.class));
            continue;
        }
        let name = text(src, *token);
        if dialect == Dialect::Wolfram {
            let builtin = crate::names::wolfram(name).is_ok_and(|symbol| {
                om_core::builtins::names().contains(&symbol.name()) || names::is_function(symbol)
            });
            highlights.push((
                token.span,
                if builtin {
                    TokenClass::Builtin
                } else {
                    token.class
                },
            ));
            continue;
        }
        let is_call = out
            .tokens
            .get(index + 1)
            .is_some_and(|next| next.kind == K::LParen && next.span.start == token.span.end);
        let is_builtin = names::atom(name, env.constants).name() != name
            || om_core::builtins::names().contains(&name)
            || (name.chars().count() > 1 || is_call)
                && names::modern(name, env.constants).is_some_and(names::is_function);
        let class = if is_builtin {
            TokenClass::Builtin
        } else {
            token.class
        };
        highlights.push((token.span, class));
    }
    let tokens = out
        .tokens
        .into_iter()
        .filter(|t| t.kind != K::Comment)
        .collect();
    let mut parser = Parser {
        src,
        tokens,
        pos: 0,
        diagnostics: out.diagnostics,
        env: env.clone(),
        depth: 0,
        groups: 0,
        bindings: Default::default(),
        dialect,
    };
    let statements = parser.statements();
    parser
        .diagnostics
        .sort_by_key(|d| (d.span.start, d.span.end));
    ParseOutput {
        statements,
        diagnostics: parser.diagnostics,
        dialect,
        tokens: highlights,
    }
}

/// Parse exactly one statement, accepting hints but rejecting all errors.
pub fn parse_expr(src: &str, dialect: Dialect) -> Result<Expr, Vec<Diagnostic>> {
    single_expression(src, parse(src, dialect))
}
/// Read a formatter-produced expression without interpreting modern underscores as WL patterns.
/// This is a serialization reader, not a source dialect; ordinary Wolfram parsing is unchanged.
pub fn parse_input_form(src: &str) -> Result<Expr, Vec<Diagnostic>> {
    single_expression(
        src,
        parse_mode(src, Dialect::Wolfram, &ParseEnv::default(), true),
    )
}
fn single_expression(src: &str, mut out: ParseOutput) -> Result<Expr, Vec<Diagnostic>> {
    if out.statements.len() != 1 {
        out.diagnostics.push(Diagnostic {
            span: Span {
                start: 0,
                end: src.len().min(u32::MAX as usize) as u32,
            },
            severity: Severity::Error,
            code: "E021",
            message: "需要恰好一条语句".into(),
            fix: None,
        });
    }
    if out
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error)
    {
        return Err(out.diagnostics);
    }
    match out.statements.pop() {
        Some(stmt) => Ok(stmt.expr),
        None => Err(out.diagnostics),
    }
}

pub(crate) fn text(src: &str, token: Token) -> &str {
    &src[token.span.start as usize..token.span.end as usize]
}

pub(crate) struct Node {
    pub expr: Expr,
    pub span: Span,
    pub height: usize,
    pub direct_name: bool,
    pub call_syntax: bool,
}
impl Node {
    pub fn atom(expr: Expr, span: Span) -> Self {
        Self {
            expr,
            span,
            height: 1,
            direct_name: false,
            call_syntax: false,
        }
    }
}
pub(crate) type Parsed = Result<Node, ()>;

pub(crate) struct Parser<'a> {
    pub src: &'a str,
    pub tokens: Vec<Token>,
    pub pos: usize,
    pub diagnostics: Vec<Diagnostic>,
    pub env: ParseEnv,
    pub depth: usize,
    pub groups: usize,
    pub bindings: std::collections::BTreeSet<Symbol>,
    pub dialect: Dialect,
}
impl Parser<'_> {
    pub fn current(&mut self) -> Option<Token> {
        if self.groups > 0 {
            while self
                .tokens
                .get(self.pos)
                .is_some_and(|t| t.kind == K::Newline)
            {
                self.pos += 1;
            }
        }
        self.tokens.get(self.pos).copied()
    }
    pub fn kind(&mut self) -> Option<K> {
        self.current().map(|t| t.kind)
    }
    pub fn bump(&mut self) -> Option<Token> {
        let token = self.current()?;
        self.pos += 1;
        Some(token)
    }
    pub fn at_span(&mut self) -> Span {
        self.current().map_or(
            Span {
                start: self.src.len() as u32,
                end: self.src.len() as u32,
            },
            |t| t.span,
        )
    }
    pub fn report(
        &mut self,
        span: Span,
        severity: Severity,
        code: &'static str,
        message: &str,
        fix: Option<Fix>,
    ) {
        if severity == Severity::Error
            && matches!(code, "E020" | "E022")
            && self
                .diagnostics
                .iter()
                .any(|d| d.code == "E024" && d.span == span)
        {
            return;
        }
        self.diagnostics.push(Diagnostic {
            span,
            severity,
            code,
            message: message.into(),
            fix,
        });
    }
    pub fn fail<T>(&mut self, code: &'static str, message: &str) -> Result<T, ()> {
        let span = self.at_span();
        self.report(span, Severity::Error, code, message, None);
        Err(())
    }
    pub fn expect(&mut self, kind: K) -> Result<Token, ()> {
        if self.kind() == Some(kind) {
            self.bump().ok_or(())
        } else {
            if matches!(kind, K::RParen | K::RBracket | K::RBrace | K::Bar)
                && self.current().is_none()
                && self.diagnostics.iter().any(|d| d.code == "E023")
            {
                return Err(());
            }
            self.fail("E022", "缺少预期的分隔符")
        }
    }
    pub fn call(&mut self, head: Symbol, args: Vec<Node>, span: Span) -> Parsed {
        let height = args.iter().map(|n| n.height).max().unwrap_or(0) + 1;
        if height > 128 {
            return self.fail("E013", "表达式嵌套超出解析限制");
        }
        Ok(Node {
            expr: Expr::call(head, args.into_iter().map(|n| n.expr)),
            span,
            height,
            direct_name: false,
            call_syntax: false,
        })
    }
    pub fn chain(&mut self, head: Symbol, left: Node, right: Node) -> Parsed {
        let span = Span {
            start: left.span.start,
            end: right.span.end,
        };
        let left_height = left.height - usize::from(left.expr.is_head(head));
        let right_height = right.height - usize::from(right.expr.is_head(head));
        let height = left_height.max(right_height) + 1;
        if height > 128 {
            return self.fail("E013", "表达式嵌套超出解析限制");
        }
        let mut args = if left.expr.is_head(head) {
            left.expr.args().to_vec()
        } else {
            vec![left.expr]
        };
        if right.expr.is_head(head) {
            args.extend_from_slice(right.expr.args());
        } else {
            args.push(right.expr);
        }
        Ok(Node {
            expr: Expr::call(head, args),
            span,
            height,
            direct_name: false,
            call_syntax: false,
        })
    }
    fn statements(&mut self) -> Vec<Stmt> {
        let mut statements = vec![];
        while self.current().is_some() {
            if matches!(self.kind(), Some(K::Newline | K::Semicolon)) {
                self.bump();
                continue;
            }
            let start = self.at_span().start;
            let result = if self.dialect == Dialect::Wolfram {
                self.wl_expression(0)
            } else if self.kind() == Some(K::Let) {
                self.declaration()
            } else {
                self.expression(0, false)
            };
            let expr = match result {
                Ok(node) => node.expr,
                Err(()) => {
                    self.recover();
                    Expr::symbol("$Failed")
                }
            };
            if !matches!(self.kind(), None | Some(K::Newline | K::Semicolon)) {
                let _ = self.fail::<()>("E020", "语句后有无法识别的输入");
                self.recover();
            }
            let mut end = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .map_or(start, |t| t.span.end);
            let trailing_null = self.dialect == Dialect::Wolfram
                && expr.is_head(om_core::BUILTIN::COMPOUND_EXPRESSION)
                && expr
                    .args()
                    .last()
                    .is_some_and(|e| e.as_symbol() == Some(om_core::BUILTIN::NULL));
            let suppress_output = self.kind() == Some(K::Semicolon) || trailing_null;
            if self.kind() == Some(K::Semicolon)
                && let Some(token) = self.bump()
            {
                end = token.span.end;
            }
            statements.push(Stmt {
                expr,
                span: Span { start, end },
                suppress_output,
            });
        }
        statements
    }

    pub(crate) fn apply(&mut self, head: Node, args: Vec<Node>, span: Span) -> Parsed {
        let height = head
            .height
            .max(args.iter().map(|n| n.height).max().unwrap_or(1))
            + 1;
        if height > 128 {
            return self.fail("E013", "表达式嵌套超出解析限制");
        }
        Ok(Node {
            expr: Expr::normal(head.expr, args.into_iter().map(|n| n.expr)),
            span,
            height,
            direct_name: false,
            call_syntax: false,
        })
    }
    fn recover(&mut self) {
        self.groups = 0;
        while !matches!(self.kind(), None | Some(K::Newline | K::Semicolon)) {
            self.bump();
        }
    }
}
