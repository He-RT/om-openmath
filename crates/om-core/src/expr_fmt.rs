//! FullForm debug formatting lives here to avoid a dependency on om-format.

use super::{Expr, ExprKind};
use om_num::{Number, Real};
use std::fmt;

impl fmt::Debug for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind() {
            ExprKind::Number(n) => number_fmt(n, f),
            ExprKind::Symbol(s) => f.write_str(s.name()),
            ExprKind::String(s) => write!(f, "{s:?}"),
            ExprKind::Normal(n) => {
                write!(f, "{:?}[", n.head)?;
                for (i, arg) in n.args.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{arg:?}")?;
                }
                f.write_str("]")
            }
        }
    }
}

fn number_fmt(n: &Number, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match n {
        Number::Integer(i) => write!(f, "{i}"),
        Number::Rational(q) => write!(f, "Rational[{}, {}]", q.numerator(), q.denominator()),
        Number::Real(Real::Machine(x)) => {
            let s = x.to_string();
            f.write_str(&s)?;
            if !s.contains('.') {
                f.write_str(".")?;
            }
            Ok(())
        }
        Number::Real(Real::Big(x)) => {
            let decimal = x.to_decimal().value();
            write!(f, "{decimal}`{}", x.precision())
        }
        Number::Complex(c) => {
            f.write_str("Complex[")?;
            number_fmt(&c.re, f)?;
            f.write_str(", ")?;
            number_fmt(&c.im, f)?;
            f.write_str("]")
        }
    }
}
