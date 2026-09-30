//! Exact elementary special values cross-checked with 256-bit balls.
use om_core::{BUILTIN as B, Expr, canonicalize};
use om_num::{Ball, CBall, Integer, Number, Rational};
use om_parse::{Dialect, parse_expr};
use om_simplify::special::eval;
fn e(s: &str) -> Expr {
    canonicalize(&parse_expr(s, Dialect::Wolfram).unwrap())
}
#[test]
fn planned_exact_values_and_principal_branches_match() {
    for (input, output) in [
        ("Sin[Pi/12]", "(Sqrt[6]-Sqrt[2])/4"),
        ("Cos[Pi/12]", "(Sqrt[6]+Sqrt[2])/4"),
        ("Tan[Pi/12]", "2-Sqrt[3]"),
        ("Sin[Pi/6]", "1/2"),
        ("Cos[Pi/3]", "1/2"),
        ("Sin[Pi/4]", "Sqrt[2]/2"),
        ("Cos[Pi/6]", "Sqrt[3]/2"),
        ("Sin[Pi/2]", "1"),
        ("Sin[Pi]", "0"),
        ("Cos[Pi]", "-1"),
        ("Tan[Pi/4]", "1"),
        ("Tan[Pi/3]", "Sqrt[3]"),
        ("Tan[Pi/2]", "ComplexInfinity"),
        ("ArcSin[1/2]", "Pi/6"),
        ("ArcCos[1/2]", "Pi/3"),
        ("ArcSin[Sqrt[2]/2]", "Pi/4"),
        ("ArcSin[Sqrt[3]/2]", "Pi/3"),
        ("ArcSin[-1]", "-Pi/2"),
        ("ArcCos[-1]", "Pi"),
        ("ArcTan[Sqrt[3]]", "Pi/3"),
        ("ArcTan[-1/Sqrt[3]]", "-Pi/6"),
        ("ArcTan[1]", "Pi/4"),
        ("Log[1]", "0"),
        ("Log[E]", "1"),
        ("Log[E^2]", "2"),
        ("Log[E^(-1/2)]", "-1/2"),
        ("Log[-2]", "Log[2]+I*Pi"),
        ("Exp[x]", "E^x"),
        ("Sqrt[x]", "x^(1/2)"),
        ("Abs[3+4*I]", "5"),
        ("Re[3+4*I]", "3"),
        ("Im[3+4*I]", "4"),
        ("Conjugate[3+4*I]", "3-4*I"),
        ("Abs[-x]", "Abs[x]"),
        ("Sin[-x]", "-Sin[x]"),
        ("Cos[-x]", "Cos[x]"),
        ("ProductLog[0]", "0"),
        ("ProductLog[E]", "1"),
        ("ProductLog[-1/E]", "-1"),
    ] {
        let parsed = parse_expr(input, Dialect::Wolfram).unwrap();
        let input = Expr::normal(parsed.head(), parsed.args().iter().map(canonicalize));
        assert_eq!(eval(&input), Some(e(output)), "{input:?}");
    }
    assert!(eval(&e("Sin[Pi/5]")).is_none());
    assert!(eval(&e("Log[E^x]")).is_none());
    assert!(eval(&e("Sin[0.5]")).is_none());
}
fn ball(expr: &Expr) -> CBall {
    fn scalar(n: &Number) -> Rational {
        match n {
            Number::Integer(n) => Rational::from(n.clone()),
            Number::Rational(q) => q.clone(),
            _ => panic!("exact scalar"),
        }
    }
    if let Some(Number::Complex(c)) = expr.as_number() {
        return CBall::exact(&scalar(&c.re), &scalar(&c.im), 256);
    }
    if let Some(n) = expr.as_number() {
        return CBall::exact(&scalar(n), &Rational::ZERO, 256);
    }
    if expr.as_symbol() == Some(B::PI) {
        return CBall {
            re: Ball::pi(256),
            im: Ball::exact(&Rational::ZERO, 256),
        };
    }
    if expr.as_symbol() == Some(B::E) {
        return CBall {
            re: Ball::exact(&Rational::ONE, 256).exp(),
            im: Ball::exact(&Rational::ZERO, 256),
        };
    }
    if expr.is_head(B::PLUS) {
        return expr.args().iter().map(ball).fold(
            CBall::exact(&Rational::ZERO, &Rational::ZERO, 256),
            |a, b| a.add(&b),
        );
    }
    if expr.is_head(B::TIMES) {
        return expr.args().iter().map(ball).fold(
            CBall::exact(&Rational::ONE, &Rational::ZERO, 256),
            |a, b| a.mul(&b),
        );
    }
    if expr.is_head(B::POWER) {
        let b = ball(&expr.args()[0]);
        return if let Some(Number::Integer(n)) = expr.args()[1].as_number() {
            b.pow_int(n)
        } else if expr.args()[1] == Expr::rational(1, 2) {
            b.sqrt()
        } else {
            b.ln().mul(&ball(&expr.args()[1])).exp()
        };
    }
    if expr.is_head(B::LOG) {
        return ball(&expr.args()[0]).ln();
    }
    panic!("unsupported oracle expression {expr:?}")
}
fn close(a: &CBall, b: &CBall) {
    let diff = a.sub(b);
    let tolerance = Rational::from(1) / Rational::from(Integer::from(10).pow(50));
    for part in [&diff.re, &diff.im] {
        assert!(part.contains_zero(), "{a:?} != {b:?}");
        assert!(
            Rational::try_from(part.rad.clone()).unwrap() < tolerance,
            "50-digit enclosure too wide"
        );
    }
}
#[test]
fn every_supported_pi_grid_value_has_a_fifty_digit_numeric_certificate() {
    for denom in [1, 2, 3, 4, 6, 12] {
        for n in -2 * denom..=2 * denom {
            let arg = om_core::mul([Expr::rational(n, denom), Expr::sym(B::PI)]);
            for head in [B::SIN, B::COS, B::TAN] {
                let source = Expr::call(head, [arg.clone()]);
                let exact = eval(&source).unwrap();
                if exact.is_head(B::DIRECTED_INFINITY) {
                    continue;
                }
                let angle = ball(&arg);
                let reference = match head {
                    B::SIN => angle.sin(),
                    B::COS => angle.cos(),
                    _ => angle.sin().div(&angle.cos()),
                };
                close(&reference, &ball(&exact));
            }
        }
    }
}
#[test]
fn inverse_tables_and_log_rules_have_fifty_digit_numeric_certificates() {
    let one = Ball::exact(&Rational::ONE, 256);
    for (head, inputs) in [
        (
            B::ARCSIN,
            vec![
                "0",
                "1/2",
                "-1/2",
                "Sqrt[2]/2",
                "-Sqrt[2]/2",
                "Sqrt[3]/2",
                "-Sqrt[3]/2",
            ],
        ),
        (
            B::ARCCOS,
            vec![
                "0",
                "1/2",
                "-1/2",
                "Sqrt[2]/2",
                "-Sqrt[2]/2",
                "Sqrt[3]/2",
                "-Sqrt[3]/2",
            ],
        ),
        (
            B::ARCTAN,
            vec![
                "0",
                "1",
                "-1",
                "Sqrt[3]",
                "-Sqrt[3]",
                "1/Sqrt[3]",
                "-1/Sqrt[3]",
            ],
        ),
    ] {
        for src in inputs {
            let x = e(src);
            let xb = ball(&x).re;
            let reference = if head == B::ARCTAN {
                xb.atan()
            } else {
                let asin = xb.div(&one.sub(&xb.mul(&xb)).sqrt()).atan();
                if head == B::ARCSIN {
                    asin
                } else {
                    Ball::pi(256).div(&Ball::exact(&2.into(), 256)).sub(&asin)
                }
            };
            close(
                &CBall {
                    re: reference,
                    im: Ball::exact(&Rational::ZERO, 256),
                },
                &ball(&eval(&Expr::call(head, [x])).unwrap()),
            );
        }
    }
    for src in ["1", "E", "E^2", "E^(-1/2)", "-2"] {
        let arg = e(src);
        close(
            &ball(&arg).ln(),
            &ball(&eval(&Expr::call(B::LOG, [arg])).unwrap()),
        );
    }
}

