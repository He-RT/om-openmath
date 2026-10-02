//! Actual stream deltas accumulate without advancing to tool execution prematurely.
use super::{Job, LIMIT, MAX_TOOLS, State};
use crate::{
    LlmError, ProviderKind, StreamEvent as E, decode_anthropic_event, decode_openai_chunk,
};
impl Job {
    pub(super) fn ingest(&mut self, chunk: &[u8]) -> Result<Vec<E>, LlmError> {
        if self
            .round
            .raw
            .len()
            .checked_add(chunk.len())
            .is_none_or(|n| n > LIMIT)
        {
            return Err(LlmError::Limit { limit: LIMIT });
        }
        self.round.raw.extend_from_slice(chunk);
        if !self.round.stream {
            return Ok(vec![]);
        }
        let frames = self.round.decoder.feed_bytes(chunk)?;
        let mut actual = vec![];
        for frame in frames {
            let events = if self.profile.kind == ProviderKind::Anthropic {
                decode_anthropic_event(frame.event.as_deref().unwrap_or("message"), &frame.data)
            } else {
                decode_openai_chunk(&frame.data)
            };
            for event in events {
                match event {
                    E::Text(text) => {
                        if self.round.reason.is_some() {
                            return Err(LlmError::State("text after finish"));
                        }
                        if self
                            .round
                            .text
                            .len()
                            .checked_add(text.len())
                            .is_none_or(|n| n > LIMIT)
                        {
                            return Err(LlmError::Limit { limit: LIMIT });
                        }
                        self.round.saw_text = true;
                        self.round.text.push_str(&text);
                        actual.push(E::Text(text));
                    }
                    E::ToolCallDelta {
                        index,
                        id,
                        name,
                        args_fragment,
                    } => {
                        if self.round.reason.is_some() {
                            return Err(LlmError::State("tools after finish"));
                        }
                        if !self.round.tools.contains_key(&index)
                            && self.round.tools.len() >= MAX_TOOLS
                        {
                            return Err(LlmError::Tools);
                        }
                        let call = self.round.tools.entry(index).or_default();
                        metadata(&mut call.id, id.as_deref())?;
                        metadata(&mut call.name, name.as_deref())?;
                        if call
                            .args
                            .len()
                            .checked_add(args_fragment.len())
                            .is_none_or(|n| n > LIMIT)
                        {
                            return Err(LlmError::Limit { limit: LIMIT });
                        }
                        call.args.push_str(&args_fragment);
                        actual.push(E::ToolCallDelta {
                            index,
                            id,
                            name,
                            args_fragment,
                        });
                    }
                    E::Finish { reason } => {
                        if let Some(old) = &self.round.reason
                            && old != &reason
                            && reason != "done"
                            && old != "done"
                        {
                            return Err(LlmError::State("conflicting finish reasons"));
                        }
                        if self.round.reason.as_deref().is_none_or(|r| r == "done") {
                            self.round.reason = Some(reason.clone());
                        }
                        actual.push(E::Finish { reason });
                    }
                    E::Error(message) => {
                        let message = self.clean(&message);
                        self.state = State::Failed(LlmError::Remote(message.clone()));
                        actual.push(E::Error(message));
                        return Ok(actual);
                    }
                }
            }
        }
        Ok(actual)
    }
}
fn metadata(target: &mut String, value: Option<&str>) -> Result<(), LlmError> {
    if let Some(value) = value.filter(|s| !s.is_empty()) {
        if !target.is_empty() && target != value {
            return Err(LlmError::Tools);
        }
        *target = value.into();
    }
    Ok(())
}
