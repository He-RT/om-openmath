//! The one authoritative Job consumer for browser strings and native response bytes.
mod profile;
mod result;
mod tools;
use super::Session;
use crate::protocol::*;
use om_llm::{Feature, Job, JobStep, LlmError, StreamEvent, Target};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
const ACTIVE: usize = 16;
const FINISHED: usize = 256;

/// Independent cancellation usable while the Session owner is executing queued work.
#[derive(Clone)]
pub struct LlmCancellation {
    flag: Arc<AtomicBool>,
    #[cfg(feature = "native")]
    token: tokio_util::sync::CancellationToken,
}
impl Default for LlmCancellation {
    fn default() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "native")]
            token: tokio_util::sync::CancellationToken::new(),
        }
    }
}
impl LlmCancellation {
    /// Cancel native IO and the independent portable CAS tool budget.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Relaxed);
        #[cfg(feature = "native")]
        self.token.cancel();
    }
    /// Native HTTP token for a host that queues real bytes back to the Session.
    #[cfg(feature = "native")]
    pub fn token(&self) -> tokio_util::sync::CancellationToken {
        self.token.clone()
    }
}
struct Pending {
    job: Job,
    feature: Feature,
    cancel: LlmCancellation,
    profile: String,
    timeout: u64,
    start: Option<f64>,
    first: Option<f64>,
    status: Option<u16>,
    completion: Option<(String, String, Dialect)>,
}
pub(super) struct LlmState {
    pub(super) target: Target,
    jobs: BTreeMap<String, Pending>,
    finished: BTreeSet<String>,
    order: VecDeque<String>,
}
impl Default for LlmState {
    fn default() -> Self {
        Self {
            target: Target::Browser,
            jobs: BTreeMap::new(),
            finished: BTreeSet::new(),
            order: VecDeque::new(),
        }
    }
}
impl Session {
    /// Select host transport before starting any jobs; no IO occurs in this method.
    pub fn set_llm_target(&mut self, target: Target) -> Result<(), String> {
        if !self.llm.jobs.is_empty() {
            return Err("Cannot change transport with active jobs".into());
        }
        #[cfg(not(any(feature = "native", feature = "external-host")))]
        if target == Target::Native {
            return Err("Native transport feature is unavailable".into());
        }
        self.llm.target = target;
        Ok(())
    }
    /// Clone an active job's cancellation scope without borrowing the Session owner.
    pub fn llm_cancellation_handle(&self, id: &str) -> Option<LlmCancellation> {
        self.llm.jobs.get(id).map(|p| p.cancel.clone())
    }
    /// Configured round timeout for a native host transport, never a secret profile DTO.
    pub fn llm_http_timeout(&self, id: &str) -> Option<u64> {
        self.llm.jobs.get(id).map(|p| p.timeout)
    }
    fn llm_now(&self) -> Option<f64> {
        self.clock
            .as_ref()
            .map(|c| c.now_ms())
            .filter(|x| x.is_finite())
    }
    fn llm_error(&self, id: &str, message: String) -> (Response, Vec<Event>) {
        let message = format!(
            "err.llm: {}",
            self.localized(
                &format!("AI 请求失败：{message}"),
                &format!("AI request failed: {message}")
            )
        );
        (
            Response::Error {
                message: message.clone(),
            },
            vec![Event::LlmError {
                request_id: id.into(),
                message,
            }],
        )
    }
    fn unknown_llm(&self, id: &str) -> (Response, Vec<Event>) {
        if self.llm.finished.contains(id) {
            (Response::Ok, vec![])
        } else {
            self.llm_error(id, self.localized("未知 AI 请求", "Unknown AI request"))
        }
    }
    fn remember_llm(&mut self, id: String) {
        if self.llm.finished.insert(id.clone()) {
            self.llm.order.push_back(id);
        }
        while self.llm.order.len() > FINISHED {
            if let Some(id) = self.llm.order.pop_front() {
                self.llm.finished.remove(&id);
            }
        }
    }
    pub(super) fn cancel_all_llm(&mut self) -> Vec<Event> {
        let mut events = vec![];
        for (id, mut p) in std::mem::take(&mut self.llm.jobs) {
            p.cancel.cancel();
            p.job.cancel();
            self.remember_llm(id.clone());
            events.push(Event::LlmError {
                request_id: id,
                message: self.localized("AI 请求已取消", "AI request cancelled"),
            });
        }
        events
    }
    pub(super) fn handle_llm(&mut self, req: Request) -> (Response, Vec<Event>) {
        match req {
            Request::LlmHttpChunk {
                request_id,
                chunk,
                status,
            } => self.llm_http_bytes(&request_id, status.unwrap_or(200), chunk.as_bytes()),
            Request::LlmHttpEnd {
                request_id,
                status,
                error,
            } => self.end_llm(&request_id, status, error),
            Request::LlmCancel { request_id } => {
                let Some(mut pending) = self.llm.jobs.remove(&request_id) else {
                    return (Response::Ok, vec![]);
                };
                pending.cancel.cancel();
                let step = pending.job.cancel();
                let events = self.llm_step(&request_id, pending, step);
                (Response::Ok, events)
            }
            _ => self.start_llm(req),
        }
    }
    fn start_llm(&mut self, req: Request) -> (Response, Vec<Event>) {
        let id = match &req {
            Request::LlmTranslate { request_id, .. }
            | Request::LlmExplain { request_id, .. }
            | Request::LlmComplete { request_id, .. }
            | Request::LlmChat { request_id, .. }
            | Request::LlmFixError { request_id, .. }
            | Request::LlmTestProfile { request_id, .. } => request_id.clone(),
            _ => return self.llm_error("", "Invalid LLM start request".into()),
        };
        if id.is_empty() || id.len() > 256 || id.chars().any(char::is_control) {
            return self.llm_error(&id, "Invalid request ID".into());
        }
        if self.llm.jobs.contains_key(&id) || self.llm.finished.contains(&id) {
            // Only the duplicate caller fails; a terminal event with the existing ID
            // would incorrectly make its legitimate client stop the active transport.
            return (
                Response::Error {
                    message: self.localized(
                        "err.llm: 重复的 AI 请求 ID",
                        "err.llm: Duplicate AI request ID",
                    ),
                },
                vec![],
            );
        }
        if self.llm.jobs.len() >= ACTIVE {
            return self.llm_error(&id, "Active job limit exceeded".into());
        }
        let (feature, input) = match self.prepare_llm_input(&req) {
            Ok(v) => v,
            Err(e) => return self.llm_error(&id, e),
        };
        let profile = match self.llm_profile(&req, feature) {
            Ok(p) => p,
            Err(e) => return self.llm_error(&id, e),
        };
        let name = profile.name.clone();
        let timeout = profile.timeout_ms;
        let (job, step) = Job::new(feature, profile, input, self.llm.target);
        let JobStep::Http(http) = step else {
            let JobStep::Failed(error) = step else {
                return self.llm_error(&id, "Invalid initial job action".into());
            };
            return self.llm_error(&id, error.to_string());
        };
        let completion = if let Request::LlmComplete {
            prefix,
            suffix,
            dialect,
            ..
        } = req
        {
            Some((prefix, suffix, dialect))
        } else {
            None
        };
        self.llm.jobs.insert(
            id.clone(),
            Pending {
                job,
                feature,
                cancel: Default::default(),
                profile: name,
                timeout,
                start: self.llm_now(),
                first: None,
                status: None,
                completion,
            },
        );
        (
            Response::LlmStarted {
                request_id: id,
                http: Some(http),
            },
            vec![],
        )
    }
    /// Feed actual native bytes and status to the same Job used by LlmHttpChunk.
    pub fn llm_http_bytes(
        &mut self,
        id: &str,
        status: u16,
        bytes: &[u8],
    ) -> (Response, Vec<Event>) {
        let Some(mut pending) = self.llm.jobs.remove(id) else {
            return self.unknown_llm(id);
        };
        if pending.cancel.flag.load(Ordering::Relaxed) {
            let step = pending.job.cancel();
            return (Response::Ok, self.llm_step(id, pending, step));
        }
        if pending.status.is_some_and(|old| old != status) {
            let step = pending
                .job
                .on_http_end(0, Some("Conflicting HTTP response status".into()));
            return (Response::Ok, self.llm_step(id, pending, step));
        }
        pending.status = Some(status);
        if !bytes.is_empty() && pending.first.is_none() {
            pending.first = self.llm_now();
        }
        let actual = pending.job.on_raw_bytes(bytes);
        let mut events = vec![];
        if (200..300).contains(&status) {
            for event in actual {
                if let StreamEvent::Text(text) = event
                    && matches!(
                        pending.feature,
                        Feature::Chat | Feature::Explain | Feature::TestProfile
                    )
                {
                    events.push(Event::LlmDelta {
                        request_id: id.into(),
                        text,
                    });
                }
            }
        }
        if pending.job.is_finished() {
            let step = pending.job.on_http_end(status, None);
            events.extend(self.llm_step(id, pending, step));
            return (Response::Ok, events);
        }
        self.llm.jobs.insert(id.into(), pending);
        (Response::Ok, events)
    }
    fn end_llm(&mut self, id: &str, status: u16, error: Option<String>) -> (Response, Vec<Event>) {
        let Some(mut pending) = self.llm.jobs.remove(id) else {
            return self.unknown_llm(id);
        };
        let step = if pending.cancel.flag.load(Ordering::Relaxed) {
            pending.job.cancel()
        } else if pending.status.is_some_and(|old| old != status) && error.is_none() {
            pending
                .job
                .on_http_end(0, Some("Conflicting HTTP response status".into()))
        } else {
            pending.job.on_http_end(status, error)
        };
        (Response::Ok, self.llm_step(id, pending, step))
    }
}
