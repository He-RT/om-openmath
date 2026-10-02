//! Optional actual HTTP IO wraps the same pure Job; CAS tools remain the host's responsibility.
use super::{Job, JobStep, State};
use crate::{HttpRequest, LlmError, StreamEvent};
mod transport;
use reqwest::{Client, redirect::Policy};
use tokio_util::sync::CancellationToken;
pub use transport::drive_native_http;

/// Build the product native transport without redirects that could forward custom credentials.
/// Environment proxy settings and TLS verification remain reqwest defaults.
pub fn native_client() -> Result<Client, LlmError> {
    Client::builder()
        .redirect(Policy::none())
        .build()
        .map_err(|_| LlmError::Transport("HTTP client initialization failed".into()))
}

/// Drive fresh HTTP rounds, forwarding actual model deltas until Done/Failed or host RunTools.
/// Use `native_client()` or a client built with `redirect(Policy::none())`.
/// This API cannot inspect the redirect policy of an externally constructed reqwest client.
pub async fn drive_native(
    job: &mut Job,
    client: &Client,
    on_event: impl FnMut(StreamEvent),
) -> JobStep {
    drive_native_cancellable(job, client, &CancellationToken::new(), on_event).await
}

/// Drive the same real lifecycle while allowing a host to abort an in-flight HTTP future.
/// The supplied client must disable redirects, as documented on `drive_native`.
pub async fn drive_native_cancellable(
    job: &mut Job,
    client: &Client,
    cancel: &CancellationToken,
    mut on_event: impl FnMut(StreamEvent),
) -> JobStep {
    if let Some(step) = job.final_step() {
        return step;
    }
    if cancel.is_cancelled() {
        return job.cancel();
    }
    if let State::Tools(calls) = &job.state {
        return JobStep::RunTools(calls.clone());
    }
    if !job.round.raw.is_empty() {
        return job.fail(LlmError::State(
            "native driver requires an untouched HTTP round",
        ));
    }
    loop {
        if cancel.is_cancelled() {
            return job.cancel();
        }
        let request = match job.request() {
            Ok(r) => r,
            Err(e) => return job.fail(e),
        };
        let step = round(job, client, request, cancel, &mut on_event).await;
        if matches!(step, JobStep::Http(_)) {
            continue;
        }
        return step;
    }
}

async fn round(
    job: &mut Job,
    client: &Client,
    http: HttpRequest,
    cancel: &CancellationToken,
    on_event: &mut impl FnMut(StreamEvent),
) -> JobStep {
    let timeout = job.profile.timeout_ms;
    let (status, error) = drive_native_http(&http, timeout, client, cancel, |status, bytes| {
        let events = job.on_raw_bytes(bytes);
        if (200..300).contains(&status) {
            for event in events {
                if cancel.is_cancelled() {
                    return false;
                }
                on_event(event);
            }
        }
        !cancel.is_cancelled() && !job.is_finished()
    })
    .await;
    if cancel.is_cancelled() {
        return job.cancel();
    }
    if job.is_finished() {
        return job.on_http_end(status, None);
    }
    job.on_http_end(status, error)
}

fn transport_error(error: &reqwest::Error) -> &'static str {
    // reqwest errors can contain credentials embedded in a configured URL; never format them.
    if error.is_timeout() {
        "HTTP request timed out"
    } else if error.is_connect() {
        "HTTP connection failed"
    } else if error.is_body() {
        "HTTP response body failed"
    } else if error.is_decode() {
        "HTTP response decoding failed"
    } else {
        "HTTP request failed"
    }
}
