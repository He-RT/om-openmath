//! User downvalues with typed, repeated, conditional and variadic patterns.
use om_core::{BUILTIN as B, Expr, Interrupt};
use om_eval::Evaluator;
use om_parse::{Dialect, parse_expr};
fn src(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn run(ev: &mut Evaluator, s: &str) -> Expr {
    ev.evaluate(&src(s), &Interrupt::default()).unwrap()
}
#[test]
fn delayed_functions_bind_parameters_and_global_values_do_not_rename_patterns() {
    let mut ev = Evaluator::new();
    run(&mut ev, "x=99");
    run(&mut ev, "f[x_]:=x^2");
    assert_eq!(run(&mut ev, "f[3]"), Expr::int(9));
    assert_eq!(run(&mut ev, "f[-2]"), Expr::int(4));
    assert_eq!(run(&mut ev, "x"), Expr::int(99));
    run(&mut ev, "f[x_]:=x+1");
    assert_eq!(run(&mut ev, "f[3]"), Expr::int(4));
    run(&mut ev, "Clear[f]");
    assert_eq!(run(&mut ev, "f[3]"), src("f[3]"));
}
#[test]
fn typed_blanks_and_literal_heads_reject_wrong_categories() {
    let mut ev = Evaluator::new();
    run(&mut ev, "f[x_Integer]:=x^2");
    assert_eq!(run(&mut ev, "f[3]"), Expr::int(9));
    assert_eq!(run(&mut ev, "f[3.]"), src("f[3.]"));
    run(&mut ev, "g[x_List]:=x");
    assert_eq!(run(&mut ev, "g[{1,2}]"), src("{1,2}"));
    assert_eq!(run(&mut ev, "g[1]"), src("g[1]"));
    run(&mut ev, "h[f[x_]]:=x");
    assert_eq!(run(&mut ev, "h[f[a]]"), Expr::symbol("a"));
    assert_eq!(run(&mut ev, "h[g[a]]"), src("h[g[a]]"));
}
#[test]
fn repeated_names_match_structural_values_consistently() {
    let mut ev = Evaluator::new();
    run(&mut ev, "f[x_,x_]:=x");
    assert_eq!(run(&mut ev, "f[3,3]"), Expr::int(3));
    assert_eq!(run(&mut ev, "f[3,4]"), src("f[3,4]"));
    assert_eq!(run(&mut ev, "f[3,3.]"), src("f[3,3.]"));
    assert_eq!(run(&mut ev, "f[a+b,b+a]"), run(&mut ev, "a+b"));
}
#[test]
fn sequence_patterns_bind_all_arguments_and_backtrack_at_literal_anchors() {
    let mut ev = Evaluator::new();
    run(&mut ev, "f[x__]:={x}");
    assert_eq!(run(&mut ev, "f[1,2,3]"), src("{1,2,3}"));
    assert_eq!(run(&mut ev, "f[]"), src("f[]"));
    run(&mut ev, "g[x___]:={x}");
    assert_eq!(run(&mut ev, "g[]"), src("{}"));
    run(&mut ev, "h[a___,0,b__]:={{a},{b}}");
    assert_eq!(run(&mut ev, "h[1,2,0,3,4]"), src("{{1,2},{3,4}}"));
    assert_eq!(run(&mut ev, "h[0,0,1]"), src("{{},{0,1}}"));
    run(&mut ev, "t[x__Integer]:={x}");
    assert_eq!(run(&mut ev, "t[1,2]"), src("{1,2}"));
    assert_eq!(run(&mut ev, "t[1,2.]"), src("t[1,2.]"));
    run(&mut ev, "same[x__,x__]:={x}");
    assert_eq!(run(&mut ev, "same[1,2,1,2]"), src("{1,2}"));
    assert_eq!(run(&mut ev, "same[1,2,1,3]"), src("same[1,2,1,3]"));
}
#[test]
fn conditions_evaluate_substituted_bindings_and_false_results_backtrack() {
    let mut ev = Evaluator::new();
    run(&mut ev, "ok[1]=True");
    run(&mut ev, "ok[2]=False");
    run(&mut ev, "f[x_/;ok[x]]:=x^2");
    assert_eq!(run(&mut ev, "f[1]"), Expr::int(1));
    assert_eq!(run(&mut ev, "f[2]"), src("f[2]"));
    assert_eq!(run(&mut ev, "f[3]"), src("f[3]"));
    run(&mut ev, "g[x_]/;ok[x]:=x+1");
    assert_eq!(run(&mut ev, "g[1]"), Expr::int(2));
    assert_eq!(run(&mut ev, "g[2]"), src("g[2]"));
}
#[test]
fn literals_on_definition_targets_are_evaluated_without_applying_old_downvalues() {
    let mut ev = Evaluator::new();
    run(&mut ev, "f[1+1]=3");
    assert_eq!(run(&mut ev, "f[2]"), Expr::int(3));
    run(&mut ev, "f[1+1]=4");
    assert_eq!(run(&mut ev, "f[2]"), Expr::int(4));
    run(&mut ev, "f[1+1]=.");
    assert_eq!(run(&mut ev, "f[2]"), src("f[2]"));
}
#[test]
fn unsupported_patterns_emit_a_warning_and_remain_unmatched() {
    let mut ev = Evaluator::new();
    for lhs in [
        "Optional[x_]",
        "Alternatives[x_,y_]",
        "PatternTest[x_,test]",
    ] {
        run(&mut ev, &format!("f[{lhs}]:=1"));
        assert_eq!(run(&mut ev, "f[2]"), src("f[2]"));
        assert!(
            ev.messages
                .take()
                .iter()
                .any(|m| m.symbol == "Pattern" && m.tag == "unsup")
        );
        run(&mut ev, "Clear[f]");
    }
}
#[test]
fn substitution_is_simultaneous_and_keeps_held_arithmetic_raw() {
    let mut ev = Evaluator::new();
    run(&mut ev, "f[x_]:=Hold[x+1]");
    assert_eq!(run(&mut ev, "f[2]"), src("Hold[2+1]"));
    assert!(run(&mut ev, "f[2]").is_head(B::HOLD));
}

#[test]
fn conditions_resume_sequence_search_and_recursion_is_bounded() {
    let mut ev = Evaluator::new();
    run(&mut ev, "check[{}]=False");
    run(&mut ev, "check[{1}]=True");
    run(&mut ev, "f[(a___)/;check[{a}],b___]:={{a},{b}}");
    assert_eq!(run(&mut ev, "f[1,2]"), src("{{1},{2}}"));
    run(&mut ev, "Clear[f]");
    run(&mut ev, "f[x_]/;f[x]:=1");
    ev.settings.recursion_limit = 8;
    assert!(matches!(
        ev.evaluate(&src("f[1]"), &Interrupt::default()),
        Err(om_eval::EvalError::Recursion(8))
    ));
    ev.settings.recursion_limit = 1024;
    assert!(matches!(
        ev.evaluate(&src("f[1]"), &Interrupt::default()),
        Err(om_eval::EvalError::Recursion(1024))
    ));
    assert_eq!(run(&mut ev, "2+2"), Expr::int(4));
}

#[test]
fn direct_matcher_charges_budget_and_substitution_is_simultaneous_in_heads() {
    let mut ev = Evaluator::new();
    let ctx = Interrupt::default();
    ctx.steps_left.set(5);
    let lhs = src("f[a___,0,b___,0,c___]");
    let value = Expr::call(om_core::Symbol::intern("f"), (1..=100).map(Expr::int));
    assert!(matches!(
        om_eval::pattern::match_rule(&lhs, &value, &mut ev, &ctx),
        Err(om_eval::EvalError::Abort(om_core::Abort::Budget))
    ));
    let bindings = std::collections::BTreeMap::from([
        (om_core::Symbol::intern("x"), Expr::symbol("y")),
        (om_core::Symbol::intern("y"), Expr::int(2)),
    ]);
    assert_eq!(
        om_eval::pattern::substitute(&src("x[y,x]"), &bindings),
        src("y[2,y]")
    );
    let deep = (0..1100).fold(src("x_"), |e, _| {
        Expr::call(om_core::Symbol::intern("f"), [e])
    });
    let value = (0..1100).fold(Expr::int(3), |e, _| {
        Expr::call(om_core::Symbol::intern("f"), [e])
    });
    assert_eq!(
        om_eval::pattern::match_rule(&deep, &value, &mut ev, &Interrupt::default())
            .unwrap()
            .unwrap()
            .get(&om_core::Symbol::intern("x")),
        Some(&Expr::int(3))
    );
}

#[test]
fn deeply_nested_definition_targets_stop_without_host_recursion() {
    let mut ev = Evaluator::new();
    let lhs = (0..1100).fold(src("x_"), |e, _| {
        Expr::call(om_core::Symbol::intern("f"), [e])
    });
    let definition = Expr::call(B::SET_DELAYED, [lhs, Expr::int(1)]);
    assert!(matches!(
        ev.evaluate(&definition, &Interrupt::default()),
        Err(om_eval::EvalError::Recursion(1024))
    ));
    assert_eq!(run(&mut ev, "2+2"), Expr::int(4));
}
