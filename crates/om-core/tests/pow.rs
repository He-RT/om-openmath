//! The forty contract vectors and principal Power boundary cases.

use om_core::{
    BUILTIN as B, Expr, canonicalize as arithmetic_tree, mul, pow, with_canonical_messages,
};
use om_num::{BigFloat, Complex, Integer, Number, Real};

fn x() -> Expr {
    Expr::symbol("x")
}
fn y() -> Expr {
    Expr::symbol("y")
}
fn p(b: Expr, e: Expr) -> Expr {
    Expr::call(B::POWER, [b, e])
}
fn a(args: impl IntoIterator<Item = Expr>) -> Expr {
    Expr::call(B::PLUS, args)
}
fn m(args: impl IntoIterator<Item = Expr>) -> Expr {
    Expr::call(B::TIMES, args)
}
fn s(e: Expr) -> Expr {
    Expr::call(B::SQRT, [e])
}
fn d(a: Expr, b: Expr) -> Expr {
    m([a, p(b, Expr::int(-1))])
}
fn i() -> Expr {
    Expr::number(Number::Complex(Box::new(Complex {
        re: Number::Integer(0.into()),
        im: Number::Integer(1.into()),
    })))
}

#[test]
fn all_forty_canonical_vectors_match_full_form() {
    let cases = [
        (
            a([Expr::symbol("a"), m([Expr::int(-1), Expr::symbol("b")])]),
            "Plus[a, Times[-1, b]]",
        ),
        (a([x(), x()]), "Times[2, x]"),
        (
            a([m([Expr::int(2), x()]), m([Expr::int(3), x()])]),
            "Times[5, x]",
        ),
        (m([x(), x()]), "Power[x, 2]"),
        (m([p(x(), Expr::int(2)), p(x(), Expr::int(-2))]), "1"),
        (a([y(), x(), Expr::int(2)]), "Plus[2, x, y]"),
        (a([p(x(), Expr::int(2)), x()]), "Plus[x, Power[x, 2]]"),
        (m([Expr::int(0), x()]), "0"),
        (s(Expr::int(12)), "Times[2, Power[3, Rational[1, 2]]]"),
        (p(Expr::int(8), Expr::rational(1, 3)), "2"),
        (
            m([s(Expr::int(2)), s(Expr::int(3))]),
            "Power[6, Rational[1, 2]]",
        ),
        (m([s(Expr::int(2)), s(Expr::int(2))]), "2"),
        (
            d(s(Expr::int(2)), s(Expr::int(3))),
            "Power[Rational[2, 3], Rational[1, 2]]",
        ),
        (s(Expr::rational(1, 2)), "Power[2, Rational[-1, 2]]"),
        (
            s(Expr::rational(12, 5)),
            "Times[2, Power[Rational[3, 5], Rational[1, 2]]]",
        ),
        (
            p(Expr::int(2), Expr::rational(-3, 2)),
            "Times[Rational[1, 2], Power[2, Rational[-1, 2]]]",
        ),
        (
            p(Expr::int(4), Expr::rational(1, 4)),
            "Power[2, Rational[1, 2]]",
        ),
        (s(Expr::int(-1)), "Complex[0, 1]"),
        (
            p(Expr::int(-8), Expr::rational(1, 3)),
            "Times[2, Power[-1, Rational[1, 3]]]",
        ),
        (
            p(Expr::int(-1), Expr::rational(4, 3)),
            "Times[-1, Power[-1, Rational[1, 3]]]",
        ),
        (
            s(p(x(), Expr::int(2))),
            "Power[Power[x, 2], Rational[1, 2]]",
        ),
        (p(s(x()), Expr::int(2)), "x"),
        (p(s(x()), Expr::rational(1, 3)), "Power[x, Rational[1, 6]]"),
        (
            p(m([x(), y()]), Expr::int(2)),
            "Times[Power[x, 2], Power[y, 2]]",
        ),
        (s(m([x(), y()])), "Power[Times[x, y], Rational[1, 2]]"),
        (
            s(m([Expr::int(2), x()])),
            "Times[Power[2, Rational[1, 2]], Power[x, Rational[1, 2]]]",
        ),
        (
            a([Expr::rational(1, 2), Expr::rational(1, 3)]),
            "Rational[5, 6]",
        ),
        (a([Expr::int(1), Expr::real(2.5)]), "3.5"),
        (a([x(), m([Expr::real(1.0), x()])]), "Times[2., x]"),
        (p(Expr::sym(B::I), Expr::int(2)), "-1"),
        (
            m([
                a([Expr::int(1), Expr::sym(B::I)]),
                a([Expr::int(1), m([Expr::int(-1), Expr::sym(B::I)])]),
            ]),
            "2",
        ),
        (d(Expr::int(1), Expr::int(0)), "ComplexInfinity"),
        (d(Expr::int(0), Expr::int(0)), "Indeterminate"),
        (
            a([
                Expr::sym(B::INFINITY),
                m([Expr::int(-1), Expr::sym(B::INFINITY)]),
            ]),
            "Indeterminate",
        ),
        (
            m([Expr::int(-2), Expr::sym(B::INFINITY)]),
            "DirectedInfinity[-1]",
        ),
        (
            m([Expr::sym(B::I), Expr::sym(B::INFINITY)]),
            "DirectedInfinity[Complex[0, 1]]",
        ),
        (p(Expr::sym(B::E), Expr::call(B::LOG, [x()])), "x"),
        (
            m([Expr::int(-1), a([Expr::int(1), x()])]),
            "Plus[-1, Times[-1, x]]",
        ),
        (
            m([Expr::int(-2), a([Expr::int(1), x()])]),
            "Times[-2, Plus[1, x]]",
        ),
        (p(x(), Expr::int(0)), "1"),
    ];
    assert_eq!(cases.len(), 40);
    for (index, (raw, expected)) in cases.into_iter().enumerate() {
        let e = arithmetic_tree(&raw);
        assert_eq!(format!("{e:?}"), expected, "vector {}: {raw:?}", index + 1);
        assert_eq!(arithmetic_tree(&e), e, "idempotence vector {}", index + 1);
    }
    assert_eq!(pow(Expr::int(0), Expr::int(0)), Expr::sym(B::INDETERMINATE));
}

