//! Actual callback catalog and current Session binding resolution; no model/tool execution.
use om_host_service::tools::catalog::{self, CatalogError};
use om_kernel::{
    KernelConfig, Session,
    protocol::{Dialect, Request, Response},
};
use serde_json::{Value, json};
fn query(value: Value) -> Vec<u8> {
    serde_json::to_vec(&value).unwrap()
}
#[test]
fn executable_docs_match_actual_callbacks_and_never_claim_example_verification() {
    let docs = om_kernel::capabilities::function_documents();
    let catalog = om_kernel::capabilities::function_catalog();
    assert_eq!(
        docs["functions"].as_array().unwrap().len(),
        catalog.functions.len()
    );
    for entry in docs["functions"].as_array().unwrap() {
        assert!(
            catalog
                .functions
                .iter()
                .any(|f| f.name == entry["descriptor"]["name"])
        );
        assert!(!entry["signature"].as_str().unwrap().is_empty());
        assert!(
            entry["examples"]
                .as_array()
                .unwrap()
                .iter()
                .all(|e| e["verification"] == "not_verified")
        );
    }
    let empty = om_kernel::source::SourceExecutionContext {
        known_functions: vec![],
        owned: vec![],
    };
    for entry in docs["functions"].as_array().unwrap() {
        assert!(
            catalog::docs(
                &query(json!({"function_id":entry["descriptor"]["id"]})),
                &empty
            )
            .is_ok(),
            "{}",
            entry["descriptor"]["name"]
        );
    }
    println!(
        "actual callback documentation rows: {}; metadata revision {}",
        catalog.functions.len(),
        catalog.metadata_version
    );
    assert_eq!(
        catalog::docs(
            &query(json!({"name":"CertainlyPlannedNotACallback"})),
            &empty
        ),
        Err(CatalogError::NotFound)
    );
    let actual = catalog::docs(&query(json!({"name":"sin"})), &empty).unwrap();
    assert_eq!(actual["function"]["descriptor"]["name"], "Sin");
    assert_eq!(
        catalog::docs(
            &query(
                json!({"function_id":actual["function"]["descriptor"]["id"],"metadata_version":1})
            ),
            &empty
        ),
        Err(CatalogError::StaleMetadata)
    );
}
#[test]
fn defaults_limits_categories_closed_fields_and_selector_nulls_are_strict() {
    let first = catalog::search(&query(json!({"query":"sin"}))).unwrap();
    assert!(first["functions"].as_array().unwrap().len() <= 8);
    assert_eq!(first["functions"][0]["name"], "Sin");
    assert_eq!(
        first,
        catalog::search(&query(json!({"query":"sin"}))).unwrap()
    );
    assert!(
        catalog::search(&query(
            json!({"query":"sin","category":"certainly-unknown"})
        ))
        .unwrap()["functions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        catalog::search(&query(json!({"query":"sin","limit":1}))).unwrap()["functions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    for value in [
        json!({}),
        json!({"query":""}),
        json!({"query":"sin","limit":0}),
        json!({"query":"sin","limit":21}),
        json!({"query":"sin","limit":1.5}),
        json!({"query":"sin","category":null}),
        json!({"query":"sin","unknown":true}),
    ] {
        assert_eq!(
            catalog::search(&query(value)),
            Err(CatalogError::InvalidArgument)
        );
    }
    assert_eq!(
        catalog::search(br#"{"query":"sin","query":"cos"}"#),
        Err(CatalogError::InvalidArgument)
    );
    let context = om_kernel::source::SourceExecutionContext {
        known_functions: vec![],
        owned: vec![],
    };
    for value in [
        json!({}),
        json!({"name":"sin","function_id":"x"}),
        json!({"name":"sin","metadata_version":0}),
        json!({"name":"sin","function_id":null}),
        json!({"name":"sin","unknown":true}),
    ] {
        assert_eq!(
            catalog::docs(&query(value), &context),
            Err(CatalogError::InvalidArgument)
        );
    }
}
#[test]
fn actual_session_user_binding_precedes_builtin_alias_without_evaluating_docs() {
    let mut config = KernelConfig::default();
    config.general.reactive = false;
    let mut session = Session::new(config, None);
    let (reply, _) = session.handle(Request::Evaluate {
        cell_id: "definition".into(),
        source: "let sin=fn(x)=>x+1".into(),
        dialect: Dialect::Modern,
    });
    let Response::Evaluated { output, .. } = reply else {
        panic!("definition was not evaluated")
    };
    assert!(
        !output
            .messages
            .iter()
            .any(|m| format!("{:?}", m.level) == "Error")
    );
    let context = session.execution_context();
    assert!(
        context.known_functions.iter().any(|n| n == "sin"),
        "actual context {context:?}; output {output:?}"
    );
    let doc = catalog::docs(&query(json!({"name":"sin"})), &context).unwrap();
    assert_eq!(doc["function"]["kind"], "user_binding");
    assert_eq!(doc["function"]["cell_id"], "definition");
    assert_eq!(doc["function"]["source_complete"], false);
    assert!(doc["examples"].as_array().unwrap().is_empty());
    let after = session.execution_context();
    assert_eq!(context.known_functions, after.known_functions);
    assert_eq!(
        context
            .owned
            .iter()
            .map(|o| (&o.symbol, &o.cell_id))
            .collect::<Vec<_>>(),
        after
            .owned
            .iter()
            .map(|o| (&o.symbol, &o.cell_id))
            .collect::<Vec<_>>()
    );
}
