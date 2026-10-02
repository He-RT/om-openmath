//! Actual HTTP status and genuine host validation decide the next action.
use super::{Feature, Job, JobResult, JobStep, MAX_RETRIES, MAX_ROUNDS, State};
use crate::{ChatMessage, LlmError, ProviderKind, Role, ToolCall, parse_fim_response};
use serde_json::Value;
use std::collections::BTreeSet;
impl Job {
    pub(super) fn http_end(&mut self, status: u16, error: Option<String>) -> JobStep {
        if !self.http_open
            && let Some(step) = self.final_step()
        {
            return step;
        }
        if matches!(
            self.state,
            State::Done(_) | State::Failed(LlmError::Cancelled)
        ) {
            return self.final_step().unwrap_or(JobStep::Failed(LlmError::State(
                "terminal result unavailable",
            )));
        }
        if matches!(self.state, State::Tools(_)) {
            return self.fail(LlmError::State("HTTP already ended"));
        }
        self.http_open = false;
        if let Some(error) = error {
            return self.fail(LlmError::Transport(self.clean(&error)));
        }
        if !(200..300).contains(&status) {
            let raw = std::str::from_utf8(&self.round.raw)
                .ok()
                .and_then(|s| serde_json::from_str::<Value>(s).ok());
            let message = raw
                .as_ref()
                .and_then(|v| v.get("error"))
                .and_then(|e| {
                    e.as_str()
                        .or_else(|| e.get("message").and_then(Value::as_str))
                })
                .or_else(|| {
                    if let State::Failed(e) = &self.state {
                        e.remote_message()
                    } else {
                        None
                    }
                })
                .unwrap_or("provider request failed");
            let mut message = self.clean(message);
            if status == 401 || status == 403 {
                message.push_str("; check API Key");
            }
            return self.fail(LlmError::Http { status, message });
        }
        if let Some(step) = self.final_step() {
            return step;
        }
        if !self.round.stream {
            let body = match std::str::from_utf8(&self.round.raw) {
                Ok(body) => body,
                Err(_) => return self.fail(LlmError::Encoding),
            };
            return match parse_fim_response(self.profile.kind, body) {
                Ok(text) => self.done(if self.feature == Feature::Complete {
                    JobResult::Completion(text)
                } else {
                    JobResult::Text(text)
                }),
                Err(error) => {
                    let error = if let Some(message) = error.remote_message() {
                        LlmError::Remote(self.clean(message))
                    } else {
                        error
                    };
                    self.fail(error)
                }
            };
        }
        if let Err(e) = self.round.decoder.finish() {
            return self.fail(e);
        }
        let Some(reason) = self.round.reason.as_deref() else {
            return self.fail(LlmError::Incomplete);
        };
        if !self.round.tools.is_empty() {
            if self.feature != Feature::Chat
                || !self.profile.supports_tools
                || self.tools.is_empty()
            {
                return self.fail(LlmError::Tools);
            }
            if !matches!(reason, "tool_calls" | "tool_use" | "done") {
                return self.fail(LlmError::Tools);
            }
            if self.rounds >= MAX_ROUNDS {
                return self.fail(LlmError::ToolRounds);
            }
            let mut ids = BTreeSet::new();
            let mut calls = vec![];
            for call in self.round.tools.values() {
                if call.id.is_empty() || call.name.is_empty() || !ids.insert(call.id.clone()) {
                    return self.fail(LlmError::Tools);
                }
                let arguments =
                    if call.args.is_empty() && self.profile.kind == ProviderKind::Anthropic {
                        "{}".into()
                    } else {
                        call.args.clone()
                    };
                if !serde_json::from_str::<Value>(&arguments).is_ok_and(|v| v.is_object()) {
                    return self.fail(LlmError::Tools);
                }
                calls.push(ToolCall {
                    id: call.id.clone(),
                    name: call.name.clone(),
                    arguments,
                });
            }
            self.rounds += 1;
            self.state = State::Tools(calls.clone());
            return JobStep::RunTools(calls);
        }
        if matches!(reason, "tool_calls" | "tool_use") {
            return self.fail(LlmError::Tools);
        }
        if !self.round.saw_text {
            return self.fail(LlmError::Incomplete);
        }
        if matches!(self.feature, Feature::Translate | Feature::Fix) {
            return self.structured();
        }
        self.done(JobResult::Text(self.round.text.clone()))
    }
    fn done(&mut self, result: JobResult) -> JobStep {
        self.state = State::Done(result.clone());
        JobStep::Done(result)
    }
    fn structured(&mut self) -> JobStep {
        let Some(validator) = self.validator.clone() else {
            return self.fail(LlmError::Input);
        };
        match validator.validate(&self.round.text) {
            Ok(suggestion) => self.done(JobResult::Suggestion(suggestion)),
            Err(diagnostic) => {
                let diagnostic = self.clean(&diagnostic);
                if self.retries >= MAX_RETRIES {
                    return self.fail(LlmError::Validation(diagnostic));
                }
                self.retries += 1;
                self.messages.push(ChatMessage {
                    role: Role::Assistant,
                    content: self.clean(&self.round.text),
                    tool_calls: vec![],
                    tool_call_id: None,
                });
                self.messages.push(ChatMessage {
                    role: Role::User,
                    content: format!(
                        "Your expression failed to parse: {diagnostic}. Return corrected JSON."
                    ),
                    tool_calls: vec![],
                    tool_call_id: None,
                });
                self.next_http()
            }
        }
    }
}
