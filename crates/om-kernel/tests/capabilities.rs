//! Catalog and capability queries expose only actual behavior, without touching session state.
use om_kernel::{KernelConfig, Session, protocol::*};
use std::collections::BTreeSet;

#[test]
fn catalog_is_versioned_deterministic_and_callback_filtered() {
    let mut s = Session::new(KernelConfig::default(), None);
    let (response, events) = s.handle(Request::GetFunctionCatalog);
    assert!(events.is_empty());
    let Response::FunctionCatalog { catalog } = response else {
        panic!("{response:?}");
    };
    assert_eq!(catalog.metadata_version, om_core::catalog::METADATA_VERSION);
    assert_eq!(
        catalog.functions.len(),
        om_eval::Evaluator::all_specs().count()
    );
    let expected: BTreeSet<_> = om_eval::Evaluator::all_docs().map(|d| d.name).collect();
    assert_eq!(
        catalog
            .functions
            .iter()
            .map(|f| f.name.as_str())
            .collect::<BTreeSet<_>>(),
        expected
    );
    assert!(
        !catalog
            .functions
            .iter()
            .any(|f| matches!(f.modern_name.as_str(), "ode" | "apply_notebook_patch"))
    );
    let solve = catalog
        .functions
        .iter()
        .find(|f| f.name == "Solve")
        .unwrap();
    assert_eq!(solve.id, "fn_000117");
    assert!(
        solve
            .options
            .iter()
            .any(|p| p.name == "cubics" && p.default_source.as_deref() == Some("False"))
    );
    assert!(!solve.options.iter().any(|p| p.name == "precision"));
    let nsolve = catalog
        .functions
        .iter()
        .find(|f| f.name == "NSolve")
        .unwrap();
    assert_eq!(nsolve.id, solve.id);
    assert!(
        nsolve
            .options
            .iter()
            .any(|p| p.name == "precision" && p.min == Some(5.0))
    );
    let json = serde_json::to_string(&Response::FunctionCatalog { catalog }).unwrap();
    let round: Response = serde_json::from_str(&json).unwrap();
    assert_eq!(json, serde_json::to_string(&round).unwrap());
}

#[test]
fn capabilities_separate_kernel_presentation_and_future_permissions() {
    let mut s = Session::new(KernelConfig::default(), None);
    let cancel = s.interrupt_handle();
    cancel.store(true, std::sync::atomic::Ordering::Relaxed);
    for platform in [
        HostPlatform::Cli,
        HostPlatform::Desktop,
        HostPlatform::Web,
        HostPlatform::Ios,
    ] {
        let request = Request::GetCapabilities { platform };
        let wire = serde_json::to_string(&request).unwrap();
        let (response, events) = s.handle(serde_json::from_str(&wire).unwrap());
        assert!(events.is_empty());
        let Response::Capabilities { capabilities: c } = response else {
            panic!("{response:?}")
        };
        assert!(!c.scene_3d);
        assert!(c.task_permissions.is_none());
        assert_eq!(c.kernel_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(c.function_ids.len(), 200);
        assert!(
            !c.rendered_outputs
                .iter()
                .any(|s| matches!(s.as_str(), "scene_3d" | "table" | "interpolation"))
        );
    }
    assert!(s.notebook.cells.is_empty());
    assert!(cancel.load(std::sync::atomic::Ordering::Relaxed));
    let (reply, _) = s.handle(Request::SaveNotebook);
    let Response::Notebook { file } = reply else {
        panic!("{reply:?}")
    };
    assert_eq!(
        serde_json::to_value(file).unwrap()["cells"],
        serde_json::json!([])
    );
}

#[test]
fn aliases_have_real_completion_hover_and_user_bindings_still_win() {
    let mut s = Session::new(KernelConfig::default(), None);
    let (r, _) = s.handle(Request::Complete {
        source: "polynomial_".into(),
        cursor: 11,
        dialect: Dialect::Modern,
    });
    let Response::Completions { items, .. } = r else {
        panic!("{r:?}")
    };
    assert!(items.iter().any(|i| i.label == "polynomial_gcd"));
    let (r, _) = s.handle(Request::Hover {
        source: "polynomial_gcd(x,x^2)".into(),
        cursor: 3,
        dialect: Dialect::Modern,
    });
    let Response::Hover { info: Some(info) } = r else {
        panic!("{r:?}")
    };
    assert!(info.signature.unwrap().starts_with("polynomial_gcd("));
    let (r, _) = s.handle(Request::Complete {
        source: "sin(x, ".into(),
        cursor: 7,
        dialect: Dialect::Modern,
    });
    let Response::Completions { items, .. } = r else {
        panic!("{r:?}")
    };
    assert!(!items.iter().any(|i| i.label == "method"));
}

#[test]
fn existing_user_alias_function_remains_callable_redefinable_and_inspectable() {
    let mut s = Session::new(KernelConfig::default(), None);
    for (id, source, dialect, expected) in [
        ("d", "let polynomial_gcd(x)=x+10", Dialect::Modern, None),
        ("q", "polynomial_gcd(3)", Dialect::Modern, Some("13")),
        ("d", "let polynomial_gcd(x)=x+20", Dialect::Modern, None),
        ("q", "polynomial_gcd(3)", Dialect::Modern, Some("23")),
    ] {
        let (r, _) = s.handle(Request::Evaluate {
            cell_id: id.into(),
            source: source.into(),
            dialect,
        });
        let Response::Evaluated { output, .. } = r else {
            panic!("{r:?}")
        };
        if let Some(expected) = expected {
            assert!(
                output
                    .items
                    .iter()
                    .any(|i| matches!(i,OutputItem::Expr {input_form,..} if input_form==expected)),
                "{output:?}"
            );
        }
    }
    let (r, _) = s.handle(Request::Hover {
        source: "polynomial_gcd(3)".into(),
        cursor: 3,
        dialect: Dialect::Modern,
    });
    let Response::Hover { info: Some(info) } = r else {
        panic!("{r:?}")
    };
    assert_eq!(info.cell_id.as_deref(), Some("d"));
    assert!(info.value.unwrap().contains("20"));
}
