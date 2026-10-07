//! Symbolic expressions and canonical constructors.
#![forbid(unsafe_code)]

/// Fixed built-in symbol identifiers.
pub mod builtins;
/// Canonical ordering and expression constructors.
pub mod canon;
/// Audited executable metadata shared by parsing and hosts.
pub mod catalog;
/// Portable computation limits and evaluation messages.
pub mod ctx;
/// Immutable expression trees and structural operations.
pub mod expr;
/// Shared finite display colors.
pub mod graphics_color;
/// Shared symbol interning.
pub mod symbol;

pub use builtins::BUILTIN;
pub use canon::{add, canonical_cmp, canonicalize, div, exp, func, mul, neg, pow, sqrt, sub};
pub use ctx::{Abort, Clock, Interrupt};
/// Evaluation messages and scoped constructor diagnostic capture.
pub mod message;
pub use expr::{Expr, ExprKind, ExprNode, Normal};
pub use message::{Message, Messages, MsgLevel, with_canonical_messages};
pub use symbol::Symbol;
