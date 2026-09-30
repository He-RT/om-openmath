//! Precedence-aware mathematical source with explicit fallbacks for calls.

use om_core::{BUILTIN as B, Expr, ExprKind, Symbol};
use om_num::{Number, Rational, Real};

pub(crate) fn format(e: &Expr, modern: bool, options: &crate::FormatOptions) -> String {
    Printer {
        modern,
        display_order: options.display_order,
    }
    .emit(e)
    .0
}

struct Printer {
    modern: bool,
    display_order: bool,
}

impl Printer {
    fn wrapped(&self, e: &Expr, min: u8) -> String {
        let (s, bp) = self.emit(e);
        if bp < min { format!("({s})") } else { s }
    }
    fn emit(&self, e: &Expr) -> (String, u8) {
        match e.kind() {
            ExprKind::Number(n) => self.number(n),
            ExprKind::Symbol(s) => (s.name().into(), 130),
            ExprKind::String(s) => (crate::text::quote(s), 130),
            ExprKind::Normal(n) => {
                let args = &n.args;
                let head = n.head.as_symbol();
                if head == Some(B::PLUS) && args.len() > 1 {
                    return self.sum(args);
                }
                if head == Some(B::TIMES) && args.len() > 1 {
                    return self.product(args);
                }
                if head == Some(B::POWER) && args.len() == 2 {
                    return (
                        format!(
                            "{}^{}",
                            self.wrapped(&args[0], 101),
                            self.wrapped(&args[1], 100)
                        ),
                        100,
                    );
                }
                if head == Some(B::LIST) {
                    let contents = self.args(args);
                    return (
                        if self.modern {
                            format!("[{contents}]")
                        } else {
                            format!("{{{contents}}}")
                        },
                        130,
                    );
                }
                if head == Some(B::DIRECTED_INFINITY) && args.is_empty() {
                    return ("ComplexInfinity".into(), 130);
                }
                if let Some(head) = head
                    && let Some((op, bp)) = self.operator(head)
                {
                    let valid = if matches!(
                        head,
                        B::AND
                            | B::OR
                            | B::EQUAL
                            | B::UNEQUAL
                            | B::LESS
                            | B::LESS_EQUAL
                            | B::GREATER
                            | B::GREATER_EQUAL
                    ) {
                        args.len() >= 2
                    } else {
                        args.len() == 2
                    };
                    if valid && !(self.modern && bp == 60 && args.len() > 2) {
                        let mut parts = Vec::new();
                        for (i, arg) in args.iter().enumerate() {
                            let min = if head == B::RULE || head == B::RULE_DELAYED {
                                if i == 0 { bp + 1 } else { bp }
                            } else {
                                bp + 1
                            };
                            parts.push(self.wrapped(arg, min));
                        }
                        return (parts.join(op), bp);
                    }
                }
                self.call(&n.head, args)
            }
        }
    }
    fn args(&self, args: &[Expr]) -> String {
        args.iter()
            .map(|e| self.emit(e).0)
            .collect::<Vec<_>>()
            .join(", ")
    }
    fn call(&self, head: &Expr, args: &[Expr]) -> (String, u8) {
        let name = if self.modern {
            head.as_symbol().map_or_else(
                || match head.kind() {
                    ExprKind::Normal(n) => self.call(&n.head, &n.args).0,
                    _ => self.emit(head).0,
                },
                |s| modern_name(s).into(),
            )
        } else {
            self.wrapped(head, 120)
        };
        let contents = self.args(args);
        (
            if self.modern {
                format!("{name}({contents})")
            } else {
                format!("{name}[{contents}]")
            },
            120,
        )
    }
    fn sum(&self, args: &[Expr]) -> (String, u8) {
        let mut args: Vec<_> = args.iter().collect();
        if self.display_order {
            args.sort_by_key(|e| std::cmp::Reverse(degree(e)));
        }
        let mut s = String::new();
        for (i, e) in args.into_iter().enumerate() {
            let negative = positive(e);
            let term = self.wrapped(negative.as_ref().unwrap_or(e), 61);
            if i == 0 {
                if negative.is_some() {
                    s.push('-');
                }
            } else {
                s.push_str(if negative.is_some() { " - " } else { " + " });
            }
            s.push_str(&term);
        }
        (s, 60)
    }
    fn product(&self, args: &[Expr]) -> (String, u8) {
        let mut numerator = vec![];
        let mut denominator = vec![];
        let mut negative = false;
        for e in args {
            if let Some(Number::Rational(q)) = e.as_number() {
                negative ^= q < &Rational::ZERO;
                let n = q.numerator().clone().into_parts().1;
                if !n.is_one() {
                    numerator.push(Expr::integer(n.into()));
                }
                denominator.push(Expr::integer(q.denominator().clone().into()));
            } else if let Some(pos) = positive(e).filter(|_| e.as_number().is_some()) {
                negative = !negative;
                if !matches!(pos.as_number(), Some(Number::Integer(n)) if n.is_one()) {
                    numerator.push(pos);
                }
            } else if e.is_head(B::POWER) && e.args().len() == 2 && positive(&e.args()[1]).is_some()
            {
                let exponent =
                    positive(&e.args()[1]).expect("invariant: negative exponent checked");
                let base = e.args()[0].clone();
                denominator.push(if exponent == Expr::int(1) {
                    base
                } else {
                    Expr::call(B::POWER, [base, exponent])
                });
            } else {
                numerator.push(e.clone());
            }
        }
        let mut s = self.factors(&numerator);
        if negative {
            s.insert(0, '-');
        }
        if !denominator.is_empty() {
            let d = self.factors(&denominator);
            let wrap = denominator.len() > 1;
            s.push('/');
            s.push_str(&if wrap { format!("({d})") } else { d });
        }
        (
            s,
            if negative && numerator.len() <= 1 && denominator.is_empty() {
                90
            } else {
                80
            },
        )
    }
    fn factors(&self, factors: &[Expr]) -> String {
        if factors.is_empty() {
            return "1".into();
        }
        let mut s = String::new();
        for (i, e) in factors.iter().enumerate() {
            if i > 0
                && !(self.modern
                    && i == 1
                    && factors[0].as_number().is_some()
                    && implicit_target(e))
            {
                s.push('*');
            }
            s.push_str(&self.wrapped(e, 81));
        }
        s
    }
    fn number(&self, n: &Number) -> (String, u8) {
        match n {
            Number::Integer(n) => (
                n.to_string(),
                if n < &om_num::Integer::ZERO { 90 } else { 130 },
            ),
            Number::Rational(q) => (format!("{}/{}", q.numerator(), q.denominator()), 80),
            Number::Real(r) => {
                let (decimal, marker, negative) = match r {
                    Real::Machine(x) => {
                        let decimal = if *x != 0.0 && (x.abs() < 0.0001 || x.abs() >= 1e16) {
                            format!("{x:e}")
                        } else {
                            x.to_string()
                        };
                        let mantissa = decimal.split('e').next().unwrap_or("");
                        let digits = mantissa
                            .chars()
                            .filter(char::is_ascii_digit)
                            .collect::<String>();
                        let marker = if digits.trim_start_matches('0').len() > 16 {
                            "`".into()
                        } else {
                            String::new()
                        };
                        (decimal, marker, x.is_sign_negative())
                    }
                    Real::Big(x) => {
                        let decimal = x
                            .clone()
                            .with_precision(x.precision() + 16)
                            .value()
                            .to_decimal()
                            .value()
                            .to_string();
                        let precision = (x.precision() as f64 - 0.25) / std::f64::consts::LOG2_10;
                        (
                            decimal,
                            format!("`{precision}"),
                            x.repr().significand() < &om_num::Integer::ZERO,
                        )
                    }
                };
                let (m, exponent) = decimal
                    .split_once('e')
                    .map_or((decimal.as_str(), None), |(m, e)| (m, Some(e)));
                let mut s = m.to_owned();
                if !s.contains('.') {
                    s.push('.');
                }
                s.push_str(&marker);
                if let Some(exponent) = exponent {
                    s.push_str(if self.modern { "e" } else { "*^" });
                    s.push_str(exponent);
                }
                (s, if negative { 90 } else { 130 })
            }
            Number::Complex(c) => {
                let re = self.number(&c.re).0;
                let im = self.number(&c.im).0;
                (format!("{re} + ({im})*I"), 60)
            }
        }
    }
    fn operator(&self, s: Symbol) -> Option<(&'static str, u8)> {
        Some(match s {
            B::RULE => (" -> ", 20),
            B::RULE_DELAYED if !self.modern => (" :> ", 20),
            B::REPLACE_ALL => (" /. ", 10),
            B::AND => (" && ", 40),
            B::OR => (" || ", 30),
            B::EQUAL => (if self.modern { " = " } else { " == " }, 60),
            B::UNEQUAL => (" != ", 60),

            B::LESS => (" < ", 60),
            B::LESS_EQUAL => (" <= ", 60),
            B::GREATER => (" > ", 60),
            B::GREATER_EQUAL => (" >= ", 60),
            _ => return None,
        })
    }
}

