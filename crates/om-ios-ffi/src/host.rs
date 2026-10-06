//! Safe Session ownership, bounded messages and cancellation independent of its queue.
#![forbid(unsafe_code)]
use om_kernel::{KernelConfig, LlmCancellation, Session, protocol::*};
use om_num::ctx::Clock;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
struct MonotonicClock(Instant);
impl Clock for MonotonicClock {
    fn now_ms(&self) -> f64 {
        self.0.elapsed().as_secs_f64() * 1000.0
    }
}
type Reply = mpsc::SyncSender<Result<String, String>>;
enum Input {
    Request(Box<Envelope<Request>>, Reply),
    Bytes {
        correlation: u64,
        id: String,
        status: u16,
        bytes: Vec<u8>,
        reply: Reply,
    },
    Stop,
}
struct Shared {
    closed: AtomicBool,
    interrupt: Arc<AtomicBool>,
    jobs: Mutex<BTreeMap<String, LlmCancellation>>,
}
#[derive(Serialize)]
struct Packet {
    response: Envelope<Response>,
    events: Vec<Envelope<Event>>,
}
/// A portable Session on its own thread; it performs no filesystem, credential or HTTP IO.
pub struct Host {
    tx: mpsc::SyncSender<Input>,
    shared: Arc<Shared>,
    thread: Mutex<Option<JoinHandle<()>>>,
}
impl Host {
    /// Initialize validated in-memory settings. Swift supplies actual ephemeral keys.
    pub fn new(config: Option<&str>) -> Result<Self, String> {
        let config: KernelConfig = config
            .map(serde_json::from_str)
            .transpose()
            .map_err(|_| "Invalid configuration")?
            .unwrap_or_default();
        let mut session = Session::new(config, Some(Arc::new(MonotonicClock(Instant::now()))));
        session.handle(Request::SetHostPlatform {
            platform: om_kernel::protocol::HostPlatform::Ios,
        });
        session.set_llm_target(om_llm::Target::Native)?;
        let shared = Arc::new(Shared {
            closed: AtomicBool::new(false),
            interrupt: session.interrupt_handle(),
            jobs: Mutex::new(BTreeMap::new()),
        });
        let worker = shared.clone();
        let (tx, rx) = mpsc::sync_channel(64);
        let thread = std::thread::Builder::new()
            .name("openmath-ios-kernel".into())
            .spawn(move || {
                crate::ffi::configure_owner_thread();
                while let Ok(input) = rx.recv() {
                    if worker.closed.load(Ordering::Acquire) {
                        break;
                    }
                    let (correlation, body, events, reply) = match input {
                        Input::Stop => break,
                        Input::Request(envelope, reply) => {
                            let (body, events) = session.handle(envelope.body);
                            (envelope.id, body, events, reply)
                        }
                        Input::Bytes {
                            correlation,
                            id,
                            status,
                            bytes,
                            reply,
                        } => {
                            let (body, events) = session.llm_http_bytes(&id, status, &bytes);
                            (correlation, body, events, reply)
                        }
                    };
                    if let Ok(mut jobs) = worker.jobs.lock() {
                        if let Response::LlmStarted { request_id, .. } = &body
                            && let Some(cancel) = session.llm_cancellation_handle(request_id)
                        {
                            jobs.insert(request_id.clone(), cancel);
                        }
                        for event in &events {
                            if let Event::LlmDone { request_id }
                            | Event::LlmError { request_id, .. } = event
                            {
                                jobs.remove(request_id);
                            }
                        }
                    }
                    let packet = Packet {
                        response: Envelope {
                            id: correlation,
                            body,
                        },
                        events: events
                            .into_iter()
                            .map(|body| Envelope { id: 0, body })
                            .collect(),
                    };
                    let result = serde_json::to_string(&packet)
                        .map_err(|_| "Cannot encode kernel packet".to_owned());
                    let _ = reply.send(result);
                }
                worker.closed.store(true, Ordering::Release);
            })
            .map_err(|_| "Cannot start kernel owner")?;
        Ok(Self {
            tx,
            shared,
            thread: Mutex::new(Some(thread)),
        })
    }
    fn wait(&self, rx: mpsc::Receiver<Result<String, String>>) -> Result<String, String> {
        loop {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(result) => return result,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err("Kernel owner stopped".into());
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if self.shared.closed.load(Ordering::Acquire) {
                        return Err("Kernel is closed".into());
                    }
                }
            }
        }
    }
    /// Dispatch one bounded JSON envelope and return actual response/events.
    pub fn request(&self, source: &str) -> Result<String, String> {
        if source.len() > 16 * 1024 * 1024 || self.shared.closed.load(Ordering::Acquire) {
            return Err("Kernel is closed or envelope exceeds limit".into());
        }
        let envelope: Envelope<Request> =
            serde_json::from_str(source).map_err(|_| "Invalid kernel envelope")?;
        if !om_kernel::protocol::request_size_allowed(source.len(), &envelope.body) {
            return Err("Kernel envelope exceeds limit".into());
        }
        if envelope.id == 0 || envelope.id > 9_007_199_254_740_991 {
            return Err("Invalid correlation ID".into());
        }
        let (tx, rx) = mpsc::sync_channel(1);
        self.tx
            .try_send(Input::Request(Box::new(envelope), tx))
            .map_err(|_| "Kernel queue is full or stopped")?;
        self.wait(rx)
    }
    /// Pass undecoded provider bytes to the sole Job's checked streaming decoder.
    pub fn feed(
        &self,
        correlation: u64,
        id: &str,
        status: u16,
        bytes: &[u8],
    ) -> Result<String, String> {
        if self.shared.closed.load(Ordering::Acquire)
            || correlation == 0
            || correlation > 9_007_199_254_740_991
            || id.len() > 8192
            || bytes.len() > 1_048_576
        {
            return Err("Invalid HTTP continuation".into());
        }
        let (tx, rx) = mpsc::sync_channel(1);
        self.tx
            .try_send(Input::Bytes {
                correlation,
                id: id.into(),
                status,
                bytes: bytes.into(),
                reply: tx,
            })
            .map_err(|_| "Kernel queue is full or stopped")?;
        self.wait(rx)
    }
    /// Directly cancel math even while the owner is busy.
    pub fn interrupt(&self) {
        self.shared.interrupt.store(true, Ordering::Relaxed);
    }
    /// Directly cancel a tool's computation flag. Swift also cancels its URLSession task.
    pub fn cancel(&self, id: &str) {
        if let Ok(jobs) = self.shared.jobs.lock()
            && let Some(job) = jobs.get(id)
        {
            job.cancel()
        }
    }
    /// Cancel all work and release the owner; repeated calls are harmless.
    pub fn close(&self) {
        self.shared.closed.store(true, Ordering::Release);
        self.interrupt();
        if let Ok(jobs) = self.shared.jobs.lock() {
            for job in jobs.values() {
                job.cancel()
            }
        }
        let _ = self.tx.try_send(Input::Stop);
        if let Ok(mut thread) = self.thread.lock()
            && let Some(thread) = thread.take()
        {
            let _ = thread.join();
        }
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        self.close()
    }
}