#[test]
fn minus_one_phases_are_periodic_for_negative_and_large_rational_exponents() {
    assert_eq!(
        pow(Expr::int(-1), Expr::rational(-1, 2)),
        Expr::number(i().as_number().unwrap().neg())
    );
    assert_eq!(pow(Expr::int(-1), Expr::rational(9, 2)), i());
    assert_eq!(
        pow(Expr::int(-1), Expr::rational(7, 3)),
        p(Expr::int(-1), Expr::rational(1, 3))
    );
    assert_eq!(
        pow(Expr::sym(B::E), mul([i(), Expr::sym(B::PI)])),
        Expr::int(-1)
    );
    assert_eq!(
        pow(
            Expr::sym(B::E),
            mul([
                Expr::number(i().as_number().unwrap().mul(&Number::Rational(
                    om_num::Rational::from(1) / om_num::Rational::from(2)
                ))),
                Expr::sym(B::PI)
            ])
        ),
        i()
    );
}

#[test]
fn rational_root_reductions_cover_both_sides_and_large_degrees() {
    assert_eq!(
        pow(Expr::rational(4, 9), Expr::rational(1, 4)),
        pow(Expr::rational(2, 3), Expr::rational(1, 2))
    );
    assert_eq!(
        pow(Expr::rational(4, 27), Expr::rational(1, 6)),
        mul([
            p(Expr::int(2), Expr::rational(1, 3)),
            p(Expr::int(3), Expr::rational(-1, 2))
        ])
    );
    assert_eq!(
        pow(Expr::int(72), Expr::rational(-1, 2)),
        mul([Expr::rational(1, 6), p(Expr::int(2), Expr::rational(-1, 2))])
    );
    let exp = Expr::number(Number::Rational(
        om_num::Rational::from(1) / om_num::Rational::from(Integer::ONE << 40),
    ));
    assert_eq!(pow(Expr::int(2), exp.clone()), p(Expr::int(2), exp));
}

