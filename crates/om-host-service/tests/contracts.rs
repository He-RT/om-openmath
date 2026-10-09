//! Structural/nullable/range checks independent of business admission.
use om_host_service::protocol::{MAX_SERIAL, Nullable, Serial, generated::*, validation};
use serde_json::{Value, json};
fn root(name: &str) -> Value {
    let text = match name {
        "host" => include_str!("../../../docs/design/macos-host-state.schema.json"),
        "editor" => include_str!("../../../docs/design/macos-editor-rendering.schema.json"),
        _ => panic!("fixture"),
    };
    serde_json::from_str(text).unwrap()
}
fn host() -> Value {
    json!({"contract_version":1,"runtime_instance_id":"runtime-1","host_phase":"starting","ui_event_sequence":0,"rust_event_sequence":0,"document":null,"agent_task":null,"pi":{"phase":"stopped","connection_id":null,"connection_generation":0,"protocol_version":1},"operations":[]})
}
#[test]
fn actual_generated_host_roundtrips_and_required_null_is_not_optional() {
    let schema = root("host");
    let value = host();
    let typed: HostState =
        validation::decode(&serde_json::to_vec(&value).unwrap(), &schema, &schema).unwrap();
    assert_eq!(serde_json::to_value(&typed).unwrap(), value);
    let mut missing = value.clone();
    missing.as_object_mut().unwrap().remove("document");
    assert!(
        validation::decode::<HostState>(&serde_json::to_vec(&missing).unwrap(), &schema, &schema)
            .is_err()
    );
    // Typed decode independently enforces presence rather than relying only on the schema helper.
    assert!(serde_json::from_value::<HostState>(missing).is_err());
    assert!(
        serde_json::from_str::<Nullable<String>>("null")
            .unwrap()
            .0
            .is_none()
    );
}
#[test]
fn identities_unknown_fields_unions_and_closed_state_are_rejected() {
    let schema = root("host");
    for (key, value) in [
        ("runtime_instance_id", json!("../document")),
        ("host_phase", json!("readyish")),
        ("ui_event_sequence", json!(MAX_SERIAL + 1)),
        ("ui_event_sequence", json!(-1)),
        ("ui_event_sequence", json!(0.5)),
        ("secret", json!("not-a-field")),
    ] {
        let mut v = host();
        v[key] = value;
        assert!(validation::validate(&v, &schema, &schema).is_err(), "{key}");
    }
    let mut v = host();
    v["host_phase"] = json!("closed");
    v["pi"]["phase"] = json!("running");
    assert!(validation::validate(&v, &schema, &schema).is_err());
    assert!(
        serde_json::from_value::<HostState>({
            let mut v = host();
            v["extra"] = json!(1);
            v
        })
        .is_err()
    );
}
#[test]
fn shared_serials_preserve_maximum_and_never_wrap_or_truncate() {
    assert_eq!(
        serde_json::from_value::<Serial>(json!(MAX_SERIAL))
            .unwrap()
            .get(),
        MAX_SERIAL
    );
    assert!(Serial::new(MAX_SERIAL).unwrap().checked_next().is_err());
    for v in [
        json!(MAX_SERIAL + 1),
        json!(-1),
        json!(0.5),
        json!("1"),
        json!(true),
    ] {
        assert!(serde_json::from_value::<Serial>(v).is_err());
    }
    assert_eq!(serde_json::from_str::<Serial>("1.0").unwrap().get(), 1);
}
#[test]
fn frame_budget_invalid_utf8_and_unknown_nullable_fields_fail_closed() {
    let schema = root("host");
    assert!(
        validation::decode::<HostState>(&vec![b' '; 2 * 1024 * 1024 + 1], &schema, &schema)
            .is_err()
    );
    assert!(validation::decode::<HostState>(&[0xff], &schema, &schema).is_err());
    let mut v = host();
    v["pi"]["connection_id"] = json!({"unknown":1});
    assert!(
        validation::decode::<HostState>(&serde_json::to_vec(&v).unwrap(), &schema, &schema)
            .is_err()
    );
}
#[test]
fn marked_text_invariants_are_not_confused_with_shape_success() {
    let schema = root("editor");
    let model = &schema["$defs"]["EditorSnapshot"];
    let key = json!({"runtime_instance_id":"runtime-1","document_id":"doc-1","document_generation":1,"cell_id":"cell-1","editor_id":"editor-1","editor_generation":1,"draft_sequence":2,"source_hash":"a".repeat(64),"requested_dialect":"Modern","effective_dialect":"Modern","analysis_generation":1,"metadata_version":26,"definition_snapshot_revision":1,"config_revision":1,"cursor_utf8":null,"selection_utf16":{"location":0,"length":0}});
    let mut value = json!({"record_type":"editor_snapshot","key":key,"source":"中文🙂","composition":"marked","marked_range_utf16":null,"committed_cell_revision":1,"is_source_dirty":true});
    assert!(validation::validate(&value, model, &schema).is_err());
    value["marked_range_utf16"] = json!({"location":0,"length":2});
    let typed: EditorSnapshot =
        validation::decode(&serde_json::to_vec(&value).unwrap(), model, &schema).unwrap();
    assert_eq!(serde_json::to_value(typed).unwrap(), value);
    value["composition"] = json!("none");
    assert!(validation::validate(&value, model, &schema).is_err());
}

