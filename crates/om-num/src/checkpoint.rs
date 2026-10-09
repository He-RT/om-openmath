//! Versioned numeric atoms. No float-to-decimal conversion or unbounded recursive deserialization.
use crate::{
    BigFloat, Complex, Integer, Number, Rational, Real,
    ctx::{Abort, Interrupt},
};
use dashu::{base::BitTest, integer::UBig};

/// Bounds verified before parsing components or allocating arbitrary-precision values.
#[derive(Clone, Copy, Debug)]
pub struct NumberLimits {
    /// Maximum complete encoded atom bytes.
    pub max_bytes: usize,
    /// Maximum saved binary precision, without silently lowering it.
    pub max_precision_bits: u32,
}
impl Default for NumberLimits {
    fn default() -> Self {
        Self {
            max_bytes: 4 * 1024 * 1024,
            max_precision_bits: 1 << 20,
        }
    }
}
/// A failed decode never returns an approximate substitute or partial atom.
#[derive(Debug, thiserror::Error)]
pub enum NumberCodecError {
    /// Wrong magic, version, tag, lengths or noncanonical input.
    #[error("invalid numeric checkpoint")]
    Invalid,
    /// Explicit bytes/precision/platform-exponent range failure.
    #[error("numeric checkpoint exceeds its limits")]
    Limit,
    /// Real operation cancellation/deadline/budget.
    #[error(transparent)]
    Abort(#[from] Abort),
}
/// Encode one finite normalized numeric atom, preserving machine bits exactly.
pub fn encode_number(
    value: &Number,
    limits: NumberLimits,
    ctx: &Interrupt,
) -> Result<Vec<u8>, NumberCodecError> {
    ctx.tick()?;
    let mut out = Vec::new();
    put(&mut out, b"OMNU\x01", limits)?;
    encode(value, true, &mut out, limits, ctx)?;
    Ok(out)
}
/// Decode data only; never evaluates mathematical source or accepts unknown extensions.
pub fn decode_number(
    bytes: &[u8],
    limits: NumberLimits,
    ctx: &Interrupt,
) -> Result<Number, NumberCodecError> {
    ctx.tick()?;
    if bytes.len() > limits.max_bytes {
        return Err(NumberCodecError::Limit);
    }
    let mut reader = Reader { bytes, position: 0 };
    if reader.take(5)? != b"OMNU\x01" {
        return Err(NumberCodecError::Invalid);
    }
    let value = decode(&mut reader, true, limits, ctx)?;
    if reader.position != bytes.len() {
        return Err(NumberCodecError::Invalid);
    }
    Ok(value)
}
fn put(out: &mut Vec<u8>, bytes: &[u8], limits: NumberLimits) -> Result<(), NumberCodecError> {
    if out
        .len()
        .checked_add(bytes.len())
        .is_none_or(|size| size > limits.max_bytes)
    {
        return Err(NumberCodecError::Limit);
    }
    out.extend_from_slice(bytes);
    Ok(())
}
fn blob(out: &mut Vec<u8>, bytes: &[u8], limits: NumberLimits) -> Result<(), NumberCodecError> {
    let size = u32::try_from(bytes.len()).map_err(|_| NumberCodecError::Limit)?;
    put(out, &size.to_le_bytes(), limits)?;
    put(out, bytes, limits)
}
fn integer(out: &mut Vec<u8>, n: &Integer, limits: NumberLimits) -> Result<(), NumberCodecError> {
    if n.clone().into_parts().1.bit_len() > limits.max_bytes.saturating_mul(8) {
        return Err(NumberCodecError::Limit);
    }
    blob(out, &n.to_le_bytes(), limits)
}
fn encode(
    value: &Number,
    scalar: bool,
    out: &mut Vec<u8>,
    limits: NumberLimits,
    ctx: &Interrupt,
) -> Result<(), NumberCodecError> {
    ctx.tick()?;
    match value {
        Number::Integer(n) => {
            put(out, &[0], limits)?;
            integer(out, n, limits)?;
        }
        Number::Rational(q) => {
            if q.denominator().is_one() {
                return Err(NumberCodecError::Invalid);
            }
            if q.denominator().bit_len() > limits.max_bytes.saturating_mul(8) {
                return Err(NumberCodecError::Limit);
            }
            put(out, &[1], limits)?;
            integer(out, q.numerator(), limits)?;
            blob(out, &q.denominator().to_le_bytes(), limits)?;
        }
        Number::Real(Real::Machine(x)) => {
            if !x.is_finite() {
                return Err(NumberCodecError::Invalid);
            }
            put(out, &[2], limits)?;
            put(out, &x.to_bits().to_le_bytes(), limits)?;
        }
        Number::Real(Real::Big(x)) => {
            let precision = u32::try_from(x.precision()).map_err(|_| NumberCodecError::Limit)?;
            if precision == 0 || precision > limits.max_precision_bits || !x.repr().is_finite() {
                return Err(NumberCodecError::Limit);
            }
            put(out, &[3], limits)?;
            integer(out, x.repr().significand(), limits)?;
            let exponent =
                i64::try_from(x.repr().exponent()).map_err(|_| NumberCodecError::Limit)?;
            put(out, &exponent.to_le_bytes(), limits)?;
            put(out, &precision.to_le_bytes(), limits)?;
        }
        Number::Complex(c) if scalar => {
            put(out, &[4], limits)?;
            encode(&c.re, false, out, limits, ctx)?;
            encode(&c.im, false, out, limits, ctx)?;
            if c.re.precision() != c.im.precision() || (c.im.is_exact() && c.im.is_zero()) {
                return Err(NumberCodecError::Invalid);
            }
        }
        Number::Complex(_) => return Err(NumberCodecError::Invalid),
    }
    Ok(())
}
struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, size: usize) -> Result<&'a [u8], NumberCodecError> {
        let end = self
            .position
            .checked_add(size)
            .filter(|&end| end <= self.bytes.len())
            .ok_or(NumberCodecError::Invalid)?;
        let bytes = &self.bytes[self.position..end];
        self.position = end;
        Ok(bytes)
    }
    fn blob(&mut self) -> Result<&'a [u8], NumberCodecError> {
        let size = u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| NumberCodecError::Invalid)?,
        ) as usize;
        self.take(size)
    }
    fn integer(&mut self) -> Result<Integer, NumberCodecError> {
        let bytes = self.blob()?;
        let value = Integer::from_le_bytes(bytes);
        if value.to_le_bytes().as_ref() != bytes {
            return Err(NumberCodecError::Invalid);
        }
        Ok(value)
    }
}
fn decode(
    reader: &mut Reader<'_>,
    scalar: bool,
    limits: NumberLimits,
    ctx: &Interrupt,
) -> Result<Number, NumberCodecError> {
    ctx.tick()?;
    Ok(match reader.take(1)?[0] {
        0 => Number::Integer(reader.integer()?),
        1 => {
            let numerator = reader.integer()?;
            let bytes = reader.blob()?;
            let denominator = UBig::from_le_bytes(bytes);
            if denominator.is_zero()
                || denominator.is_one()
                || denominator.to_le_bytes().as_ref() != bytes
            {
                return Err(NumberCodecError::Invalid);
            }
            let value = Rational::from_parts(numerator.clone(), denominator.clone());
            if value.numerator() != &numerator || value.denominator() != &denominator {
                return Err(NumberCodecError::Invalid);
            }
            Number::Rational(value)
        }
        2 => {
            let bits = u64::from_le_bytes(
                reader
                    .take(8)?
                    .try_into()
                    .map_err(|_| NumberCodecError::Invalid)?,
            );
            let value = f64::from_bits(bits);
            if !value.is_finite() {
                return Err(NumberCodecError::Invalid);
            }
            Number::Real(Real::Machine(value))
        }
        3 => {
            let significand = reader.integer()?;
            let exponent = i64::from_le_bytes(
                reader
                    .take(8)?
                    .try_into()
                    .map_err(|_| NumberCodecError::Invalid)?,
            );
            let precision = u32::from_le_bytes(
                reader
                    .take(4)?
                    .try_into()
                    .map_err(|_| NumberCodecError::Invalid)?,
            );
            if precision == 0
                || precision > limits.max_precision_bits
                || significand.clone().into_parts().1.bit_len() > precision as usize
            {
                return Err(NumberCodecError::Limit);
            }
            let exponent = isize::try_from(exponent).map_err(|_| NumberCodecError::Limit)?;
            if significand.is_zero() {
                if exponent != 0 && exponent != -1 {
                    return Err(NumberCodecError::Invalid);
                }
            } else if exponent == isize::MAX
                || exponent == isize::MIN
                || significand.to_le_bytes()[0] & 1 == 0
            {
                // Dashu reserves extreme exponents for special values. Reject even mantissas
                // before its normalization could shift an attacker-controlled extreme exponent.
                return Err(NumberCodecError::Invalid);
            }
            let value = BigFloat::from_parts(significand.clone(), exponent)
                .with_precision(precision as usize)
                .value();
            if !value.repr().is_finite()
                || value.repr().significand() != &significand
                || value.repr().exponent() != exponent
            {
                return Err(NumberCodecError::Invalid);
            }
            Number::Real(Real::Big(value))
        }
        4 if scalar => {
            let re = decode(reader, false, limits, ctx)?;
            let im = decode(reader, false, limits, ctx)?;
            if re.precision() != im.precision() || (im.is_exact() && im.is_zero()) {
                return Err(NumberCodecError::Invalid);
            }
            Number::Complex(Box::new(Complex { re, im }))
        }
        _ => return Err(NumberCodecError::Invalid),
    })
}
