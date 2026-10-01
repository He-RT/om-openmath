//! Sans-IO configurable LLM integration.
#![forbid(unsafe_code)]

mod types;

pub use types::{ChatMessage, HttpRequest, ProviderKind, Role, Suggestion, ToolCall};