#[test]
fn remaining_exact_rules_have_fifty_digit_numeric_certificates() {
    for (head, inputs) in [(B::ARCSIN, vec!["1", "-1"]), (B::ARCCOS, vec!["1", "-1"])] {
        for src in inputs {
            let arg = e(src);
            let value = eval(&Expr::call(head, [arg.clone()])).unwrap();
            let recovered = if head == B::ARCSIN {
                ball(&value).sin()
            } else {
                ball(&value).cos()
            };
            close(&recovered, &ball(&arg));
        }
    }
    let value = e("3+4*I");
    let input = ball(&value);
    let re = eval(&Expr::call(B::RE, [value.clone()])).unwrap();
    let im = eval(&Expr::call(B::IM, [value.clone()])).unwrap();
    close(
        &CBall {
            re: input.re.clone(),
            im: Ball::exact(&Rational::ZERO, 256),
        },
        &ball(&re),
    );
    close(
        &CBall {
            re: input.im.clone(),
            im: Ball::exact(&Rational::ZERO, 256),
        },
        &ball(&im),
    );
    let conjugate = eval(&Expr::call(B::CONJUGATE, [value.clone()])).unwrap();
    close(
        &CBall {
            re: input.re.clone(),
            im: Ball::exact(&Rational::ZERO, 256).sub(&input.im),
        },
        &ball(&conjugate),
    );
    let magnitude = eval(&Expr::call(B::ABS, [value])).unwrap();
    close(
        &CBall {
            re: input.re.mul(&input.re).add(&input.im.mul(&input.im)).sqrt(),
            im: Ball::exact(&Rational::ZERO, 256),
        },
        &ball(&magnitude),
    );
    for src in ["0", "E", "-1/E"] {
        let arg = e(src);
        let result = ball(&eval(&Expr::call(B::PRODUCT_LOG, [arg.clone()])).unwrap());
        close(&result.mul(&result.exp()), &ball(&arg));
    }
    let x = e("1/3");
    close(
        &ball(&eval(&Expr::call(B::EXP, [x.clone()])).unwrap()),
        &ball(&x).exp(),
    );
    close(
        &ball(&eval(&Expr::call(B::SQRT, [x.clone()])).unwrap()),
        &ball(&x).sqrt(),
    );
    for head in [B::SIN, B::COS] {
        let negative = om_core::neg(x.clone());
        let expected = if head == B::SIN {
            ball(&negative).sin()
        } else {
            ball(&negative).cos()
        };
        let symbolic = eval(&Expr::call(head, [om_core::neg(Expr::symbol("x"))])).unwrap();
        let substituted = symbolic.replace_all(&[(Expr::symbol("x"), x.clone())]);
        let inner = if head == B::SIN {
            ball(&x).sin()
        } else {
            ball(&x).cos()
        };
        let result = if substituted.is_head(B::TIMES) {
            CBall::exact(&(-Rational::ONE), &Rational::ZERO, 256).mul(&inner)
        } else {
            inner
        };
        close(&expected, &result);
    }
}
