//! Shared actual native byte transport for either a standalone Job or the Session queue.
use super::transport_error;
use crate::HttpRequest;
use reqwest::{Client, Method};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
/// Transport one actual request without owning or decoding a second Job state machine.
/// Supply a client with redirects disabled. Callback false stops reading immediately.
/// Status zero means no response headers arrived; errors never contain raw URLs/credentials.
pub async fn drive_native_http(
    http: &HttpRequest,
    timeout_ms: u64,
    client: &Client,
    cancel: &CancellationToken,
    mut on_chunk: impl FnMut(u16, &[u8]) -> bool,
) -> (u16, Option<String>) {
    if cancel.is_cancelled() {
        return (0, Some("HTTP request cancelled".into()));
    }
    let mut status = 0;
    let error = tokio::select! {
        biased;
        _=cancel.cancelled()=>Some("HTTP request cancelled".into()),
        result=tokio::time::timeout(Duration::from_millis(timeout_ms),send(http,client,&mut status,cancel,&mut on_chunk))=>match result {
            Ok(error)=>error,
            Err(_)=>Some("HTTP request timed out".into()),
        },
    };
    (status, error)
}
async fn send(
    http: &HttpRequest,
    client: &Client,
    status: &mut u16,
    cancel: &CancellationToken,
    on_chunk: &mut impl FnMut(u16, &[u8]) -> bool,
) -> Option<String> {
    let method = match http.method.parse::<Method>() {
        Ok(method) => method,
        Err(_) => return Some("Invalid HTTP method".into()),
    };
    let mut request = client.request(method, &http.url).body(http.body.clone());
    for (name, value) in &http.headers {
        request = request.header(name, value);
    }
    let mut response = match request.send().await {
        Ok(response) => response,
        Err(error) => return Some(transport_error(&error).into()),
    };
    *status = response.status().as_u16();
    let mut size = 0usize;
    loop {
        match response.chunk().await {
            Ok(Some(bytes)) => {
                size = size.saturating_add(bytes.len());
                let keep_reading = on_chunk(*status, &bytes);
                if cancel.is_cancelled() {
                    return Some("HTTP request cancelled".into());
                }
                if size > super::super::LIMIT {
                    return Some("HTTP response limit exceeded".into());
                }
                if !keep_reading {
                    return None;
                }
            }
            Ok(None) => return None,
            Err(error) => return Some(transport_error(&error).into()),
        }
    }
}
