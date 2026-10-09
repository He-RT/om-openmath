//! Offline cross-language contract fixture; does not create a runtime or admit an operation.
use om_host_service::protocol::{MAX_CONTROL_BYTES, generated::HostState, validation};
use std::io::Read;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .take((MAX_CONTROL_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/design/macos-host-state.schema.json"
    ))?;
    let value: HostState = validation::decode(&bytes, &schema, &schema)?;
    println!("{}", serde_json::to_string(&value)?);
    Ok(())
}
