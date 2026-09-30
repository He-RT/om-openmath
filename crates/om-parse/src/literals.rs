//! Numeric and string conversion without canonical expression constructors.

use om_core::Expr;
use om_num::{BigFloat, Integer, Number, Rational, Real};

pub(crate) fn number(text: &str) -> Result<Expr, &'static str> {
    if text.len() > 1_000_000 {
        return Err("数值字面量超出解析资源限制");
    }
    let text = text.replace('_', "");
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        return Integer::from_str_radix(hex, 16)
            .map(Expr::integer)
            .map_err(|_| "非法整数");
    }
    if text.contains(['`', '*']) || text.contains("^^") {
        return Err("现代数字格式不支持 Wolfram 后缀");
    }
    if !text.contains(['.', 'e', 'E']) {
        return text
            .parse::<Integer>()
            .map(Expr::integer)
            .map_err(|_| "非法整数");
    }
    let (mantissa, exponent) = text
        .split_once(['e', 'E'])
        .map_or((text.as_str(), None), |(m, e)| (m, Some(e)));
    if let Some(exponent) = exponent {
        let exponent = exponent
            .parse::<i64>()
            .map_err(|_| "数值字面量超出解析资源限制")?;
        if exponent.unsigned_abs() > 1_000_000 {
            return Err("数值字面量超出解析资源限制");
        }
    }
    let digits = mantissa.replace('.', "");
    let significant = digits.trim_start_matches('0').len().max(1);
    let nonzero = digits.bytes().any(|c| c != b'0');
    if significant <= 16
        && let Ok(value) = text.parse::<f64>()
        && value.is_finite()
        && (value != 0.0 || !nonzero)
    {
        return Ok(Expr::real(value));
    }
    let bits = if significant > 16 {
        (significant as f64 * std::f64::consts::LOG2_10).ceil() as usize
    } else {
        53
    };
    let exact = Rational::from_str_decimal(&text).map_err(|_| "非法十进制数字")?;
    let big: BigFloat = exact.to_float(bits).value();
    Ok(Expr::number(Number::Real(Real::Big(big))))
}

pub(crate) fn string(text: &str) -> Result<Expr, &'static str> {
    let mut result = String::new();
    let mut chars = text[1..text.len() - 1].chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }
        let escaped = match chars.next() {
            Some('n') => '\n',
            Some('r') => '\r',
            Some('t') => '\t',
            Some('b') => '\u{8}',
            Some('f') => '\u{c}',
            Some('"') => '"',
            Some('\\') => '\\',
            Some('/') => '/',
            Some('u') => {
                let high = hex4(&mut chars)?;
                let scalar = if (0xd800..=0xdbff).contains(&high) {
                    if chars.next() != Some('\\') || chars.next() != Some('u') {
                        return Err("Unicode 转义缺少低代理项");
                    }
                    let low = hex4(&mut chars)?;
                    if !(0xdc00..=0xdfff).contains(&low) {
                        return Err("非法 Unicode 代理项");
                    }
                    0x10000 + (high - 0xd800) * 0x400 + (low - 0xdc00)
                } else {
                    high
                };
                char::from_u32(scalar).ok_or("非法 Unicode 字符")?
            }
            _ => return Err("非法字符串转义"),
        };
        result.push(escaped);
    }
    Ok(Expr::string(&result))
}
fn hex4(chars: &mut std::str::Chars<'_>) -> Result<u32, &'static str> {
    let mut value = 0;
    for _ in 0..4 {
        value = value * 16
            + chars
                .next()
                .and_then(|c| c.to_digit(16))
                .ok_or("非法 Unicode 转义")?;
    }
    Ok(value)
}