pub(crate) fn positive(e: &Expr) -> Option<Expr> {
    if let Some(n) = e.as_number() {
        let negative = match n {
            Number::Integer(n) => n < &om_num::Integer::ZERO,
            Number::Rational(q) => q < &Rational::ZERO,
            Number::Real(Real::Machine(x)) => *x < 0.0,
            Number::Real(Real::Big(x)) => x.repr().significand() < &om_num::Integer::ZERO,
            _ => false,
        };
        return negative.then(|| Expr::number(n.neg()));
    }
    if e.is_head(B::TIMES)
        && let Some(first) = e.args().first()
        && let Some(first) = positive(first)
    {
        let args = std::iter::once(first).chain(e.args()[1..].iter().cloned());
        return Some(om_core::mul(args));
    }
    None
}
pub(crate) fn degree(e: &Expr) -> i64 {
    if e.as_number().is_some() {
        0
    } else if e.is_head(B::POWER) && e.args().len() == 2 {
        e.args()[1]
            .as_number()
            .and_then(|n| n.to_f64())
            .filter(|x| x.is_finite() && x.fract() == 0.0)
            .map_or(1, |n| n as i64)
    } else if e.is_head(B::TIMES) {
        e.args().iter().map(degree).fold(0i64, i64::saturating_add)
    } else {
        1
    }
}
fn implicit_target(e: &Expr) -> bool {
    e.as_symbol().is_some()
        || (e.is_head(B::POWER) && e.args().len() == 2 && e.args()[0].as_symbol().is_some())
}
pub(crate) fn modern_name(s: Symbol) -> &'static str {
    match s {
        B::SIN => "sin",
        B::COS => "cos",
        B::TAN => "tan",
        B::LOG => "log",
        B::ARCSIN => "asin",
        B::ARCCOS => "acos",
        B::ARCTAN => "atan",
        B::SQRT => "sqrt",
        B::EXP => "exp",
        B::ABS => "abs",
        B::SOLVE => "solve",
        B::NSOLVE => "nsolve",
        B::FIND_ROOT => "findroot",
        B::REDUCE => "reduce",
        B::ELIMINATE => "eliminate",
        B::FUNCTION => "function",
        B::SLOT => "slot",
        B::PART => "part",
        _ => s.name(),
    }
}
