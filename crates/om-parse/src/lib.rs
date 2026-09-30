//! Modern and Wolfram-subset expression parser.
#![forbid(unsafe_code)]

/// Shared tokenization for both input dialects.
pub mod lexer;

/// Input syntax requested by the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Dialect {
    /// Conventional mathematical syntax and lowercase function names.
    Modern,
    /// Supported Wolfram Language input syntax.
    Wolfram,
    /// Detect syntax at the parser entry point.
    Auto,
}

/// Half-open UTF-8 byte offsets into the original source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Span {
    /// Inclusive starting byte offset.
    pub start: u32,
    /// Exclusive ending byte offset.
    pub end: u32,
}

/// A source diagnostic and optional repair.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Diagnostic {
    /// Source bytes responsible for the diagnostic.
    pub span: Span,
    /// Importance of the diagnostic.
    pub severity: Severity,
    /// Stable machine-readable diagnostic code.
    pub code: &'static str,
    /// Human-readable explanation.
    pub message: String,
    /// Optional source edit that addresses the issue.
    pub fix: Option<Fix>,
}

/// A replacement of a source span.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Fix {
    /// Bytes to replace; an empty span inserts text.
    pub span: Span,
    /// Replacement source text.
    pub replacement: String,
    /// Human-readable action label.
    pub label: String,
}

/// Diagnostic importance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Severity {
    /// Input cannot be interpreted as written.
    Error,
    /// Input is accepted but may have unintended semantics.
    Warning,
    /// An explanatory suggestion for valid input.
    Hint,
}

/// Source highlighting category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum TokenClass {
    /// Numeric literal.
    Number,
    /// Symbol name.
    Identifier,
    /// Recognized built-in name.
    Builtin,
    /// Operator or separator.
    Operator,
    /// Grouping delimiter.
    Bracket,
    /// Quoted string.
    String,
    /// Discarded source comment.
    Comment,
    /// Reserved language word.
    Keyword,
    /// Invalid source token.
    Error,
}
