//! Owned wire adapters preserve lower-layer JSON without borrowing diagnostic codes.
use serde::{Deserialize, Serialize};
use ts_rs::TS;
/// Dialect values accepted by the wire protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub enum Dialect {
    /// Modern.
    Modern,
    /// Wolfram.
    Wolfram,
    /// Auto.
    Auto,
}

impl From<om_parse::Dialect> for Dialect {
    fn from(value: om_parse::Dialect) -> Self {
        match value {
            om_parse::Dialect::Modern => Self::Modern,
            om_parse::Dialect::Wolfram => Self::Wolfram,
            om_parse::Dialect::Auto => Self::Auto,
        }
    }
}

impl From<Dialect> for om_parse::Dialect {
    fn from(value: Dialect) -> Self {
        match value {
            Dialect::Modern => Self::Modern,
            Dialect::Wolfram => Self::Wolfram,
            Dialect::Auto => Self::Auto,
        }
    }
}

/// Severity values accepted by the wire protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub enum Severity {
    /// Error.
    Error,
    /// Warning.
    Warning,
    /// Hint.
    Hint,
}

impl From<om_parse::Severity> for Severity {
    fn from(value: om_parse::Severity) -> Self {
        match value {
            om_parse::Severity::Error => Self::Error,
            om_parse::Severity::Warning => Self::Warning,
            om_parse::Severity::Hint => Self::Hint,
        }
    }
}

/// TokenClass values accepted by the wire protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub enum TokenClass {
    /// Number.
    Number,
    /// Identifier.
    Identifier,
    /// Builtin.
    Builtin,
    /// Operator.
    Operator,
    /// Bracket.
    Bracket,
    /// String.
    String,
    /// Comment.
    Comment,
    /// Keyword.
    Keyword,
    /// Error.
    Error,
}

impl From<om_parse::TokenClass> for TokenClass {
    fn from(value: om_parse::TokenClass) -> Self {
        match value {
            om_parse::TokenClass::Number => Self::Number,
            om_parse::TokenClass::Identifier => Self::Identifier,
            om_parse::TokenClass::Builtin => Self::Builtin,
            om_parse::TokenClass::Operator => Self::Operator,
            om_parse::TokenClass::Bracket => Self::Bracket,
            om_parse::TokenClass::String => Self::String,
            om_parse::TokenClass::Comment => Self::Comment,
            om_parse::TokenClass::Keyword => Self::Keyword,
            om_parse::TokenClass::Error => Self::Error,
        }
    }
}

/// MsgLevel values accepted by the wire protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub enum MsgLevel {
    /// Info.
    Info,
    /// Warning.
    Warning,
    /// Error.
    Error,
}

impl From<om_core::MsgLevel> for MsgLevel {
    fn from(value: om_core::MsgLevel) -> Self {
        match value {
            om_core::MsgLevel::Info => Self::Info,
            om_core::MsgLevel::Warning => Self::Warning,
            om_core::MsgLevel::Error => Self::Error,
        }
    }
}

/// Level values accepted by the wire protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub enum Level {
    /// Major.
    Major,
    /// Minor.
    Minor,
}

impl From<om_solve::Level> for Level {
    fn from(value: om_solve::Level) -> Self {
        match value {
            om_solve::Level::Major => Self::Major,
            om_solve::Level::Minor => Self::Minor,
        }
    }
}

/// Half-open UTF-8 byte offsets into the original source.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct Span {
    /// Inclusive byte offset.
    pub start: u32,
    /// Exclusive byte offset.
    pub end: u32,
}

impl From<om_parse::Span> for Span {
    fn from(value: om_parse::Span) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

/// An optional source repair.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct Fix {
    /// Bytes replaced by this edit.
    pub span: Span,
    /// Replacement source.
    pub replacement: String,
    /// Display label.
    pub label: String,
}

impl From<&om_parse::Fix> for Fix {
    fn from(value: &om_parse::Fix) -> Self {
        Self {
            span: value.span.into(),
            replacement: value.replacement.clone(),
            label: value.label.clone(),
        }
    }
}

/// An owned parser diagnostic; arbitrary wire codes need no static lifetime.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct Diagnostic {
    /// Source bytes responsible for the diagnostic.
    pub span: Span,
    /// Diagnostic importance.
    pub severity: Severity,
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable explanation.
    pub message: String,
    /// Suggested source edit.
    pub fix: Option<Fix>,
}

impl From<&om_parse::Diagnostic> for Diagnostic {
    fn from(value: &om_parse::Diagnostic) -> Self {
        Self {
            span: value.span.into(),
            severity: value.severity.into(),
            code: value.code.into(),
            message: value.message.clone(),
            fix: value.fix.as_ref().map(Fix::from),
        }
    }
}

/// An owned evaluator message.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct Message {
    /// Issuing built-in.
    pub symbol: String,
    /// Stable message tag.
    pub tag: String,
    /// Human-readable explanation.
    pub text: String,
    /// Importance.
    pub level: MsgLevel,
}

impl From<&om_core::Message> for Message {
    fn from(value: &om_core::Message) -> Self {
        Self {
            symbol: value.symbol.clone(),
            tag: value.tag.clone(),
            text: value.text.clone(),
            level: value.level.into(),
        }
    }
}

/// Verification evidence, with the solver's externally tagged JSON representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub enum Verification {
    /// Original residual proved exactly zero.
    Exact,
    /// Certified by the algebraic construction.
    ByConstruction,
    /// Original residual certified at the stated decimal precision.
    Numeric {
        /// Certified decimal digits.
        digits: u32,
    },
    /// No verification evidence.
    Unverified,
}
impl From<om_solve::Verification> for Verification {
    fn from(value: om_solve::Verification) -> Self {
        match value {
            om_solve::Verification::Exact => Self::Exact,
            om_solve::Verification::ByConstruction => Self::ByConstruction,
            om_solve::Verification::Numeric { digits } => Self::Numeric { digits },
            om_solve::Verification::Unverified => Self::Unverified,
        }
    }
}
