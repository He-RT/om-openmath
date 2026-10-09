//! Bounded events preserve a resync signal instead of silently losing terminal facts.
use crate::protocol::{
    Nullable, Serial,
    generated::{EventBatch, EventEnvelope, EventEnvelopeEventKind},
};
use serde_json::Value;
use std::{
    collections::VecDeque,
    sync::{Condvar, Mutex},
    time::Duration,
};
struct State {
    sequence: Serial,
    events: VecDeque<EventEnvelope>,
    resync: bool,
    closed: bool,
}
pub(crate) struct Events {
    state: Mutex<State>,
    ready: Condvar,
    capacity: usize,
    runtime: String,
}
impl Events {
    pub fn new(runtime: String, capacity: usize) -> Self {
        Self {
            state: Mutex::new(State {
                sequence: Serial::new(0).expect("invariant: zero serial"),
                events: VecDeque::new(),
                resync: false,
                closed: false,
            }),
            ready: Condvar::new(),
            capacity,
            runtime,
        }
    }
    pub fn emit(
        &self,
        kind: EventEnvelopeEventKind,
        request: Option<String>,
        operation: Option<String>,
        payload: Value,
    ) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let Ok(next) = state.sequence.checked_next() else {
            state.closed = true;
            state.resync = true;
            self.ready.notify_all();
            return;
        };
        state.sequence = next;
        if state.events.len() >= self.capacity {
            state.events.pop_front();
            state.resync = true;
        }
        state.events.push_back(EventEnvelope {
            protocol_version: 1,
            runtime_instance_id: self.runtime.clone(),
            rust_event_sequence: next,
            request_ref: Nullable(request),
            operation_ref: Nullable(operation),
            document_binding: Nullable(None),
            event_kind: kind,
            payload,
        });
        self.ready.notify_all();
    }
    pub fn poll(&self, wait_ms: u32, max_bytes: usize) -> Result<EventBatch, &'static str> {
        if !(128..=512 * 1024).contains(&max_bytes) || wait_ms > 100 {
            return Err("INVALID_ARGUMENT");
        }
        let state = self.state.lock().map_err(|_| "INTERNAL_ERROR")?;
        let mut state = if state.events.is_empty() && !state.closed {
            self.ready
                .wait_timeout(state, Duration::from_millis(wait_ms.into()))
                .map_err(|_| "INTERNAL_ERROR")?
                .0
        } else {
            state
        };
        let mut events = Vec::new();
        let mut used = serde_json::to_vec(&serde_json::json!({"protocol_version":1,"events":[],"needs_resync":false,"last_rust_event_sequence":state.sequence})).map_err(|_| "INTERNAL_ERROR")?.len();
        while let Some(next) = state.events.front() {
            let bytes = serde_json::to_vec(next)
                .map_err(|_| "INTERNAL_ERROR")?
                .len()
                + 1;
            if used + bytes > max_bytes {
                if events.is_empty() {
                    state.events.pop_front();
                    state.resync = true;
                }
                break;
            }
            used += bytes;
            events.push(
                state
                    .events
                    .pop_front()
                    .expect("invariant: observed queued event"),
            );
        }
        let resync = std::mem::take(&mut state.resync);
        Ok(EventBatch {
            protocol_version: 1,
            events,
            needs_resync: resync,
            last_rust_event_sequence: state.sequence,
        })
    }
    pub fn sequence(&self) -> Result<Serial, &'static str> {
        self.state
            .lock()
            .map(|state| state.sequence)
            .map_err(|_| "INTERNAL_ERROR")
    }
    pub fn close(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.closed = true;
            self.ready.notify_all();
        }
    }
}
