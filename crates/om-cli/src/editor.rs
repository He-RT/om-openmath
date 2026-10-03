//! Reedline's deterministic editing features delegate to shared kernel services.
use crate::host::Host;
use nu_ansi_term::{Color, Style};
use om_kernel::{Session, protocol::*};
use reedline::{
    Completer, Highlighter, Prompt, PromptEditMode, PromptHistorySearch, StyledText,
    ValidationResult, Validator,
};
use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
};
#[derive(Clone)]
pub struct Services {
    pub session: Arc<Mutex<Session>>,
    pub dialect: Dialect,
}
impl Completer for Services {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<reedline::Suggestion> {
        let Ok(cursor) = u32::try_from(pos) else {
            return vec![];
        };
        let Ok(mut session) = self.session.lock() else {
            return vec![];
        };
        let Response::Completions { items, from, to } = session
            .handle(Request::Complete {
                source: line.into(),
                cursor,
                dialect: self.dialect,
            })
            .0
        else {
            return vec![];
        };
        items
            .into_iter()
            .map(|i| reedline::Suggestion {
                value: snippet(&i.insert_text),
                description: i.detail,
                span: reedline::Span::new(from as usize, to as usize),
                append_whitespace: false,
                ..Default::default()
            })
            .collect()
    }
}
impl Highlighter for Services {
    fn highlight(&self, line: &str, cursor: usize) -> StyledText {
        let mut styled = StyledText::new();
        let tokens = self
            .session
            .lock()
            .ok()
            .and_then(|mut s| {
                match s
                    .handle(Request::Preview {
                        source: line.into(),
                        dialect: self.dialect,
                        cursor: u32::try_from(cursor).ok(),
                    })
                    .0
                {
                    Response::Preview(p) => Some(p.tokens),
                    _ => None,
                }
            })
            .unwrap_or_default();
        let mut position = 0;
        for (span, kind) in tokens {
            let (start, end) = (span.start as usize, span.end as usize);
            if start < position
                || !line.is_char_boundary(start)
                || !line.is_char_boundary(end)
                || end > line.len()
            {
                continue;
            }
            if start > position {
                styled.push((Style::new(), line[position..start].into()));
            }
            let color = match kind {
                TokenClass::Number => Color::Yellow,
                TokenClass::Builtin | TokenClass::Keyword => Color::Cyan,
                TokenClass::Comment => Color::DarkGray,
                TokenClass::String => Color::Purple,
                TokenClass::Error => Color::Red,
                _ => Color::White,
            };
            styled.push((color.normal(), line[start..end].into()));
            position = end;
        }
        if position < line.len() {
            styled.push((Style::new(), line[position..].into()));
        }
        styled
    }
}
impl Validator for Services {
    fn validate(&self, line: &str) -> ValidationResult {
        if line.starts_with(':') || line.starts_with('?') {
            return ValidationResult::Complete;
        }
        let incomplete = self.session.lock().ok().is_some_and(|mut s| {
            match s
                .handle(Request::Preview {
                    source: line.into(),
                    dialect: self.dialect,
                    cursor: None,
                })
                .0
            {
                Response::Preview(p) => p
                    .diagnostics
                    .iter()
                    .any(|d| ["E002", "E003", "E023"].contains(&d.code.as_str())),
                _ => false,
            }
        });
        if incomplete {
            ValidationResult::Incomplete
        } else {
            ValidationResult::Complete
        }
    }
}
pub struct TerminalPrompt {
    pub wolfram: bool,
    pub next: u32,
}
impl TerminalPrompt {
    pub fn from(host: &Host) -> Self {
        Self {
            wolfram: host.dialect == Dialect::Wolfram,
            next: host.count + 1,
        }
    }
}
impl Prompt for TerminalPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
    fn render_prompt_indicator(&self, _: PromptEditMode) -> Cow<'_, str> {
        Cow::Owned(if self.wolfram {
            format!("In[{}]:= ", self.next)
        } else {
            "❯ ".into()
        })
    }
    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("… ")
    }
    fn render_prompt_history_search_indicator(&self, _: PromptHistorySearch) -> Cow<'_, str> {
        Cow::Borrowed("history ❯ ")
    }
}
fn snippet(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' && chars.peek() == Some(&'{') {
            chars.next();
            let mut inner = String::new();
            for c in chars.by_ref() {
                if c == '}' {
                    break;
                }
                inner.push(c);
            }
            if let Some((_, text)) = inner.split_once(':') {
                out.push_str(text);
            }
        } else {
            out.push(c);
        }
    }
    out
}
