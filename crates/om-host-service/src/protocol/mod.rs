//! Strict native host contract types. Shape validation is separate from authorization/state admission.
use serde::{Deserialize, Serialize};

/// DTO fields generated from the machine contracts.
pub mod generated;
pub mod validation;
/// Native control frame schema and typed decoding.
pub mod wire;

/// Largest exact shared integer supported by Rust, Swift and JavaScript.
pub const MAX_SERIAL: u64 = 9_007_199_254_740_991;
/// Maximum ordinary control frame size; large artifacts are referenced separately.
pub const MAX_CONTROL_BYTES: usize = 2 * 1024 * 1024;

/// Canonical messages and native DTOs share the exact required-null representation.
pub use om_agent_contract::Nullable;

/// Shared exactly representable monotonic counter. Construction and decode check its range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Serial(u64);
impl Serial {
    /// Check a counter without truncating or wrapping.
    pub fn new(value: u64) -> Result<Self, String> {
        if value <= MAX_SERIAL {
            Ok(Self(value))
        } else {
            Err("serial exceeds exact shared range".into())
        }
    }
    /// Retrieve the exact counter.
    pub fn get(self) -> u64 {
        self.0
    }
    /// Increment without wrapping.
    pub fn checked_next(self) -> Result<Self, String> {
        self.0
            .checked_add(1)
            .ok_or_else(|| "serial overflow".into())
            .and_then(Self::new)
    }
}
impl<'de> Deserialize<'de> for Serial {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let integer = value
            .as_u64()
            .or_else(|| {
                value.as_f64().and_then(|v| {
                    (v.is_finite() && v >= 0.0 && v.fract() == 0.0 && v <= MAX_SERIAL as f64)
                        .then_some(v as u64)
                })
            })
            .ok_or_else(|| serde::de::Error::custom("expected exact nonnegative shared integer"))?;
        Self::new(integer).map_err(serde::de::Error::custom)
    }
}
