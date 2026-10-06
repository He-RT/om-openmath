//! Independent isolated exploration, immutable context and source-only notebook contracts.
use om_kernel::{Session, protocol::*};
use std::collections::BTreeMap;
fn eval(s: &mut Session, id: &str, source: &str) -> CellOutput {
    let r = s
        .handle(Request::Evaluate {
            cell_id: id.into(),
            source: source.into(),
            dialect: Dialect::Modern,
        })
        .0;
    let Response::Evaluated { output, .. } = r else {
        panic!("{r:?}")
    };
    output
}
fn query(output: &CellOutput, id: &str) -> ExploreQuery {
    let OutputItem::Explore {
        out_index, view_id, ..
    } = &output.items[0]
    else {
        panic!("{output:?}")
    };
    assert!(output.messages.is_empty(), "{output:?}");
    ExploreQuery {
        cell_id: id.into(),
        out_index: *out_index,
        view_id: view_id.clone(),
    }
}
fn point(v: f64) -> BTreeMap<String, f64> {
    BTreeMap::from([("a".into(), v)])
}
fn expr(item: &OutputItem) -> &str {
    let OutputItem::Expr { input_form, .. } = item else {
        panic!("{item:?}")
    };
    input_form
}
#[test]
fn real_scalar_matrix_curve_and_main_bindings_are_independently_checked() {
    let mut s = Session::new(Default::default(), None);
    eval(&mut s, "main", "let a=99;let x=88;let rate=2");
    let o = eval(
        &mut s,
        "e",
        "explore([[a,a^2],[rate,a+rate]],controls:{a:0..4})",
    );
    let q = query(&o, "e");
    let OutputItem::Explore { result, .. } = &o.items[0] else {
        panic!()
    };
    assert_eq!(expr(&result.item), "{{2., 4.}, {2, 4.}}");
    let Response::Explored { result } = s
        .handle(Request::SampleExplore {
            query: q,
            values: point(3.),
            revision: 4,
        })
        .0
    else {
        panic!()
    };
    assert_eq!(result.revision, 4);
    assert_eq!(expr(&result.item), "{{3., 9.}, {2, 5.}}");
    assert_eq!(
        expr(&eval(&mut s, "probe", "[a,x,rate]").items[0]),
        "{99, 88, 2}"
    );
    let o = eval(
        &mut s,
        "curve",
        "explore(plot(a*x,x:0..1),controls:{a:1..4},initial:{a:3})",
    );
    let q = query(&o, "curve");
    let Response::Explored { result } = s
        .handle(Request::SampleExplore {
            query: q,
            values: point(4.),
            revision: 5,
        })
        .0
    else {
        panic!()
    };
    let OutputItem::Plot { data, .. } = &*result.item else {
        panic!()
    };
    for (x, y) in data.curves.iter().flat_map(|c| &c.segments).flatten() {
        assert!((y - 4. * x).abs() < 1e-12);
    }
}
#[test]
fn context_import_has_frozen_real_definitions_random_history_and_no_credentials_or_replay() {
    let mut s = Session::new(Default::default(), None);
    eval(&mut s, "defs", "let rate=2;let f(x)=rate*x;seed_random(42)");
    let o = eval(&mut s, "e", "explore(f(a)+rate,controls:{a:0..4})");
    let q = query(&o, "e");
    let Response::ExploreContext { context, .. } =
        s.handle(Request::GetExploreContext { query: q.clone() }).0
    else {
        panic!()
    };
    assert!(
        !context.contains("api_key") && !context.contains("llm") && !context.contains("profiles")
    );
    eval(&mut s, "defs", "let rate=20;let f(x)=rate*x");
    let mut worker = Session::new(Default::default(), None);
    let Response::Explored { result } = worker
        .handle(Request::RunExploreContext {
            context: context.clone(),
            values: point(3.),
            revision: 8,
        })
        .0
    else {
        panic!()
    };
    assert_eq!(expr(&result.item), "8.");
    assert!(worker.notebook.cells.is_empty());
    assert!(matches!(
        s.handle(Request::SampleExplore {
            query: q,
            values: point(3.),
            revision: 9
        })
        .0,
        Response::Error { .. }
    ));
    assert_eq!(expr(&eval(&mut worker, "outside", "rate").items[0]), "rate");
    let Response::Notebook { file } = s.handle(Request::SaveNotebook).0 else {
        panic!()
    };
    let file = serde_json::to_string(&file).unwrap();
    assert!(!file.contains("context") && !file.contains("view_id"));
}
#[test]
fn invalid_controls_writes_overflow_parameters_and_stale_ids_do_not_succeed() {
    let mut s = Session::new(Default::default(), None);
    for source in [
        "explore(a,controls:{a:1..1})",
        "explore(a,controls:{a:0..4},initial:{a:5})",
        "explore(a,controls:{a:0..4},initial:{b:2})",
        "explore(a,controls:{sin:0..4})",
        "explore(assign(z,9),controls:{a:0..4})",
        "explore(explore(a,controls:{a:0..4}),controls:{b:0..1})",
    ] {
        let output = eval(&mut s, "bad", source);
        assert!(
            !output.messages.is_empty()
                || output
                    .items
                    .iter()
                    .any(|i| matches!(i, OutputItem::Error { .. })),
            "{source}: {output:?}"
        );
    }
    let o = eval(&mut s, "e", "explore(a^2,controls:{a:0..4})");
    let mut q = query(&o, "e");
    for values in [
        point(-1.),
        point(f64::NAN),
        BTreeMap::new(),
        BTreeMap::from([("b".into(), 2.)]),
    ] {
        assert!(matches!(
            s.handle(Request::SampleExplore {
                query: q.clone(),
                values,
                revision: 1
            })
            .0,
            Response::Error { .. }
        ));
    }
    q.view_id = "wrong".into();
    assert!(matches!(
        s.handle(Request::GetExploreContext { query: q }).0,
        Response::Error { .. }
    ));
    assert_eq!(expr(&eval(&mut s, "probe", "z").items[0]), "z");
}

