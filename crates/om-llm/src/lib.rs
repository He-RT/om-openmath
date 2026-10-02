//! Sans-IO configurable LLM integration.
#![forbid(unsafe_code)]

mod chat;
mod job;
/// Embedded prompts and actual host tool schemas, without IO or CAS dependencies.
pub mod prompts;
pub use job::{Feature, Job, JobInput, JobResult, JobStep, SuggestionValidator};
mod fim;
pub use fim::{build_fim_request, parse_fim_response, try_build_fim_request};
mod stream;
pub use stream::{
    NdjsonDecoder, SseDecoder, SseMessage, StreamEvent, decode_anthropic_event, decode_openai_chunk,
};
mod types;
pub use chat::{LlmError, Profile, Target, ToolSpec, build_chat_request, try_build_chat_request};

pub use types::{ChatMessage, HttpRequest, ProviderKind, Role, Suggestion, ToolCall};
