//! Query results distinguish real callbacks from planning and never evaluate requested names as code.
use om_core::{BUILTIN as B, Expr, Interrupt};
use om_eval::Evaluator;
fn eval(ev: &mut Evaluator, s: &str) -> Expr {
    ev.evaluate(
        &om_parse::parse_expr(s, om_parse::Dialect::Modern).unwrap(),
        &Interrupt::default(),
    )
    .unwrap()
}
fn field<'a>(e: &'a Expr, key: &str) -> &'a Expr {
    &e.args()
        .iter()
        .find(|r| r.is_head(B::RULE) && r.args()[0] == Expr::string(key))
        .unwrap_or_else(|| panic!("missing {key}: {e:?}"))
        .args()[1]
}
#[test]
fn help_and_options_expose_real_stable_ids_hold_roles_and_precision_boundaries() {
    let mut ev = Evaluator::new();
    let help = eval(&mut ev, "help(\"map\")");
    assert_eq!(field(&help, "id"), &Expr::string("fn_000063"));
    assert_eq!(field(&help, "executable"), &Expr::sym(B::TRUE));
    let runtime = field(&help, "runtime");
    assert_eq!(field(runtime, "pipe_arg"), &Expr::int(2));
    assert_eq!(field(runtime, "modern_name"), &Expr::string("map"));
    let options = eval(&mut ev, "options(\"quantity\")");
    assert_eq!(field(&options, "executable"), &Expr::sym(B::TRUE));
    assert_eq!(
        field(&field(&options, "options").args()[0], "name"),
        &Expr::string("unit")
    );
    assert!(
        matches!(field(field(&options,"options").args().first().unwrap(),"default_context").kind(),om_core::ExprKind::String(text) if text.contains("单位必填"))
    );
}
#[test]
fn planned_and_deferred_docs_are_browsable_but_not_executable() {
    let mut ev = Evaluator::new();
    for (name, status) in [("scene", "planned"), ("apply_notebook_patch", "deferred")] {
        let help = eval(&mut ev, &format!("help(\"{name}\")"));
        assert_eq!(field(&help, "status"), &Expr::string(status));
        assert_eq!(field(&help, "executable"), &Expr::sym(B::FALSE));
        assert_eq!(field(&help, "runtime"), &Expr::sym(B::NULL));
        let options = eval(&mut ev, &format!("options(\"{name}\")"));
        assert!(field(&options, "options").args().is_empty());
    }
    let planned = eval(
        &mut ev,
        "functions(category:\"calculus\",stage:\"planned\")",
    );
    let expected = om_core::catalog::documentation()["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["category"] == "calculus" && e["status"] == "planned")
        .map(|e| Expr::string(e["name"].as_str().unwrap()))
        .collect::<Vec<_>>();
    let actual = planned
        .args()
        .iter()
        .map(|e| field(e, "name").clone())
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    assert!(
        planned
            .args()
            .iter()
            .all(|r| field(r, "executable") == &Expr::sym(B::FALSE))
    );
    let deferred = eval(&mut ev, "functions(category:\"agent\",stage:\"deferred\")");
    assert_eq!(deferred.args().len(), 6);
    assert!(
        deferred
            .args()
            .iter()
            .all(|r| field(r, "executable") == &Expr::sym(B::FALSE))
    );
}
#[test]
fn current_function_identities_and_unknown_host_capabilities_remain_truthful() {
    let mut ev = Evaluator::new();
    let current = eval(&mut ev, "functions()");
    let names: Vec<_> = current
        .args()
        .iter()
        .map(|r| field(r, "name").clone())
        .collect();
    assert!(names.contains(&Expr::string("explore")));
    assert!(!names.contains(&Expr::string("scene")));
    assert!(!names.contains(&Expr::string("apply_notebook_patch")));
    assert!(
        current
            .args()
            .iter()
            .all(|r| field(r, "executable") == &Expr::sym(B::TRUE))
    );
    let caps = eval(&mut ev, "capabilities()");
    assert_eq!(
        field(&caps, "metadata_version"),
        &Expr::int(om_core::catalog::METADATA_VERSION.into())
    );
    assert_eq!(field(&caps, "host_presentation"), &Expr::sym(B::NULL));
    assert_eq!(field(&caps, "task_permissions"), &Expr::sym(B::NULL));
    assert_eq!(
        field(&caps, "function_ids").args().len(),
        current.args().len()
    );
}
#[test]
fn query_names_are_held_unknown_input_is_diagnosed_and_budget_is_real() {
    let mut ev = Evaluator::new();
    eval(&mut ev, "let n=0");
    ev.messages.take();
    eval(&mut ev, "help(sequence(assign(n,1),map))");
    assert_eq!(eval(&mut ev, "n"), Expr::int(0));
    assert!(!ev.messages.take().is_empty());
    for source in [
        "functions(category:\"not-a-category\")",
        "help(\"unregistered-name\")",
    ] {
        ev.messages.take();
        eval(&mut ev, source);
        assert!(!ev.messages.take().is_empty());
    }
    let source = om_parse::parse_expr("functions()", om_parse::Dialect::Modern).unwrap();
    let ctx = Interrupt::default();
    ctx.steps_left.set(20);
    assert!(ev.evaluate(&source, &ctx).is_err());
}
#[test]
fn string_name_bindings_are_read_safely_and_shadowed_aliases_are_not_misreported() {
    let mut ev = Evaluator::new();
    eval(&mut ev, "let name=\"map\"");
    assert_eq!(
        field(&eval(&mut ev, "help(name)"), "id"),
        &Expr::string("fn_000063")
    );
    let sym = om_core::Symbol::intern("quantity");
    ev.evaluate(
        &Expr::call(B::SET, [Expr::sym(sym), Expr::int(3)]),
        &Interrupt::default(),
    )
    .unwrap();
    ev.messages.take();
    eval(&mut ev, "help(\"quantity\")");
    assert!(!ev.messages.take().is_empty());
    assert_eq!(
        field(&eval(&mut ev, "help(\"Quantity\")"), "executable"),
        &Expr::sym(B::TRUE)
    );
}
