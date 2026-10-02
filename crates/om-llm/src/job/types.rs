//! Explicit host validation and real feature/input/state outputs from PLAN §11.4.
use crate::{ChatMessage, HttpRequest, LlmError, Suggestion, ToolCall, ToolSpec};
use std::sync::Arc;
/// Requested user-facing assistant feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    /// Natural-language source proposal.
    Translate,
    /// Explain genuine supplied computation data.
    Explain,
    /// Raw completion (post-filtering is a later host feature).
    Complete,
    /// Tool-assisted chat.
    Chat,
    /// Source correction proposal.
    Fix,
    /// Actual provider connectivity probe.
    TestProfile,
}
/// Required pure host interface for real source/format verification.
pub trait SuggestionValidator: Send + Sync {
    /// Return only a genuinely validated/rendered suggestion or actual diagnostic.
    fn validate(&self, response: &str) -> Result<Suggestion, String>;
}
/// Actual already-prepared input; prompt/context assembly is the later host integration.
pub enum JobInput {
    /// Tool-enabled messages and actual schemas.
    Chat {
        /// Actual conversation.
        messages: Vec<ChatMessage>,
        /// Real host tool definitions.
        tools: Vec<ToolSpec>,
    },
    /// Plain streamed text for Explain/TestProfile.
    Text {
        /// Actual conversation.
        messages: Vec<ChatMessage>,
    },
    /// Structured proposal requiring host verification, never model renderings.
    Structured {
        /// Actual conversation.
        messages: Vec<ChatMessage>,
        /// Mandatory genuine host parser/formatter adapter.
        validator: Arc<dyn SuggestionValidator>,
    },
    /// Actual source prefix/suffix for raw completion.
    Completion {
        /// Before the cursor.
        prefix: String,
        /// After the cursor.
        suffix: String,
    },
}
/// Completed actual feature output.
#[derive(Clone, Debug)]
pub enum JobResult {
    /// Actual model text.
    Text(String),
    /// Host-verified source and rendering.
    Suggestion(Suggestion),
    /// Actual raw completion text.
    Completion(String),
}
/// One concrete next action/result; the core never performs network or CAS IO.
#[derive(Clone, Debug)]
pub enum JobStep {
    /// Actual checked transport proposal.
    Http(HttpRequest),
    /// Actual complete calls for the host readonly tool handler.
    RunTools(Vec<ToolCall>),
    /// Real completed result.
    Done(JobResult),
    /// Checked failure.
    Failed(LlmError),
}
