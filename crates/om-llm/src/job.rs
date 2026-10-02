//! Pure Job lifecycle owns genuine provider state; no HTTP or CAS execution here.
mod ingest;
mod response;
mod types;
use crate::{
    ChatMessage, HttpRequest, LlmError, Profile, ProviderKind, Role, SseDecoder, StreamEvent,
    Target, ToolCall, ToolSpec, try_build_chat_request, try_build_fim_request,
};
use std::{collections::BTreeMap, sync::Arc};
pub use types::{Feature, JobInput, JobResult, JobStep, SuggestionValidator};
const LIMIT: usize = 1_048_576;
const MAX_TOOLS: usize = 64;
const MAX_ROUNDS: u32 = 6;
const MAX_RETRIES: u32 = 2;
#[derive(Default)]
struct Pending {
    id: String,
    name: String,
    args: String,
}
struct Round {
    stream: bool,
    raw: Vec<u8>,
    decoder: SseDecoder,
    text: String,
    reason: Option<String>,
    saw_text: bool,
    tools: BTreeMap<u32, Pending>,
}
impl Round {
    fn new(stream: bool) -> Self {
        Self {
            stream,
            raw: vec![],
            decoder: SseDecoder::new(),
            text: String::new(),
            reason: None,
            saw_text: false,
            tools: BTreeMap::new(),
        }
    }
}
enum State {
    Http,
    Tools(Vec<ToolCall>),
    Done(JobResult),
    Failed(LlmError),
}
/// Actual sans-IO request/stream/tool/validation lifecycle shared by future hosts.
pub struct Job {
    feature: Feature,
    profile: Profile,
    target: Target,
    messages: Vec<ChatMessage>,
    tools: Vec<ToolSpec>,
    validator: Option<Arc<dyn SuggestionValidator>>,
    completion: Option<(String, String)>,
    state: State,
    round: Round,
    rounds: u32,
    retries: u32,
    http_open: bool,
}
impl Job {
    /// Start one real feature. Invalid input returns Failed, never a dummy request or panic.
    pub fn new(
        feature: Feature,
        mut profile: Profile,
        input: JobInput,
        target: Target,
    ) -> (Job, JobStep) {
        let valid = matches!(
            (&input, feature),
            (JobInput::Chat { .. }, Feature::Chat)
                | (
                    JobInput::Text { .. },
                    Feature::Explain | Feature::TestProfile
                )
                | (
                    JobInput::Structured { .. },
                    Feature::Translate | Feature::Fix
                )
                | (JobInput::Completion { .. }, Feature::Complete)
        );
        let (messages, tools, validator, completion) = match input {
            JobInput::Chat { messages, tools } => (messages, tools, None, None),
            JobInput::Text { messages } => (messages, vec![], None, None),
            JobInput::Structured {
                messages,
                validator,
            } => (messages, vec![], Some(validator), None),
            JobInput::Completion { prefix, suffix } => {
                (vec![], vec![], None, Some((prefix, suffix)))
            }
        };
        if feature == Feature::TestProfile {
            profile.max_tokens = 8;
        }
        let mut job = Self {
            feature,
            profile,
            target,
            messages,
            tools,
            validator,
            completion,
            state: State::Http,
            round: Round::new(true),
            rounds: 0,
            retries: 0,
            http_open: false,
        };
        let step = if valid {
            job.next_http()
        } else {
            job.fail(LlmError::Input)
        };
        (job, step)
    }
    fn request(&self) -> Result<HttpRequest, LlmError> {
        if let Some((prefix, suffix)) = &self.completion {
            return try_build_fim_request(&self.profile, prefix, suffix, self.target);
        }
        if self.feature == Feature::TestProfile
            && !matches!(
                self.profile.kind,
                ProviderKind::OpenaiChat | ProviderKind::Anthropic
            )
        {
            let prompt = self
                .messages
                .iter()
                .rev()
                .find(|m| m.role == Role::User)
                .ok_or(LlmError::Input)?
                .content
                .as_str();
            return try_build_fim_request(&self.profile, prompt, "", self.target);
        }
        try_build_chat_request(
            &self.profile,
            &self.messages,
            &self.tools,
            matches!(self.feature, Feature::Translate | Feature::Fix),
            self.target,
        )
    }
    fn next_http(&mut self) -> JobStep {
        match self.request() {
            Ok(request) => {
                self.round = Round::new(request.stream);
                self.state = State::Http;
                self.http_open = true;
                JobStep::Http(request)
            }
            Err(error) => self.fail(error),
        }
    }
    fn fail(&mut self, error: LlmError) -> JobStep {
        self.http_open = false;
        self.state = State::Failed(error.clone());
        JobStep::Failed(error)
    }
    fn final_step(&self) -> Option<JobStep> {
        match &self.state {
            State::Done(result) => Some(JobStep::Done(result.clone())),
            State::Failed(error) => Some(JobStep::Failed(error.clone())),
            _ => None,
        }
    }
    /// Actual str chunks (browser decoder must preserve UTF-8); errors are returned as events.
    pub fn on_bytes(&mut self, chunk: &str) -> Vec<StreamEvent> {
        self.on_raw_bytes(chunk.as_bytes())
    }
    /// Actual arbitrary native byte chunks, with no lossy text conversion.
    pub fn on_raw_bytes(&mut self, chunk: &[u8]) -> Vec<StreamEvent> {
        if !matches!(self.state, State::Http) {
            return vec![];
        }
        match self.ingest(chunk) {
            Ok(events) => events,
            Err(error) => {
                let message = self.clean(&error.to_string());
                self.fail(error);
                self.http_open = true;
                vec![StreamEvent::Error(message)]
            }
        }
    }
    /// Resolve actual HTTP completion; finish deltas alone never dispatch tools/results.
    pub fn on_http_end(&mut self, status: u16, error: Option<String>) -> JobStep {
        self.http_end(status, error)
    }
    /// Append actual current tool results once, then produce the real next request.
    pub fn tool_results(&mut self, results: Vec<(String, String)>) -> JobStep {
        if let Some(step) = self.final_step() {
            return step;
        }
        let State::Tools(calls) = &self.state else {
            return self.fail(LlmError::State("not waiting for tool results"));
        };
        let mut actual = BTreeMap::new();
        let mut size = 0usize;
        for (id, content) in results {
            size = match size.checked_add(content.len()) {
                Some(n) if n <= LIMIT => n,
                _ => return self.fail(LlmError::Limit { limit: LIMIT }),
            };
            if actual.insert(id, content).is_some() {
                return self.fail(LlmError::Tools);
            }
        }
        if actual.len() != calls.len() || calls.iter().any(|c| !actual.contains_key(&c.id)) {
            return self.fail(LlmError::Tools);
        }
        let calls = calls.clone();
        self.messages.push(ChatMessage {
            role: Role::Assistant,
            content: self.round.text.clone(),
            tool_calls: calls.clone(),
            tool_call_id: None,
        });
        for call in calls {
            let Some(content) = actual.remove(&call.id) else {
                return self.fail(LlmError::Tools);
            };
            self.messages.push(ChatMessage {
                role: Role::Tool,
                content,
                tool_calls: vec![],
                tool_call_id: Some(call.id),
            });
        }
        self.next_http()
    }
    /// Terminal result/failure state; late data cannot resurrect a finished job.
    pub fn is_finished(&self) -> bool {
        matches!(self.state, State::Done(_) | State::Failed(_))
    }
    /// Cancel active work without transport IO; terminal work retains its actual result.
    pub fn cancel(&mut self) -> JobStep {
        if let Some(step) = self.final_step() {
            step
        } else {
            self.fail(LlmError::Cancelled)
        }
    }
    fn clean(&self, message: &str) -> String {
        let mut value = message.to_owned();
        let mut secrets: Vec<&str> = self
            .profile
            .api_key
            .iter()
            .map(String::as_str)
            .chain(self.profile.extra_headers.values().map(String::as_str))
            .filter(|s| !s.is_empty())
            .collect();
        let tokens: Vec<_> = secrets
            .iter()
            .filter_map(|s| {
                s.strip_prefix("Bearer ")
                    .or_else(|| s.strip_prefix("Basic "))
            })
            .collect();
        secrets.extend(tokens);
        secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
        secrets.dedup();
        for secret in secrets {
            value = value.replace(secret, "***");
        }
        value
    }
}
impl std::fmt::Debug for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Job")
            .field("feature", &self.feature)
            .field("target", &self.target)
            .field("finished", &self.is_finished())
            .field("tool_rounds", &self.rounds)
            .field("validation_retries", &self.retries)
            .finish_non_exhaustive()
    }
}
