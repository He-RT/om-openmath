//! Each accepted operation owns a cancellation token and monotonic terminal state.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

/// Actual operation state, distinct from a tool's request-level success.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Admitted but not started.
    Queued,
    /// Worker is executing.
    Running,
    /// Stop signalled; worker has not settled.
    Cancelling,
    /// Actual result completed.
    Completed,
    /// Actual worker settled after cancellation.
    Cancelled,
    /// Worker or computation failed.
    Failed,
}
/// Retained actual outcome; source data is not included in Debug.
#[derive(Clone, Serialize)]
pub struct OperationStatus {
    /// Trusted operation identity.
    pub operation_ref: String,
    /// Actual lifecycle phase.
    pub phase: Phase,
    /// Terminal result, bounded and retained for resynchronization.
    pub result: Option<Value>,
    /// Structured failure code, when present.
    pub error_code: Option<String>,
}
struct Entry {
    status: OperationStatus,
    cancel: Arc<AtomicBool>,
}
/// Admission/stop/terminal facts are serialized under one short lock, never a CAS lock.
pub(crate) struct Operations {
    state: Mutex<State>,
    capacity: usize,
}
struct State {
    entries: BTreeMap<String, Entry>,
    recent: VecDeque<String>,
}
impl Operations {
    pub fn new(capacity: usize) -> Self {
        Self {
            state: Mutex::new(State {
                entries: BTreeMap::new(),
                recent: VecDeque::new(),
            }),
            capacity,
        }
    }
    pub fn admit(&self, id: &str) -> Result<Arc<AtomicBool>, &'static str> {
        let mut state = self.state.lock().map_err(|_| "INTERNAL_ERROR")?;
        let entries = &mut state.entries;
        if entries.contains_key(id) {
            return Err("DUPLICATE_OPERATION");
        }
        if entries
            .values()
            .filter(|e| {
                !matches!(
                    e.status.phase,
                    Phase::Completed | Phase::Cancelled | Phase::Failed
                )
            })
            .count()
            >= self.capacity
            || entries.len() >= 4096
        {
            return Err("BUDGET_EXCEEDED");
        }
        let token = Arc::new(AtomicBool::new(false));
        entries.insert(
            id.into(),
            Entry {
                status: OperationStatus {
                    operation_ref: id.into(),
                    phase: Phase::Queued,
                    result: None,
                    error_code: None,
                },
                cancel: token.clone(),
            },
        );
        Ok(token)
    }
    pub fn start(&self, id: &str) -> bool {
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        let Some(entry) = state.entries.get_mut(id) else {
            return false;
        };
        if !matches!(entry.status.phase, Phase::Queued | Phase::Cancelling) {
            return false;
        }
        if entry.cancel.load(Ordering::Acquire) {
            return false;
        }
        entry.status.phase = Phase::Running;
        true
    }
    pub fn cancel(&self, id: &str) -> Result<bool, &'static str> {
        let mut state = self.state.lock().map_err(|_| "INTERNAL_ERROR")?;
        let entry = state.entries.get_mut(id).ok_or("INVALID_REFERENCE")?;
        if matches!(
            entry.status.phase,
            Phase::Completed | Phase::Cancelled | Phase::Failed
        ) {
            return Ok(false);
        }
        entry.cancel.store(true, Ordering::Release);
        entry.status.phase = Phase::Cancelling;
        Ok(true)
    }
    pub fn finish(&self, id: &str, result: Result<Value, String>) -> Option<OperationStatus> {
        let mut state = self.state.lock().ok()?;
        let entry = state.entries.get_mut(id)?;
        if matches!(
            entry.status.phase,
            Phase::Completed | Phase::Failed | Phase::Cancelled
        ) {
            return None;
        }
        if entry.cancel.load(Ordering::Acquire) {
            entry.status.phase = Phase::Cancelled;
            entry.status.result = None;
            entry.status.error_code = Some("CANCELLED".into())
        } else {
            match result {
                Ok(result) => {
                    entry.status.phase = Phase::Completed;
                    entry.status.result = Some(result)
                }
                Err(code) => {
                    entry.status.phase = Phase::Failed;
                    entry.status.error_code = Some(code)
                }
            }
        }
        let completed = entry.status.clone();
        state.recent.push_back(id.to_owned());
        // Expire by completion order, never by a user-selected lexical ID.
        while state.recent.len() > 32 {
            if let Some(key) = state.recent.pop_front()
                && let Some(old) = state.entries.get_mut(&key)
                && old.status.result.is_some()
            {
                old.status.result = None;
                old.status.error_code = Some("RESULT_DETAILS_EXPIRED".into());
            }
        }
        Some(completed)
    }
    pub fn token(&self, id: &str) -> Option<Arc<AtomicBool>> {
        self.state
            .lock()
            .ok()?
            .entries
            .get(id)
            .map(|e| e.cancel.clone())
    }
    pub fn status(&self, id: &str) -> Option<OperationStatus> {
        self.state
            .lock()
            .ok()?
            .entries
            .get(id)
            .map(|e| e.status.clone())
    }
    pub fn all(&self) -> Vec<OperationStatus> {
        self.state
            .lock()
            .map(|state| state.entries.values().map(|e| e.status.clone()).collect())
            .unwrap_or_default()
    }
    pub fn stop_all(&self) {
        if let Ok(mut state) = self.state.lock() {
            for entry in state.entries.values_mut() {
                if !matches!(
                    entry.status.phase,
                    Phase::Completed | Phase::Failed | Phase::Cancelled
                ) {
                    entry.cancel.store(true, Ordering::Release);
                    entry.status.phase = Phase::Cancelling;
                }
            }
        }
    }
    pub fn summaries(&self) -> Vec<OperationStatus> {
        self.state
            .lock()
            .map(|state| {
                state
                    .entries
                    .values()
                    .filter(|e| {
                        !matches!(
                            e.status.phase,
                            Phase::Completed | Phase::Cancelled | Phase::Failed
                        ) || state.recent.contains(&e.status.operation_ref)
                    })
                    .map(|e| OperationStatus {
                        operation_ref: e.status.operation_ref.clone(),
                        phase: e.status.phase.clone(),
                        result: None,
                        error_code: e.status.error_code.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}
