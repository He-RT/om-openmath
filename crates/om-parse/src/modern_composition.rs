//! Source-local pipeline, lambda and record lowering; no evaluator or host actions.
use crate::{
    Span,
    lexer::TokenKind as K,
    literals, names,
    parser::{Node, Parsed, Parser, text},
};
use om_core::{BUILTIN as B, Expr, Symbol};
impl Parser<'_> {
    pub(super) fn pipeline(&mut self, input: Node) -> Parsed {
        let start = input.span.start;
        self.operand_newlines()?;
        let token = self.expect(K::Identifier)?;
        let name = text(self.src, token);
        let raw = Symbol::intern(name);
        let symbol = if self.env.known_functions.contains(&raw) || self.bindings.contains(&raw) {
            raw
        } else {
            names::modern(name, self.env.constants).unwrap_or(raw)
        };
        if om_core::catalog::by_runtime(symbol.name()).is_none_or(|f| f.pipe_arg == 0) {
            return self.fail("E026", "管道目标没有已知主要输入位置");
        }
        if self
            .current()
            .is_none_or(|t| t.kind != K::LParen || t.span.start != token.span.end)
        {
            return self.fail("E026", "管道右侧需要相邻括号的函数调用");
        }
        let result =
            self.function_call_input(Node::atom(Expr::sym(symbol), token.span), Some(input))?;
        Ok(Node {
            span: Span {
                start,
                end: result.span.end,
            },
            ..result
        })
    }
    pub(super) fn lambda_ahead(&self) -> bool {
        let mut tokens = self.tokens[self.pos..]
            .iter()
            .filter(|t| !matches!(t.kind, K::Comment | K::Newline));
        if tokens.next().is_none_or(|t| t.kind != K::LParen) {
            return false;
        }
        let mut depth = 1;
        for token in tokens.by_ref() {
            match token.kind {
                K::LParen => depth += 1,
                K::RParen => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                return tokens.next().is_some_and(|t| t.kind == K::LambdaArrow);
            }
        }
        false
    }
    pub(super) fn lambda(&mut self, source: Span) -> Parsed {
        self.expect(K::LParen)?;
        let previous_groups = self.groups;
        self.groups += 1;
        let previous = self.bindings.clone();
        let result = (|| {
            let mut params = vec![];
            let mut seen = std::collections::BTreeSet::new();
            if self.kind() != Some(K::RParen) {
                loop {
                    let token = self.expect(K::Identifier)?;
                    let symbol = Symbol::intern(text(self.src, token));
                    if !seen.insert(symbol) {
                        return self.fail("E026", "匿名函数形参重复");
                    }
                    self.bindings.insert(symbol);
                    params.push(Node::atom(Expr::sym(symbol), token.span));
                    if self.kind() != Some(K::Comma) {
                        break;
                    }
                    self.bump();
                }
            }
            let end = self.expect(K::RParen)?.span.end;
            let parameters = self.call(
                B::LIST,
                params,
                Span {
                    start: source.start,
                    end,
                },
            )?;
            self.groups = previous_groups;
            self.expect(K::LambdaArrow)?;
            let body = self.expression(0, false)?;
            let span = Span {
                start: source.start,
                end: body.span.end,
            };
            self.call(B::FUNCTION, vec![parameters, body], span)
        })();
        self.bindings = previous;
        self.groups = previous_groups;
        result.map(|mut node| {
            node.call_syntax = true;
            node
        })
    }
    pub(super) fn record_ahead(&self) -> bool {
        let mut tokens = self.tokens[self.pos..]
            .iter()
            .filter(|t| !matches!(t.kind, K::Comment | K::Newline));
        tokens
            .next()
            .is_some_and(|t| matches!(t.kind, K::Identifier | K::String))
            && tokens.next().is_some_and(|t| t.kind == K::Colon)
    }
    pub(super) fn record_literal(&mut self, source: Span) -> Parsed {
        self.groups += 1;
        let result = (|| {
            let mut entries = vec![];
            let mut keys = std::collections::BTreeSet::new();
            loop {
                let key = self.bump().ok_or(())?;
                let key_expr = match key.kind {
                    K::Identifier => Expr::string(text(self.src, key)),
                    K::String => literals::string(text(self.src, key)).map_err(|_| ())?,
                    _ => return self.fail("E026", "记录键需要标识符或字符串"),
                };
                if matches!(key_expr.kind(), om_core::ExprKind::String(s) if !keys.insert(s.to_string()))
                {
                    return self.fail("E026", "记录键重复");
                }
                self.expect(K::Colon)?;
                let value = self.expression(0, false)?;
                let span = Span {
                    start: key.span.start,
                    end: value.span.end,
                };
                entries.push(self.call(
                    B::RULE,
                    vec![Node::atom(key_expr, key.span), value],
                    span,
                )?);
                if self.kind() != Some(K::Comma) {
                    break;
                }
                self.bump();
            }
            let end = self.expect(K::RBrace)?.span.end;
            self.call(
                B::RECORD,
                entries,
                Span {
                    start: source.start,
                    end,
                },
            )
        })();
        self.groups -= 1;
        result
    }
}
