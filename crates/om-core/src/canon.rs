//! Canonical expression ordering and, in later tasks, canonical constructors.

#[path = "order.rs"]
mod order;
pub use order::canonical_cmp;

#[path = "add.rs"]
mod add;
pub use add::add;
