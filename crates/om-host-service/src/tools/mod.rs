//! Actual native tool data services, independent of model loops and host IO.
pub mod catalog;
pub mod dispatch;
pub mod operations;
pub mod patch;
pub mod read;
pub mod results;
pub mod run;
pub mod scratch;

fn schema() -> &'static serde_json::Value {
    static SCHEMA: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../docs/design/agent-tools.schema.json"
        ))
        .expect("checked tools schema")
    })
}
pub(crate) fn checked_data(name: &str, value: serde_json::Value) -> Result<serde_json::Value, ()> {
    crate::protocol::validation::validate(&value, &schema()["$defs"][name], schema())
        .map_err(|_| ())?;
    if serde_json::to_vec(&value).map_err(|_| ())?.len() > 256 * 1024 {
        return Err(());
    }
    Ok(value)
}
