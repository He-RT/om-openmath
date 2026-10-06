//! The only unsafe boundary: caller-owned readable bytes become owned safe Rust values.
use crate::Host;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};
// Apple's pthread QoS function affects only the current owner thread.
#[cfg(target_vendor = "apple")]
unsafe extern "C" {
    fn pthread_set_qos_class_self_np(class: u32, relative_priority: i32) -> i32;
}
/// Match Swift's user-initiated work queue without a cross-QoS blocking wait.
pub(crate) fn configure_owner_thread() {
    #[cfg(target_vendor = "apple")]
    // SAFETY: valid public QOS_CLASS_USER_INITIATED (0x19), relative priority zero;
    // the function touches only this thread and has no pointer/ownership inputs.
    unsafe {
        let _ = pthread_set_qos_class_self_np(0x19, 0);
    }
}
/// Rust-owned UTF-8 returned to Swift. Release exactly once using om_ios_buffer_free.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OmBuffer {
    /// Readable bytes valid until release.
    pub data: *const u8,
    /// Exact byte length (not NUL terminated).
    pub len: usize,
}
static HOSTS: OnceLock<Mutex<BTreeMap<u64, Arc<Host>>>> = OnceLock::new();
static BUFFERS: OnceLock<Mutex<BTreeMap<usize, Box<[u8]>>>> = OnceLock::new();
static NEXT: AtomicU64 = AtomicU64::new(1);
fn hosts() -> &'static Mutex<BTreeMap<u64, Arc<Host>>> {
    HOSTS.get_or_init(Default::default)
}
fn buffers() -> &'static Mutex<BTreeMap<usize, Box<[u8]>>> {
    BUFFERS.get_or_init(Default::default)
}
fn host(id: u64) -> Result<Arc<Host>, String> {
    hosts()
        .lock()
        .map_err(|_| "Host registry unavailable")?
        .get(&id)
        .cloned()
        .ok_or_else(|| "Unknown session".into())
}
fn output(result: Result<String, String>) -> OmBuffer {
    let text = match result {
        Ok(text) => text,
        Err(message) => serde_json::json!({"bridge_error":message}).to_string(),
    };
    let bytes = text.into_bytes().into_boxed_slice();
    let data = bytes.as_ptr();
    let len = bytes.len();
    match buffers().lock() {
        Ok(mut owned) => {
            owned.insert(data as usize, bytes);
            OmBuffer { data, len }
        }
        Err(_) => OmBuffer {
            data: std::ptr::null(),
            len: 0,
        },
    }
}
// SAFETY: this helper is called only under the corresponding exported caller contract.
unsafe fn input<'a>(data: *const u8, len: usize, max: usize) -> Result<&'a [u8], String> {
    if len > max || (data.is_null() && len != 0) {
        return Err("Invalid input buffer".into());
    }
    if len == 0 {
        return Ok(&[]);
    }
    // SAFETY: caller guarantees a readable allocation of at least len bytes during this call.
    Ok(unsafe { std::slice::from_raw_parts(data, len) })
}
/// The fixed ABI revision expected by the Swift wrapper.
#[unsafe(no_mangle)]
pub extern "C" fn om_ios_abi_version() -> u32 {
    1
}
/// Create a handle or return zero on invalid configuration/startup.
/// # Safety
/// data must point to len readable bytes for this call; null is permitted only when len=0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn om_ios_create(data: *const u8, len: usize) -> u64 {
    std::panic::catch_unwind(|| {
        // SAFETY: the exported caller contract supplies this readable buffer.
        let bytes = unsafe { input(data, len, 2_097_152) }.ok()?;
        let config = if bytes.is_empty() {
            None
        } else {
            Some(std::str::from_utf8(bytes).ok()?)
        };
        let host = Arc::new(Host::new(config).ok()?);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        hosts().lock().ok()?.insert(id, host);
        Some(id)
    })
    .ok()
    .flatten()
    .unwrap_or(0)
}
/// Dispatch a JSON request. Errors return a secret-safe bridge_error object.
/// # Safety
/// data must point to len readable bytes for this call; null is permitted only when len=0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn om_ios_request(id: u64, data: *const u8, len: usize) -> OmBuffer {
    let result = std::panic::catch_unwind(|| {
        // SAFETY: the exported caller contract supplies this readable buffer.
        let bytes = unsafe { input(data, len, 16 * 1024 * 1024) }?;
        let text = std::str::from_utf8(bytes).map_err(|_| "Invalid UTF-8")?;
        host(id)?.request(text)
    })
    .unwrap_or_else(|_| Err("Kernel request failed".into()));
    output(result)
}
/// Feed actual HTTP response bytes without lossy string conversion.
/// # Safety
/// Both buffers must be readable for their declared lengths during this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn om_ios_http_bytes(
    id: u64,
    correlation: u64,
    request: *const u8,
    request_len: usize,
    status: u16,
    data: *const u8,
    len: usize,
) -> OmBuffer {
    let result = std::panic::catch_unwind(|| {
        // SAFETY: both readable buffers are covered by the exported caller contract.
        let name = std::str::from_utf8(unsafe { input(request, request_len, 8192) }?)
            .map_err(|_| "Invalid request ID")?;
        // SAFETY: the body buffer is readable for len bytes during this call.
        let bytes = unsafe { input(data, len, 1_048_576) }?;
        host(id)?.feed(correlation, name, status, bytes)
    })
    .unwrap_or_else(|_| Err("HTTP continuation failed".into()));
    output(result)
}
/// Set the computation cancellation flag independently of the owner queue.
#[unsafe(no_mangle)]
pub extern "C" fn om_ios_interrupt(id: u64) {
    if let Ok(host) = host(id) {
        host.interrupt()
    }
}
/// Set an active AI tool cancellation flag independently of the owner queue.
/// # Safety
/// data must point to len readable UTF-8 bytes for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn om_ios_cancel(id: u64, data: *const u8, len: usize) {
    // SAFETY: the caller supplies a readable buffer; invalid lengths are rejected.
    if let Ok(bytes) = unsafe { input(data, len, 8192) }
        && let Ok(name) = std::str::from_utf8(bytes)
        && let Ok(host) = host(id)
    {
        host.cancel(name)
    }
}
/// Remove a live handle, cancel work and join its owner. Unknown handles are harmless.
#[unsafe(no_mangle)]
pub extern "C" fn om_ios_destroy(id: u64) {
    let removed = hosts().lock().ok().and_then(|mut h| h.remove(&id));
    if let Some(host) = removed {
        host.close()
    }
}
/// Release only a registered returned buffer. Invalid/double release does not dereference it.
#[unsafe(no_mangle)]
pub extern "C" fn om_ios_buffer_free(buffer: OmBuffer) {
    if let Ok(mut owned) = buffers().lock()
        && owned
            .get(&(buffer.data as usize))
            .is_some_and(|b| b.len() == buffer.len)
    {
        owned.remove(&(buffer.data as usize));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn read(buffer: OmBuffer) -> serde_json::Value {
        // SAFETY: this buffer is owned by the registry until explicitly freed below.
        let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.len) };
        let result = serde_json::from_slice(bytes).expect("real bridge JSON");
        om_ios_buffer_free(buffer);
        result
    }
    #[test]
    fn abi_rejects_invalid_buffers_releases_ownership_and_closes_unknown_handles() {
        assert_eq!(om_ios_abi_version(), 1);
        // SAFETY: null and zero is the specified empty config.
        let id = unsafe { om_ios_create(std::ptr::null(), 0) };
        assert_ne!(id, 0);
        // SAFETY: null/nonzero is rejected before any dereference by the ABI.
        let invalid = unsafe { om_ios_request(id, std::ptr::null(), 1) };
        assert!(read(invalid).get("bridge_error").is_some());
        let request = br#"{"id":1,"body":{"type":"get_config"}}"#;
        // SAFETY: request is readable for exactly request.len() during the call.
        let response = unsafe { om_ios_request(id, request.as_ptr(), request.len()) };
        assert!(
            buffers()
                .lock()
                .unwrap()
                .contains_key(&(response.data as usize))
        );
        assert_eq!(read(response)["response"]["id"], 1);
        assert!(
            !buffers()
                .lock()
                .unwrap()
                .contains_key(&(response.data as usize))
        );
        om_ios_buffer_free(response);
        om_ios_buffer_free(OmBuffer {
            data: std::ptr::null(),
            len: 999,
        });
        om_ios_destroy(id);
        om_ios_destroy(id);
        om_ios_interrupt(id);
        // SAFETY: request remains a valid readable allocation, although the handle is closed.
        assert!(
            read(unsafe { om_ios_request(id, request.as_ptr(), request.len()) })
                .get("bridge_error")
                .is_some()
        );
    }
}
