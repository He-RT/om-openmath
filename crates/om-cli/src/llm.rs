//! Native provider requests keep one shared Session job and explicit terminal confirmation.
use crate::{host::Host, render};
use om_kernel::protocol::*;
use std::{
    io::{self, Write},
    sync::atomic::Ordering,
};
#[derive(Default)]
pub struct ResultView {
    pub suggestions: Vec<Suggestion>,
    pub text: String,
    pub failed: bool,
}
impl Host {
    pub fn drive_llm(&mut self, request: Request) -> Result<ResultView, String> {
        let id = match &request {
            Request::LlmTranslate { request_id, .. }
            | Request::LlmExplain { request_id, .. }
            | Request::LlmTestProfile { request_id, .. }
            | Request::LlmComplete { request_id, .. } => request_id.clone(),
            _ => return Err("Unsupported terminal AI operation".into()),
        };
        let mut session = self
            .session
            .lock()
            .map_err(|_| "Session state unavailable")?;
        let (response, events) = session.handle(request);
        let Response::LlmStarted {
            http: Some(mut http),
            ..
        } = response
        else {
            return Err(match response {
                Response::Error { message } => message,
                _ => "AI transport did not start".into(),
            });
        };
        let mut result = ResultView::default();
        let mut initial = events;
        self.signal.evaluating.store(true, Ordering::Relaxed);
        loop {
            let cancel = session
                .llm_cancellation_handle(&id)
                .ok_or("AI cancellation scope unavailable")?;
            if let Ok(mut current) = self.signal.llm.lock() {
                *current = Some(cancel.clone());
            }
            let timeout = session.llm_http_timeout(&id).unwrap_or(60000);
            let mut events = std::mem::take(&mut initial);
            let (status, error) = self.runtime.block_on(om_llm::drive_native_http(
                &http,
                timeout,
                &self.client,
                &cancel.token(),
                |status, bytes| {
                    let (_reply, new) = session.llm_http_bytes(&id, status, bytes);
                    for event in &new {
                        if let Event::LlmDelta { text, .. } = event
                            && !self.json
                        {
                            print!("{text}");
                            let _ = io::stdout().flush();
                        }
                    }
                    events.extend(new);
                    session.llm_cancellation_handle(&id).is_some()
                },
            ));
            events.extend(
                session
                    .handle(Request::LlmHttpEnd {
                        request_id: id.clone(),
                        status,
                        error,
                    })
                    .1,
            );
            let mut next = None;
            for event in events {
                match event {
                    Event::LlmHttp { http, .. } => next = Some(http),
                    Event::LlmDelta { text, .. } => result.text.push_str(&text),
                    Event::LlmSuggestion { suggestion, .. } => result.suggestions.push(suggestion),
                    Event::LlmProfileTest {
                        response,
                        latency_ms,
                        first_byte_ms,
                        ..
                    } => {
                        if self.json {
                            println!(
                                "{}",
                                serde_json::json!({"profile_reply":response,"latency_ms":latency_ms,"first_byte_ms":first_byte_ms})
                            );
                        } else {
                            println!(
                                "{} · {}: {} ms · {}: {} ms",
                                response,
                                render::text(
                                    render::zh(session.effective_language()),
                                    "耗时",
                                    "latency"
                                ),
                                latency_ms.map_or("—".into(), |v| format!("{v:.0}")),
                                render::text(
                                    render::zh(session.effective_language()),
                                    "首字节",
                                    "first byte"
                                ),
                                first_byte_ms.map_or("—".into(), |v| format!("{v:.0}"))
                            );
                        }
                    }
                    Event::LlmError { message, .. } => {
                        eprintln!("{message}");
                        result.failed = true;
                    }
                    _ => {}
                }
            }
            if let Some(request) = next {
                http = request;
            } else {
                break;
            }
        }
        self.signal.evaluating.store(false, Ordering::Relaxed);
        if let Ok(mut current) = self.signal.llm.lock() {
            *current = None;
        }
        if !self.json && !result.text.is_empty() {
            println!();
        }
        Ok(result)
    }
    pub fn ask(&mut self, text: String) -> Result<ResultView, String> {
        self.next += 1;
        self.drive_llm(Request::LlmTranslate {
            request_id: format!("cli-ask-{}", self.next),
            text,
            cell_id: None,
        })
    }
    pub fn explain(&mut self) -> Result<ResultView, String> {
        let id = self
            .last
            .as_ref()
            .map(|(id, _)| id.clone())
            .ok_or("No recorded output")?;
        self.next += 1;
        self.drive_llm(Request::LlmExplain {
            request_id: format!("cli-explain-{}", self.next),
            cell_id: id,
            step_id: None,
            out_index: None,
        })
    }
    pub fn confirm_ai(&self, feature: &str) -> Result<bool, String> {
        let session = self
            .session
            .lock()
            .map_err(|_| "Session state unavailable")?;
        let route = if feature == "ask" {
            &session.config.llm.translate
        } else if feature == "complete" {
            &session.config.llm.complete
        } else {
            &session.config.llm.explain
        };
        let profile = session
            .config
            .llm
            .profiles
            .iter()
            .find(|p| &p.name == route)
            .ok_or("AI profile is not configured")?;
        let zh = render::zh(session.effective_language());
        let notice = if feature == "complete" && session.config.llm.send_context {
            render::text(
                zh,
                "将发送当前代码和最多三个前文源码",
                "Sends current code and up to three prior source cells",
            )
        } else if feature == "ask" {
            render::text(
                zh,
                "将发送问题和当前已定义符号名称",
                "Sends the question and live symbol names",
            )
        } else {
            render::text(
                zh,
                "将发送真实计算输入、结果与步骤",
                "Sends actual computation input, result and steps",
            )
        };
        eprintln!("{notice} → {}", profile.base_url);
        drop(session);
        eprint!("{} [y/N] ", render::text(zh, "继续？", "Continue?"));
        io::stderr().flush().map_err(|e| e.to_string())?;
        let mut answer = String::new();
        io::stdin()
            .read_line(&mut answer)
            .map_err(|e| e.to_string())?;
        Ok(answer.trim().eq_ignore_ascii_case("y"))
    }
}
