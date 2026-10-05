//! Bounded exact decimal data conversion; precision tags are not part of JSON/CSV.
use super::*;
use om_num::{BitTest, Integer};
pub(super) fn decimal(text: &str, ctx: &Interrupt) -> Result<Rational, EvalError> {
    if text.len() > 20000 {
        return Err(error("十进制数据超过20000位限制"));
    }
    let (negative, text) = if let Some(s) = text.strip_prefix('-') {
        (true, s)
    } else {
        (false, text.strip_prefix('+').unwrap_or(text))
    };
    let (mantissa, exponent) = if let Some(i) = text.find(['e', 'E']) {
        (
            &text[..i],
            text[i + 1..]
                .parse::<i32>()
                .map_err(|_| error("无效十进制指数"))?,
        )
    } else {
        (text, 0)
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if whole.len() + fraction.len() == 0
        || !whole
            .bytes()
            .chain(fraction.bytes())
            .all(|c| c.is_ascii_digit())
    {
        return Err(error("需要纯十进制数值"));
    }
    for _ in text.bytes() {
        ctx.tick()?;
    }
    let exponent = exponent
        .checked_sub(i32::try_from(fraction.len()).map_err(|_| error("输入过长"))?)
        .filter(|e| e.unsigned_abs() <= 20000)
        .ok_or_else(|| error("十进制指数超过资源界限"))?;
    let mut numerator = format!("{whole}{fraction}")
        .parse::<Integer>()
        .map_err(|_| error("无效整数"))?;
    if negative {
        numerator = -numerator;
    }
    let mut scale = Integer::ONE;
    for _ in 0..exponent.unsigned_abs() {
        ctx.tick()?;
        scale *= 10;
    }
    Ok(if exponent >= 0 {
        Rational::from(numerator * scale)
    } else {
        Rational::from_parts(numerator, scale.into_parts().1)
    })
}
pub(super) fn encode(e: &Expr, ctx: &Interrupt) -> Result<String, EvalError> {
    let q = rational(e)?;
    if q.numerator().bit_len() > 140000 || q.denominator().bit_len() > 70000 {
        return Err(error("数据数值超过20000位资源界限"));
    }
    let mut d = Integer::from(q.denominator().clone());
    let (mut twos, mut fives) = (0usize, 0usize);
    for (factor, count) in [(2u32, &mut twos), (5u32, &mut fives)] {
        while &d % factor == 0 {
            ctx.tick()?;
            d /= factor;
            *count += 1;
            if *count > 20000 {
                return Err(error("数据小数位超过20000位限制"));
            }
        }
    }
    if d != Integer::ONE {
        return Err(error(
            "纯数据不能精确表示非终止十进制有理数；请先明确请求数值近似",
        ));
    }
    let places = twos.max(fives);
    let negative = q.numerator() < &Integer::ZERO;
    let mut n = if negative {
        -q.numerator()
    } else {
        q.numerator().clone()
    };
    for (factor, count) in [(2, places - twos), (5, places - fives)] {
        for _ in 0..count {
            ctx.tick()?;
            n *= factor;
        }
    }
    let mut text = n.to_string();
    let mut exponent = -(places as i32);
    while text.len() > 1 && text.ends_with('0') {
        ctx.tick()?;
        text.pop();
        exponent += 1;
    }
    if text.len().saturating_add(exponent.to_string().len() + 2) > 20000 {
        return Err(error("数据十进制有效位超过20000位限制"));
    }
    if exponent.unsigned_abs() > 20000 {
        return Err(error("数据十进制指数超过资源界限"));
    }
    if exponent != 0 && (exponent.unsigned_abs() > 128 || text.len() > 128) {
        if negative {
            text.insert(0, '-');
        }
        return Ok(format!("{text}e{exponent}"));
    }
    if exponent > 0 {
        text.push_str(&"0".repeat(exponent as usize));
    }
    let places = if exponent < 0 {
        (-exponent) as usize
    } else {
        0
    };
    if places > 0 {
        if text.len() <= places {
            text = format!("{}{}", "0".repeat(places + 1 - text.len()), text);
        }
        text.insert(text.len() - places, '.');
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    if negative {
        text.insert(0, '-');
    }
    Ok(text)
}
