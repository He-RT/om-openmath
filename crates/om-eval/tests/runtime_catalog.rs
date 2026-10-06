//! Runtime descriptors are checked against the actual callback specifications.
use om_core::{Symbol, catalog};
use om_eval::{Arity, Attributes as A, Evaluator};

#[test]
fn runtime_schemas_match_every_actual_callback() {
    for spec in Evaluator::all_specs() {
        let entry = catalog::by_runtime(spec.symbol.name()).expect("actual callback metadata");
        let (min, max) = match spec.arity {
            Arity::Exactly(n) => (n as u32, Some(n as u32)),
            Arity::Range(a, b) => (a as u32, Some(b as u32)),
            Arity::AtLeast(n) => (n as u32, None),
            Arity::Any => (0, None),
        };
        assert_eq!(
            (entry.arity.min, entry.arity.max),
            (min, max),
            "{}",
            spec.symbol.name()
        );
        for (name, attribute) in [
            ("hold_all", A::HOLD_ALL),
            ("hold_first", A::HOLD_FIRST),
            ("hold_rest", A::HOLD_REST),
            ("listable", A::LISTABLE),
            ("protected", A::PROTECTED),
        ] {
            assert_eq!(
                entry.attributes.iter().any(|s| s == name),
                spec.attrs.contains(attribute),
                "{} {name}",
                spec.symbol.name()
            );
        }
    }
    assert_eq!(catalog::functions().len(), Evaluator::all_specs().count());
    assert!(catalog::by_alias("fit").is_none());
    assert!(catalog::by_alias("apply_notebook_patch").is_none());
    assert_eq!(catalog::by_alias("map").unwrap().pipe_arg, 2);
    assert_eq!(catalog::by_alias("algebraic_root").unwrap().id, "fn_000090");
    assert_eq!(catalog::by_alias("solve").unwrap().name, "Solve");
    assert!(Evaluator::doc(Symbol::intern("Solve")).is_some());
}

#[test]
fn per_call_steps_use_actual_recording_without_changing_session_defaults() {
    let mut evaluator = Evaluator::new();
    let context = om_core::Interrupt::default();
    let source =
        om_parse::parse_expr("solve(x^2=4,x,steps:false)", om_parse::Dialect::Modern).unwrap();
    evaluator.evaluate(&source, &context).unwrap();
    assert!(evaluator.last_steps.is_none());
    assert!(evaluator.settings.record_steps);
    let source =
        om_parse::parse_expr("solve(x^2=4,x,steps:true)", om_parse::Dialect::Modern).unwrap();
    evaluator.evaluate(&source, &context).unwrap();
    assert!(
        evaluator
            .last_steps
            .as_ref()
            .is_some_and(|s| !s.root.is_empty())
    );
    assert!(evaluator.settings.record_steps);
}

#[test]
fn every_preferred_invocation_is_real_modern_source_for_the_claimed_callback() {
    for entry in om_core::catalog::functions() {
        let args = vec!["1"; entry.arity.min as usize].join(",");
        let source = format!("{}({args})", entry.modern_name);
        let parsed = om_parse::parse_expr(&source, om_parse::Dialect::Modern)
            .unwrap_or_else(|d| panic!("{source}: {d:?}"));
        assert_eq!(
            parsed.head_symbol(),
            Some(Symbol::intern(&entry.name)),
            "{source}"
        );
    }
}
