//! Validate actual canonical fixture envelopes against the existing authoritative design schema.
use om_host_service::protocol::validation;
fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("actual fixture event JSON array");
    let records: Vec<serde_json::Value> =
        serde_json::from_slice(&std::fs::read(path).expect("fixture bytes")).expect("fixture JSON");
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/design/macos-storage.schema.json"
    ))
    .expect("original storage schema");
    for record in &records {
        let bytes = serde_json::to_vec(record).expect("actual event envelope");
        let actual: serde_json::Value =
            validation::decode(&bytes, &schema["$defs"]["SessionEvent"], &schema)
                .expect("actual canonical event matches original schema");
        assert_eq!(&actual, record);
    }
    println!(
        "{} actual canonical session events match the original closed schema",
        records.len()
    );
}
