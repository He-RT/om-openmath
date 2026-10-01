//! Shared source/protocol fixtures for Session integration tests.
use om_core::canonicalize;
use om_kernel::{Session, config::KernelConfig, protocol::*};
use serde_json::Value;
/// Start an isolated sequential session.
pub fn sequential() -> Session {
    let mut config = KernelConfig::default();
    config.general.reactive = false;
    Session::new(config, None)
}

/// Drive the actual Session with a JSON request fixture.
pub fn request(session: &mut Session, value: Value) -> (Response, Vec<Event>) {
    session.handle(serde_json::from_value(value).unwrap())
}

/// Run one source cell through the public protocol.
pub fn output(session: &mut Session, id: &str, source: &str, dialect: Dialect) -> CellOutput {
    let (response, events) = session.handle(Request::Evaluate {
        cell_id: id.into(),
        source: source.into(),
        dialect,
    });
    assert!(events.is_empty()); // These fixtures use sequential sessions or cells without dependents.
    match response {
        Response::Evaluated {
            cell_id,
            output,
            reran,
        } => {
            assert_eq!(cell_id, id);
            assert!(reran.is_empty());
            output
        }
        other => panic!("expected evaluation, got {other:?}"),
    }
}

/// Inspect actual source forms of expression and solver outputs.
pub fn expressions(output: &CellOutput) -> Vec<(u32, String)> {
    output
        .items
        .iter()
        .filter_map(|item| match item {
            OutputItem::Expr {
                out_index,
                input_form,
                ..
            }
            | OutputItem::Solutions {
                out_index,
                input_form,
                ..
            } => Some((*out_index, input_form.clone())),
            _ => None,
        })
        .collect()
}

/// Compare canonical source-form mathematics.
pub fn same(source: &str, expected: &str) {
    let parse = |s| canonicalize(&om_parse::parse_expr(s, om_parse::Dialect::Wolfram).unwrap());
    assert_eq!(parse(source), parse(expected));
}
