//! Canonical expression ordering and, in later tasks, canonical constructors.

#[path = "order.rs"]
mod order;
pub use order::canonical_cmp;

#[path = "add.rs"]
mod add;
pub use add::add;

#[path = "mul.rs"]
mod mul;
pub use mul::mul;
#[path = "product_power.rs"]
mod product_power;
#[path = "special.rs"]
mod special;
