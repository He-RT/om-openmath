//! Actual terminal results, tool execution and next requests from the single Session Job.
use super::{Feature, JobStep, LlmError, Pending, Session};
use crate::protocol::*;
use om_llm::JobResult;
impl Session {
    pub(super) fn llm_step(
        &mut self,
        id: &str,
        mut pending: Pending,
        mut step: JobStep,
    ) -> Vec<Event> {
        let mut events = vec![];
        loop {
            match step {
                JobStep::Http(http) => {
                    pending.status = None;
                    self.llm.jobs.insert(id.into(), pending);
                    events.push(Event::LlmHttp {
                        request_id: id.into(),
                        http,
                    });
                    return events;
                }
                JobStep::RunTools(calls) => {
                    let mut results = vec![];
                    let mut bytes = 0usize;
                    for call in calls {
                        let safe_arguments = pending.job.redact_credentials(&call.arguments);
                        let mut output = if safe_arguments != call.arguments {
                            super::tools::ToolOutput {content:serde_json::json!({"error":"Tool arguments contain a configured credential"}).to_string(),suggestion:None}
                        } else {
                            self.llm_tool(&call, &pending.cancel)
                        };
                        output.content = pending.job.redact_credentials(&output.content);
                        if let Some(suggestion) = output.suggestion {
                            events.push(Event::LlmSuggestion {
                                request_id: id.into(),
                                suggestion,
                            });
                        }
                        events.push(Event::LlmToolCall {
                            request_id: id.into(),
                            name: call.name,
                            arguments: safe_arguments,
                            result_summary: output.content.clone(),
                        });
                        results.push((call.id, output.content));
                        bytes = bytes.saturating_add(results.last().map_or(0, |r| r.1.len()));
                        if bytes > crate::assistant::LIMIT
                            || pending
                                .cancel
                                .flag
                                .load(std::sync::atomic::Ordering::Relaxed)
                        {
                            break;
                        }
                    }
                    step = if pending
                        .cancel
                        .flag
                        .load(std::sync::atomic::Ordering::Relaxed)
                    {
                        pending.job.cancel()
                    } else {
                        pending.job.tool_results(results)
                    };
                }
                JobStep::Done(result) => {
                    match result {
                        JobResult::Suggestion(suggestion) => events.push(Event::LlmSuggestion {
                            request_id: id.into(),
                            suggestion,
                        }),
                        JobResult::Completion(raw) => {
                            if let Some((prefix, suffix, dialect)) = &pending.completion
                                && let Some(text) =
                                    self.filter_llm_completion(prefix, suffix, &raw, *dialect)
                            {
                                events.push(Event::LlmDelta {
                                    request_id: id.into(),
                                    text,
                                });
                            }
                        }
                        JobResult::Text(response) => {
                            if pending.feature == Feature::TestProfile {
                                let latency_ms = pending
                                    .start
                                    .zip(self.llm_now())
                                    .map(|(start, end)| (end - start).max(0.0))
                                    .filter(|n| n.is_finite());
                                let first_byte_ms = pending
                                    .start
                                    .zip(pending.first)
                                    .map(|(start, end)| (end - start).max(0.0))
                                    .filter(|n| n.is_finite());
                                events.push(Event::LlmProfileTest {
                                    request_id: id.into(),
                                    profile: pending.profile.clone(),
                                    latency_ms,
                                    first_byte_ms,
                                    response,
                                });
                            }
                        }
                    }
                    pending.cancel.cancel();
                    self.remember_llm(id.into());
                    events.push(Event::LlmDone {
                        request_id: id.into(),
                    });
                    return events;
                }
                JobStep::Failed(error) => {
                    pending.cancel.cancel();
                    self.remember_llm(id.into());
                    let message = if error == LlmError::Cancelled {
                        self.localized("AI 请求已取消", "AI request cancelled")
                    } else {
                        format!("{}: {error}", pending.profile)
                    };
                    events.push(Event::LlmError {
                        request_id: id.into(),
                        message,
                    });
                    return events;
                }
            }
        }
    }
}
