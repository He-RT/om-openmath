//! Evaluation messages and scoped constructor diagnostic capture.

use serde::Serialize;
use std::cell::RefCell;

/// A diagnostic identified by its Wolfram-style symbol and tag.
#[derive(Clone, Debug, Serialize)]
pub struct Message {
    /// The built-in issuing the message.
    pub symbol: String,
    /// The message identifier, such as infy.
    pub tag: String,
    /// Human-readable explanation.
    pub text: String,
    /// Diagnostic severity.
    pub level: MsgLevel,
}
/// Severity of an evaluation message.
#[derive(Clone, Copy, Debug, Serialize)]
pub enum MsgLevel {
    /// Informational diagnostic.
    Info,
    /// Evaluation continues with a symbolic or special result.
    Warning,
    /// Evaluation failed.
    Error,
}
/// An owned message accumulator.
#[derive(Default)]
pub struct Messages(Vec<Message>);
impl Messages {
    /// Append a message.
    pub fn push(&mut self, message: Message) {
        self.0.push(message);
    }
    /// Drain all messages in their emission order.
    pub fn take(&mut self) -> Vec<Message> {
        std::mem::take(&mut self.0)
    }
}

thread_local! {
    static CAPTURES: RefCell<Vec<Vec<Message>>> = const { RefCell::new(Vec::new()) };
}
struct Capture {
    active: bool,
}
impl Drop for Capture {
    fn drop(&mut self) {
        if self.active {
            CAPTURES.with(|c| {
                c.borrow_mut().pop();
            });
        }
    }
}
/// Run a computation and capture its canonical-constructor messages.
/// Nested captures are isolated. Unwinding discards only the interrupted scope.
pub fn with_canonical_messages<T>(f: impl FnOnce() -> T) -> (T, Vec<Message>) {
    CAPTURES.with(|c| c.borrow_mut().push(Vec::new()));
    let mut guard = Capture { active: true };
    let value = f();
    let messages = CAPTURES.with(|c| c.borrow_mut().pop().unwrap_or_default());
    guard.active = false;
    (value, messages)
}
pub(crate) fn emit(symbol: &str, tag: &str, text: &str) {
    CAPTURES.with(|c| {
        if let Some(messages) = c.borrow_mut().last_mut() {
            messages.push(Message {
                symbol: symbol.into(),
                tag: tag.into(),
                text: text.into(),
                level: MsgLevel::Warning,
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_captures_and_panics_do_not_leak_messages() {
        let (_, outer) = with_canonical_messages(|| {
            emit("A", "outer", "first");
            let (_, inner) = with_canonical_messages(|| emit("B", "inner", "nested"));
            assert_eq!(inner.len(), 1);
            let result = std::panic::catch_unwind(|| {
                with_canonical_messages(|| {
                    emit("C", "panic", "discard");
                    panic!("unwind scope");
                })
            });
            assert!(result.is_err());
            emit("A", "outer", "last");
        });
        assert_eq!(outer.len(), 2);
        let (_, next) = with_canonical_messages(|| {});
        assert!(next.is_empty());
    }
    #[test]
    fn messages_drain_in_order_and_serialize_contract_fields() {
        let (_, captured) = with_canonical_messages(|| emit("Power", "infy", "infinite result"));
        let mut messages = Messages::default();
        messages.push(captured[0].clone());
        let taken = messages.take();
        assert!(messages.take().is_empty());
        let json = serde_json::to_value(&taken[0]).unwrap();
        assert_eq!(json["symbol"], "Power");
        assert_eq!(json["tag"], "infy");
        assert_eq!(json["level"], "Warning");
    }
}
