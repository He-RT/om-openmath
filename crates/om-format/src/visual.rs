//! Presentation syntax is distinct from lossless source syntax.

use crate::source::{degree, modern_name, positive};
use om_core::{BUILTIN as B, Expr, ExprKind, Symbol};
use om_num::{Number, Rational, Real};

pub(crate) fn format(e: &Expr, tex: bool) -> String {
    Printer { tex }.emit(e).0
}
struct Printer {
    tex: bool,
}
impl Printer {
    fn choose<'a>(&self, tex: &'a str, unicode: &'a str) -> &'a str {
        if self.tex { tex } else { unicode }
    }
    fn group(&self, s: &str) -> String {
        if self.tex {
            format!("\\left({s}\\right)")
        } else {
            format!("({s})")
        }
    }
    fn wrapped(&self, e: &Expr, min: u8) -> String {
        let (s, bp) = self.emit(e);
        if bp < min { self.group(&s) } else { s }
    }
    fn emit(&self, e: &Expr) -> (String, u8) {
        match e.kind() {
            ExprKind::Number(n) => self.number(n),
            ExprKind::Symbol(s) => (self.symbol(*s), 130),
            ExprKind::String(s) => (
                if self.tex {
                    format!("\\text{{{}}}", escape(s))
                } else {
                    crate::text::quote(s)
                },
                130,
            ),
            ExprKind::Normal(n) => {
                let args = &n.args;
                match n.head.as_symbol() {
                    Some(B::HOLD_FORM) if args.len() == 1 => self.emit(&args[0]),
                    Some(B::PLUS) if args.len() > 1 => self.sum(args),
                    Some(B::TIMES) if args.len() > 1 => self.product(args),
                    Some(B::POWER) if args.len() == 2 => self.power(&args[0], &args[1]),
                    Some(B::LIST) => (
                        if self.tex {
                            format!("\\left[{}\\right]", self.args(args))
                        } else {
                            format!("[{}]", self.args(args))
                        },
                        130,
                    ),
                    Some(B::FACTORIAL) if args.len() == 1 => {
                        (format!("{}!", self.wrapped(&args[0], 110)), 110)
                    }
                    Some(B::ABS) if args.len() == 1 => (
                        if self.tex {
                            format!("\\left|{}\\right|", self.emit(&args[0]).0)
                        } else {
                            format!("|{}|", self.emit(&args[0]).0)
                        },
                        130,
                    ),
                    Some(B::PART) if args.len() >= 2 => {
                        let base = self.wrapped(&args[0], 120);
                        let indices = self.args(&args[1..]);
                        (
                            if self.tex {
                                format!("{base}_{{{indices}}}")
                            } else {
                                format!("{base}[{indices}]")
                            },
                            120,
                        )
                    }
                    Some(B::DIRECTED_INFINITY) if args.is_empty() => {
                        (self.choose("\\tilde{\\infty}", "∞̃").into(), 130)
                    }
                    Some(B::DIRECTED_INFINITY) if args.len() == 1 && args[0] == Expr::int(1) => {
                        (self.choose("\\infty", "∞").into(), 130)
                    }
                    Some(B::DIRECTED_INFINITY) if args.len() == 1 && args[0] == Expr::int(-1) => {
                        (self.choose("-\\infty", "−∞").into(), 90)
                    }
                    Some(B::NOT) if args.len() == 1 => (
                        format!(
                            "{}{}",
                            self.choose("\\neg ", "¬"),
                            self.wrapped(&args[0], 51)
                        ),
                        40,
                    ),
                    Some(head) if args.len() >= 2 && self.operator(head).is_some() => {
                        let (op, bp) = self.operator(head).expect("invariant: operator checked");
                        (
                            args.iter()
                                .map(|e| self.wrapped(e, bp + 1))
                                .collect::<Vec<_>>()
                                .join(op),
                            bp,
                        )
                    }
                    _ => {
                        let head = if let Some(s) = n.head.as_symbol() {
                            self.function(s)
                        } else {
                            self.wrapped(&n.head, 120)
                        };
                        (format!("{head}{}", self.group(&self.args(args))), 120)
                    }
                }
            }
        }
    }
    fn args(&self, args: &[Expr]) -> String {
        args.iter()
            .map(|e| self.emit(e).0)
            .collect::<Vec<_>>()
            .join(", ")
    }
    fn sum(&self, args: &[Expr]) -> (String, u8) {
        let mut args: Vec<_> = args.iter().collect();
        args.sort_by_key(|e| std::cmp::Reverse(degree(e)));
        let mut out = String::new();
        for (i, e) in args.into_iter().enumerate() {
            let pos = positive(e);
            if i == 0 {
                if pos.is_some() {
                    out.push_str(self.choose("-", "−"));
                }
            } else {
                out.push_str(if pos.is_some() {
                    self.choose(" - ", " − ")
                } else {
                    " + "
                });
            }
            out.push_str(&self.wrapped(pos.as_ref().unwrap_or(e), 61));
        }
        (out, 60)
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
                if pos != Expr::int(1) {
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
        let (mut s, bp) = if denominator.is_empty() {
            (self.factors(&numerator), 80)
        } else {
            self.fraction(&numerator, &denominator)
        };
        if negative {
            s.insert_str(0, self.choose("-", "−"));
        }
        (s, bp)
    }
    fn factors(&self, args: &[Expr]) -> String {
        if args.is_empty() {
            return "1".into();
        }
        let mut out = String::new();
        for (i, e) in args.iter().enumerate() {
            if i > 0 {
                if self.tex {
                    out.push_str(if e.as_number().is_some() {
                        " \\cdot "
                    } else {
                        " "
                    });
                } else if !(i == 1
                    && args[0].as_number().is_some()
                    && (e.as_symbol().is_some() || e.is_head(B::POWER)))
                {
                    out.push('·');
                }
            }
            out.push_str(&self.wrapped(e, 81));
        }
        out
    }
    fn fraction(&self, numerator: &[Expr], denominator: &[Expr]) -> (String, u8) {
        if self.tex {
            let plain = |args: &[Expr]| {
                if args.len() == 1 {
                    self.emit(&args[0]).0
                } else {
                    self.factors(args)
                }
            };
            (
                format!("\\frac{{{}}}{{{}}}", plain(numerator), plain(denominator)),
                130,
            )
        } else {
            let mut d = self.factors(denominator);
            if denominator.len() > 1 {
                d = self.group(&d);
            }
            (format!("{}/{d}", self.factors(numerator)), 80)
        }
    }
    fn power(&self, base: &Expr, exponent: &Expr) -> (String, u8) {
        if let Some(pos) = positive(exponent) {
            return self.fraction(
                &[],
                &[if pos == Expr::int(1) {
                    base.clone()
                } else {
                    Expr::call(B::POWER, [base.clone(), pos])
                }],
            );
        }
        if *exponent == Expr::rational(1, 2) || *exponent == Expr::rational(1, 3) {
            let square = *exponent == Expr::rational(1, 2);
            if self.tex {
                return (
                    format!(
                        "\\sqrt{}{{{}}}",
                        if square { "" } else { "[3]" },
                        self.emit(base).0
                    ),
                    130,
                );
            }
            return (
                format!(
                    "{}{}",
                    if square { "√" } else { "∛" },
                    self.wrapped(base, 130)
                ),
                100,
            );
        }
        let b = self.wrapped(base, 101);
        if self.tex {
            (format!("{b}^{{{}}}", self.emit(exponent).0), 100)
        } else if let Some(Number::Integer(n)) = exponent.as_number() {
            (format!("{b}{}", superscript(&n.to_string())), 100)
        } else {
            (format!("{b}^{}", self.wrapped(exponent, 100)), 100)
        }
    }
    fn number(&self, n: &Number) -> (String, u8) {
        match n {
            Number::Integer(n) => (
                n.to_string().replace('-', self.choose("-", "−")),
                if n < &om_num::Integer::ZERO { 90 } else { 130 },
            ),
            Number::Rational(q) => {
                let negative = q < &Rational::ZERO;
                let a = q.numerator().clone().into_parts().1;
                let d = q.denominator();
                let sign = if negative {
                    self.choose("-", "−")
                } else {
                    ""
                };
                (
                    if self.tex {
                        format!("{sign}\\frac{{{a}}}{{{d}}}")
                    } else {
                        format!("{sign}{a}/{d}")
                    },
                    if self.tex {
                        if negative { 90 } else { 130 }
                    } else {
                        80
                    },
                )
            }
            Number::Real(r) => {
                let decimal = match r {
                    Real::Machine(x) => {
                        if *x != 0. && (x.abs() < 0.0001 || x.abs() >= 1e16) {
                            format!("{x:e}")
                        } else {
                            x.to_string()
                        }
                    }
                    Real::Big(x) => x.to_decimal().value().to_string(),
                };
                let negative = decimal.starts_with('-');
                let s = if let Some((m, e)) = decimal.split_once('e') {
                    if self.tex {
                        format!("{m} \\times 10^{{{e}}}")
                    } else {
                        format!("{}×10{}", m.replace('-', "−"), superscript(e))
                    }
                } else {
                    decimal.replace('-', self.choose("-", "−"))
                };
                (
                    s,
                    if decimal.contains('e') {
                        80
                    } else if negative {
                        90
                    } else {
                        130
                    },
                )
            }
            Number::Complex(c) => {
                let i = self.choose("\\mathrm{i}", "i");
                let neg = positive(&Expr::number(c.im.clone()));
                let im = neg.as_ref().and_then(Expr::as_number).unwrap_or(&c.im);
                let imaginary = if im.is_one() {
                    i.to_owned()
                } else {
                    format!("{}{i}", self.wrapped(&Expr::number(im.clone()), 81))
                };
                if c.re.is_zero() {
                    (
                        format!(
                            "{}{imaginary}",
                            if neg.is_some() {
                                self.choose("-", "−")
                            } else {
                                ""
                            }
                        ),
                        80,
                    )
                } else {
                    (
                        format!(
                            "{}{}{imaginary}",
                            self.number(&c.re).0,
                            if neg.is_some() {
                                self.choose(" - ", " − ")
                            } else {
                                " + "
                            }
                        ),
                        60,
                    )
                }
            }
        }
    }
    fn symbol(&self, s: Symbol) -> String {
        match s {
            B::PI => self.choose("\\pi", "π").into(),
            B::E => "e".into(),
            B::I => self.choose("\\mathrm{i}", "i").into(),
            B::INFINITY => self.choose("\\infty", "∞").into(),
            _ if !self.tex => s.name().into(),
            _ => greek(s.name()).map_or_else(
                || {
                    if s.name().chars().count() == 1 && s.name().chars().all(char::is_alphanumeric)
                    {
                        s.name().into()
                    } else {
                        format!("\\mathrm{{{}}}", escape(s.name()))
                    }
                },
                str::to_owned,
            ),
        }
    }
    fn function(&self, s: Symbol) -> String {
        if !self.tex {
            return modern_name(s).into();
        }
        match s {
            B::SIN => "\\sin".into(),
            B::COS => "\\cos".into(),
            B::TAN => "\\tan".into(),
            B::COT => "\\cot".into(),
            B::SEC => "\\sec".into(),
            B::CSC => "\\csc".into(),
            B::LOG => "\\log".into(),
            B::EXP => "\\exp".into(),
            B::SINH => "\\sinh".into(),
            B::COSH => "\\cosh".into(),
            B::TANH => "\\tanh".into(),
            _ => format!("\\operatorname{{{}}}", escape(s.name())),
        }
    }
    fn operator(&self, s: Symbol) -> Option<(&str, u8)> {
        Some(match s {
            B::EQUAL => (" = ", 50),
            B::UNEQUAL => (self.choose(" \\ne ", " ≠ "), 50),
            B::LESS => (" < ", 50),
            B::LESS_EQUAL => (self.choose(" \\le ", " ≤ "), 50),
            B::GREATER => (" > ", 50),
            B::GREATER_EQUAL => (self.choose(" \\ge ", " ≥ "), 50),
            B::AND => (self.choose(" \\land ", " ∧ "), 30),
            B::OR => (self.choose(" \\lor ", " ∨ "), 20),
            B::RULE => (self.choose(" \\to ", " → "), 10),
            _ => return None,
        })
    }
}
fn superscript(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '0' => '⁰',
            '1' => '¹',
            '2' => '²',
            '3' => '³',
            '4' => '⁴',
            '5' => '⁵',
            '6' => '⁶',
            '7' => '⁷',
            '8' => '⁸',
            '9' => '⁹',
            '-' => '⁻',
            '+' => '⁺',
            _ => c,
        })
        .collect()
}
fn escape(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\textbackslash{}"),
            '_' | '{' | '}' | '%' | '&' | '$' | '#' => {
                out.push('\\');
                out.push(c);
            }
            '^' => out.push_str("\\textasciicircum{}"),
            '~' => out.push_str("\\textasciitilde{}"),
            c if c.is_control() => out.push(' '),
            _ => out.push(c),
        }
    }
    out
}
fn greek(name: &str) -> Option<&'static str> {
    Some(match name {
        "α" => "\\alpha",
        "β" => "\\beta",
        "γ" => "\\gamma",
        "δ" => "\\delta",
        "ε" => "\\epsilon",
        "ζ" => "\\zeta",
        "η" => "\\eta",
        "θ" => "\\theta",
        "ι" => "\\iota",
        "κ" => "\\kappa",
        "λ" => "\\lambda",
        "μ" => "\\mu",
        "ν" => "\\nu",
        "ξ" => "\\xi",
        "ο" => "o",
        "π" => "\\pi",
        "ρ" => "\\rho",
        "σ" | "ς" => "\\sigma",
        "τ" => "\\tau",
        "υ" => "\\upsilon",
        "φ" => "\\phi",
        "χ" => "\\chi",
        "ψ" => "\\psi",
        "ω" => "\\omega",
        "Γ" => "\\Gamma",
        "Δ" => "\\Delta",
        "Θ" => "\\Theta",
        "Λ" => "\\Lambda",
        "Ξ" => "\\Xi",
        "Π" => "\\Pi",
        "Σ" => "\\Sigma",
        "Υ" => "\\Upsilon",
        "Φ" => "\\Phi",
        "Ψ" => "\\Psi",
        "Ω" => "\\Omega",
        _ => return None,
    })
}
