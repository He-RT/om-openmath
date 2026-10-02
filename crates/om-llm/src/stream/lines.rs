//! Bounded scalar/line decoding is linear even for one-byte multibyte chunks.
use crate::LlmError;
pub(super) const LIMIT: usize = 1_048_576;
pub(super) struct Lines {
    buf: String,
    scalar: Vec<u8>,
    start: bool,
    skip_lf: bool,
    failed: bool,
    closed: bool,
    limit: usize,
}
impl Lines {
    pub fn new(limit: usize) -> Self {
        Self {
            buf: String::new(),
            scalar: Vec::with_capacity(4),
            start: true,
            skip_lf: false,
            failed: false,
            closed: false,
            limit,
        }
    }
    pub fn feed(&mut self, bytes: &[u8]) -> Result<Vec<String>, LlmError> {
        if self.failed || self.closed {
            return Err(LlmError::Terminal);
        }
        let result = self.process(bytes);
        if result.is_err() {
            self.failed = true;
            self.buf.clear();
            self.scalar.clear();
        }
        result
    }
    fn process(&mut self, bytes: &[u8]) -> Result<Vec<String>, LlmError> {
        let mut lines = vec![];
        for &byte in bytes {
            let c = if self.scalar.is_empty() && byte < 0x80 {
                byte as char
            } else {
                self.scalar.push(byte);
                match std::str::from_utf8(&self.scalar) {
                    Ok(s) => {
                        let c = s.chars().next().ok_or(LlmError::Encoding)?;
                        self.scalar.clear();
                        c
                    }
                    Err(e) if e.error_len().is_none() && self.scalar.len() < 4 => continue,
                    Err(_) => return Err(LlmError::Encoding),
                }
            };
            if self.start {
                self.start = false;
                if c == '\u{feff}' {
                    continue;
                }
            }
            if self.skip_lf {
                self.skip_lf = false;
                if c == '\n' {
                    continue;
                }
            }
            if c == '\r' || c == '\n' {
                lines.push(std::mem::take(&mut self.buf));
                self.skip_lf = c == '\r';
            } else {
                if self
                    .buf
                    .len()
                    .checked_add(c.len_utf8())
                    .is_none_or(|n| n > self.limit)
                {
                    return Err(LlmError::Limit { limit: self.limit });
                }
                self.buf.push(c);
            }
        }
        Ok(lines)
    }
    pub fn finish(&mut self) -> Result<Option<String>, LlmError> {
        if self.failed {
            return Err(LlmError::Terminal);
        }
        if self.closed {
            return Ok(None);
        }
        self.closed = true;
        if !self.scalar.is_empty() {
            self.failed = true;
            self.scalar.clear();
            self.buf.clear();
            return Err(LlmError::Encoding);
        }
        Ok(Some(std::mem::take(&mut self.buf)))
    }
}
