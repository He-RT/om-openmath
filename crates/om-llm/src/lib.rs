//! Sans-IO configurable LLM integration.
#![forbid(unsafe_code)]

mod chat;
mod stream;
pub use stream::{
    NdjsonDecoder, SseDecoder, SseMessage, StreamEvent, decode_anthropic_event, decode_openai_chunk,
};
mod types;
pub use chat::{LlmError, Profile, Target, ToolSpec, build_chat_request, try_build_chat_request};

pub use types::{ChatMessage, HttpRequest, ProviderKind, Role, Suggestion, ToolCall};
