//! A dedicated Session owner receives JSON and acknowledged raw bytes; no Session lock/await.
mod actor;
use om_kernel::protocol::{Envelope, Request};
use om_kernel::{LlmCancellation, native::ConfigStore};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
};
use tokio::{runtime::Runtime, sync::oneshot};
type Reply = oneshot::Sender<Result<String, String>>;
type EventSink = Arc<dyn Fn(String) + Send + Sync>;
enum Input {
    Request(String, Reply),
    Bytes {
        id: String,
        status: u16,
        bytes: Vec<u8>,
        ack: mpsc::SyncSender<bool>,
    },
    End {
        id: String,
        status: u16,
        error: Option<String>,
    },
    Secret {
        profile: String,
        key: Option<String>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    Stop,
}
struct Shared {
    interrupt: Arc<AtomicBool>,
    closed: AtomicBool,
    jobs: Mutex<BTreeMap<String, LlmCancellation>>,
    events: Mutex<Option<EventSink>>,
}
/// Desktop requests, native HTTP and direct cancellation share one dedicated Session thread.
pub struct KernelHost {
    tx: mpsc::SyncSender<Input>,
    shared: Arc<Shared>,
    thread: Mutex<Option<JoinHandle<()>>>,
    runtime: Mutex<Option<Runtime>>,
}
impl KernelHost {
    /// Bind actual native storage and start the owner thread; startup errors remain explicit.
    pub fn new(store: ConfigStore) -> Result<Self, String> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .max_blocking_threads(16)
            .enable_all()
            .build()
            .map_err(|_| "Native HTTP runtime initialization failed")?;
        let handle = runtime.handle().clone();
        let (tx, rx) = mpsc::sync_channel(64);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker_tx = tx.clone();
        let thread = std::thread::Builder::new()
            .name("openmath-kernel".into())
            .spawn(move || actor::run(store, rx, worker_tx, handle, ready_tx))
            .map_err(|_| "Kernel owner thread initialization failed")?;
        let shared = match ready_rx
            .recv()
            .map_err(|_| "Kernel owner initialization failed")?
        {
            Ok(shared) => shared,
            Err(error) => {
                let _ = thread.join();
                runtime.shutdown_background();
                return Err(error);
            }
        };
        Ok(Self {
            tx,
            shared,
            thread: Mutex::new(Some(thread)),
            runtime: Mutex::new(Some(runtime)),
        })
    }
    /// Return only the matching public response; internal HTTP requests never leave this host.
    pub async fn request(&self, envelope: String) -> Result<String, String> {
        if self.shared.closed.load(Ordering::Relaxed) {
            return Err("Kernel host is closed".into());
        }
        if envelope.len() > 16 * 1024 * 1024 {
            return Err("Kernel envelope exceeds limit".into());
        }
        let request: Envelope<Request> =
            serde_json::from_str(&envelope).map_err(|_| "Invalid kernel envelope")?;
        if !om_kernel::protocol::request_size_allowed(envelope.len(), &request.body) {
            return Err("Kernel envelope exceeds limit".into());
        }
        if request.id == 0 || request.id > 9_007_199_254_740_991 {
            return Err("Invalid request correlation ID".into());
        }
        if let Request::LlmCancel { request_id } = &request.body {
            self.cancel(request_id);
        }
        if matches!(request.body, Request::Interrupt) {
            self.interrupt();
        }
        let (reply, rx) = oneshot::channel();
        self.tx
            .try_send(Input::Request(envelope, reply))
            .map_err(|_| "Kernel queue is unavailable or full")?;
        rx.await.map_err(|_| "Kernel owner stopped")?
    }
    /// Replace the desktop event subscription with a callback receiving zero-ID JSON envelopes.
    pub fn subscribe(&self, sink: EventSink) -> Result<(), String> {
        *self
            .shared
            .events
            .lock()
            .map_err(|_| "Kernel event subscription failed")? = Some(sink);
        Ok(())
    }
    /// Set the actual shared evaluation cancellation flag without waiting for the queue.
    pub fn interrupt(&self) {
        self.shared.interrupt.store(true, Ordering::Relaxed);
    }
    fn cancel(&self, id: &str) {
        if let Ok(jobs) = self.shared.jobs.lock()
            && let Some(job) = jobs.get(id)
        {
            job.cancel();
        }
    }
    /// Set/delete the bound profile vault entry on the owner thread, never returning its contents.
    pub async fn secret(&self, profile: String, key: Option<String>) -> Result<(), String> {
        if profile.is_empty()
            || profile.len() > 256
            || key.as_ref().is_some_and(|s| s.len() > 1_048_576)
        {
            return Err("Invalid secret profile/input".into());
        }
        let (reply, rx) = oneshot::channel();
        self.tx
            .try_send(Input::Secret {
                profile,
                key,
                reply,
            })
            .map_err(|_| "Kernel queue is unavailable or full")?;
        rx.await.map_err(|_| "Kernel owner stopped")?
    }
}
impl Drop for KernelHost {
    fn drop(&mut self) {
        self.shared.closed.store(true, Ordering::Relaxed);
        self.interrupt();
        if let Ok(jobs) = self.shared.jobs.lock() {
            for job in jobs.values() {
                job.cancel();
            }
        }
        let _ = self.tx.try_send(Input::Stop);
        if let Ok(mut thread) = self.thread.lock()
            && let Some(thread) = thread.take()
        {
            let _ = thread.join();
        }
        if let Ok(mut runtime) = self.runtime.lock()
            && let Some(runtime) = runtime.take()
        {
            runtime.shutdown_background();
        }
    }
}
