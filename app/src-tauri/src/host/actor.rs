//! The only native Session lives here; transport futures exchange acknowledged byte messages.
use super::{EventSink, Input, Shared};
use om_kernel::{Session, native::ConfigStore, protocol::*};
use om_num::ctx::Clock;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use tokio::runtime::Handle;
struct NativeClock(Instant);
impl Clock for NativeClock {
    fn now_ms(&self) -> f64 {
        self.0.elapsed().as_secs_f64() * 1000.0
    }
}
pub(super) fn run(
    store: ConfigStore,
    rx: mpsc::Receiver<Input>,
    tx: mpsc::SyncSender<Input>,
    runtime: Handle,
    ready: mpsc::SyncSender<Result<Arc<Shared>, String>>,
) {
    let mut session = match Session::new_native(store, Some(Arc::new(NativeClock(Instant::now()))))
    {
        Ok(session) => session,
        Err(error) => {
            let _ = ready.send(Err(error.to_string()));
            return;
        }
    };
    let client = match om_llm::native_client() {
        Ok(client) => client,
        Err(error) => {
            let _ = ready.send(Err(error.to_string()));
            return;
        }
    };
    let shared = Arc::new(Shared {
        interrupt: session.interrupt_handle(),
        closed: AtomicBool::new(false),
        jobs: Mutex::new(BTreeMap::new()),
        events: Mutex::new(None),
    });
    if ready.send(Ok(shared.clone())).is_err() {
        return;
    }
    let host = Actor {
        tx,
        runtime,
        shared,
        client,
    };
    loop {
        if host.shared.closed.load(Ordering::Relaxed) {
            break;
        }
        let input = match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(input) => input,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        match input {
            Input::Request(raw, reply) => {
                let answer = (|| {
                    let request: Envelope<Request> =
                        serde_json::from_str(&raw).map_err(|_| "Invalid kernel envelope")?;
                    let (mut response, events) = session.handle(request.body);
                    if let Response::LlmStarted { request_id, http } = &mut response
                        && let Some(http) = http.take()
                    {
                        host.http(&session, request_id, http);
                    }
                    host.events(&session, events);
                    serde_json::to_string(&Envelope {
                        id: request.id,
                        body: response,
                    })
                    .map_err(|_| "Kernel response serialization failed".into())
                })();
                let _ = reply.send(answer);
            }
            Input::Bytes {
                id,
                status,
                bytes,
                ack,
            } => {
                let (_, events) = session.llm_http_bytes(&id, status, &bytes);
                host.events(&session, events);
                let _ = ack.send(session.llm_cancellation_handle(&id).is_some());
            }
            Input::End { id, status, error } => {
                let (_, events) = session.handle(Request::LlmHttpEnd {
                    request_id: id,
                    status,
                    error,
                });
                host.events(&session, events);
            }
            Input::Secret {
                profile,
                key,
                reply,
            } => {
                let result = session
                    .set_profile_secret(&profile, key.as_deref())
                    .map_err(|e| e.to_string());
                let _ = reply.send(result);
            }
            Input::Stop => break,
        }
    }
    if let Ok(jobs) = host.shared.jobs.lock() {
        for job in jobs.values() {
            job.cancel();
        }
    }
}
struct Actor {
    tx: mpsc::SyncSender<Input>,
    runtime: Handle,
    shared: Arc<Shared>,
    client: reqwest::Client,
}
impl Actor {
    fn emit(&self, event: Event) {
        let sink: Option<EventSink> = self.shared.events.lock().ok().and_then(|sink| sink.clone());
        if let Some(sink) = sink
            && let Ok(value) = serde_json::to_string(&Envelope { id: 0, body: event })
        {
            sink(value);
        }
    }
    fn events(&self, session: &Session, events: Vec<Event>) {
        for event in events {
            if let Event::LlmHttp { request_id, http } = event {
                self.http(session, &request_id, http);
                continue;
            }
            if let Event::LlmDone { request_id } | Event::LlmError { request_id, .. } = &event
                && let Ok(mut jobs) = self.shared.jobs.lock()
            {
                jobs.remove(request_id);
            }
            self.emit(event);
        }
    }
    fn http(&self, session: &Session, id: &str, http: HttpRequest) {
        let Some(cancel) = session.llm_cancellation_handle(id) else {
            return;
        };
        let timeout = session.llm_http_timeout(id).unwrap_or(60_000);
        let id = id.to_owned();
        if let Ok(mut jobs) = self.shared.jobs.lock() {
            jobs.insert(id.clone(), cancel.clone());
        }
        let tx = self.tx.clone();
        let shared = self.shared.clone();
        let client = self.client.clone();
        let runtime = self.runtime.clone();
        // Byte acknowledgements apply synchronous backpressure; isolate them from the
        // two async IO/timer workers so one CAS operation cannot starve HTTP cancellation.
        self.runtime.spawn_blocking(move || {
            runtime.block_on(async move {
                let started = Instant::now();
                let token = cancel.token();
                let duration = Duration::from_millis(timeout);
                let mut timed_out = false;
                let (status, mut error) =
                    om_llm::drive_native_http(&http, timeout, &client, &token, |status, bytes| {
                        let (ack, rx) = mpsc::sync_channel(1);
                        let mut input = Input::Bytes {
                            id: id.clone(),
                            status,
                            bytes: bytes.to_vec(),
                            ack,
                        };
                        loop {
                            if shared.closed.load(Ordering::Relaxed) || token.is_cancelled() {
                                return false;
                            }
                            if started.elapsed() >= duration {
                                timed_out = true;
                                return false;
                            }
                            match tx.try_send(input) {
                                Ok(()) => break,
                                Err(mpsc::TrySendError::Disconnected(_)) => return false,
                                Err(mpsc::TrySendError::Full(value)) => {
                                    input = value;
                                    std::thread::sleep(Duration::from_millis(1));
                                }
                            }
                        }
                        loop {
                            if shared.closed.load(Ordering::Relaxed) || token.is_cancelled() {
                                return false;
                            }
                            let remaining = duration.saturating_sub(started.elapsed());
                            if remaining.is_zero() {
                                timed_out = true;
                                return false;
                            }
                            match rx.recv_timeout(remaining.min(Duration::from_millis(5))) {
                                Ok(value) => return value,
                                Err(mpsc::RecvTimeoutError::Disconnected) => return false,
                                Err(mpsc::RecvTimeoutError::Timeout) => {}
                            }
                        }
                    })
                    .await;
                if timed_out {
                    error = Some("HTTP request timed out".into());
                }
                let _ = tx.send(Input::End { id, status, error });
            })
        });
    }
}
