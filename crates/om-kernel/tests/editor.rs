//! Editor protocol uses real parsing/docs/definitions without executing source.
pub mod support;
use om_kernel::{
    Session,
    config::{Constants, Language},
    protocol::*,
};
use std::sync::atomic::Ordering;
use support::*;
fn preview(s: &mut Session, src: &str, d: Dialect, cursor: Option<u32>) -> PreviewResult {
    let (r, e) = s.handle(Request::Preview {
        source: src.into(),
        dialect: d,
        cursor,
    });
    assert!(e.is_empty());
    let Response::Preview(p) = r else {
        panic!("{r:?}")
    };
    p
}
fn complete(
    s: &mut Session,
    src: &str,
    d: Dialect,
    cursor: u32,
) -> (Vec<CompletionItem>, u32, u32) {
    let (r, e) = s.handle(Request::Complete {
        source: src.into(),
        dialect: d,
        cursor,
    });
    assert!(e.is_empty());
    let Response::Completions { items, from, to } = r else {
        panic!("{r:?}")
    };
    (items, from, to)
}
fn hover(s: &mut Session, src: &str, d: Dialect, cursor: u32) -> Option<HoverInfo> {
    let (r, e) = s.handle(Request::Hover {
        source: src.into(),
        dialect: d,
        cursor,
    });
    assert!(e.is_empty());
    let Response::Hover { info } = r else {
        panic!("{r:?}")
    };
    info
}
#[test]
fn preview_preserves_actual_diagnostics_tokens_raw_poles_and_executable_actions() {
    let mut s = sequential();
    let src = "(x^2-1)/(x-1)=0";
    let p = preview(&mut s, src, Dialect::Modern, None);
    assert_eq!(p.dialect, Dialect::Modern);
    assert!(p.latex.is_some());
    assert_eq!(p.actions.len(), 1);
    assert!(p.actions[0].source.contains("(x-1)"));
    assert!(
        p.tokens
            .iter()
            .all(|(span, _)| span.end as usize <= src.len())
    );
    let o = output(&mut s, "action", &p.actions[0].source, Dialect::Modern);
    same(&expressions(&o)[0].1, "{{x->-1}}");
    let p = preview(&mut s, "α  (x)", Dialect::Modern, None);
    assert!(p.diagnostics.iter().any(|d| d.code == "W001"));
    let p = preview(&mut s, "solve(x^2=2", Dialect::Modern, None);
    assert!(p.latex.is_none() && p.actions.is_empty());
    assert!(p.diagnostics.iter().any(|d| d.fix.is_some()));
    let p = preview(&mut s, "x+y+z+α=1", Dialect::Modern, None);
    assert_eq!(p.actions.len(), 3);
    let p = preview(&mut s, "x^2==4", Dialect::Wolfram, None);
    assert!(p.actions[0].source.starts_with("Solve["));
}
#[test]
fn preview_selection_empty_input_and_source_effects_do_not_run_or_reset_state() {
    let mut s = sequential();
    output(&mut s, "a", "let a=2", Dialect::Modern);
    s.handle(Request::Interrupt);
    for src in ["let a=99", "Clear[a]", "Solve[x^1000==1,x]", "a:=a+1"] {
        preview(&mut s, src, Dialect::Wolfram, None);
        assert!(s.interrupt_handle().load(Ordering::Relaxed));
    }
    assert_eq!(s.notebook.cells.len(), 1);
    assert_eq!(s.notebook.cells[0].exec_count, Some(1));
    assert!(
        preview(&mut s, "# comment", Dialect::Modern, None)
            .latex
            .is_none()
    );
    assert_eq!(
        preview(&mut s, "x+1\ny+2", Dialect::Modern, Some(1)).latex,
        Some(om_format::latex(&om_core::canonicalize(
            &om_parse::parse_expr("x+1", om_parse::Dialect::Modern).unwrap()
        )))
    );
    assert_eq!(
        preview(&mut s, "x+1\ny+2", Dialect::Modern, None).latex,
        Some(om_format::latex(&om_core::canonicalize(
            &om_parse::parse_expr("y+2", om_parse::Dialect::Modern).unwrap()
        )))
    );
    assert_eq!(
        expressions(&output(&mut s, "probe", "a", Dialect::Wolfram))[0],
        (2, "2".into())
    );
}
#[test]
fn completion_aliases_snippets_unicode_ranges_and_context_options_are_executable() {
    let mut s = sequential();
    let (items, from, to) = complete(&mut s, "sol", Dialect::Modern, 3);
    assert_eq!((from, to), (0, 3));
    assert!(items.iter().any(|i| i.label == "solve"));
    assert!(
        items
            .iter()
            .any(|i| i.kind == CompletionKind::Snippet && i.insert_text.contains("${1:equation}"))
    );
    for (prefix, label) in [
        ("asi", "asin"),
        ("find_", "find_root"),
        ("imp", "implicitplot"),
        ("polynomialg", "polynomialgcd"),
    ] {
        let (items, _, _) = complete(&mut s, prefix, Dialect::Modern, prefix.len() as u32);
        assert!(
            items.iter().any(|i| i.label == label),
            "{prefix}: {items:?}"
        );
    }
    let (items, _, _) = complete(&mut s, "Sol", Dialect::Wolfram, 3);
    assert!(items.iter().any(|i| i.label == "Solve"));
    output(&mut s, "α", "let αbeta=2", Dialect::Modern);
    let (items, from, to) = complete(&mut s, "αbeta+1", Dialect::Modern, 2);
    assert_eq!((from, to), (0, 6));
    assert!(items.iter().any(|i| i.label == "αbeta"));
    let (items, from, _) = complete(&mut s, "solve(sin(x), dom", Dialect::Modern, 17);
    assert!(
        items
            .iter()
            .any(|i| i.kind == CompletionKind::Keyword && i.insert_text == "domain: ")
    );
    assert_eq!(from, 14);
    for src in [
        "sin(dom",
        "solve([dom",
        "# solve(x, dom",
        "\"solve(x, dom\"",
    ] {
        let (items, _, _) = complete(&mut s, src, Dialect::Modern, src.len() as u32);
        assert!(
            !items
                .iter()
                .any(|i| i.kind == CompletionKind::Keyword && i.label == "domain"),
            "{src}"
        );
    }
}
#[test]
fn hover_actual_docs_and_stored_delayed_definitions_keep_language_and_live_ownership() {
    let mut s = sequential();
    let h = hover(&mut s, "sin(x)", Dialect::Modern, 1).unwrap();
    assert!(h.signature.unwrap().starts_with("sin("));
    assert!(!h.summary.is_empty());
    assert!(!h.examples.is_empty());
    s.config.general.language = Language::En;
    let h = hover(&mut s, "Solve[x^2==2,x]", Dialect::Wolfram, 2).unwrap();
    assert!(h.summary.is_ascii());
    assert!(h.signature.unwrap().starts_with("Solve["));
    output(&mut s, "a", "a=2", Dialect::Wolfram);
    let h = hover(&mut s, "a", Dialect::Wolfram, 1).unwrap();
    assert_eq!(h.value.as_deref(), Some("2"));
    assert_eq!(h.cell_id.as_deref(), Some("a"));
    output(&mut s, "a", "a:=counter=counter+1", Dialect::Wolfram);
    let h = hover(&mut s, "a", Dialect::Wolfram, 0).unwrap();
    assert!(h.value.unwrap().contains("counter"));
    assert!(hover(&mut s, "counter", Dialect::Wolfram, 1).is_none());
    output(&mut s, "f", "f[x_]:=x^2", Dialect::Wolfram);
    let h = hover(&mut s, "f[2]", Dialect::Wolfram, 0).unwrap();
    assert_eq!(
        h.value,
        Some(om_format::input_form(
            &om_parse::parse_expr("f[x_]:=x^2", om_parse::Dialect::Wolfram).unwrap()
        ))
    );
    assert_eq!(h.cell_id.as_deref(), Some("f"));
    s.handle(Request::UpsertCell {
        cell: CellInput {
            id: "f".into(),
            kind: CellKind::Math,
            source: "prose".into(),
            dialect: Dialect::Modern,
        },
    });
    assert_eq!(
        hover(&mut s, "f", Dialect::Wolfram, 1)
            .unwrap()
            .cell_id
            .as_deref(),
        Some("f")
    );
}
#[test]
fn byte_cursor_validation_is_atomic_and_constants_follow_configuration() {
    let mut s = sequential();
    for req in [
        Request::Preview {
            source: "α".into(),
            dialect: Dialect::Modern,
            cursor: Some(1),
        },
        Request::Complete {
            source: "α".into(),
            dialect: Dialect::Modern,
            cursor: 1,
        },
        Request::Hover {
            source: "α".into(),
            dialect: Dialect::Modern,
            cursor: 9,
        },
    ] {
        assert!(
            matches!(s.handle(req).0,Response::Error{message} if message.contains("err.cursor"))
        );
    }
    assert!(s.notebook.cells.is_empty());
    s.config.general.constants = Constants::Strict;
    let p = preview(&mut s, "e=1", Dialect::Modern, None);
    assert_eq!(p.actions.len(), 1);
    assert!(p.actions[0].source.contains("e"));
    s.config.general.constants = Constants::Math;
    assert!(
        preview(&mut s, "e=1", Dialect::Modern, None)
            .actions
            .is_empty()
    );
}
