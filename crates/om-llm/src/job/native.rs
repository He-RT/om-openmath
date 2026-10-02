//! Optional actual HTTP IO wraps the same pure Job; CAS tools remain the host's responsibility.
use super::{Job, JobStep, State};
use crate::{HttpRequest, LlmError, StreamEvent};
use reqwest::{Client, Method, redirect::Policy};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

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
        let timeout = Duration::from_millis(job.profile.timeout_ms);
        let step = tokio::select! {
            biased;
            _=cancel.cancelled()=>job.cancel(),
            result=tokio::time::timeout(timeout,round(job,client,request,cancel,&mut on_event))=>match result {
                Ok(step)=>step,
                Err(_)=>job.on_http_end(0,Some("HTTP request timed out".into())),
            },
        };
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
    let method = match http.method.parse::<Method>() {
        Ok(method) => method,
        Err(_) => return job.on_http_end(0, Some("invalid HTTP method".into())),
    };
    let mut request = client.request(method, &http.url).body(http.body);
    for (name, value) in http.headers {
        request = request.header(name, value);
    }
    let mut response = match request.send().await {
        Ok(response) => response,
        Err(error) => return job.on_http_end(0, Some(transport_error(&error).into())),
    };
    let status = response.status().as_u16();
    loop {
        match response.chunk().await {
            Ok(Some(bytes)) => {
                let events = job.on_raw_bytes(&bytes);
                if (200..300).contains(&status) {
                    for event in events {
                        if cancel.is_cancelled() {
                            return job.cancel();
                        }
                        on_event(event);
                    }
                }
                if cancel.is_cancelled() {
                    return job.cancel();
                }
                if job.is_finished() {
                    return job.on_http_end(status, None);
                }
            }
            Ok(None) => return job.on_http_end(status, None),
            Err(error) => return job.on_http_end(status, Some(transport_error(&error).into())),
        }
    }
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
