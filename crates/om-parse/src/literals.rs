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
    if text.contains('`') {
        return wolfram_number(&text);
    }
    if text.contains('*') || text.contains("^^") {
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
    string_impl(text, false)
}
pub(crate) fn wolfram_string(text: &str) -> Result<Expr, &'static str> {
    string_impl(text, true)
}
fn string_impl(text: &str, named: bool) -> Result<Expr, &'static str> {
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
            Some('[') if named => {
                let mut name = String::new();
                let mut closed = false;
                for c in chars.by_ref() {
                    if c == ']' {
                        closed = true;
                        break;
                    }
                    name.push(c);
                }
                if !closed {
                    return Err("不完整的具名字符");
                }
                crate::lexer::named::value(&name).ok_or("未知具名字符")?
            }
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

pub(crate) fn wolfram_number(text: &str) -> Result<Expr, &'static str> {
    if text.len() > 1_000_000 {
        return Err("数值字面量超出解析资源限制");
    }
    if let Some((base, digits)) = text.split_once("^^") {
        let base = base.parse::<u32>().map_err(|_| "非法数字基数")?;
        if !(2..=36).contains(&base) {
            return Err("非法数字基数");
        }
        return Integer::from_str_radix(digits, base)
            .map(Expr::integer)
            .map_err(|_| "非法基数数字");
    }
    let normalized = text.replace("*^", "e");
    let Some((mantissa, suffix)) = normalized.split_once('`') else {
        let fallback = number(&normalized)?;
        let mantissa = normalized.split(['e', 'E']).next().unwrap_or("");
        let digits = mantissa.replace('.', "");
        let significant = digits.trim_start_matches('0').len().max(1);
        return if normalized.contains(['.', 'e', 'E']) && significant <= 17 {
            machine_literal(&normalized, fallback)
        } else {
            Ok(fallback)
        };
    };
    let (precision, exponent) = suffix.split_once('e').map_or((suffix, ""), |(p, e)| (p, e));
    let literal = if exponent.is_empty() {
        mantissa.to_owned()
    } else {
        format!("{mantissa}e{exponent}")
    };
    let fallback = number(&literal)?;
    if precision.is_empty() {
        return machine_literal(&literal, fallback);
    }
    let precision = precision.parse::<f64>().map_err(|_| "非法精度标记")?;
    let bits = (precision * std::f64::consts::LOG2_10).ceil();
    if !(1.0..=1_000_000.0).contains(&bits) {
        return Err("精度超出解析资源限制");
    }
    let exact = Rational::from_str_decimal(&literal).map_err(|_| "非法十进制数字")?;
    let value: BigFloat = exact.to_float(bits as usize).value();
    Ok(Expr::number(Number::Real(Real::Big(value))))
}

fn machine_literal(literal: &str, fallback: Expr) -> Result<Expr, &'static str> {
    let number = fallback.as_number().ok_or("非法机器数字")?;
    if let Ok(value) = literal.parse::<f64>()
        && value.is_finite()
        && (value != 0.0 || number.is_zero())
    {
        return Ok(Expr::real(value));
    }
    let exact = Rational::from_str_decimal(literal).map_err(|_| "非法十进制数字")?;
    let value: BigFloat = exact.to_float(53).value();
    Ok(Expr::number(Number::Real(Real::Big(value))))
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
