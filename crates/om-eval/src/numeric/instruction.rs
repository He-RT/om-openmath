//! Real stack operations use the host's f64 functions and explicit domain guards.
#[derive(Clone)]
pub(super) enum Instruction {
    Constant(f64),
    Variable(usize),
    Add(usize),
    Mul(usize),
    Pow,
    Unary(fn(f64) -> f64),
    Binary(fn(f64, f64) -> f64),
    Special(fn(f64, &om_core::Interrupt) -> Result<f64, om_analysis::Error>),
    SpecialBinary(fn(f64, f64, &om_core::Interrupt) -> Result<f64, om_analysis::Error>),
}
pub(super) fn special(name: &str) -> Option<Instruction> {
    use om_analysis::special as s;
    Some(match name {
        "Erf" => Instruction::Special(s::erf),
        "Erfc" => Instruction::Special(s::erfc),
        "Gamma" => Instruction::Special(s::gamma),
        "LogGamma" => Instruction::Special(s::log_gamma),
        "Beta" => Instruction::SpecialBinary(s::beta),
        _ => return None,
    })
}
pub(super) fn unary(name: &str) -> Option<fn(f64) -> f64> {
    Some(match name {
        "Sin" => f64::sin,
        "Cos" => f64::cos,
        "Tan" => f64::tan,
        "Cot" => |x| 1.0 / x.tan(),
        "Sec" => |x| 1.0 / x.cos(),
        "Csc" => |x| 1.0 / x.sin(),
        "ArcSin" => f64::asin,
        "ArcCos" => f64::acos,
        "ArcTan" => f64::atan,
        "ArcCot" => |x| (1.0 / x).atan(),
        "ArcSec" => |x| (1.0 / x).acos(),
        "ArcCsc" => |x| (1.0 / x).asin(),
        "Sinh" => f64::sinh,
        "Cosh" => f64::cosh,
        "Tanh" => f64::tanh,
        "Coth" => |x| 1.0 / x.tanh(),
        "Sech" => |x| 1.0 / x.cosh(),
        "Csch" => |x| 1.0 / x.sinh(),
        "ArcSinh" => f64::asinh,
        "ArcCosh" => f64::acosh,
        "ArcTanh" => f64::atanh,
        "ArcCoth" => |x| (1.0 / x).atanh(),
        "ArcSech" => |x| {
            if x > 0. && x <= 1. {
                (1. - x * x).sqrt().ln_1p() - x.ln()
            } else {
                f64::NAN
            }
        },
        "ArcCsch" => |x| {
            if x.abs() >= 1. {
                (1. / x).asinh()
            } else if x == 0. {
                f64::NAN
            } else {
                x.signum() * (x.hypot(1.).ln_1p() - x.abs().ln())
            }
        },
        "Exp" => f64::exp,
        "Log" => f64::ln,
        "Sqrt" => f64::sqrt,
        "CubeRoot" => f64::cbrt,
        "Abs" => f64::abs,
        "Sign" => |x| if x == 0.0 { 0.0 } else { x.signum() },
        "Re" | "Conjugate" => |x| x,
        "Im" => |_| 0.0,
        "Arg" => |x| {
            if x == 0.0 {
                f64::NAN
            } else if x < 0.0 {
                std::f64::consts::PI
            } else {
                0.0
            }
        },
        "Floor" => f64::floor,
        "Ceiling" => f64::ceil,
        "Round" => f64::round_ties_even,
        "Minus" => |x| -x,
        _ => return None,
    })
}
pub(super) fn binary(name: &str) -> Option<fn(f64, f64) -> f64> {
    Some(match name {
        "Log" => |base, x| {
            if base <= 0.0 || base == 1.0 || x <= 0.0 {
                f64::NAN
            } else {
                x.log(base)
            }
        },
        "ArcTan" => |x, y| {
            if x == 0.0 && y == 0.0 {
                f64::NAN
            } else {
                y.atan2(x)
            }
        },
        "Divide" => |x, y| x / y,
        "Subtract" => |x, y| x - y,
        "Mod" => |x, y| x - y * (x / y).floor(),
        _ => return None,
    })
}
