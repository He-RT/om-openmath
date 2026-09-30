//! Binary serialization preserves the value and context without decimal round trips.

use super::{BigFloat, Integer};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error, ser::Error as _};

#[derive(Serialize, Deserialize)]
struct BinaryFloat {
    significand: Integer,
    exponent: isize,
    precision: u32,
}

pub(super) fn serialize<S: Serializer>(x: &BigFloat, serializer: S) -> Result<S::Ok, S::Error> {
    if !x.repr().is_finite() {
        return Err(S::Error::custom("Number requires a finite float"));
    }
    let precision = u32::try_from(x.precision()).map_err(S::Error::custom)?;
    BinaryFloat {
        significand: x.repr().significand().clone(),
        exponent: x.repr().exponent(),
        precision,
    }
    .serialize(serializer)
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BigFloat, D::Error> {
    let x = BinaryFloat::deserialize(deserializer)?;
    if x.precision == 0 {
        return Err(D::Error::custom("Number requires positive bit precision"));
    }
    Ok(BigFloat::from_parts(x.significand, x.exponent)
        .with_precision(x.precision as usize)
        .value())
}
