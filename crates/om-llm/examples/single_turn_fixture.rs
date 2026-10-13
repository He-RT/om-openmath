//! Offline test adapter: one invocation, one request/body and one decoded response.
//! It performs no HTTP, reads no credentials, grants no tools and owns no agent loop.
use om_llm::single_turn::{Request, prepare};
use serde::Deserialize;
use std::io::{Read, Write};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    request: Request,
    message_id: String,
    task_id: String,
    turn_id: String,
    status: u16,
    response: Option<String>,
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = vec![];
    std::io::stdin()
        .take(12 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 12 * 1024 * 1024 {
        return Err("fixture input bound".into());
    }
    let input: Input = serde_json::from_slice(&bytes)?;
    let (wire, mut decoder) = prepare(
        &input.request,
        &input.message_id,
        &input.task_id,
        &input.turn_id,
    )?;
    let body: serde_json::Value = serde_json::from_slice(&wire.body)?;
    let Some(response) = input.response else {
        serde_json::to_writer(std::io::stdout().lock(), &serde_json::json!({"body":body}))?;
        return Ok(());
    };
    decoder.headers(input.status)?;
    // Exercise real UTF8 scalar splits, rather than handing the parser complete frames.
    for byte in response.as_bytes() {
        decoder.feed(std::slice::from_ref(byte))?;
    }
    let result = decoder.finish()?;
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({"body":body,"result":result}),
    )?;
    std::io::stdout().write_all(b"\n")?;
    Ok(())
}

fn main() {
    if run().is_err() {
        // Fixture input/provider data is deliberately never included in error output.
        eprintln!("SINGLE_TURN_FIXTURE_FAILED");
        std::process::exit(1);
    }
}
