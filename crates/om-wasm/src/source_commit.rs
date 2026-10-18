//! Browser queue adapter for the existing non-evaluating source-owner operation.
use om_kernel::{
    Session,
    protocol::{NotebookFile, Request, Response},
};

pub(super) fn commit(
    session: &mut Session,
    expected: &str,
    desired: &str,
) -> Result<String, String> {
    if expected.len() > 16 * 1024 * 1024 || desired.len() > 16 * 1024 * 1024 {
        return Err("Source file exceeds limit".into());
    }
    let expected: NotebookFile =
        serde_json::from_str(expected).map_err(|_| "Invalid expected source")?;
    let desired: NotebookFile =
        serde_json::from_str(desired).map_err(|_| "Invalid desired source")?;
    let (Response::NotebookState { state }, _) = session.handle(Request::GetNotebookState) else {
        return Err("Source snapshot unavailable".into());
    };
    if serde_json::to_value(expected).map_err(|_| "Invalid expected source")?
        != serde_json::to_value(state.file).map_err(|_| "Invalid current source")?
    {
        return Err("STALE_SNAPSHOT".into());
    }
    session.apply_source_file_without_evaluation(desired)?;
    serde_json::to_string(&session.handle(Request::GetNotebookState).0)
        .map_err(|_| "Cannot encode source acknowledgement".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn snapshot(session: &mut Session) -> Value {
        serde_json::to_value(session.handle(Request::GetNotebookState).0).unwrap()
    }
    #[test]
    fn atomic_source_change_invalidates_dependencies_without_running_or_discarding_unrelated_output()
     {
        let mut session = Session::new(Default::default(), None);
        for (id, source) in [("a", "let a=2"), ("b", "a+1"), ("unrelated", "10+20")] {
            session.handle(Request::Evaluate {
                cell_id: id.into(),
                source: source.into(),
                dialect: om_kernel::protocol::Dialect::Modern,
            });
        }
        let old = snapshot(&mut session);
        let before = old["state"]["file"].clone();
        let mut after = before.clone();
        after["cells"][0]["source"] = json!("let a=5");
        let accepted: Value = serde_json::from_str(
            &commit(&mut session, &before.to_string(), &after.to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(accepted["state"]["file"], after);
        assert_eq!(accepted["state"]["cells"][0]["status"], "Stale");
        assert_eq!(accepted["state"]["cells"][1]["status"], "Stale");
        assert_eq!(accepted["state"]["cells"][2], old["state"]["cells"][2]);
        assert_eq!(
            accepted["state"]["cells"][1]["exec_count"],
            old["state"]["cells"][1]["exec_count"]
        );
        assert!(commit(&mut session, &before.to_string(), &before.to_string()).is_err());
        assert_eq!(snapshot(&mut session), accepted);
        let mut invalid = after.clone();
        invalid["cells"][1]["id"] = json!("a");
        assert!(commit(&mut session, &after.to_string(), &invalid.to_string()).is_err());
        assert_eq!(snapshot(&mut session), accepted);
    }
}
