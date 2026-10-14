//! Single immutable request/response codec. Pi alone owns any subsequent tool/model loop.
mod types;
pub use types::*;

mod build;
pub use build::build;

mod decode;
pub use decode::Decoder;

/// Compile a body and freeze its response policy from the same trusted snapshot.
/// Output identities come from the native owner; a provider cannot select them.
pub fn prepare(
    request: &Request,
    message_id: &str,
    task_id: &str,
    turn_id: &str,
) -> std::result::Result<(Wire, Decoder), Error> {
    if ![message_id, task_id, turn_id]
        .into_iter()
        .all(build::identity)
    {
        return Err(Error::Input);
    }
    let wire = build(request)?;
    let decoder = Decoder::new(ResponseScope {
        binding: wire.binding.clone(),
        message_id: message_id.into(),
        task_id: task_id.into(),
        turn_id: turn_id.into(),
        tools: request.tools.iter().map(|t| t.name.clone()).collect(),
        stream: wire.stream,
    });
    Ok((wire, decoder))
}