#[test]
fn graph_context_preserves_precision_holes_callable_heads_and_random_without_replaying_assignments()
{
    let mut s = Session::new(Default::default(), None);
    eval(
        &mut s,
        "state",
        "let high=decimal(\"0.1\",precision:50); let f(x)=1/(x-1); seed_random(42)",
    );
    for source in [
        "explore(high+a,controls:{a:0..4})",
        "explore([random_uniform(),a],controls:{a:0..4})",
        "explore(plot(f(x)+a,x:0..2),controls:{a:0..4})",
    ] {
        let output = eval(&mut s, "e", source);
        let q = query(&output, "e");
        let Response::ExploreContext { context, .. } =
            s.handle(Request::GetExploreContext { query: q.clone() }).0
        else {
            panic!()
        };
        let Response::Explored { result: a } = s
            .handle(Request::SampleExplore {
                query: q,
                values: point(2.),
                revision: 3,
            })
            .0
        else {
            panic!()
        };
        let Response::Explored { result: b } = Session::new(Default::default(), None)
            .handle(Request::RunExploreContext {
                context: context.clone(),
                values: point(2.),
                revision: 3,
            })
            .0
        else {
            panic!()
        };
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            serde_json::to_value(&b).unwrap(),
            "{source}"
        );
        if let OutputItem::Plot { request, data } = &*a.item {
            assert!(data.curves[0].segments.len() >= 2);
            let mut request = request.clone();
            request.x_range = (0., 0.8);
            request.y_range = Some((-8., 8.));
            let Response::Plot { data } = Session::new(Default::default(), None)
                .handle(Request::SampleExplorePlot {
                    context,
                    values: point(2.),
                    request,
                })
                .0
            else {
                panic!()
            };
            for (x, y) in data.curves[0].segments.iter().flatten() {
                assert!((y - (1. / (x - 1.) + 2.)).abs() < 1e-10);
            }
        }
    }
    let a = expr(&eval(&mut s, "random", "random_uniform()").items[0]).to_string();
    let mut reference = Session::new(Default::default(), None);
    eval(&mut reference, "seed", "seed_random(42)");
    assert_eq!(
        a,
        expr(&eval(&mut reference, "random", "random_uniform()").items[0])
    );
}
#[test]
fn frozen_graph_rejects_corrupt_references_versions_limits_and_readonly_writes() {
    let mut s = Session::new(Default::default(), None);
    let o = eval(&mut s, "e", "explore(a^2,controls:{a:0..4})");
    let q = query(&o, "e");
    let Response::ExploreContext { context, .. } =
        s.handle(Request::GetExploreContext { query: q }).0
    else {
        panic!()
    };
    let mut bad: serde_json::Value = serde_json::from_str(&context).unwrap();
    for field in ["version", "source"] {
        let mut value = bad.clone();
        value[field] = 999999.into();
        let r = Session::new(Default::default(), None)
            .handle(Request::RunExploreContext {
                context: value.to_string(),
                values: point(2.),
                revision: 1,
            })
            .0;
        assert!(matches!(r, Response::Error { .. }));
    }
    bad["nodes"][0] = serde_json::json!({"Normal":{"head":0,"args":[0]}});
    assert!(matches!(
        Session::new(Default::default(), None)
            .handle(Request::RunExploreContext {
                context: bad.to_string(),
                values: point(2.),
                revision: 1
            })
            .0,
        Response::Error { .. }
    ));
    assert!(matches!(
        Session::new(Default::default(), None)
            .handle(Request::InspectExploreExpression {
                context,
                values: point(2.),
                source: "Set[leak,9]".into(),
                numeric: false
            })
            .0,
        Response::Error { .. }
    ));
}