#[test]
fn zero_and_infinity_rules_are_explicit_and_diagnostics_are_captured() {
    let infinity = Expr::call(B::DIRECTED_INFINITY, [Expr::int(-1)]);
    assert_eq!(pow(infinity.clone(), Expr::int(-2)), Expr::int(0));
    assert_eq!(
        pow(infinity.clone(), Expr::int(2)),
        Expr::call(B::DIRECTED_INFINITY, [Expr::int(1)])
    );
    assert_eq!(
        pow(infinity.clone(), Expr::int(0)),
        Expr::sym(B::INDETERMINATE)
    );
    assert_eq!(pow(Expr::int(1), infinity), Expr::sym(B::INDETERMINATE));
    assert_eq!(pow(Expr::int(0), i()), Expr::sym(B::INDETERMINATE));
    assert_eq!(pow(Expr::int(0), x()), p(Expr::int(0), x()));
    let (e, msgs) = with_canonical_messages(|| pow(Expr::int(0), Expr::int(-1)));
    assert_eq!(e, Expr::call(B::DIRECTED_INFINITY, []));
    assert_eq!(msgs.len(), 1);
    assert_eq!((&*msgs[0].symbol, &*msgs[0].tag), ("Power", "infy"));
    let z = Expr::number(Number::Complex(Box::new(Complex {
        re: Number::Integer(1.into()),
        im: Number::Integer(1.into()),
    })));
    let directed = mul([z, Expr::sym(B::INFINITY)]);
    assert_eq!(mul([directed.clone()]), directed);
    assert_eq!(mul([Expr::int(2), directed.clone()]), directed);
}

#[test]
fn exact_bit_limit_preserves_original_power_and_unit_exceptions() {
    let huge = Expr::integer(Integer::ONE << 100);
    let raw = p(Expr::int(2), huge.clone());
    let (e, msgs) = with_canonical_messages(|| pow(Expr::int(2), huge.clone()));
    assert_eq!(e, raw);
    assert_eq!(msgs.len(), 1);
    assert_eq!((&*msgs[0].symbol, &*msgs[0].tag), ("General", "ovfl"));
    assert_eq!(pow(Expr::int(1), huge.clone()), Expr::int(1));
    assert_eq!(pow(Expr::int(-1), huge), Expr::int(1));
    let huge = Expr::integer(Integer::ONE << 100);
    let (e, msgs) = with_canonical_messages(|| pow(Expr::real(2.0), huge.clone()));
    assert_eq!(e, p(Expr::real(2.0), huge.clone()));
    assert_eq!(msgs.len(), 1);
    assert_eq!(pow(Expr::real(1.0), huge), Expr::real(1.0));
}

#[test]
fn approximate_powers_preserve_precision_and_principal_branch() {
    assert_eq!(pow(Expr::int(2), Expr::real(1.0)), Expr::real(2.0));
    assert_eq!(pow(Expr::real(1.0), Expr::int(2)), Expr::real(1.0));
    let approximate_i = Expr::number(Number::Complex(Box::new(Complex {
        re: Number::Real(Real::Machine(0.0)),
        im: Number::Real(Real::Machine(1.0)),
    })));
    assert_eq!(
        pow(approximate_i, Expr::int(2)),
        Expr::number(Number::Complex(Box::new(Complex {
            re: Number::Real(Real::Machine(-1.0)),
            im: Number::Real(Real::Machine(0.0)),
        })))
    );
    let sqrt = pow(Expr::real(-4.0), Expr::rational(1, 2));
    let (re, im) = sqrt.as_number().unwrap().to_complex_f64();
    assert!(re.abs() < 1e-14 && (im - 2.0).abs() < 1e-14);
    let high = Expr::number(Number::Real(Real::Big(
        BigFloat::from(2).with_precision(160).value(),
    )));
    let result = pow(high, Expr::rational(1, 2));
    assert!(matches!(result.as_number(),Some(Number::Real(Real::Big(n))) if n.precision()==160));
    let n = result.as_number().unwrap().to_f64().unwrap();
    assert!((n - 2.0f64.sqrt()).abs() < 1e-15);
    let overflow = pow(Expr::real(2.0), Expr::int(1024));
    assert!(matches!(overflow.as_number(),Some(Number::Real(Real::Big(n))) if n.precision()==53));
    let exp = pow(Expr::sym(B::E), Expr::real(1.0));
    assert!((exp.as_number().unwrap().to_f64().unwrap() - std::f64::consts::E).abs() < 1e-15);
}

