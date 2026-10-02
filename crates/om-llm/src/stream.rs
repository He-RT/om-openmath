//! Actual incremental framing and provider deltas; no transport or job state machine.
mod lines;
mod provider;
use crate::LlmError;
pub use provider::{decode_anthropic_event, decode_openai_chunk};
/// Actual text/tool/finish/error delta emitted by a provider frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamEvent {
    /// Model text delta.
    Text(String),
    /// Actual indexed tool fields; arguments remain untrusted JSON text fragments.
    ToolCallDelta {
        /// Provider tool/block index.
        index: u32,
        /// Present provider call identifier fragment.
        id: Option<String>,
        /// Present provider function name fragment.
        name: Option<String>,
        /// Actual argument text fragment.
        args_fragment: String,
    },
    /// Actual finish marker; consumers must handle duplicate/end markers idempotently.
    Finish {
        /// Specific provider reason or terminal marker.
        reason: String,
    },
    /// Actual provider error text or a secret-safe malformed-frame diagnostic.
    Error(String),
}
/// One fully framed SSE event; unknown fields/connection metadata are not reconstructed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SseMessage {
    /// Named event when nonempty; None is the standard message event.
    pub event: Option<String>,
    /// Actual multiline data, without its framing newline.
    pub data: String,
}
/// Bounded SSE framing with exact Unicode and cross-chunk CRLF handling.
pub struct SseDecoder {
    buf: String,
    event: Option<String>,
    lines: lines::Lines,
    limit: usize,
    failed: bool,
}
impl Default for SseDecoder {
    fn default() -> Self {
        Self::new()
    }
}
impl SseDecoder {
    /// Start a decoder with the default one-MiB line/event budget.
    pub fn new() -> Self {
        Self::with_limit(lines::LIMIT)
    }
    /// Start with an explicit byte budget; no unbounded partial line is retained.
    pub fn with_limit(limit: usize) -> Self {
        Self {
            buf: String::new(),
            event: None,
            lines: lines::Lines::new(limit),
            limit,
            failed: false,
        }
    }
    /// Convenience text feed for validated streams; decoder failure panics rather than losing data.
    /// Runtime provider paths must use [`Self::try_feed`] or [`Self::feed_bytes`].
    pub fn feed(&mut self, chunk: &str) -> Vec<SseMessage> {
        match self.try_feed(chunk) {
            Ok(frames) => frames,
            Err(e) => panic!("invalid SSE stream: {e}"),
        }
    }
    /// Checked text feed; a failed call returns no partial events and becomes terminal.
    pub fn try_feed(&mut self, chunk: &str) -> Result<Vec<SseMessage>, LlmError> {
        self.feed_bytes(chunk.as_bytes())
    }
    /// Checked bytes feed preserves partial UTF-8 scalars without lossy replacement.
    pub fn feed_bytes(&mut self, chunk: &[u8]) -> Result<Vec<SseMessage>, LlmError> {
        if self.failed {
            return Err(LlmError::Terminal);
        }
        let result = self.process(chunk);
        if result.is_err() {
            self.failed = true;
            self.buf.clear();
            self.event = None;
        }
        result
    }
    fn process(&mut self, chunk: &[u8]) -> Result<Vec<SseMessage>, LlmError> {
        let mut frames = vec![];
        for line in self.lines.feed(chunk)? {
            if line.is_empty() {
                let event = self.event.take();
                if !self.buf.is_empty() {
                    self.buf.pop();
                    frames.push(SseMessage {
                        event,
                        data: std::mem::take(&mut self.buf),
                    });
                }
                continue;
            }
            if line.starts_with(':') {
                continue;
            }
            let (name, value) = line.split_once(':').unwrap_or((&line, ""));
            let value = value.strip_prefix(' ').unwrap_or(value);
            match name {
                "event" => {
                    self.event = (!value.is_empty()).then(|| value.into());
                }
                "data" => {
                    if self
                        .buf
                        .len()
                        .checked_add(value.len() + 1)
                        .is_none_or(|n| n + self.event.as_ref().map_or(0, String::len) > self.limit)
                    {
                        return Err(LlmError::Limit { limit: self.limit });
                    }
                    self.buf.push_str(value);
                    self.buf.push('\n');
                }
                _ => {}
            }
            if self.buf.len() + self.event.as_ref().map_or(0, String::len) > self.limit {
                return Err(LlmError::Limit { limit: self.limit });
            }
        }
        Ok(frames)
    }
    /// Finish the stream, rejecting truncated UTF-8 and discarding incomplete SSE events.
    pub fn finish(&mut self) -> Result<Vec<SseMessage>, LlmError> {
        if self.failed {
            return Err(LlmError::Terminal);
        }
        let result = self.lines.finish();
        self.buf.clear();
        self.event = None;
        match result {
            Ok(_) => Ok(vec![]),
            Err(e) => {
                self.failed = true;
                Err(e)
            }
        }
    }
}
/// Incremental bounded newline-delimited JSON records, preserving the actual record text.
pub struct NdjsonDecoder {
    lines: lines::Lines,
    failed: bool,
}
impl Default for NdjsonDecoder {
    fn default() -> Self {
        Self::new()
    }
}
impl NdjsonDecoder {
    /// Start with the default one-MiB record budget.
    pub fn new() -> Self {
        Self::with_limit(lines::LIMIT)
    }
    /// Start with an explicit record byte budget.
    pub fn with_limit(limit: usize) -> Self {
        Self {
            lines: lines::Lines::new(limit),
            failed: false,
        }
    }
    /// Checked text records feed.
    pub fn try_feed(&mut self, chunk: &str) -> Result<Vec<String>, LlmError> {
        self.feed_bytes(chunk.as_bytes())
    }
    /// Checked byte feed retains incomplete UTF-8/JSON lines until complete.
    pub fn feed_bytes(&mut self, chunk: &[u8]) -> Result<Vec<String>, LlmError> {
        if self.failed {
            return Err(LlmError::Terminal);
        }
        let result = self.lines.feed(chunk).and_then(Self::records);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn records(lines: Vec<String>) -> Result<Vec<String>, LlmError> {
        let mut records = vec![];
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            serde_json::from_str::<serde_json::Value>(&line).map_err(|_| LlmError::Json)?;
            records.push(line);
        }
        Ok(records)
    }
    /// Validate and return a complete final JSON record even without its terminating newline.
    pub fn finish(&mut self) -> Result<Vec<String>, LlmError> {
        if self.failed {
            return Err(LlmError::Terminal);
        }
        let result = self
            .lines
            .finish()
            .and_then(|line| Self::records(line.into_iter().collect()));
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}
