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
pub use canon::{add, canonical_cmp, mul};
pub use expr::{Expr, ExprKind, ExprNode, Normal};
pub use symbol::Symbol;