enum Oracle {
    Exact(Number),
    Approx(om_num::CBall),
}
impl Oracle {
    fn ball(&self) -> om_num::CBall {
        fn scalar(n: &Number) -> om_num::Rational {
            match n {
                Number::Integer(n) => om_num::Rational::from(n.clone()),
                Number::Rational(q) => q.clone(),
                Number::Real(Real::Machine(x)) => om_num::Rational::try_from(*x).unwrap(),
                Number::Real(Real::Big(x)) => om_num::Rational::try_from(x.clone()).unwrap(),
                _ => panic!("scalar expected"),
            }
        }
        match self {
            Self::Approx(b) => b.clone(),
            Self::Exact(Number::Complex(c)) => {
                om_num::CBall::exact(&scalar(&c.re), &scalar(&c.im), 256)
            }
            Self::Exact(n) => om_num::CBall::exact(&scalar(n), &om_num::Rational::ZERO, 256),
        }
    }
    fn add(self, other: Self) -> Self {
        if let (Self::Exact(a), Self::Exact(b)) = (&self, &other) {
            Self::Exact(a.add(b))
        } else {
            Self::Approx(self.ball().add(&other.ball()))
        }
    }
    fn mul(self, other: Self) -> Self {
        if let (Self::Exact(a), Self::Exact(b)) = (&self, &other) {
            Self::Exact(a.mul(b))
        } else {
            Self::Approx(self.ball().mul(&other.ball()))
        }
    }
    fn pow(self, exp: Self) -> Option<Self> {
        if let Self::Exact(Number::Integer(e)) = &exp {
            if let Self::Exact(b) = &self {
                return b.pow_int(e).ok().map(Self::Exact);
            }
            let b = self.ball();
            if b.re.contains_zero() && b.im.contains_zero() {
                return None;
            }
            return Some(Self::Approx(b.pow_int(e)));
        }
        let half = om_num::Rational::from(1) / om_num::Rational::from(2);
        if let Self::Exact(Number::Rational(q)) = &exp {
            if let Self::Exact(n) = &self {
                let r = match n {
                    Number::Integer(n) => Some(om_num::Rational::from(n.clone())),
                    Number::Rational(r) => Some(r.clone()),
                    _ => None,
                };
                if let Some(r) = r
                    && *q == half
                {
                    let a = Integer::from(r.numerator().clone().into_parts().1);
                    let b = Integer::from(r.denominator().clone());
                    if let (Some(a), Some(b)) =
                        (om_num::exact_root(&a, 2), om_num::exact_root(&b, 2))
                    {
                        let root =
                            Number::Rational(om_num::Rational::from(a) / om_num::Rational::from(b))
                                .normalize();
                        return Some(Self::Exact(if r < om_num::Rational::ZERO {
                            Number::Complex(Box::new(Complex {
                                re: Number::Integer(0.into()),
                                im: root,
                            }))
                            .normalize()
                        } else {
                            root
                        }));
                    }
                }
            }
            if *q == half {
                return Some(Self::Approx(self.ball().sqrt()));
            }
        }
        let b = self.ball();
        if b.re.contains_zero() && b.im.contains_zero() {
            return None;
        }
        let logarithm = b.ln().mul(&exp.ball());
        if !logarithm.re.mid.to_f64().value().is_finite() {
            return None;
        }
        Some(Self::Approx(logarithm.exp()))
    }
}
fn oracle(e: &Expr, xv: &om_num::Rational, yv: &om_num::Rational) -> Option<Oracle> {
    if let Some(n) = e.as_number() {
        Some(Oracle::Exact(n.clone()))
    } else if *e == x() {
        Some(Oracle::Exact(Number::Rational(xv.clone()).normalize()))
    } else if *e == y() {
        Some(Oracle::Exact(Number::Rational(yv.clone()).normalize()))
    } else if e.is_head(B::PLUS) {
        e.args()
            .iter()
            .try_fold(Oracle::Exact(Number::Integer(0.into())), |v, e| {
                Some(v.add(oracle(e, xv, yv)?))
            })
    } else if e.is_head(B::TIMES) {
        e.args()
            .iter()
            .try_fold(Oracle::Exact(Number::Integer(1.into())), |v, e| {
                Some(v.mul(oracle(e, xv, yv)?))
            })
    } else if e.is_head(B::POWER) && e.args().len() == 2 {
        oracle(&e.args()[0], xv, yv)?.pow(oracle(&e.args()[1], xv, yv)?)
    } else {
        None
    }
}
fn numeric_value(e: &Expr, xv: &om_num::Rational, yv: &om_num::Rational) -> Option<(f64, f64)> {
    let b = oracle(e, xv, yv)?.ball();
    let re = b.re.mid.to_f64().value();
    let im = b.im.mid.to_f64().value();
    let rad = b.re.rad.to_f64().value().hypot(b.im.rad.to_f64().value());
    (re.is_finite() && im.is_finite() && rad < 1e-20 * (1.0 + re.hypot(im))).then_some((re, im))
}

