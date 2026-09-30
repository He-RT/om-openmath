//! Symbolic expressions and canonical constructors.
#![forbid(unsafe_code)]

/// Fixed built-in symbol identifiers.
pub mod builtins;
/// Canonical ordering and expression constructors.
pub mod canon;
/// Immutable expression trees and structural operations.
pub mod expr;
/// Shared symbol interning.
pub mod symbol;

pub use builtins::BUILTIN;
pub use canon::{add, canonical_cmp, canonicalize, div, exp, func, mul, neg, pow, sqrt, sub};
/// Evaluation messages and scoped constructor diagnostic capture.
pub mod message;
pub use expr::{Expr, ExprKind, ExprNode, Normal};
pub use message::{Message, Messages, MsgLevel, with_canonical_messages};
pub use symbol::Symbol;
