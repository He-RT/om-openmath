//! Stable opaque tokens, short guards and two-phase closing; never dereference a C handle.
use crate::ffi::{OmHostBuffer, OmHostHandle};
use om_host_service::scheduler::NativeHost;
use std::{
    collections::BTreeMap,
    sync::{Arc, Condvar, Mutex, OnceLock},
};
struct Calls {
    active: usize,
    closing: bool,
}
pub(crate) struct Entry {
    pub host: NativeHost,
    calls: Mutex<Calls>,
    settled: Condvar,
}
/// An admitted ABI call keeps its owner alive until all use of the host has ended.
pub(crate) struct Guard(Arc<Entry>);
impl Guard {
    pub fn host(&self) -> &NativeHost {
        &self.0.host
    }
    pub fn close_begin(&self) -> Result<(), &'static str> {
        self.0.close_begin()
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        // Recover only to release the lifetime count; future admission still rejects poison.
        let mut calls = self.0.calls.lock().unwrap_or_else(|p| p.into_inner());
        calls.active -= 1;
        if calls.active == 0 {
            self.0.settled.notify_all();
        }
    }
}
impl Entry {
    fn close_begin(&self) -> Result<(), &'static str> {
        self.calls.lock().map_err(|_| "INTERNAL_ERROR")?.closing = true;
        self.host.close_begin();
        Ok(())
    }
    fn finish(&self) -> Result<(), &'static str> {
        self.close_begin()?;
        let mut calls = self.calls.lock().map_err(|_| "INTERNAL_ERROR")?;
        while calls.active != 0 {
            calls = self.settled.wait(calls).map_err(|_| "INTERNAL_ERROR")?;
        }
        drop(calls);
        self.host.close_finish().map_err(|_| "INTERNAL_ERROR")
    }
}
#[derive(Default)]
struct Registry {
    live: BTreeMap<usize, Arc<Entry>>,
    // Small permanent tombstones prevent an old C pointer from aliasing a new host (ABA).
    // Bounded to 4096 process-lifetime creations. The full NativeHost is removed at finish.
    tokens: BTreeMap<usize, Box<OmHostHandle>>,
}
static HOSTS: OnceLock<Mutex<Registry>> = OnceLock::new();
#[cfg(test)]
pub(crate) static TEST_LOCK: Mutex<()> = Mutex::new(());
fn hosts() -> &'static Mutex<Registry> {
    HOSTS.get_or_init(Default::default)
}
pub(crate) fn insert(host: NativeHost) -> Result<*mut OmHostHandle, &'static str> {
    let mut registry = hosts().lock().map_err(|_| "INTERNAL_ERROR")?;
    if registry.live.len() >= 128 || registry.tokens.len() >= 4096 {
        return Err("BUDGET_EXCEEDED");
    }
    let mut token = Box::new(OmHostHandle {
        _token: registry.tokens.len() + 1,
    });
    let pointer = &mut *token as *mut OmHostHandle;
    registry.tokens.insert(pointer as usize, token);
    registry.live.insert(
        pointer as usize,
        Arc::new(Entry {
            host,
            calls: Mutex::new(Calls {
                active: 0,
                closing: false,
            }),
            settled: Condvar::new(),
        }),
    );
    Ok(pointer)
}
pub(crate) fn acquire(
    pointer: *mut OmHostHandle,
    allow_closing: bool,
) -> Result<Guard, &'static str> {
    // Hold registry admission until the guard count is incremented. Otherwise finish could
    // remove/join between the Arc clone and increment, returning before a real call settles.
    let registry = hosts().lock().map_err(|_| "INTERNAL_ERROR")?;
    let entry = registry
        .live
        .get(&(pointer as usize))
        .cloned()
        .ok_or("INVALID_REFERENCE")?;
    {
        let mut calls = entry.calls.lock().map_err(|_| "INTERNAL_ERROR")?;
        if calls.closing && !allow_closing {
            return Err("HOST_CLOSING");
        }
        if calls.active >= 256 {
            return Err("BUDGET_EXCEEDED");
        }
        calls.active += 1;
    }
    drop(registry);
    Ok(Guard(entry))
}
pub(crate) fn finish(pointer: *mut OmHostHandle) -> Result<(), &'static str> {
    let entry = hosts()
        .lock()
        .map_err(|_| "INTERNAL_ERROR")?
        .live
        .remove(&(pointer as usize))
        .ok_or("INVALID_REFERENCE")?;
    entry.finish()
}
#[derive(Default)]
struct Buffers {
    bytes: usize,
    owned: BTreeMap<usize, Box<[u8]>>,
}
static BUFFERS: OnceLock<Mutex<Buffers>> = OnceLock::new();
fn buffers() -> &'static Mutex<Buffers> {
    BUFFERS.get_or_init(Default::default)
}
pub(crate) fn output(bytes: Vec<u8>) -> OmHostBuffer {
    if bytes.is_empty() {
        return OmHostBuffer::empty();
    }
    let Ok(mut buffers) = buffers().lock() else {
        return OmHostBuffer::empty();
    };
    if buffers.owned.len() >= 1024 || buffers.bytes.saturating_add(bytes.len()) > 64 * 1024 * 1024 {
        return OmHostBuffer::empty();
    }
    let mut bytes = bytes.into_boxed_slice();
    let buffer = OmHostBuffer {
        ptr: bytes.as_mut_ptr(),
        len: bytes.len(),
    };
    buffers.bytes += buffer.len;
    buffers.owned.insert(buffer.ptr as usize, bytes);
    buffer
}
pub(crate) fn free(buffer: OmHostBuffer) {
    if let Ok(mut buffers) = buffers().lock()
        && buffers
            .owned
            .get(&(buffer.ptr as usize))
            .is_some_and(|bytes| bytes.len() == buffer.len)
        && let Some(bytes) = buffers.owned.remove(&(buffer.ptr as usize))
    {
        buffers.bytes -= bytes.len();
        // Drop only our stored allocation; no reconstruction/dereference of caller-supplied ptr.
    }
}
#[cfg(test)]
pub(crate) fn counts() -> (usize, usize, usize) {
    let live = hosts().lock().unwrap().live.len();
    let buffers = buffers().lock().unwrap();
    (live, buffers.owned.len(), buffers.bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finishing_waits_for_real_active_guard_and_stale_token_never_rebinds() {
        let _serial = TEST_LOCK.lock().unwrap();
        let init=om_host_service::protocol::wire::decode_init(br#"{"protocol_version":1,"runtime_instance_id":"registry-test","max_pending_operations":2,"event_capacity":2}"#).unwrap();
        let pointer = insert(NativeHost::new(init.clone()).unwrap()).unwrap();
        let guard = acquire(pointer, false).unwrap();
        let address = pointer as usize;
        let (sender, receiver) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            sender.send(finish(address as *mut OmHostHandle)).unwrap();
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while acquire(pointer, false).is_ok() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(
            receiver
                .recv_timeout(std::time::Duration::from_millis(20))
                .is_err()
        );
        assert!(acquire(pointer, false).is_err());
        drop(guard);
        assert!(
            receiver
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap()
                .is_ok()
        );
        thread.join().unwrap();
        let second = insert(NativeHost::new(init).unwrap()).unwrap();
        assert_ne!(pointer, second);
        assert!(acquire(pointer, true).is_err());
        finish(second).unwrap();
    }
}
