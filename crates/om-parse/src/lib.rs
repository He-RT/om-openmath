//! Modern and Wolfram-subset expression parser.
#![forbid(unsafe_code)]

/// Shared tokenization for both input dialects.
pub mod lexer;

mod diagnostics;
mod literals;
mod modern;
mod modern_options;
mod names;
mod names_editor;
pub use names_editor::{identifier_symbol, modern_name};
mod parser;
mod wolfram;

pub use parser::{detect_dialect, parse, parse_expr, parse_with};

use om_core::{Expr, Symbol};
use std::collections::BTreeSet;

/// Constants recognized in mathematical input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConstantMode {
    /// Recognize e and i as E and I.
    #[default]
    Math,
    /// Keep e and i as user symbols.
    Strict,
}

/// Session information used to resolve modern syntax.
#[derive(Clone, Debug, Default)]
pub struct ParseEnv {
    /// Previously declared user functions.
    pub known_functions: BTreeSet<Symbol>,
    /// Interpretation of the mathematical e and i aliases.
    pub constants: ConstantMode,
}

/// Statements, diagnostics and highlighting for a source cell.
#[derive(Debug)]
pub struct ParseOutput {
    /// Statements in source order, including partial results after errors.
    pub statements: Vec<Stmt>,
    /// Lexical and syntactic diagnostics.
    pub diagnostics: Vec<Diagnostic>,
    /// Effective dialect, with Auto resolved.
    pub dialect: Dialect,
    /// Highlighting spans into the original source.
    pub tokens: Vec<(Span, TokenClass)>,
}

/// A source statement and its raw expression tree.
#[derive(Debug)]
pub struct Stmt {
    /// Expression before canonicalization or evaluation.
    pub expr: Expr,
    /// Source byte range, including an output-suppressing semicolon if present.
    pub span: Span,
    /// Whether a trailing semicolon hides its output.
    pub suppress_output: bool,
}

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
