//! Bounded unit text grammar and exact SI dimension/scale definitions. No source evaluation.
use super::*;
use om_num::Integer;
#[derive(Clone)]
pub(super) struct Unit {
    pub dims: [i16; 7],
    pub scale: Rational,
}
impl Unit {
    fn one() -> Self {
        Self {
            dims: [0; 7],
            scale: Rational::ONE,
        }
    }
    pub fn combine(&self, b: &Self, divide: bool) -> Result<Self, EvalError> {
        let mut dims = [0; 7];
        for (i, d) in dims.iter_mut().enumerate() {
            let value =
                i32::from(self.dims[i]) + i32::from(b.dims[i]) * if divide { -1 } else { 1 };
            if value.abs() > 128 {
                return Err(error("单位量纲指数超过128界限"));
            }
            *d = value as i16;
        }
        Ok(Self {
            dims,
            scale: if divide {
                &self.scale / &b.scale
            } else {
                &self.scale * &b.scale
            },
        })
    }
    pub fn power(&self, n: i32, ctx: &Interrupt) -> Result<Self, EvalError> {
        if n.unsigned_abs() > 32 {
            return Err(error("单位乘方首版限-32..32整数"));
        }
        let mut dims = [0; 7];
        for (i, d) in dims.iter_mut().enumerate() {
            let value = i32::from(self.dims[i]) * n;
            if value.abs() > 128 {
                return Err(error("单位量纲指数超过128界限"));
            }
            *d = value as i16;
        }
        let mut scale = Rational::ONE;
        for _ in 0..n.unsigned_abs() {
            ctx.tick()?;
            scale *= &self.scale;
        }
        if n < 0 {
            scale = Rational::ONE / scale;
        }
        Ok(Self { dims, scale })
    }
    pub fn base_text(&self) -> String {
        let names = ["m", "kg", "s", "A", "K", "mol", "cd"];
        let terms: Vec<_> = self
            .dims
            .iter()
            .enumerate()
            .filter(|(_, n)| **n != 0)
            .map(|(i, n)| {
                if *n == 1 {
                    names[i].to_owned()
                } else {
                    format!("{}^{n}", names[i])
                }
            })
            .collect();
        if terms.is_empty() {
            "1".into()
        } else {
            terms.join("*")
        }
    }
}
fn factor10(n: i32) -> Rational {
    let mut integer = Integer::ONE;
    for _ in 0..n.unsigned_abs() {
        integer *= 10;
    }
    if n >= 0 {
        Rational::from(integer)
    } else {
        Rational::from_parts(Integer::ONE, integer.into_parts().1)
    }
}
fn defined(name: &str) -> Option<(Unit, bool)> {
    let (dims, scale, prefix) = match name {
        "1" | "rad" | "sr" => ([0; 7], Rational::ONE, true),
        "m" => ([1, 0, 0, 0, 0, 0, 0], Rational::ONE, true),
        "kg" => ([0, 1, 0, 0, 0, 0, 0], Rational::ONE, false),
        "g" => ([0, 1, 0, 0, 0, 0, 0], factor10(-3), true),
        "s" => ([0, 0, 1, 0, 0, 0, 0], Rational::ONE, true),
        "A" => ([0, 0, 0, 1, 0, 0, 0], Rational::ONE, true),
        "K" => ([0, 0, 0, 0, 1, 0, 0], Rational::ONE, true),
        "mol" => ([0, 0, 0, 0, 0, 1, 0], Rational::ONE, true),
        "cd" => ([0, 0, 0, 0, 0, 0, 1], Rational::ONE, true),
        "Hz" | "Bq" => ([0, 0, -1, 0, 0, 0, 0], Rational::ONE, true),
        "N" => ([1, 1, -2, 0, 0, 0, 0], Rational::ONE, true),
        "Pa" => ([-1, 1, -2, 0, 0, 0, 0], Rational::ONE, true),
        "J" => ([2, 1, -2, 0, 0, 0, 0], Rational::ONE, true),
        "W" => ([2, 1, -3, 0, 0, 0, 0], Rational::ONE, true),
        "C" => ([0, 0, 1, 1, 0, 0, 0], Rational::ONE, true),
        "V" => ([2, 1, -3, -1, 0, 0, 0], Rational::ONE, true),
        "F" => ([-2, -1, 4, 2, 0, 0, 0], Rational::ONE, true),
        "ohm" | "Ω" => ([2, 1, -3, -2, 0, 0, 0], Rational::ONE, true),
        "S" => ([-2, -1, 3, 2, 0, 0, 0], Rational::ONE, true),
        "Wb" => ([2, 1, -2, -1, 0, 0, 0], Rational::ONE, true),
        "T" => ([0, 1, -2, -1, 0, 0, 0], Rational::ONE, true),
        "H" => ([2, 1, -2, -2, 0, 0, 0], Rational::ONE, true),
        "lm" => ([0, 0, 0, 0, 0, 0, 1], Rational::ONE, true),
        "lx" => ([-2, 0, 0, 0, 0, 0, 1], Rational::ONE, true),
        "Gy" | "Sv" => ([2, 0, -2, 0, 0, 0, 0], Rational::ONE, true),
        "kat" => ([0, 0, -1, 0, 0, 1, 0], Rational::ONE, true),
        "L" | "l" => ([3, 0, 0, 0, 0, 0, 0], factor10(-3), true),
        "t" => ([0, 1, 0, 0, 0, 0, 0], factor10(3), true),
        "min" => ([0, 0, 1, 0, 0, 0, 0], Rational::from(60), false),
        "h" => ([0, 0, 1, 0, 0, 0, 0], Rational::from(3600), false),
        "d" => ([0, 0, 1, 0, 0, 0, 0], Rational::from(86400), false),
        "in" => (
            [1, 0, 0, 0, 0, 0, 0],
            Rational::from_parts(127.into(), 5000u32.into()),
            false,
        ),
        "ft" => (
            [1, 0, 0, 0, 0, 0, 0],
            Rational::from_parts(381.into(), 1250u32.into()),
            false,
        ),
        "yd" => (
            [1, 0, 0, 0, 0, 0, 0],
            Rational::from_parts(1143.into(), 1250u32.into()),
            false,
        ),
        "mi" => (
            [1, 0, 0, 0, 0, 0, 0],
            Rational::from_parts(201168.into(), 125u32.into()),
            false,
        ),
        _ => return None,
    };
    Some((Unit { dims, scale }, prefix))
}
fn named(name: &str) -> Result<Unit, EvalError> {
    if let Some((unit, _)) = defined(name) {
        return Ok(unit);
    }
    for (prefix, n) in [
        ("da", 1),
        ("Q", 30),
        ("R", 27),
        ("Y", 24),
        ("Z", 21),
        ("E", 18),
        ("P", 15),
        ("T", 12),
        ("G", 9),
        ("M", 6),
        ("k", 3),
        ("h", 2),
        ("d", -1),
        ("c", -2),
        ("m", -3),
        ("μ", -6),
        ("µ", -6),
        ("u", -6),
        ("n", -9),
        ("p", -12),
        ("f", -15),
        ("a", -18),
        ("z", -21),
        ("y", -24),
        ("r", -27),
        ("q", -30),
    ] {
        if let Some(base) = name.strip_prefix(prefix)
            && let Some((mut unit, true)) = defined(base)
        {
            unit.scale *= factor10(n);
            return Ok(unit);
        }
    }
    Err(error("未知单位、复合前缀、仿射温标或货币不在首版支持范围"))
}
#[derive(Clone)]
enum Token {
    Name(String),
    Number(i32),
    Mul,
    Div,
    Power,
    Open,
    Close,
}
struct Parser<'a> {
    tokens: Vec<Token>,
    at: usize,
    ctx: &'a Interrupt,
}
impl Parser<'_> {
    fn product(&mut self, depth: usize) -> Result<Unit, EvalError> {
        if depth > 16 {
            return Err(error("单位括号嵌套超过16层"));
        }
        let mut value = self.factor(depth)?;
        loop {
            let divide = match self.tokens.get(self.at) {
                Some(Token::Div) => {
                    self.at += 1;
                    true
                }
                Some(Token::Mul) => {
                    self.at += 1;
                    false
                }
                Some(Token::Name(_) | Token::Open) => false,
                _ => break,
            };
            self.ctx.tick()?;
            value = value.combine(&self.factor(depth)?, divide)?;
        }
        Ok(value)
    }
    fn factor(&mut self, depth: usize) -> Result<Unit, EvalError> {
        self.ctx.tick()?;
        let mut value = match self.tokens.get(self.at).cloned() {
            Some(Token::Name(name)) => {
                self.at += 1;
                named(&name)?
            }
            Some(Token::Number(1)) => {
                self.at += 1;
                Unit::one()
            }
            Some(Token::Open) => {
                self.at += 1;
                let value = self.product(depth + 1)?;
                if !matches!(self.tokens.get(self.at), Some(Token::Close)) {
                    return Err(error("单位括号不完整"));
                }
                self.at += 1;
                value
            }
            _ => return Err(error("单位需要名称、1或括号")),
        };
        if matches!(self.tokens.get(self.at), Some(Token::Power)) {
            self.at += 1;
            let Some(Token::Number(n)) = self.tokens.get(self.at).cloned() else {
                return Err(error("单位指数须为整数"));
            };
            self.at += 1;
            value = value.power(n, self.ctx)?;
        }
        Ok(value)
    }
}
pub(super) fn parse(text: &str, ctx: &Interrupt) -> Result<Unit, EvalError> {
    if text.is_empty() || text.len() > 256 {
        return Err(error("单位文字需要1..256字节"));
    }
    let mut chars = text.chars().peekable();
    let mut tokens = vec![];
    while let Some(c) = chars.next() {
        ctx.tick()?;
        if c.is_whitespace() {
            continue;
        }
        let token = match c {
            '*' | '·' => Token::Mul,
            '/' => Token::Div,
            '^' => Token::Power,
            '(' => Token::Open,
            ')' => Token::Close,
            c if c.is_alphabetic() => {
                let mut name = c.to_string();
                while chars.peek().is_some_and(|c| c.is_alphabetic()) {
                    ctx.tick()?;
                    name.push(chars.next().ok_or_else(|| error("单位文字扫描失败"))?);
                }
                Token::Name(name)
            }
            c if c.is_ascii_digit() || matches!(c, '+' | '-') => {
                let mut text = c.to_string();
                while chars.peek().is_some_and(|c| c.is_ascii_digit()) {
                    ctx.tick()?;
                    text.push(chars.next().ok_or_else(|| error("单位指数扫描失败"))?);
                }
                Token::Number(text.parse().map_err(|_| error("单位整数溢出或无效"))?)
            }
            _ => return Err(error("单位文字含不支持的字符；仅接受单位语法，不执行代码")),
        };
        tokens.push(token);
        if tokens.len() > 64 {
            return Err(error("单位词元超过64项界限"));
        }
    }
    let mut parser = Parser { tokens, at: 0, ctx };
    let value = parser.product(0)?;
    if parser.at != parser.tokens.len() {
        return Err(error("单位文字末尾含无效结构"));
    }
    Ok(value)
}
