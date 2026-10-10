//! Audited C input boundary: copy borrowed bytes, catch panic and return owned versioned packets.
use crate::registry;
use om_host_service::{
    protocol::{self, Nullable, generated::*, wire},
    scheduler::NativeHost,
};
use serde::Serialize;
use serde_json::json;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Opaque stable registry token; callers must never dereference or free this pointer.
#[repr(C)]
pub struct OmHostHandle {
    pub(crate) _token: usize,
}
/// Rust-owned exact bytes. Release once with `om_host_buffer_free`; empty is NULL/0.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OmHostBuffer {
    /// Valid allocation until it is freed. Not NUL-terminated.
    pub ptr: *mut u8,
    /// Exact readable byte count.
    pub len: usize,
}
impl OmHostBuffer {
    pub(crate) fn empty() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            len: 0,
        }
    }
}
/// Successful create has a handle and empty error; failed create has NULL and an owned error.
#[repr(C)]
pub struct OmHostCreateResult {
    /// Opaque service identity consumed by background `close_finish`.
    pub handle: *mut OmHostHandle,
    /// Failure packet to free, even when no handle was created.
    pub error: OmHostBuffer,
}
fn code(error: &str) -> HostErrorCode {
    match error {
        "INVALID_REFERENCE" | "DUPLICATE_OPERATION" => HostErrorCode::InvalidReference,
        "PERMISSION_DENIED" => HostErrorCode::PermissionDenied,
        "STALE_DOCUMENT" | "STALE_RUNTIME" | "STALE_SOURCE" | "STALE_SNAPSHOT"
        | "PREVIEW_MISMATCH" => HostErrorCode::StaleDocument,
        "EDITING_BUSY" | "UNDO_CONFLICT" => HostErrorCode::EditingBusy,
        "OPERATION_IN_PROGRESS" => HostErrorCode::NotAvailable,
        "HOST_CLOSING" | "NOT_AVAILABLE" | "CONTEXT_NOT_READY" => HostErrorCode::NotAvailable,
        "BUDGET_EXCEEDED" => HostErrorCode::BudgetExceeded,
        "CANCELLED" => HostErrorCode::Cancelled,
        "INTERNAL_ERROR" | "OWNER_PANIC" | "WORKER_PANIC" => HostErrorCode::InternalError,
        _ => HostErrorCode::InvalidArgument,
    }
}
fn failure(error: &str) -> OmHostBuffer {
    let message = match error {
        "HOST_CLOSING" => "Host is closing",
        "STALE_RUNTIME" => "Runtime identity is stale",
        "DUPLICATE_OPERATION" => "Operation identity has already been used",
        _ => "Native host request rejected",
    };
    encoded(&HostFailure {
        protocol_version: 1,
        error: HostError {
            code: code(error),
            message: message.into(),
            field_path: Nullable(None),
            operation_ref: Nullable(None),
            outcome_known: true,
            retryable: matches!(error, "EDITING_BUSY" | "OPERATION_IN_PROGRESS"),
        },
    })
}
fn encoded<T: Serialize>(value: &T) -> OmHostBuffer {
    serde_json::to_vec(value)
        .map(registry::output)
        .unwrap_or_else(|_| OmHostBuffer::empty())
}
fn packet<T: Serialize>(run: impl FnOnce() -> Result<T, String>) -> OmHostBuffer {
    catch_unwind(AssertUnwindSafe(|| match run() {
        Ok(value) => encoded(&value),
        Err(error) => failure(&error),
    }))
    .unwrap_or_else(|_| {
        catch_unwind(|| failure("INTERNAL_ERROR")).unwrap_or_else(|_| OmHostBuffer::empty())
    })
}
fn integer(run: impl FnOnce() -> Result<i32, &'static str>) -> i32 {
    match catch_unwind(AssertUnwindSafe(run)).unwrap_or(Err("INTERNAL_ERROR")) {
        Ok(value) => value,
        Err("INVALID_ARGUMENT") => -1,
        Err("INVALID_REFERENCE") => -2,
        Err("HOST_CLOSING") => -3,
        Err("BUDGET_EXCEEDED") => -5,
        Err(_) => -4,
    }
}
// SAFETY: only called under exported readable-input contracts. We never keep the caller's ptr.
unsafe fn copied_input(pointer: *const u8, len: usize, max: usize) -> Result<Vec<u8>, String> {
    if len > max {
        return Err("BUDGET_EXCEEDED".into());
    }
    if pointer.is_null() && len != 0 {
        return Err("INVALID_ARGUMENT".into());
    }
    if len == 0 {
        return Ok(Vec::new());
    }
    // SAFETY: caller promises a readable allocation for len bytes during this call; len is bounded.
    Ok(unsafe { std::slice::from_raw_parts(pointer, len) }.to_vec())
}
/// Copy validated initialization and start safe owners; reports explicit startup errors.
/// # Safety
/// `json` must be readable for `len` bytes during this call. NULL is allowed only for len=0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn om_host_create(json: *const u8, len: usize) -> OmHostCreateResult {
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: the caller provides the documented readable buffer; copy before decoding.
        let bytes = unsafe { copied_input(json, len, protocol::MAX_CONTROL_BYTES) }?;
        let host = NativeHost::new(wire::decode_init(&bytes)?)?;
        registry::insert(host).map_err(str::to_owned)
    }))
    .unwrap_or_else(|_| Err("INTERNAL_ERROR".into()));
    catch_unwind(AssertUnwindSafe(|| match result {
        Ok(handle) => OmHostCreateResult {
            handle,
            error: OmHostBuffer::empty(),
        },
        Err(error) => OmHostCreateResult {
            handle: std::ptr::null_mut(),
            error: failure(&error),
        },
    }))
    .unwrap_or(OmHostCreateResult {
        handle: std::ptr::null_mut(),
        error: OmHostBuffer::empty(),
    })
}
/// Copy, validate and admit a bounded request; returns admission, never waits for CAS completion.
/// # Safety
/// `json` must be readable for `len` bytes during this call. NULL is allowed only for len=0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn om_host_submit(
    host: *mut OmHostHandle,
    json: *const u8,
    len: usize,
) -> OmHostBuffer {
    packet(|| {
        let guard = registry::acquire(host, false).map_err(str::to_owned)?;
        // SAFETY: caller contract guarantees readable input. The owned copy outlives this call.
        guard
            .host()
            .submit(&unsafe { copied_input(json, len, protocol::MAX_CONTROL_BYTES) }?)
    })
}
/// Bounded background event poll (wait<=100 ms); returned buffer may be a typed HostFailure.
#[unsafe(no_mangle)]
pub extern "C" fn om_host_next_events(
    host: *mut OmHostHandle,
    wait_ms: u32,
    max_bytes: usize,
) -> OmHostBuffer {
    packet(|| {
        registry::acquire(host, true)
            .map_err(str::to_owned)?
            .host()
            .next_events(wait_ms, max_bytes)
    })
}
/// Read owner facts and their sequence fence on a background caller, without the event queue.
/// # Safety
/// `json` must be readable for len bytes during this call; NULL only for len=0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn om_host_read_snapshot(
    host: *mut OmHostHandle,
    json: *const u8,
    len: usize,
) -> OmHostBuffer {
    packet(|| {
        let guard = registry::acquire(host, true).map_err(str::to_owned)?;
        // SAFETY: exported caller-readable contract applies and input is copied before decode.
        guard
            .host()
            .read_snapshot(&unsafe { copied_input(json, len, protocol::MAX_CONTROL_BYTES) }?)
    })
}
/// Trusted source/editor/physical-store control port; JSON is copied and decoded in safe Rust.
/// # Safety
/// `json` must point to len readable bytes for this call; NULL only for zero length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn om_host_source_command(
    host: *mut OmHostHandle,
    json: *const u8,
    len: usize,
) -> OmHostBuffer {
    packet(|| {
        let guard = registry::acquire(host, false).map_err(str::to_owned)?;
        // SAFETY: exported readable-buffer contract, bounded copy before any deferred use.
        guard
            .host()
            .source_command(&unsafe { copied_input(json, len, protocol::MAX_CONTROL_BYTES) }?)
    })
}
/// Trusted kernel/source/physical-store port; bounded input is copied before safe Rust handles it.
/// # Safety
/// `json` must point to len readable bytes during this call; NULL only for zero length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn om_host_kernel_command(
    host: *mut OmHostHandle,
    json: *const u8,
    len: usize,
) -> OmHostBuffer {
    packet(|| {
        let guard = registry::acquire(host, true).map_err(str::to_owned)?;
        // SAFETY: exported readable-buffer contract, bounded copy before any deferred use.
        guard
            .host()
            .kernel_command(&unsafe { copied_input(json, len, protocol::MAX_CONTROL_BYTES) }?)
    })
}
/// Trusted immutable-result command/status port. Input is copied and safe Rust validates scope.
/// # Safety
/// `json` must point to len readable bytes during this call; NULL only for zero length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn om_host_result_command(
    host: *mut OmHostHandle,
    json: *const u8,
    len: usize,
) -> OmHostBuffer {
    packet(|| {
        let guard = registry::acquire(host, true).map_err(str::to_owned)?;
        // SAFETY: exported caller-readable contract; bounded copy before any background use.
        guard
            .host()
            .result_command(&unsafe { copied_input(json, len, protocol::MAX_CONTROL_BYTES) }?)
    })
}
/// Signal the operation directly. 1=signalled, 0=terminal; negative codes are in the C header.
/// # Safety
/// `operation_id` must be readable for len bytes during this call; NULL only for len=0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn om_host_cancel(
    host: *mut OmHostHandle,
    operation_id: *const u8,
    len: usize,
) -> i32 {
    integer(|| {
        let guard = registry::acquire(host, true)?;
        // SAFETY: exported readable-input contract applies; IDs have a strict small byte budget.
        let bytes =
            unsafe { copied_input(operation_id, len, 256) }.map_err(|_| "INVALID_ARGUMENT")?;
        let id = std::str::from_utf8(&bytes).map_err(|_| "INVALID_ARGUMENT")?;
        if id.is_empty()
            || !id
                .bytes()
                .enumerate()
                .all(|(i, c)| c.is_ascii_alphanumeric() || (i > 0 && b"._:-".contains(&c)))
        {
            return Err("INVALID_ARGUMENT");
        }
        guard.host().cancel(id).map(i32::from).map_err(|error| {
            if error == "INVALID_REFERENCE" {
                "INVALID_REFERENCE"
            } else {
                "INTERNAL_ERROR"
            }
        })
    })
}
/// Revoke admission and signal cancellation without joining or waiting for CAS.
#[unsafe(no_mangle)]
pub extern "C" fn om_host_close_begin(host: *mut OmHostHandle) -> OmHostBuffer {
    packet(|| {
        registry::acquire(host, true)
            .map_err(str::to_owned)?
            .close_begin()
            .map_err(str::to_owned)?;
        Ok(json!({"protocol_version":1,"closing":true}))
    })
}
/// Background-only. Consume the handle, wait for guards/owners to settle, and release the service.
#[unsafe(no_mangle)]
pub extern "C" fn om_host_close_finish(host: *mut OmHostHandle) -> i32 {
    integer(|| {
        registry::finish(host)?;
        Ok(0)
    })
}
/// Release exactly the registered allocation; empty/no-longer-owned/mismatched buffer is ignored.
/// Do not reuse a copied buffer after freeing it; allocation addresses can be reused for buffers.
#[unsafe(no_mangle)]
pub extern "C" fn om_host_buffer_free(buffer: OmHostBuffer) {
    let _ = catch_unwind(AssertUnwindSafe(|| registry::free(buffer)));
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packet_and_integer_panics_do_not_cross_the_abi() {
        let _serial = registry::TEST_LOCK.lock().unwrap();
        let buffer = packet::<serde_json::Value>(|| panic!("synthetic private payload"));
        // SAFETY: output is still registered and this test reads exactly its reported length.
        let value: serde_json::Value =
            serde_json::from_slice(unsafe { std::slice::from_raw_parts(buffer.ptr, buffer.len) })
                .unwrap();
        assert_eq!(value["error"]["code"], "INTERNAL_ERROR");
        assert!(!value.to_string().contains("synthetic private payload"));
        om_host_buffer_free(buffer);
        assert_eq!(integer(|| panic!("fixture")), -4);
    }
}
