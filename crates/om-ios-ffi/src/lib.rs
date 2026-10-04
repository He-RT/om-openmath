//! iOS host and audited foreign-function boundary.
#![deny(unsafe_op_in_unsafe_fn)]
mod ffi;
mod host;
pub use ffi::*;
pub use host::Host;