#[test]
fn previous_output_history_and_local_source_dependencies_remain_real_and_source_only() {
    let mut s = Session::new(Default::default(), None);
    eval(&mut s, "prior", "7");
    let output = eval(&mut s, "e", "explore(a+out(),controls:{a:0..4})");
    let OutputItem::Explore { result, .. } = &output.items[0] else {
        panic!("{output:?}")
    };
    assert_eq!(expr(&result.item), "9.");
    let mut s = Session::new(Default::default(), None);
    eval(&mut s, "a", "let a=99");
    eval(&mut s, "rate", "let rate=2");
    let output = eval(&mut s, "e", "explore(a+rate,controls:{a:0..4})");
    let q = query(&output, "e");
    let e = s.notebook.cells.iter().find(|c| c.id == "e").unwrap();
    assert!(!e.uses.iter().any(|s| s.name() == "a"));
    assert!(e.uses.iter().any(|s| s.name() == "rate"));
    eval(&mut s, "a", "let a=100");
    assert!(matches!(
        s.handle(Request::SampleExplore {
            query: q.clone(),
            values: point(3.),
            revision: 1
        })
        .0,
        Response::Explored { .. }
    ));
    eval(&mut s, "rate", "let rate=5");
    assert!(matches!(
        s.handle(Request::SampleExplore {
            query: q,
            values: point(3.),
            revision: 2
        })
        .0,
        Response::Error { .. }
    ));
}
#[test]
fn cancellation_inside_detached_sampling_does_not_return_partial_success_and_next_task_recovers() {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    struct Cancel {
        enabled: AtomicBool,
        calls: AtomicUsize,
        flag: Mutex<Option<Arc<AtomicBool>>>,
    }
    impl om_core::Clock for Cancel {
        fn now_ms(&self) -> f64 {
            if self.enabled.load(Ordering::Relaxed)
                && self.calls.fetch_add(1, Ordering::Relaxed) > 2
                && let Some(f) = self.flag.lock().unwrap().as_ref()
            {
                f.store(true, Ordering::Relaxed);
            }
            0.
        }
    }
    let mut main = Session::new(Default::default(), None);
    let output = eval(
        &mut main,
        "e",
        "explore(region_plot(x^2+y^2<a,x:-2..2,y:-2..2),controls:{a:0.1..4})",
    );
    let q = query(&output, "e");
    let Response::ExploreContext { context, .. } =
        main.handle(Request::GetExploreContext { query: q }).0
    else {
        panic!()
    };
    let clock = Arc::new(Cancel {
        enabled: AtomicBool::new(true),
        calls: AtomicUsize::new(0),
        flag: Mutex::new(None),
    });
    let mut worker = Session::new(Default::default(), Some(clock.clone()));
    *clock.flag.lock().unwrap() = Some(worker.interrupt_handle());
    assert!(
        matches!(worker.handle(Request::RunExploreContext{context:context.clone(),values:point(1.),revision:1}).0,Response::Error{message}if message.contains("$Aborted"))
    );
    clock.enabled.store(false, Ordering::Relaxed);
    assert!(matches!(
        worker
            .handle(Request::RunExploreContext {
                context,
                values: point(2.),
                revision: 2
            })
            .0,
        Response::Explored { .. }
    ));
    assert!(worker.notebook.cells.is_empty());
}

#[test]
fn a_user_function_binding_keeps_precedence_over_the_new_modern_alias() {
    let mut s = Session::new(Default::default(), None);
    let output = eval(&mut s, "function", "let explore(x)=x+10");
    assert!(output.messages.is_empty(), "{output:?}");
    assert_eq!(expr(&eval(&mut s, "call", "explore(3)").items[0]), "13");
}
