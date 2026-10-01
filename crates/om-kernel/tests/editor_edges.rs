//! Lexical boundaries, deterministic ranking and ownership lifecycle editor regressions.
pub mod support;
use om_kernel::{Session, protocol::*};
use std::sync::atomic::Ordering;
use support::*;
fn complete(s: &mut Session, src: &str, cursor: u32) -> Vec<CompletionItem> {
    let Response::Completions { items, .. } = s
        .handle(Request::Complete {
            source: src.into(),
            dialect: Dialect::Modern,
            cursor,
        })
        .0
    else {
        panic!()
    };
    items
}
fn hover(s: &mut Session, src: &str, cursor: u32) -> Option<HoverInfo> {
    let Response::Hover { info } = s
        .handle(Request::Hover {
            source: src.into(),
            dialect: Dialect::Modern,
            cursor,
        })
        .0
    else {
        panic!()
    };
    info
}
#[test]
fn completion_ranks_prefix_initials_subsequence_and_is_stable_capped_at_fifty() {
    let mut s = sequential();
    let defs = (0..80)
        .map(|i| format!("let editor_symbol_{i:02}={i}"))
        .collect::<Vec<_>>()
        .join("\n");
    output(&mut s, "defs", &defs, Dialect::Modern);
    let items = complete(&mut s, "editor", 6);
    assert_eq!(items.len(), 50);
    assert!(items.iter().all(|i| i.label.starts_with("editor_symbol_")));
    let again = complete(&mut s, "editor", 6);
    assert_eq!(
        serde_json::to_value(&items).unwrap(),
        serde_json::to_value(again).unwrap()
    );
    let items = complete(&mut s, "pgc", 3);
    assert!(items.iter().any(|i| i.label == "polynomialgcd"));
    let items = complete(&mut s, "sov", 3);
    assert!(items.iter().any(|i| i.label == "solve"));
    assert!(complete(&mut s, "", 0).len() <= 50);
    assert_eq!(s.notebook.cells[0].exec_count, Some(80));
}
#[test]
fn options_remain_in_current_calls_and_comments_strings_part_and_groupings_do_not_leak() {
    let mut s = sequential();
    output(&mut s, "a", "let a=2", Dialect::Modern);
    for src in [
        "solve (x, dom",
        "solve(x), dom",
        "solve((dom",
        "solve(sin(dom",
        "solve(x, \"dom",
        "solve(x, # dom",
    ] {
        let items = complete(&mut s, src, src.len() as u32);
        assert!(
            !items.iter().any(|i| i.label == "domain"),
            "{src}: {items:?}"
        );
    }
    let src = "solve(sin(x), dom";
    assert!(
        complete(&mut s, src, src.len() as u32)
            .iter()
            .any(|i| i.label == "domain")
    );
    for src in ["\"abc\"", "# abc", "12.34", "x_", "a\"text\""] {
        assert!(
            hover(&mut s, src, if src.starts_with('a') { 1 } else { 0 }).is_none(),
            "{src}"
        );
    }
    let src = "Solve[[x, Wor";
    let Response::Completions { items, .. } = s
        .handle(Request::Complete {
            source: src.into(),
            dialect: Dialect::Wolfram,
            cursor: src.len() as u32,
        })
        .0
    else {
        panic!()
    };
    assert!(!items.iter().any(|i| i.label == "WorkingPrecision"));
}
#[test]
fn hover_uses_actual_ownvalue_precedence_truncates_unicode_and_tracks_clear_transfer_delete() {
    let mut s = sequential();
    output(&mut s, "sin", "let sin=7", Dialect::Modern);
    assert_eq!(hover(&mut s, "sin", 1).unwrap().value.as_deref(), Some("7"));
    assert!(hover(&mut s, "sin(x)", 1).unwrap().value.is_none());
    let long = "α".repeat(260);
    output(
        &mut s,
        "long",
        &format!("long=\"{long}\""),
        Dialect::Wolfram,
    );
    let h = hover(&mut s, "long", 4).unwrap();
    let value = h.value.unwrap();
    assert_eq!(value.chars().count(), 200);
    assert!(value.ends_with('…'));
    output(&mut s, "a", "let a=2", Dialect::Modern);
    output(&mut s, "b", "let a=3", Dialect::Modern);
    assert_eq!(hover(&mut s, "a", 1).unwrap().cell_id.as_deref(), Some("b"));
    s.handle(Request::DeleteCell {
        cell_id: "a".into(),
    });
    assert_eq!(hover(&mut s, "a", 1).unwrap().value.as_deref(), Some("3"));
    output(&mut s, "clear", "Clear[a]", Dialect::Wolfram);
    assert!(hover(&mut s, "a", 1).is_none());
    s.handle(Request::Interrupt);
    complete(&mut s, "sol", 3);
    hover(&mut s, "sin", 1);
    assert!(s.interrupt_handle().load(Ordering::Relaxed));
}
#[test]
fn raw_preview_actions_preserve_comments_and_error_cells_do_not_display_partial_math() {
    let mut s = sequential();
    let src = "x (* preserve comment *)^2==4";
    let Response::Preview(p) = s
        .handle(Request::Preview {
            source: src.into(),
            dialect: Dialect::Wolfram,
            cursor: None,
        })
        .0
    else {
        panic!()
    };
    assert!(p.actions[0].source.contains("preserve comment"));
    let o = output(&mut s, "action", &p.actions[0].source, Dialect::Wolfram);
    same(&expressions(&o)[0].1, "{{x->-2},{x->2}}");
    let Response::Preview(p) = s
        .handle(Request::Preview {
            source: "x+1\nsolve(".into(),
            dialect: Dialect::Modern,
            cursor: Some(1),
        })
        .0
    else {
        panic!()
    };
    assert!(p.latex.is_none() && p.actions.is_empty());
    assert!(!p.tokens.is_empty());
}

