//! One real native Session owns terminal evaluation and provider event consumption.
use crate::{args::Args, render};
use om_kernel::{
    KernelConfig, LlmCancellation, Session, config::Language, native::ConfigStore, protocol::*,
};
use om_num::ctx::Clock;
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
pub struct Time(Instant);
impl Time {
    pub fn new() -> Self {
        Self(Instant::now())
    }
}
impl Clock for Time {
    fn now_ms(&self) -> f64 {
        self.0.elapsed().as_secs_f64() * 1000.0
    }
}
#[derive(Default)]
pub struct Signal {
    pub evaluating: AtomicBool,
    pub flag: Mutex<Option<Arc<AtomicBool>>>,
    pub llm: Mutex<Option<LlmCancellation>>,
}
impl Signal {
    pub fn stop(&self) {
        if !self.evaluating.load(Ordering::Relaxed) {
            return;
        }
        if let Ok(flag) = self.flag.lock()
            && let Some(flag) = &*flag
        {
            flag.store(true, Ordering::Relaxed);
        }
        if let Ok(cancel) = self.llm.lock()
            && let Some(cancel) = &*cancel
        {
            cancel.cancel();
        }
    }
}
pub struct Host {
    pub session: Arc<Mutex<Session>>,
    pub signal: Arc<Signal>,
    pub dialect: Dialect,
    pub json: bool,
    pub count: u32,
    pub next: u64,
    pub last: Option<(String, CellOutput)>,
    pub path: Option<PathBuf>,
    pub stored_general: om_kernel::config::GeneralConfig,
    pub runtime: tokio::runtime::Runtime,
    pub client: reqwest::Client,
}
impl Host {
    pub fn new(args: &Args) -> Result<Self, String> {
        let clock = Some(Arc::new(Time::new()) as Arc<dyn Clock>);
        let mut session = if args.no_config {
            let mut s = Session::new(KernelConfig::default(), clock);
            s.set_llm_target(om_llm::Target::Native)
                .map_err(|e| e.to_string())?;
            s
        } else {
            let store = if let Some(path) = &args.config {
                ConfigStore::new(
                    path.clone(),
                    Arc::new(om_kernel::native::NativeCredentials),
                    om_kernel::native::KeyStorage::Vault,
                )
            } else {
                ConfigStore::system_default().map_err(|e| e.to_string())?
            };
            Session::new_native(store, clock).map_err(|e| e.to_string())?
        };
        let stored_general = session.config.general.clone();
        let system = if ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|n| std::env::var(n).ok())
            .find(|v| !v.is_empty())
            .is_some_and(|v| v.starts_with("zh"))
        {
            Language::ZhCn
        } else {
            Language::En
        };
        session.handle(Request::SetSystemLanguage { language: system });
        if args.language != Language::Auto {
            session.config.general.language = args.language;
        }
        session.config.general.reactive = false;
        session.config.general.auto_run_dependents = false;
        session.config.general.auto_plot = false;
        let dialect = args
            .dialect
            .unwrap_or(match session.config.general.dialect {
                om_kernel::config::ConfigDialect::Modern => Dialect::Modern,
                om_kernel::config::ConfigDialect::Wolfram => Dialect::Wolfram,
                om_kernel::config::ConfigDialect::Auto => Dialect::Auto,
            });
        let signal = Arc::new(Signal::default());
        *signal.flag.lock().map_err(|_| "Signal state unavailable")? =
            Some(session.interrupt_handle());
        let path = if args.no_config {
            None
        } else {
            Some(args.config.clone().unwrap_or_else(|| {
                ConfigStore::system_default()
                    .map(|s| s.path().to_path_buf())
                    .unwrap_or_default()
            }))
        };
        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            signal,
            dialect,
            json: args.json,
            count: 0,
            next: 0,
            last: None,
            path,
            stored_general,
            runtime: tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .map_err(|_| "Native runtime unavailable")?,
            client: om_llm::native_client().map_err(|e| e.to_string())?,
        })
    }
    pub fn zh(&self) -> bool {
        self.session
            .lock()
            .map(|s| render::zh(s.effective_language()))
            .unwrap_or(false)
    }
    pub fn evaluate(&mut self, source: String, labels: bool) -> Result<i32, String> {
        self.next += 1;
        let id = format!("cli-{}", self.next);
        let mut session = self
            .session
            .lock()
            .map_err(|_| "Session state unavailable")?;
        let (preview, _) = session.handle(Request::Preview {
            source: source.clone(),
            dialect: self.dialect,
            cursor: None,
        });
        let diagnostics = if let Response::Preview(p) = preview {
            p.diagnostics
        } else {
            vec![]
        };
        let parse_error = diagnostics.iter().any(|d| d.severity == Severity::Error);
        self.signal.evaluating.store(true, Ordering::Relaxed);
        let (response, _) = session.handle(Request::Evaluate {
            cell_id: id.clone(),
            source: source.clone(),
            dialect: self.dialect,
        });
        self.signal.evaluating.store(false, Ordering::Relaxed);
        if let Some(cell) = session.notebook.cells.iter().find(|c| c.id == id)
            && let Some(n) = cell.exec_count
        {
            self.count = n;
        }
        let zh = render::zh(session.effective_language());
        drop(session);
        let Response::Evaluated { output, .. } = response else {
            return Err("Kernel did not return an evaluation".into());
        };
        let failed = output
            .items
            .iter()
            .any(|i| matches!(i, OutputItem::Error { .. }))
            || output.messages.iter().any(|m| {
                m.level == MsgLevel::Error
                    || m.text.starts_with("invalid input:")
                    || matches!(m.tag.as_str(), "argx" | "argrx")
            });
        if parse_error {
            render::diagnostics(&source, &diagnostics, zh);
        }
        render::output(&output, self.json, labels, zh).map_err(|e| e.to_string())?;
        self.last = Some((id, output));
        Ok(if parse_error {
            2
        } else if failed {
            1
        } else {
            0
        })
    }
}
