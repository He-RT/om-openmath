//! Holding, sequence expansion and list threading in the heap evaluator.
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
fn hold_and_holdform_preserve_raw_structure_and_side_effects() {
    let mut ev = Evaluator::new();
    for s in [
        "Hold[1+1]",
        "HoldForm[1+1]",
        "Hold[x=2]",
        "Hold[Sequence[1+1,2+2]]",
    ] {
        assert_eq!(run(&mut ev, s), src(s), "{s}");
    }
    assert_eq!(run(&mut ev, "x"), Expr::symbol("x"));
    let held = run(&mut ev, "HoldForm[1+1]");
    assert_eq!(om_format::latex(&held), "1 + 1");
    assert_eq!(om_format::unicode_form(&held), "1 + 1");
    assert_eq!(om_format::full_form(&held), "HoldForm[Plus[1, 1]]");
}
#[test]
fn holdfirst_and_holdrest_evaluate_only_the_allowed_arguments() {
    let mut ev = Evaluator::new();
    run(&mut ev, "x=2");
    assert_eq!(run(&mut ev, "x=3"), Expr::int(3));
    assert_eq!(run(&mut ev, "x"), Expr::int(3));
    assert_eq!(run(&mut ev, "x:>(y=7)"), src("3:>(y=7)"));
    assert_eq!(run(&mut ev, "y"), Expr::symbol("y"));
    assert_eq!(run(&mut ev, "x:>1+1"), src("3:>1+1"));
    assert_eq!(run(&mut ev, "x->1+1"), src("3->2"));
}
#[test]
fn sequence_expansion_follows_argument_evaluation_and_holdall() {
    let mut ev = Evaluator::new();
    assert_eq!(run(&mut ev, "f[Sequence[1+1,2+2],5]"), src("f[2,4,5]"));
    assert_eq!(
        run(&mut ev, "f[Sequence[],1,Sequence[Sequence[2,3]]]"),
        src("f[1,2,3]")
    );
    assert_eq!(run(&mut ev, "{Sequence[1,2],3}"), src("{1,2,3}"));
    assert_eq!(run(&mut ev, "Plus[Sequence[1,2],3]"), Expr::int(6));
    assert_eq!(
        run(&mut ev, "Hold[Sequence[1+1,2]]"),
        src("Hold[Sequence[1+1,2]]")
    );
    assert_eq!(run(&mut ev, "Out[Sequence[]]"), src("Out[]"));
}
#[test]
fn listable_threads_matching_lists_broadcasts_scalars_and_handles_empty_lists() {
    let mut ev = Evaluator::new();
    for (s, expected) in [
        ("{1,2}+{3,4}", "{4,6}"),
        ("{1,2}+10", "{11,12}"),
        ("2*{1,2,3}", "{2,4,6}"),
        ("{1,2}^{2,3}", "{1,8}"),
        ("{{1,2},{3,4}}+10", "{{11,12},{13,14}}"),
        ("Sqrt[{4,9}]", "{2,3}"),
        ("{}+3", "{}"),
        ("{}+{}", "{}"),
    ] {
        assert_eq!(run(&mut ev, s), src(expected), "{s}");
    }
    assert_eq!(run(&mut ev, "{1,2}+(x=3)"), src("{4,5}"));
    assert_eq!(run(&mut ev, "x"), Expr::int(3));
}
#[test]
fn mismatched_lengths_emit_one_message_and_preserve_the_expression() {
    let mut ev = Evaluator::new();
    for s in ["{1,2}+{3,4,5}", "{}+{1}", "{1,2}^{3}"] {
        assert_eq!(run(&mut ev, s), src(s));
        let messages = ev.messages.take();
        assert_eq!(
            messages
                .iter()
                .filter(|m| m.symbol == "Thread" && m.tag == "tdlen")
                .count(),
            1
        );
    }
    assert_eq!(run(&mut ev, "2+2"), Expr::int(4));
}
#[test]
fn wide_threading_obeys_budget_and_does_not_recurse_through_the_host_stack() {
    let mut ev = Evaluator::new();
    let list = Expr::call(B::LIST, (0..2000).map(Expr::int));
    let expr = Expr::call(B::PLUS, [list, Expr::int(10)]);
    let result = ev.evaluate(&expr, &Interrupt::default()).unwrap();
    assert_eq!(result.args().len(), 2000);
    for (i, arg) in result.args().iter().enumerate() {
        assert_eq!(*arg, Expr::int(i as i64 + 10));
    }
    let ctx = Interrupt::default();
    ctx.steps_left.set(2030);
    assert!(ev.evaluate(&expr, &ctx).is_err());
    assert_eq!(run(&mut ev, "2+2"), Expr::int(4));
}
