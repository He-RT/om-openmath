//! Independent SI definitions, exact factors and dimensional arithmetic through the real evaluator.
use om_core::{Expr, Interrupt};
use om_eval::Evaluator;
fn eval(ev: &mut Evaluator, s: &str) -> Expr {
    ev.evaluate(
        &om_parse::parse_expr(s, om_parse::Dialect::Modern).unwrap(),
        &Interrupt::default(),
    )
    .unwrap()
}
#[test]
fn exact_si_prefixes_and_fixed_conversions() {
    let mut ev = Evaluator::new();
    for (source, value) in [
        (
            "magnitude(convert_units(quantity(1,unit:\"km\"),\"m\"))",
            Expr::int(1000),
        ),
        (
            "magnitude(convert_units(quantity(1,unit:\"mg\"),\"kg\"))",
            Expr::rational(1, 1_000_000),
        ),
        (
            "magnitude(convert_units(quantity(1,unit:\"h\"),\"s\"))",
            Expr::int(3600),
        ),
        (
            "magnitude(convert_units(quantity(1,unit:\"L\"),\"m^3\"))",
            Expr::rational(1, 1000),
        ),
        (
            "magnitude(convert_units(quantity(1,unit:\"in\"),\"m\"))",
            Expr::rational(127, 5000),
        ),
        (
            "magnitude(convert_units(quantity(1,unit:\"km/h\"),\"m/s\"))",
            Expr::rational(5, 18),
        ),
        (
            "magnitude(convert_units(quantity(2,unit:\"N\"),\"kg*m/s^2\"))",
            Expr::int(2),
        ),
        (
            "magnitude(convert_units(quantity(1,unit:\"μs\"),\"s\"))",
            Expr::rational(1, 1_000_000),
        ),
    ] {
        assert_eq!(eval(&mut ev, source), value, "{source}");
    }
    assert_eq!(
        eval(
            &mut ev,
            "magnitude(convert_units(quantity(1,unit:\"Qm\"),\"m\"))"
        ),
        eval(&mut ev, "10^30")
    );
    assert_eq!(
        eval(
            &mut ev,
            "magnitude(convert_units(quantity(1,unit:\"qm\"),\"m\"))"
        ),
        eval(&mut ev, "1/10^30")
    );
    assert_eq!(
        eval(&mut ev, "unit(quantity(1,unit:\" km \"))"),
        Expr::string("km")
    );
}
#[test]
fn products_powers_and_compatible_sums_preserve_real_dimensions() {
    let mut ev = Evaluator::new();
    for (source, value) in [
        (
            "magnitude(quantity(1,unit:\"km\")+quantity(250,unit:\"m\"))",
            Expr::rational(5, 4),
        ),
        (
            "magnitude(convert_units(quantity(2,unit:\"m\")*quantity(3,unit:\"m\"),\"cm^2\"))",
            Expr::int(60000),
        ),
        (
            "magnitude(convert_units(quantity(6,unit:\"m\")/quantity(2,unit:\"s\"),\"m/s\"))",
            Expr::int(3),
        ),
        (
            "magnitude(convert_units(quantity(2,unit:\"cm\")^2,\"m^2\"))",
            Expr::rational(1, 2500),
        ),
        (
            "magnitude(convert_units(quantity(2,unit:\"m\")-quantity(2,unit:\"m\"),\"m\"))",
            Expr::int(0),
        ),
    ] {
        assert_eq!(eval(&mut ev, source), value, "{source}");
    }
    assert_eq!(
        eval(&mut ev, "magnitude(quantity(2,unit:\"m\")*0)"),
        Expr::int(0)
    );
    let value = eval(
        &mut ev,
        "magnitude(convert_units(quantity(1.25,unit:\"km\"),\"m\"))",
    );
    assert_eq!(value.as_number().unwrap().to_f64(), Some(1250.));
    assert!(!value.as_number().unwrap().is_exact());
}
#[test]
fn incompatible_dimensions_affine_units_and_unknown_input_are_real_failures() {
    let mut ev = Evaluator::new();
    for source in [
        "quantity(1,unit:\"degC\")",
        "quantity(1,unit:\"USD\")",
        "quantity(1,unit:\"mkg\")",
        "quantity(1,unit:\"m;run()\")",
        "convert_units(quantity(1,unit:\"m\"),\"s\")",
        "quantity(1,unit:\"m\")+quantity(1,unit:\"s\")",
        "quantity(1,unit:\"m\")+1",
        "quantity(0,unit:\"s\")^(-1)",
        "quantity(1,unit:\"m^100000\")",
    ] {
        ev.messages.take();
        eval(&mut ev, source);
        assert!(!ev.messages.take().is_empty(), "{source}");
    }
    let p = om_parse::parse_expr(
        "convert_units(quantity(1,unit:\"kg*m/(s^2)\"),\"N\")",
        om_parse::Dialect::Modern,
    )
    .unwrap();
    let ctx = Interrupt::default();
    ctx.steps_left.set(10);
    assert!(ev.evaluate(&p, &ctx).is_err());
}
#[test]
fn derived_relations_explicit_arithmetic_and_input_precision_are_preserved() {
    let mut ev = Evaluator::new();
    for (source, expected) in [
        (
            "magnitude(convert_units(quantity(1,unit:\"Pa\")*quantity(2,unit:\"m^3\"),\"J\"))",
            "2",
        ),
        (
            "magnitude(convert_units(quantity(1,unit:\"V\")*quantity(2,unit:\"A\"),\"W\"))",
            "2",
        ),
        (
            "magnitude(convert_units(quantity(1,unit:\"F\")*quantity(2,unit:\"V\"),\"C\"))",
            "2",
        ),
        (
            "magnitude(convert_units(quantity(1,unit:\"H\")*quantity(2,unit:\"A\"),\"Wb\"))",
            "2",
        ),
        (
            "magnitude(convert_units(quantity(1,unit:\"T\")*quantity(2,unit:\"m^2\"),\"Wb\"))",
            "2",
        ),
        (
            "magnitude(convert_units(quantity(1,unit:\"kg m s^-2\"),\"N\"))",
            "1",
        ),
        ("magnitude(Subtract(quantity(2,unit:\"1\"),1))", "1"),
        ("magnitude(Divide(4,quantity(2,unit:\"s\")))", "2"),
        (
            "magnitude(quantity(quantity(1000,unit:\"m\"),unit:\"km\"))",
            "1",
        ),
    ] {
        assert_eq!(eval(&mut ev, source), eval(&mut ev, expected), "{source}");
    }
    let original = eval(&mut ev, "decimal(\"0.1\",precision:50)");
    let result = eval(
        &mut ev,
        "magnitude(convert_units(quantity(decimal(\"0.1\",precision:50),unit:\"km\"),\"m\"))",
    );
    assert_eq!(
        original.as_number().unwrap().precision(),
        result.as_number().unwrap().precision()
    );
    assert_eq!(result.as_number().unwrap().to_f64(), Some(100.));
}
