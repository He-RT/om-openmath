//! Editor requests read actual source/docs/storage without evaluation or cancellation changes.
use super::Session;
use crate::{
    config::{Constants, Language},
    protocol::*,
};
use om_core::{BUILTIN as B, Expr, Symbol};
use om_eval::Evaluator;
impl Session {
    fn cursor(&self, source: &str, cursor: u32) -> bool {
        usize::try_from(cursor)
            .ok()
            .is_some_and(|p| p <= source.len() && source.is_char_boundary(p))
    }
    fn cursor_error(&self) -> Response {
        self.error(
            "err.cursor",
            "光标必须位于源码的 UTF-8 字节边界内",
            "Cursor must be a UTF-8 byte boundary within the source",
        )
    }
    fn constants(&self) -> om_parse::ConstantMode {
        match self.config.general.constants {
            Constants::Math => om_parse::ConstantMode::Math,
            Constants::Strict => om_parse::ConstantMode::Strict,
        }
    }
    pub(super) fn preview(
        &self,
        source: String,
        dialect: Dialect,
        cursor: Option<u32>,
    ) -> Response {
        if let Some(p) = cursor
            && !self.cursor(&source, p)
        {
            return self.cursor_error();
        }
        let parsed = self.parse_source(&source, dialect);
        let mut latex = None;
        let mut actions = vec![];
        if !parsed
            .diagnostics
            .iter()
            .any(|d| d.severity == om_parse::Severity::Error)
        {
            let selected = cursor
                .and_then(|p| {
                    parsed
                        .statements
                        .iter()
                        .find(|s| s.span.start <= p && p <= s.span.end)
                        .or_else(|| parsed.statements.iter().rev().find(|s| s.span.start <= p))
                        .or_else(|| parsed.statements.first())
                })
                .or_else(|| parsed.statements.last());
            if let Some(statement) = selected {
                latex = Some(om_format::latex(&om_core::canonicalize(&statement.expr)));
                if statement.expr.is_head(B::EQUAL) {
                    let mut symbols: Vec<_> = crate::editor::free_axes(&statement.expr)
                        .into_iter()
                        .collect();
                    symbols.sort_by_key(|s| s.name());
                    let raw = source[statement.span.start as usize..statement.span.end as usize]
                        .trim_end()
                        .trim_end_matches(';');
                    for symbol in symbols.into_iter().take(3) {
                        let action = if parsed.dialect == om_parse::Dialect::Modern {
                            format!("solve({raw}\n, {})", symbol.name())
                        } else {
                            format!("Solve[{raw}\n, {}]", symbol.name())
                        };
                        actions.push(CellAction {
                            label_key: "action.solve".into(),
                            source: action,
                        });
                    }
                }
            }
        }
        Response::Preview(PreviewResult {
            latex,
            diagnostics: parsed.diagnostics.iter().map(Diagnostic::from).collect(),
            tokens: parsed
                .tokens
                .iter()
                .map(|(span, class)| ((*span).into(), (*class).into()))
                .collect(),
            dialect: parsed.dialect.into(),
            actions,
        })
    }
    pub(super) fn complete(&self, source: String, dialect: Dialect, cursor: u32) -> Response {
        if !self.cursor(&source, cursor) {
            return self.cursor_error();
        }
        let dialect = self.effective_dialect(&source, dialect);
        let tokens = crate::editor::tokens(&source, dialect);
        let word = crate::editor::word(&tokens, &source, cursor, dialect);
        let (from, to) = word.map_or((cursor, cursor), |w| (w.from, w.to));
        let prefix = &source[from as usize..cursor as usize];
        let items = if crate::editor::blocked(&tokens, cursor) {
            vec![]
        } else {
            crate::editor::items(
                prefix,
                dialect,
                &self.eval.defs.defined_symbols(),
                crate::editor::call_context(&tokens, &source, cursor, dialect, self.constants()),
            )
        };
        Response::Completions { items, from, to }
    }
    pub(super) fn hover(&self, source: String, dialect: Dialect, cursor: u32) -> Response {
        if !self.cursor(&source, cursor) {
            return self.cursor_error();
        }
        let dialect = self.effective_dialect(&source, dialect);
        let tokens = crate::editor::tokens(&source, dialect);
        if crate::editor::blocked(&tokens, cursor) {
            return Response::Hover { info: None };
        }
        let info = crate::editor::word(&tokens, &source, cursor, dialect).and_then(|word| {
            let name = &source[word.from as usize..word.to as usize];
            let bare = om_parse::identifier_symbol(name, dialect, self.constants(), false)?;
            let stored = self.eval.defs.own_value(bare).is_some()
                || !self.eval.defs.down_values(bare).is_empty();
            let symbol = if !word.call && stored {
                bare
            } else {
                om_parse::identifier_symbol(name, dialect, self.constants(), true)?
            };
            if let Some(doc) = Evaluator::doc(symbol) {
                Some(HoverInfo {
                    signature: Some(crate::editor::signature(doc, dialect)),
                    summary: self.localized(doc.summary_zh, doc.summary_en),
                    examples: doc.examples.iter().map(|s| (*s).into()).collect(),
                    value: None,
                    cell_id: None,
                })
            } else {
                self.user_hover(symbol, dialect)
            }
        });
        Response::Hover { info }
    }
    pub(super) fn variables(&self) -> Response {
        let mut symbols = self
            .eval
            .defs
            .defined_symbols()
            .into_iter()
            .collect::<Vec<_>>();
        symbols.sort_by_key(|s| s.name());
        Response::Variables {
            items: symbols
                .into_iter()
                .filter_map(|symbol| {
                    self.user_hover(symbol, om_parse::Dialect::Wolfram)
                        .map(|info| (symbol.name().into(), info))
                })
                .collect(),
        }
    }
    fn user_hover(&self, symbol: Symbol, dialect: om_parse::Dialect) -> Option<HoverInfo> {
        let own = self.eval.defs.own_value(symbol);
        let down = self.eval.defs.down_values(symbol);
        if own.is_none() && down.is_empty() {
            return None;
        }
        let value = if let Some(own) = own {
            om_format::input_form(own)
        } else {
            down.iter()
                .map(|r| {
                    om_format::input_form(&Expr::call(
                        if r.delayed { B::SET_DELAYED } else { B::SET },
                        [r.lhs.clone(), r.rhs.clone()],
                    ))
                })
                .collect::<Vec<_>>()
                .join("; ")
        };
        let mut chars = value.chars();
        let mut short: String = chars.by_ref().take(200).collect();
        if chars.next().is_some() {
            short = short.chars().take(199).collect();
            short.push('…');
        }
        let signature = if own.is_none() {
            down.first().map(|r| {
                if dialect == om_parse::Dialect::Modern {
                    om_format::modern_form(&r.lhs)
                } else {
                    om_format::input_form(&r.lhs)
                }
            })
        } else {
            None
        };
        Some(HoverInfo {
            signature,
            summary: if self.effective_language() == Language::En {
                "Stored definition (unevaluated)".into()
            } else {
                "当前存储的定义（未求值）".into()
            },
            examples: vec![],
            value: Some(short),
            cell_id: self.owners.get(&symbol).cloned(),
        })
    }
}
