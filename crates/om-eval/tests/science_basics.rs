//! Exact independent definitions verify basic statistics and data transformations.
use om_core::{Expr, Interrupt};
use om_eval::Evaluator;
use om_parse::Dialect;
fn eval(ev: &mut Evaluator, s: &str) -> Expr {
    let p = om_parse::parse_expr(s, Dialect::Modern).unwrap();
    ev.evaluate(&p, &Interrupt::default()).unwrap()
}
fn equal(ev: &mut Evaluator, s: &str, w: &str) {
    let a = eval(ev, s);
    let b = om_parse::parse_expr(w, Dialect::Wolfram).unwrap();
    assert_eq!(om_core::canonicalize(&a), om_core::canonicalize(&b), "{s}");
}
#[test]
fn exact_sample_definitions_and_type7_quantiles() {
    let mut ev = Evaluator::new();
    for (s, e) in [
        ("mean([1,2,3])", "2"),
        ("median([1,2,3,4])", "5/2"),
        ("variance([1,2,3])", "1"),
        ("variance([1,2,3],sample:false)", "2/3"),
        ("std([1,2,3])", "1"),
        ("covariance([1,2,3],[2,4,6])", "2"),
        ("correlation([1,2,3],[2,4,6])", "1"),
        ("quantile([0,10,20],1/4)", "5"),
        ("percentile([0,10,20],25)", "5"),
    ] {
        equal(&mut ev, s, e);
    }
}
#[test]
fn numeric_boundaries_preserve_exactness() {
    let mut ev = Evaluator::new();
    for (s, e) in [
        ("integer_part(-7/3)", "-2"),
        ("fractional_part(-7/3)", "-1/3"),
        ("min([3,1],2)", "1"),
        ("minmax([3,1,2])", "{1,3}"),
        ("clip(2)", "1"),
        ("rationalize(0.333333,tolerance:0.001)", "1/3"),
        ("chop([0.000000000001,1])", "{0,1}"),
    ] {
        equal(&mut ev, s, e);
    }
}
#[test]
fn data_composition_is_deterministic_and_uses_real_predicates() {
    let mut ev = Evaluator::new();
    for (s, e) in [
        ("[1,2,3,4] |> filter(fn(x)=>x>2)", "{3,4}"),
        ("sort([3,1,2])", "{1,2,3}"),
        ("sort_by([-2,1,3],fn(x)=>abs(x))", "{1,-2,3}"),
        ("unique([1,2,1])", "{1,2}"),
        ("take([1,2,3],-2)", "{2,3}"),
        ("drop([1,2,3],-1)", "{1,2}"),
        ("flatten([[1,2],[3]])", "{1,2,3}"),
        ("reshape([1,2,3,4],[2,2])", "{{1,2},{3,4}}"),
        ("zip([1,2],[3,4])", "{{1,3},{2,4}}"),
        ("[1,2,3] |> fold(fn(a,b)=>a+b,0)", "6"),
    ] {
        equal(&mut ev, s, e);
    }
}
#[test]
fn empty_samples_zero_variance_and_invalid_shapes_decline_honestly() {
    let mut ev = Evaluator::new();
    for s in [
        "mean([])",
        "variance([1])",
        "correlation([1,1],[2,3])",
        "quantile([1,2],2)",
        "reshape([1,2,3],[2,2])",
        "filter([1],fn(x)=>x)",
    ] {
        ev.messages.take();
        eval(&mut ev, s);
        assert!(!ev.messages.take().is_empty(), "{s}");
    }
}

#[test]
fn grouping_preserves_first_occurrence_and_sort_keys_execute_once() {
    let mut ev = Evaluator::new();
    let groups = eval(&mut ev, "counts([2,1,2])");
    assert_eq!(groups.args().len(), 2);
    assert_eq!(groups.args()[0].args()[0].args()[1], Expr::int(2));
    assert_eq!(groups.args()[0].args()[1].args()[1], Expr::int(2));
    let groups = eval(&mut ev, "group_by([1,2,3,4],fn(x)=>mod(x,2))");
    assert_eq!(groups.args().len(), 2);
    assert_eq!(
        groups.args()[0].args()[1].args()[1],
        om_parse::parse_expr("{1,3}", Dialect::Wolfram).unwrap()
    );
    let assignment = om_parse::parse_expr("let n=0", Dialect::Modern).unwrap();
    ev.evaluate(&assignment, &Interrupt::default()).unwrap();
    equal(
        &mut ev,
        "sort_by([3,1,2],fn(x)=>sequence(assign(n,n+1),x))",
        "{1,2,3}",
    );
    equal(&mut ev, "n", "3");
}

#[test]
fn exact_matrix_operations_reconstruct_and_reject_singular_or_mismatched_inputs() {
    let mut ev = Evaluator::new();
    for (source, expected) in [
        ("identity(2)", "{{1,0},{0,1}}"),
        ("diag([2,3])", "{{2,0},{0,3}}"),
        ("transpose([[1,2,3],[4,5,6]])", "{{1,4},{2,5},{3,6}}"),
        ("trace([[1,2],[3,4]])", "5"),
        ("det([[1,2],[3,4]])", "-2"),
        ("det([[1/2,1/3],[1/4,1/5]])", "1/60"),
        ("inverse([[1,2],[3,4]])", "{{-2,1},{3/2,-1/2}}"),
        ("[[1,2],[3,4]] @ inverse([[1,2],[3,4]])", "{{1,0},{0,1}}"),
        ("rank([[1,2],[2,4]])", "1"),
        ("null_space([[1,2],[2,4]])", "{{-2,1}}"),
        ("linear_solve([[2,1],[1,3]],[1,2])", "{1/5,3/5}"),
        ("cross([1,0,0],[0,1,0])", "{0,0,1}"),
        ("norm([3,4])", "5"),
        ("normalize([3,4])", "{3/5,4/5}"),
    ] {
        equal(&mut ev, source, expected);
    }
    for source in [
        "inverse([[1,2],[2,4]])",
        "linear_solve([[0,0]],[1])",
        "normalize([0,0])",
    ] {
        ev.messages.take();
        eval(&mut ev, source);
        assert!(!ev.messages.take().is_empty(), "{source}");
    }
}

#[test]
fn decimal_uses_textual_exact_input_and_rescale_keeps_rational_values() {
    let mut ev = Evaluator::new();
    let value = eval(&mut ev, "decimal(\"0.1\",precision:50)");
    let Some(om_num::Number::Real(om_num::Real::Big(number))) = value.as_number() else {
        panic!("{value:?}")
    };
    let ratio = om_num::Rational::try_from(number.clone()).unwrap();
    let error = ratio - om_num::Rational::from_parts(1.into(), 10u32.into());
    let error = if error < om_num::Rational::ZERO {
        -error
    } else {
        error
    };
    assert!(
        error
            < om_num::Rational::from_parts(1.into(), (om_num::Integer::ONE << 166).into_parts().1)
    );
    equal(&mut ev, "rescale([1,2,3])", "{0,1/2,1}");
    equal(&mut ev, "rescale(3,from:[0,10],to:[0,1])", "3/10");
    for source in [
        "decimal(\"sin(1)\")",
        "decimal(\"NaN\")",
        "decimal(\"0.1e-2147483648\")",
        "rescale([2,2])",
    ] {
        ev.messages.take();
        eval(&mut ev, source);
        assert!(!ev.messages.take().is_empty(), "{source}");
    }
}