#[test]
fn dialect_marker_is_comment_context_and_preview_cursor_before_math_selects_first_statement() {
    let mut s = sequential();
    output(&mut s, "wl", "let wl=2", Dialect::Modern);
    let src = "%wl\nSolve[x^2==2,x]";
    let Response::Hover { info } = s
        .handle(Request::Hover {
            source: src.into(),
            dialect: Dialect::Auto,
            cursor: 2,
        })
        .0
    else {
        panic!()
    };
    assert!(info.is_none());
    let Response::Completions { items, .. } = s
        .handle(Request::Complete {
            source: src.into(),
            dialect: Dialect::Auto,
            cursor: 2,
        })
        .0
    else {
        panic!()
    };
    assert!(items.is_empty());
    let Response::Preview(p) = s
        .handle(Request::Preview {
            source: "# comment\nx+1\ny+2".into(),
            dialect: Dialect::Modern,
            cursor: Some(0),
        })
        .0
    else {
        panic!()
    };
    assert_eq!(
        p.latex,
        Some(om_format::latex(&om_core::canonicalize(
            &om_parse::parse_expr("x+1", om_parse::Dialect::Modern).unwrap()
        )))
    );
}
#[test]
fn pure_function_parameters_do_not_become_free_axis_solve_actions() {
    let mut s = sequential();
    let Response::Preview(p) = s
        .handle(Request::Preview {
            source: "Function[t,t+a][x]==0".into(),
            dialect: Dialect::Wolfram,
            cursor: None,
        })
        .0
    else {
        panic!()
    };
    assert_eq!(p.actions.len(), 2);
    assert!(!p.actions.iter().any(|a| a.source.ends_with(", t]")));
    assert!(p.actions.iter().any(|a| a.source.ends_with(", x]")));
    assert!(p.actions.iter().any(|a| a.source.ends_with(", a]")));
}
