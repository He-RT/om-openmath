//! Non-evaluating feature inputs from actual session source, diagnostics, docs and records.
use super::Session;
use crate::{
    assistant::{LIMIT, SuggestionParser, context_comment, filter_completion},
    config::Language,
    protocol::*,
};
use om_llm::{Feature, JobInput, prompts};
use serde_json::json;
use std::sync::Arc;

impl Session {
    /// Prepare actual feature messages without evaluation, IO or state/history changes.
    pub fn prepare_llm_input(&self, request: &Request) -> Result<(Feature, JobInput), String> {
        let lang = if self.config.general.language == Language::En {
            "English"
        } else {
            "Simplified Chinese"
        };
        let structured = |messages| JobInput::Structured {
            messages,
            validator: Arc::new(SuggestionParser),
        };
        let result = match request {
            Request::LlmTranslate { text, .. } => {
                let functions = om_eval::Evaluator::all_docs()
                    .map(|doc| doc.name)
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut defined = self
                    .eval
                    .defs
                    .defined_symbols()
                    .into_iter()
                    .map(|s| s.name())
                    .collect::<Vec<_>>();
                defined.sort_unstable();
                (
                    Feature::Translate,
                    structured(prompts::translate_messages(
                        lang,
                        &functions,
                        &defined.join(", "),
                        text,
                    )),
                )
            }
            Request::LlmExplain {
                cell_id, step_id, ..
            } => {
                let cell = self
                    .notebook
                    .cells
                    .iter()
                    .find(|c| &c.id == cell_id)
                    .ok_or("Unknown explanation cell")?;
                if cell.status != CellStatus::Done {
                    return Err("Explain requires a current successful computation".into());
                }
                let record = cell
                    .records
                    .iter()
                    .rev()
                    .find(|r| r.steps.is_some())
                    .ok_or("No recorded computation steps for this cell")?;
                let mut steps =
                    crate::output::render_steps(record.steps.as_ref().ok_or("No recorded steps")?);
                if let Some(id) = step_id {
                    let mut work: Vec<_> = steps.root.iter().collect();
                    let mut selected = None;
                    while let Some(step) = work.pop() {
                        if &step.id == id {
                            selected = Some(step.clone());
                            break;
                        }
                        work.extend(step.children.iter());
                    }
                    steps.root = vec![selected.ok_or("Unknown recorded step ID")?];
                }
                if steps.root.is_empty() {
                    return Err("No recorded computation steps for this cell".into());
                }
                (
                    Feature::Explain,
                    JobInput::Text {
                        messages: prompts::explain_messages(
                            lang,
                            &om_format::input_form(&record.input),
                            &om_format::input_form(&record.value),
                            &json!(steps),
                        ),
                    },
                )
            }
            Request::LlmFixError { cell_id, .. } => {
                let cell = self
                    .notebook
                    .cells
                    .iter()
                    .find(|c| &c.id == cell_id)
                    .ok_or("Unknown repair cell")?;
                let parsed = self.parse_source(&cell.source, cell.dialect);
                let dialect = if parsed.dialect == om_parse::Dialect::Wolfram {
                    "wolfram"
                } else {
                    "modern"
                };
                let diagnostics = json!(parsed.diagnostics);
                let messages = cell
                    .output
                    .as_ref()
                    .map(|o| json!(o.messages))
                    .unwrap_or_else(|| json!([]));
                (
                    Feature::Fix,
                    structured(prompts::fix_messages(
                        lang,
                        &cell.source,
                        dialect,
                        &diagnostics,
                        &messages,
                    )),
                )
            }
            Request::LlmChat { messages, .. } => (
                Feature::Chat,
                JobInput::Chat {
                    messages: prompts::chat_messages(lang, messages),
                    tools: prompts::chat_tools(),
                },
            ),
            Request::LlmComplete {
                prefix,
                suffix,
                dialect,
                ..
            } => {
                let source = format!("{prefix}{suffix}");
                let dialect = self.effective_dialect(&source, *dialect);
                let mut context = String::new();
                if self.config.llm.send_context {
                    let to = self
                        .notebook
                        .cells
                        .iter()
                        .rposition(|c| c.source == source)
                        .unwrap_or(self.notebook.cells.len());
                    let mut previous = self.notebook.cells[..to]
                        .iter()
                        .rev()
                        .filter(|c| c.kind == CellKind::Math)
                        .take(3)
                        .collect::<Vec<_>>();
                    previous.reverse();
                    for cell in previous {
                        if context
                            .len()
                            .saturating_add(cell.source.len())
                            .saturating_add(prefix.len())
                            .saturating_add(suffix.len())
                            > LIMIT
                        {
                            return Err("Completion context exceeds source limit".into());
                        }
                        context.push_str(&context_comment(&cell.source, dialect));
                    }
                }
                context.push_str(prefix);
                (
                    Feature::Complete,
                    JobInput::Completion {
                        prefix: context,
                        suffix: suffix.clone(),
                    },
                )
            }
            Request::LlmTestProfile { .. } => (
                Feature::TestProfile,
                JobInput::Text {
                    messages: vec![ChatMessage {
                        role: Role::User,
                        content: "Reply with the single word: pong".into(),
                        tool_calls: vec![],
                        tool_call_id: None,
                    }],
                },
            ),
            _ => return Err("Request has no LLM feature input".into()),
        };
        let size = match &result.1 {
            JobInput::Completion { prefix, suffix } => prefix.len().saturating_add(suffix.len()),
            JobInput::Chat { messages, .. }
            | JobInput::Text { messages }
            | JobInput::Structured { messages, .. } => serde_json::to_string(messages)
                .map_err(|_| "Cannot encode feature messages")?
                .len(),
        };
        if size > LIMIT {
            return Err("Feature input exceeds source limit".into());
        }
        Ok(result)
    }
    /// Post-process a real completion using current parser settings and the first local item.
    pub fn filter_llm_completion(
        &self,
        prefix: &str,
        suffix: &str,
        suggestion: &str,
        dialect: Dialect,
    ) -> Option<String> {
        let source = format!("{prefix}{suffix}");
        let cursor = u32::try_from(prefix.len()).ok()?;
        let Response::Completions { items, from, .. } =
            self.complete(source.clone(), dialect, cursor)
        else {
            return None;
        };
        let typed = prefix.get(from as usize..)?;
        let first = items.first().map(|i| i.insert_text.as_str());
        let text = filter_completion(
            prefix,
            suffix,
            suggestion,
            self.effective_dialect(&source, dialect),
            first,
        )?;
        if first.and_then(|s| s.strip_prefix(typed)) == Some(text.as_str()) {
            return None;
        }
        Some(text)
    }
}
