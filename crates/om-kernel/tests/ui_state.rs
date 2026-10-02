//! UI metadata/title/locale updates preserve genuine definitions and source-only storage.
use om_kernel::{KernelConfig, Session, config::Language, protocol::*};
#[test]
fn title_and_host_language_updates_do_not_reset_actual_definitions_or_history() {
    let mut s = Session::new(KernelConfig::default(), None);
    s.handle(Request::Evaluate {
        cell_id: "a".into(),
        source: "a=9;".into(),
        dialect: Dialect::Wolfram,
    });
    assert!(matches!(
        s.handle(Request::RenameNotebook {
            title: "My math".into()
        })
        .0,
        Response::Ok
    ));
    s.handle(Request::SetSystemLanguage {
        language: Language::En,
    });
    assert_eq!(s.config.general.language, Language::Auto);
    assert_eq!(s.effective_language(), Language::En);
    let Response::NotebookState { state } = s.handle(Request::GetNotebookState).0 else {
        panic!()
    };
    assert_eq!(state.file.title, "My math");
    assert_eq!(state.cells[0].exec_count, Some(1));
    let Response::Evaluated { output, .. } = s
        .handle(Request::Evaluate {
            cell_id: "read".into(),
            source: "a+1".into(),
            dialect: Dialect::Wolfram,
        })
        .0
    else {
        panic!()
    };
    assert!(matches!(&output.items[0],OutputItem::Expr {input_form,..}if input_form=="10"));
    let Response::Hover { info: Some(info) } = s
        .handle(Request::Hover {
            source: "a".into(),
            cursor: 1,
            dialect: Dialect::Wolfram,
        })
        .0
    else {
        panic!()
    };
    assert!(info.summary.is_ascii());
    s.config.general.language = Language::ZhCn;
    assert_eq!(s.effective_language(), Language::ZhCn);
    let Response::Variables { items } = s.handle(Request::GetVariables).0 else {
        panic!()
    };
    assert_eq!(items[0].0, "a");
    assert_eq!(items[0].1.value.as_deref(), Some("9"));
    s.handle(Request::Evaluate {
        cell_id: "clear".into(),
        source: "Clear[a]".into(),
        dialect: Dialect::Wolfram,
    });
    let Response::Variables { items } = s.handle(Request::GetVariables).0 else {
        panic!()
    };
    assert!(items.is_empty());
}