#[test]
fn native_wire_dispatch_is_typed_and_cannot_change_permissions_by_adding_fields() {
    use om_host_service::protocol::wire;
    let init = json!({"protocol_version":1,"runtime_instance_id":"runtime-1","max_pending_operations":32,"event_capacity":128});
    wire::decode_init(&serde_json::to_vec(&init).unwrap()).unwrap();
    let mut request = json!({"protocol_version":1,"runtime_instance_id":"runtime-1","request_ref":"req-1","operation_id":null,"document_binding":null,"task_binding":null,"body":{"kind":"get_capabilities"}});
    let typed = wire::decode_request(&serde_json::to_vec(&request).unwrap()).unwrap();
    assert!(matches!(
        typed.body,
        HostRequestBody::ReadHostCapabilities(_)
    ));
    assert_eq!(serde_json::to_value(typed).unwrap(), request);
    request["body"]["grants"] = json!(["write_document"]);
    assert!(wire::decode_request(&serde_json::to_vec(&request).unwrap()).is_err());
    request["body"] = json!({"kind":"delete_arbitrary_file","path":"/tmp/unbound"});
    assert!(wire::decode_request(&serde_json::to_vec(&request).unwrap()).is_err());
}
#[test]
fn durable_io_ack_requires_actual_receipt_fields_and_cannot_claim_success_from_null() {
    use om_host_service::protocol::wire;
    let schema = wire::schema();
    let mut ack = json!({"runtime_instance_id":"runtime-1","document_binding":null,"operation_ref":"op-1","io_request_ref":"io-1","outcome":"durable","receipt_ref":null,"receipt_hash":null,"error":null});
    assert!(validation::validate(&ack, &schema["$defs"]["IOAck"], schema).is_err());
    ack["receipt_ref"] = json!("receipt-1");
    ack["receipt_hash"] = json!("a".repeat(64));
    let typed: IOAck = validation::decode(
        &serde_json::to_vec(&ack).unwrap(),
        &schema["$defs"]["IOAck"],
        schema,
    )
    .unwrap();
    assert_eq!(serde_json::to_value(typed).unwrap(), ack);
}

#[test]
fn unsupported_schema_features_and_false_schemas_are_not_silently_accepted() {
    let value = json!(1);
    assert!(validation::validate(&value, &json!(false), &json!(false)).is_err());
    assert!(
        validation::validate(
            &value,
            &json!({"type":"number","exclusiveMinimum":2}),
            &json!({})
        )
        .is_err()
    );
    assert!(
        validation::validate(
            &value,
            &json!({"$ref":"https://outside.invalid/schema"}),
            &json!({})
        )
        .is_err()
    );
}

#[test]
fn positive_but_stale_runtime_and_document_generations_fail_owner_binding() {
    use om_host_service::protocol::wire;
    let value = json!({"protocol_version":1,"runtime_instance_id":"runtime-1","request_ref":"req-1","operation_id":null,"document_binding":{"document_id":"doc-1","generation":2,"document_revision":3,"execution_epoch":4},"task_binding":null,"body":{"kind":"get_state"}});
    let request = wire::decode_request(&serde_json::to_vec(&value).unwrap()).unwrap();
    let current = DocumentBinding {
        document_id: "doc-1".into(),
        generation: Serial::new(3).unwrap(),
        document_revision: Serial::new(3).unwrap(),
        execution_epoch: Serial::new(4).unwrap(),
    };
    assert_eq!(
        wire::check_owner_binding(&request, "runtime-2", Some(&current)),
        Err("STALE_RUNTIME")
    );
    assert_eq!(
        wire::check_owner_binding(&request, "runtime-1", Some(&current)),
        Err("STALE_DOCUMENT")
    );
    assert_eq!(
        wire::check_owner_binding(&request, "runtime-1", None),
        Err("STALE_DOCUMENT")
    );
}