fn trees() -> impl proptest::strategy::Strategy<Value = Expr> {
    use proptest::prelude::*;
    let leaf = prop_oneof![
        (-3i64..=3).prop_map(Expr::int),
        Just(Expr::rational(1, 2)),
        Just(x()),
        Just(y())
    ];
    leaf.prop_recursive(4, 24, 3, |inner| {
        prop_oneof![
            proptest::collection::vec(inner.clone(), 0..3).prop_map(a),
            proptest::collection::vec(inner.clone(), 0..3).prop_map(m),
            (
                inner,
                prop_oneof![(-3i64..=3).prop_map(Expr::int), Just(Expr::rational(1, 2))]
            )
                .prop_map(|(b, e)| p(b, e))
        ]
    })
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config {cases:256, rng_seed:proptest::test_runner::RngSeed::Fixed(0x43414e4f4e), ..proptest::test_runner::Config::default()})]
    #[test]
    fn depth_four_trees_are_idempotent_and_preserve_numeric_value(
        tree in trees(),xn in -3i64..=3,xd in 1i64..=3,yn in -3i64..=3,yd in 1i64..=3) {
        use proptest::prelude::*;
        let canonical=arithmetic_tree(&tree);
        prop_assert_eq!(&canonical,&arithmetic_tree(&canonical));
        let xv=om_num::Rational::from(xn)/om_num::Rational::from(xd);
        let yv=om_num::Rational::from(yn)/om_num::Rational::from(yd);
        let reference=numeric_value(&tree,&xv,&yv);
        let candidate=numeric_value(&canonical,&xv,&yv);
        // Only a finite, sufficiently narrow oracle enclosure can certify this comparison.
        prop_assume!(reference.is_some() && candidate.is_some());
        let (ar,ai)=reference.unwrap();
        let (br,bi)=candidate.unwrap();
        let relative=(ar-br).hypot(ai-bi)/(1.0+ar.hypot(ai));
        prop_assert!(relative<1e-9,"raw={tree:?}, canonical={canonical:?}, x={xv}, y={yv}, error={relative}");
    }
}
