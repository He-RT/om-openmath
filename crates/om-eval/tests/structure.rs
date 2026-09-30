//! M4.4 list, iteration, replacement and pure-function builtin batch.
use om_core::{Expr, Interrupt, canonicalize};
use om_eval::Evaluator;
use om_parse::{Dialect, parse_expr};
fn src(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn run(ev: &mut Evaluator, s: &str) -> Expr {
    ev.evaluate(&src(s), &Interrupt::default()).unwrap()
}
#[test]
fn list_access_mapping_and_application_preserve_heads_and_one_based_indices() {
    let mut ev = Evaluator::new();
    for (input, output) in [
        ("List[1+1,2+2]", "{2,4}"),
        ("List[]", "{}"),
        ("Part[{a,b,c},2]", "b"),
        ("Part[{a,b,c},-1]", "c"),
        ("Part[f[a,b],0]", "f"),
        ("Part[{{1,2},{3,4}},2,1]", "3"),
        ("Length[{1,2}]", "2"),
        ("Length[x]", "0"),
        ("First[{1,2}]", "1"),
        ("First[f[a,b]]", "a"),
        ("Last[{1,2}]", "2"),
        ("Last[f[a,b]]", "b"),
        ("Rest[{1,2}]", "{2}"),
        ("Rest[f[a,b]]", "f[b]"),
        ("Append[{1},2]", "{1,2}"),
        ("Append[f[a],b]", "f[a,b]"),
        ("Map[f,{1,2}]", "{f[1],f[2]}"),
        ("Map[f,g[a,b]]", "g[f[a],f[b]]"),
        ("Apply[Plus,{1,2,3}]", "6"),
        ("Apply[f,g[a,b]]", "f[a,b]"),
        ("Range[3]", "{1,2,3}"),
        ("Range[2,5]", "{2,3,4,5}"),
        ("Range[3,1,-1]", "{3,2,1}"),
        ("Range[0,1,1/3]", "{0,1/3,2/3,1}"),
    ] {
        assert_eq!(run(&mut ev, input), canonicalize(&src(output)), "{input}");
    }
    let invalid = src("Part[{1,2},3]");
    assert_eq!(
        ev.evaluate(&invalid, &Interrupt::default()).unwrap(),
        invalid
    );
    assert!(
        ev.messages
            .take()
            .iter()
            .any(|m| m.symbol == "Part" && m.tag == "partw")
    );
}
#[test]
fn pure_functions_substitute_slots_and_respect_nested_function_scope() {
    let mut ev = Evaluator::new();
    for (input, output) in [
        ("(#^2&)[3]", "9"),
        ("(#1+#2&)[3,4]", "7"),
        ("Map[#^2&,{1,2,3}]", "{1,4,9}"),
        ("Function[x,x^2][3]", "9"),
        ("Function[{x,y},x+y][3,4]", "7"),
        ("Function[Function[#^2]][3][4]", "16"),
    ] {
        assert_eq!(run(&mut ev, input), canonicalize(&src(output)), "{input}");
    }
    assert_eq!(run(&mut ev, "Slot[1]"), src("Slot[1]"));
    assert_eq!(run(&mut ev, "Slot[2]"), src("Slot[2]"));
}
#[test]
fn finite_iterators_shadow_variables_restore_state_and_preserve_readonly_mode() {
    let mut ev = Evaluator::new();
    run(&mut ev, "i=99");
    for (input, output) in [
        ("Table[i^2,{i,3}]", "{1,4,9}"),
        ("Table[a,{3}]", "{a,a,a}"),
        ("Table[i,{i,3,1,-1}]", "{3,2,1}"),
        ("Table[i*j,{i,2},{j,2}]", "{{1,2},{2,4}}"),
        ("Sum[i,{i,1,3}]", "6"),
        ("Sum[i^2,{i,1,3}]", "14"),
        ("Product[i,{i,1,4}]", "24"),
        ("Product[i+1,{i,1,3}]", "24"),
        ("Table[i,{i,0}]", "{}"),
        ("Sum[i,{i,0}]", "0"),
        ("Product[i,{i,0}]", "1"),
    ] {
        assert_eq!(run(&mut ev, input), canonicalize(&src(output)), "{input}");
    }
    assert_eq!(run(&mut ev, "i"), Expr::int(99));
    let mut fork = ev.fork_readonly();
    assert_eq!(run(&mut fork, "Table[i,{i,3}]"), src("{1,2,3}"));
    assert_eq!(run(&mut fork, "i"), Expr::int(99));
    let ctx = Interrupt::default();
    ctx.steps_left.set(40);
    assert!(ev.evaluate(&src("Table[i^2,{i,1000}]"), &ctx).is_err());
    assert_eq!(run(&mut ev, "i"), Expr::int(99));
    run(&mut ev, "Table[i=7,{i,2}]");
    assert_eq!(run(&mut ev, "i"), Expr::int(99));
}
#[test]
fn replacement_is_simultaneous_pattern_aware_and_repeated_replacement_is_bounded() {
    let mut ev = Evaluator::new();
    for (input, output) in [
        ("{x,y}/.{x->y,y->1}", "{y,1}"),
        ("f[1,2]/.f[x_,y_]:>x+y", "3"),
        ("{f[1],f[2]}/.f[x_]:>x^2", "{1,4}"),
        ("x/.x:>2+2", "4"),
        ("x//.{x->y,y->2}", "2"),
        ("f[f[1]]//.f[x_]:>x", "1"),
    ] {
        assert_eq!(run(&mut ev, input), canonicalize(&src(output)), "{input}");
    }
    ev.settings.iteration_limit = 3;
    assert!(run(&mut ev, "x//.{x->y,y->x}").is_head(om_core::BUILTIN::HOLD));
    assert!(
        ev.messages
            .take()
            .iter()
            .any(|m| m.symbol == "ReplaceRepeated" && m.tag == "rrlim")
    );
    assert_eq!(run(&mut ev, "Element[x,Reals]"), src("Element[x,Reals]"));
    assert_eq!(
        run(&mut ev, "Element[x,Integers]"),
        src("Element[x,Integers]")
    );
}
#[test]
fn all_registered_structure_functions_have_help_and_work_after_budget_failure() {
    let names = [
        "List",
        "Part",
        "Length",
        "First",
        "Last",
        "Rest",
        "Append",
        "Range",
        "Map",
        "Apply",
        "Table",
        "Sum",
        "Product",
        "Function",
        "Slot",
        "Rule",
        "ReplaceAll",
        "ReplaceRepeated",
        "Element",
    ];
    for name in names {
        let doc = Evaluator::doc(om_core::Symbol::intern(name)).unwrap();
        assert!(
            !doc.summary_zh.is_empty() && !doc.summary_en.is_empty() && !doc.examples.is_empty()
        );
    }
    let mut ev = Evaluator::new();
    let ctx = Interrupt::default();
    ctx.steps_left.set(0);
    assert!(ev.evaluate(&src("Table[i,{i,3}]"), &ctx).is_err());
    assert_eq!(run(&mut ev, "2+2"), Expr::int(4));
}

#[test]
fn recursive_iterator_bounds_use_heap_frames_and_restore_outer_scopes() {
    let mut ev = Evaluator::new();
    run(&mut ev, "i=99");
    run(&mut ev, "n:=Table[0,{i,n}]");
    assert!(matches!(
        ev.evaluate(&src("Table[i,{i,n}]"), &Interrupt::default()),
        Err(om_eval::EvalError::Recursion(1024))
    ));
    assert_eq!(run(&mut ev, "i"), Expr::int(99));
    assert_eq!(run(&mut ev, "2+2"), Expr::int(4));
}

#[test]
fn rule_patterns_shadow_global_values_and_recursive_rhs_stays_bounded() {
    let mut ev = Evaluator::new();
    run(&mut ev, "x=99");
    assert_eq!(run(&mut ev, "f[3]/.f[x_]:>x+1"), Expr::int(4));
    assert_eq!(run(&mut ev, "f[3]/.f[x_]->x+1"), Expr::int(4));
    assert_eq!(run(&mut ev, "x"), Expr::int(99));
    run(&mut ev, "r:=Rule[x,r]");
    assert!(matches!(
        ev.evaluate(&src("r"), &Interrupt::default()),
        Err(om_eval::EvalError::Recursion(1024))
    ));
    assert_eq!(run(&mut ev, "x"), Expr::int(99));
}
