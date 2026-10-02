//! Explanation targets the exact selected statement record, never another result's steps.
pub mod support;
use om_kernel::protocol::*;
use om_llm::JobInput;
use support::*;
#[test]
fn selected_output_explains_its_own_input_result_and_steps_without_mutation() {
    let mut s = sequential();
    let o = output(
        &mut s,
        "multi",
        "Solve[x^2==4,x]\nSolve[y^2==9,y]",
        Dialect::Wolfram,
    );
    let before = serde_json::to_value(s.handle(Request::GetNotebookState).0).unwrap();
    let (_, JobInput::Text { messages }) = s
        .prepare_llm_input(&Request::LlmExplain {
            request_id: "explain".into(),
            cell_id: "multi".into(),
            step_id: None,
            out_index: Some(1),
        })
        .unwrap()
    else {
        panic!()
    };
    let text = &messages[1].content;
    assert!(text.contains("x"));
    assert!(!text.contains("y^2"));
    assert_eq!(
        before,
        serde_json::to_value(s.handle(Request::GetNotebookState).0).unwrap()
    );
    assert!(
        s.prepare_llm_input(&Request::LlmExplain {
            request_id: "bad".into(),
            cell_id: "multi".into(),
            step_id: None,
            out_index: Some(999)
        })
        .is_err()
    );
    let OutputItem::Solutions {
        steps: Some(steps),
        out_index,
        ..
    } = &o.items[0]
    else {
        panic!()
    };
    let (_, JobInput::Text { messages }) = s
        .prepare_llm_input(&Request::LlmExplain {
            request_id: "selected".into(),
            cell_id: "multi".into(),
            out_index: Some(*out_index),
            step_id: Some(steps.root[0].id.clone()),
        })
        .unwrap()
    else {
        panic!()
    };
    assert!(messages[1].content.contains(&steps.root[0].id));
}
