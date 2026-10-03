//! Optional terminal AI hints only read a nonblocking cache from the key-handling path.
use crate::host::{Host, Time};
use om_kernel::{
    KernelConfig, LlmCancellation, Session,
    native::{ConfigStore, KeyStorage, NativeCredentials},
    protocol::*,
};
use reedline::{DefaultHinter, Hinter, History};
use std::{
    path::PathBuf,
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};
struct Candidate {
    line: String,
    pos: usize,
    dialect: Dialect,
    config: KernelConfig,
    sources: Vec<String>,
    revision: u64,
}
#[derive(Default)]
struct State {
    pending: Option<Candidate>,
    revision: u64,
    cache: Option<(String, usize, String)>,
    cancel: Option<LlmCancellation>,
    closed: bool,
}
pub struct AiHinter {
    history: DefaultHinter,
    current: String,
    last: Option<(String, usize, Dialect)>,
    main: Arc<Mutex<Session>>,
    shared: Arc<(Mutex<State>, Condvar)>,
}
impl AiHinter {
    pub fn new(host: &Host) -> Self {
        let shared = Arc::new((Mutex::new(State::default()), Condvar::new()));
        let worker = shared.clone();
        let path = host.path.clone();
        std::thread::spawn(move || worker_loop(worker, path));
        Self {
            history: DefaultHinter::default(),
            current: String::new(),
            last: None,
            main: host.session.clone(),
            shared,
        }
    }
    fn invalidate(&mut self) {
        self.last = None;
        if let Ok(mut state) = self.shared.0.try_lock() {
            state.revision += 1;
            state.pending = None;
            state.cache = None;
            if let Some(cancel) = state.cancel.take() {
                cancel.cancel();
            }
        }
    }
}
impl Hinter for AiHinter {
    fn handle(
        &mut self,
        line: &str,
        pos: usize,
        history: &dyn History,
        ansi: bool,
        cwd: &str,
    ) -> String {
        self.current.clear();
        let prior = self.history.handle(line, pos, history, ansi, cwd);
        if !prior.is_empty() {
            self.invalidate();
            self.current = self.history.complete_hint();
            return prior;
        }
        let Some(tail) = line.get(..pos) else {
            self.invalidate();
            return String::new();
        };
        if pos != line.len()
            || tail
                .lines()
                .last()
                .unwrap_or_default()
                .trim()
                .chars()
                .count()
                < 3
            || line.len() > 1_048_576
        {
            self.invalidate();
            self.current.clear();
            return String::new();
        }
        let Ok(main) = self.main.try_lock() else {
            return String::new();
        };
        if !main.config.cli.ai_hints {
            drop(main);
            self.invalidate();
            return String::new();
        }
        let dialect = match main.config.general.dialect {
            om_kernel::config::ConfigDialect::Wolfram => Dialect::Wolfram,
            om_kernel::config::ConfigDialect::Modern => Dialect::Modern,
            om_kernel::config::ConfigDialect::Auto => Dialect::Auto,
        };
        let snapshot = (line.to_string(), pos, dialect);
        let mut notify = false;
        if self.last.as_ref() != Some(&snapshot)
            && let Ok(mut state) = self.shared.0.try_lock()
        {
            state.revision += 1;
            if let Some(cancel) = state.cancel.take() {
                cancel.cancel();
            }
            let sources = main
                .notebook
                .cells
                .iter()
                .rev()
                .filter(|c| c.kind == CellKind::Math)
                .take(3)
                .map(|c| c.source.clone())
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            state.pending = Some(Candidate {
                line: line.into(),
                pos,
                dialect,
                config: main.config.clone(),
                sources,
                revision: state.revision,
            });
            state.cache = None;
            self.last = Some(snapshot);
            notify = true;
        }
        drop(main);
        if notify {
            self.shared.1.notify_one();
        }
        self.current = self
            .shared
            .0
            .try_lock()
            .ok()
            .and_then(|state| {
                state
                    .cache
                    .as_ref()
                    .filter(|(old, p, _)| old == line && *p == pos)
                    .map(|(_, _, text)| text.clone())
            })
            .unwrap_or_default();
        if ansi {
            nu_ansi_term::Color::DarkGray
                .italic()
                .paint(&self.current)
                .to_string()
        } else {
            self.current.clone()
        }
    }
    fn complete_hint(&self) -> String {
        self.current.clone()
    }
    fn next_hint_token(&self) -> String {
        let mut seen = false;
        self.current
            .chars()
            .take_while(|c| {
                if seen && c.is_whitespace() {
                    false
                } else {
                    if !c.is_whitespace() {
                        seen = true;
                    }
                    true
                }
            })
            .collect()
    }
}
impl Drop for AiHinter {
    fn drop(&mut self) {
        if let Ok(mut state) = self.shared.0.lock() {
            state.closed = true;
            if let Some(cancel) = state.cancel.take() {
                cancel.cancel();
            }
        }
        self.shared.1.notify_one();
    }
}
fn worker_loop(shared: Arc<(Mutex<State>, Condvar)>, path: Option<PathBuf>) {
    loop {
        let Ok(mut state) = shared.0.lock() else {
            return;
        };
        while state.pending.is_none() && !state.closed {
            let Ok(next) = shared.1.wait(state) else {
                return;
            };
            state = next;
        }
        if state.closed {
            return;
        }
        let Some(mut candidate) = state.pending.take() else {
            continue;
        };
        let mut idle = Instant::now();
        loop {
            let wait = Duration::from_millis(350).saturating_sub(idle.elapsed());
            if wait.is_zero() {
                break;
            }
            let Ok((next, _)) = shared.1.wait_timeout(state, wait) else {
                return;
            };
            state = next;
            if state.closed {
                return;
            }
            if let Some(latest) = state.pending.take() {
                candidate = latest;
                idle = Instant::now();
            }
        }
        drop(state);
        let hint = compute(&candidate, path.as_deref(), &shared);
        if let Ok(mut state) = shared.0.lock()
            && state.revision == candidate.revision
            && !state.closed
        {
            state.cache = hint.map(|text| (candidate.line, candidate.pos, text));
            state.cancel = None;
        }
    }
}
fn compute(
    candidate: &Candidate,
    path: Option<&std::path::Path>,
    shared: &Arc<(Mutex<State>, Condvar)>,
) -> Option<String> {
    let clock = Some(Arc::new(Time::new()) as Arc<dyn om_num::ctx::Clock>);
    let mut session = if let Some(path) = path {
        Session::new_native(
            ConfigStore::new(
                path.to_path_buf(),
                Arc::new(NativeCredentials),
                KeyStorage::Vault,
            ),
            clock,
        )
        .ok()?
    } else {
        let mut session = Session::new(candidate.config.clone(), clock);
        session.set_llm_target(om_llm::Target::Native).ok()?;
        session
    };
    session.config = candidate.config.clone();
    for (i, source) in candidate.sources.iter().enumerate() {
        session.handle(Request::UpsertCell {
            cell: CellInput {
                id: format!("ctx-{i}"),
                kind: CellKind::Math,
                source: source.clone(),
                dialect: Dialect::Auto,
            },
        });
    }
    session.handle(Request::UpsertCell {
        cell: CellInput {
            id: "current".into(),
            kind: CellKind::Math,
            source: candidate.line.clone(),
            dialect: candidate.dialect,
        },
    });
    let (start, _) = session.handle(Request::LlmComplete {
        request_id: "hint".into(),
        prefix: candidate.line[..candidate.pos].into(),
        suffix: candidate.line[candidate.pos..].into(),
        dialect: candidate.dialect,
    });
    let Response::LlmStarted {
        http: Some(http), ..
    } = start
    else {
        return None;
    };
    let cancel = session.llm_cancellation_handle("hint")?;
    {
        let mut state = shared.0.lock().ok()?;
        if state.closed || state.revision != candidate.revision {
            cancel.cancel();
            return None;
        }
        state.cancel = Some(cancel.clone());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    let client = om_llm::native_client().ok()?;
    let timeout = session.llm_http_timeout("hint")?;
    let mut events = vec![];
    let (status, error) = runtime.block_on(om_llm::drive_native_http(
        &http,
        timeout,
        &client,
        &cancel.token(),
        |status, bytes| {
            events.extend(session.llm_http_bytes("hint", status, bytes).1);
            true
        },
    ));
    events.extend(
        session
            .handle(Request::LlmHttpEnd {
                request_id: "hint".into(),
                status,
                error,
            })
            .1,
    );
    if events.iter().any(|e| matches!(e, Event::LlmError { .. })) {
        return None;
    }
    events.into_iter().find_map(|e| {
        if let Event::LlmDelta { text, .. } = e {
            Some(text)
        } else {
            None
        }
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn key_path_does_not_wait_on_a_busy_cache() {
        let session = Arc::new(Mutex::new(Session::new(KernelConfig::default(), None)));
        let shared = Arc::new((Mutex::new(State::default()), Condvar::new()));
        let mut hinter = AiHinter {
            history: DefaultHinter::default(),
            current: String::new(),
            last: None,
            main: session,
            shared: shared.clone(),
        };
        let history = reedline::FileBackedHistory::default();
        let guard = shared.0.lock().unwrap();
        let now = Instant::now();
        assert_eq!(hinter.handle("x^2", 3, &history, false, ""), "");
        assert!(now.elapsed() < Duration::from_millis(20));
        drop(guard);
    }

    #[test]
    fn disabled_or_busy_session_never_leaves_an_acceptable_stale_hint() {
        let session = Arc::new(Mutex::new(Session::new(KernelConfig::default(), None)));
        let mut hinter = AiHinter {
            history: DefaultHinter::default(),
            current: "old suggestion".into(),
            last: None,
            main: session.clone(),
            shared: Arc::new((Mutex::new(State::default()), Condvar::new())),
        };
        let history = reedline::FileBackedHistory::default();
        assert_eq!(hinter.handle("x^2", 3, &history, false, ""), "");
        assert_eq!(hinter.complete_hint(), "");
        hinter.current = "old suggestion".into();
        let guard = session.lock().unwrap();
        assert_eq!(hinter.handle("x^3", 3, &history, false, ""), "");
        assert_eq!(hinter.complete_hint(), "");
        drop(guard);
    }
}
