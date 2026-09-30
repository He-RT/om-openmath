//! Source-backed canonical vectors and parser/formatter round trips.
use om_core::{BUILTIN as B, Expr, canonicalize};
use om_format::{FormatOptions, full_form, input_form, modern_form};
use om_num::{BigFloat, Number, Real};
use om_parse::{Dialect, parse_expr};

fn round_trip(e: &Expr) {
    for (source, dialect) in [
        (input_form(e), Dialect::Wolfram),
        (modern_form(e), Dialect::Modern),
    ] {
        let parsed =
            parse_expr(&source, dialect).unwrap_or_else(|d| panic!("{e:?} -> {source}: {d:?}"));
        assert_eq!(canonicalize(&parsed), *e, "{e:?} -> {source}");
    }
}
#[test]
fn all_forty_full_form_vectors_match_and_both_source_forms_round_trip() {
    let vectors = [
        ("a-b", "Plus[a, Times[-1, b]]"),
        ("x+x", "Times[2, x]"),
        ("2x+3x", "Times[5, x]"),
        ("x*x", "Power[x, 2]"),
        ("x^2*x^-2", "1"),
        ("y+x+2", "Plus[2, x, y]"),
        ("x^2+x", "Plus[x, Power[x, 2]]"),
        ("0*x", "0"),
        ("Sqrt[12]", "Times[2, Power[3, Rational[1, 2]]]"),
        ("8^(1/3)", "2"),
        ("Sqrt[2]Sqrt[3]", "Power[6, Rational[1, 2]]"),
        ("Sqrt[2]Sqrt[2]", "2"),
        ("Sqrt[2]/Sqrt[3]", "Power[Rational[2, 3], Rational[1, 2]]"),
        ("Sqrt[1/2]", "Power[2, Rational[-1, 2]]"),
        (
            "Sqrt[12/5]",
            "Times[2, Power[Rational[3, 5], Rational[1, 2]]]",
        ),
        (
            "2^(-3/2)",
            "Times[Rational[1, 2], Power[2, Rational[-1, 2]]]",
        ),
        ("4^(1/4)", "Power[2, Rational[1, 2]]"),
        ("(-1)^(1/2)", "Complex[0, 1]"),
        ("(-8)^(1/3)", "Times[2, Power[-1, Rational[1, 3]]]"),
        ("(-1)^(4/3)", "Times[-1, Power[-1, Rational[1, 3]]]"),
        ("(x^2)^(1/2)", "Power[Power[x, 2], Rational[1, 2]]"),
        ("(x^(1/2))^2", "x"),
        ("(x^(1/2))^(1/3)", "Power[x, Rational[1, 6]]"),
        ("(x y)^2", "Times[Power[x, 2], Power[y, 2]]"),
        ("(x y)^(1/2)", "Power[Times[x, y], Rational[1, 2]]"),
        (
            "(2x)^(1/2)",
            "Times[Power[2, Rational[1, 2]], Power[x, Rational[1, 2]]]",
        ),
        ("1/2+1/3", "Rational[5, 6]"),
        ("1+2.5", "3.5"),
        ("x+1.0x", "Times[2., x]"),
        ("I^2", "-1"),
        ("(1+I)(1-I)", "2"),
        ("1/0", "ComplexInfinity"),
        ("0/0", "Indeterminate"),
        ("Infinity-Infinity", "Indeterminate"),
        ("-2*Infinity", "DirectedInfinity[-1]"),
        ("I*Infinity", "DirectedInfinity[Complex[0, 1]]"),
        ("E^Log[x]", "x"),
        ("-(1+x)", "Plus[-1, Times[-1, x]]"),
        ("-2(1+x)", "Times[-2, Plus[1, x]]"),
        ("x^0", "1"),
    ];
    assert_eq!(vectors.len(), 40);
    for (source, expected) in vectors {
        let e = canonicalize(&parse_expr(source, Dialect::Wolfram).unwrap());
        assert_eq!(full_form(&e), expected, "{source}");
        round_trip(&e);
    }
    assert_eq!(
        full_form(&canonicalize(&parse_expr("0^0", Dialect::Wolfram).unwrap())),
        "Indeterminate"
    );
}
#[test]
fn standard_forms_are_readable_and_precedence_preserves_meaning() {
    for (source, wolfram, modern) in [
        ("1+2*x", "2*x + 1", "2x + 1"),
        ("x^2+x+1", "x^2 + x + 1", "x^2 + x + 1"),
        ("Sin[x]", "Sin[x]", "sin(x)"),
        ("{1,2,x}", "{1, 2, x}", "[1, 2, x]"),
        ("a-b", "a - b", "a - b"),
        ("(x+1)/(x-1)", "(x + 1)/(x - 1)", "(x + 1)/(x - 1)"),
        ("x->1", "x -> 1", "x -> 1"),
        ("x==1", "x == 1", "x = 1"),
        (
            "Root[#^2-2&,1]",
            "Root[Function[Slot[1]^2 - 2], 1]",
            "Root(function(slot(1)^2 - 2), 1)",
        ),
    ] {
        let e = canonicalize(&parse_expr(source, Dialect::Wolfram).unwrap());
        assert_eq!(input_form(&e), wolfram, "{source}");
        assert_eq!(modern_form(&e), modern, "{source}");
        round_trip(&e);
    }
    assert!(FormatOptions::default().display_order);
    for source in [
        "(-x)^2",
        "(x^2)^(1/2)",
        "x^(y^z)",
        "(a+b)*c",
        "Not[x==y]",
        "x<y&&y<=z",
        "x==y==z",
        "x<y<z",
        "f[x->y->z]",
        "x/.{x->1}",
        "Part[v,1]",
        "Set[x,5]",
        "SetDelayed[f[Pattern[x,Blank[]]],x^2]",
    ] {
        round_trip(&canonicalize(
            &parse_expr(source, Dialect::Wolfram).unwrap(),
        ));
    }
}
#[test]
fn strings_machine_values_and_big_precision_survive_reparsing() {
    for e in [
        Expr::string("α\n\r\t\"\\\u{0}\u{8}😀"),
        Expr::real(f64::MAX),
        Expr::real(f64::MIN_POSITIVE),
        Expr::real(f64::from_bits(1)),
        Expr::real(-0.0),
        Expr::real(1.2345678901234567),
        Expr::rational(-7, 11),
    ] {
        round_trip(&e);
    }
    for bits in [2, 53, 100, 160, 257] {
        for text in ["1.234567890123456789", "0.000001", "-234.5", "0"] {
            let q = om_num::Rational::from_str_decimal(text).unwrap();
            let x: BigFloat = q.to_float(bits).value();
            round_trip(&Expr::number(Number::Real(Real::Big(x))));
        }
    }
}
fn trees() -> impl proptest::strategy::Strategy<Value = Expr> {
    use proptest::prelude::*;
    prop_oneof![
        (-3i64..=3).prop_map(Expr::int),
        Just(Expr::rational(1, 2)),
        Just(Expr::symbol("x")),
        Just(Expr::symbol("y"))
    ]
    .prop_recursive(4, 32, 4, |inner| {
        prop_oneof![
            proptest::collection::vec(inner.clone(), 0..4)
                .prop_map(|args| Expr::call(B::PLUS, args)),
            proptest::collection::vec(inner.clone(), 0..4)
                .prop_map(|args| Expr::call(B::TIMES, args)),
            (inner.clone(), -3i64..=3).prop_map(|(b, e)| Expr::call(B::POWER, [b, Expr::int(e)])),
            proptest::collection::vec(inner, 0..4).prop_map(|args| Expr::call(B::LIST, args))
        ]
    })
}
proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config {cases:256,rng_seed:proptest::test_runner::RngSeed::Fixed(0x464f524d), ..proptest::test_runner::Config::default()})]
    #[test]
    fn canonical_depth_four_trees_round_trip(tree in trees()) { round_trip(&canonicalize(&tree)); }
}

#[test]
fn compound_heads_round_trip_without_changing_grouped_multiplication() {
    for src in ["f[x][y]", "Derivative[1][f][x]", "(a+b)[x]"] {
        round_trip(&canonicalize(&parse_expr(src, Dialect::Wolfram).unwrap()));
    }
    assert_eq!(
        canonicalize(&parse_expr("(x+1)(x-1)", Dialect::Modern).unwrap()),
        canonicalize(&parse_expr("(x+1)(x-1)", Dialect::Wolfram).unwrap())
    );
}

#[test]
fn display_order_can_be_disabled_and_does_not_mutate_the_tree() {
    let e = canonicalize(&parse_expr("x^2+x+1", Dialect::Wolfram).unwrap());
    let options = FormatOptions {
        display_order: false,
    };
    assert_eq!(om_format::input_form_with(&e, &options), "1 + x + x^2");
    assert_eq!(om_format::modern_form_with(&e, &options), "1 + x + x^2");
    assert_eq!(full_form(&e), "Plus[1, x, Power[x, 2]]");
}
