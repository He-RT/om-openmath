//! OpenMath WebAssembly bindings.
#![forbid(unsafe_code)]
use om_kernel::{
    KernelConfig, Session,
    protocol::{Envelope, Event, Request, Response},
};
use om_num::ctx::Clock;
use serde::Serialize;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
struct BrowserClock;
impl Clock for BrowserClock {
    fn now_ms(&self) -> f64 {
        #[cfg(target_arch = "wasm32")]
        {
            js_sys::Date::now()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0.0, |d| d.as_secs_f64() * 1000.0)
        }
    }
}
#[derive(Serialize)]
struct Packet {
    response: Envelope<Response>,
    events: Vec<Envelope<Event>>,
}
/// Actual shared-kernel binding owned by a browser Web Worker.
#[wasm_bindgen]
pub struct Kernel {
    session: Session,
}
#[wasm_bindgen]
impl Kernel {
    /// Initialize actual portable settings; configuration remains in this Worker only.
    #[wasm_bindgen(constructor)]
    pub fn new(config: Option<String>) -> Result<Kernel, JsValue> {
        #[cfg(target_arch = "wasm32")]
        console_error_panic_hook::set_once();
        let config = config
            .map(|s| serde_json::from_str::<KernelConfig>(&s))
            .transpose()
            .map_err(|_| js_error("Invalid kernel configuration"))?
            .unwrap_or_default();
        Ok(Self {
            session: Session::new(config, Some(Arc::new(BrowserClock))),
        })
    }
    /// Dispatch an actual typed JSON envelope and return its matching reply/events packet.
    pub fn request(&mut self, envelope: &str) -> Result<String, JsValue> {
        dispatch(&mut self.session, envelope).map_err(|e| js_error(&e))
    }
}
fn dispatch(session: &mut Session, source: &str) -> Result<String, String> {
    if source.len() > 16 * 1024 * 1024 {
        return Err("Kernel envelope exceeds limit".into());
    }
    let request: Envelope<Request> =
        serde_json::from_str(source).map_err(|_| "Invalid kernel envelope")?;
    if !om_kernel::protocol::request_size_allowed(source.len(), &request.body) {
        return Err("Kernel envelope exceeds limit".into());
    }
    if request.id == 0 || request.id > 9_007_199_254_740_991 {
        return Err("Invalid request correlation ID".into());
    }
    let (response, events) = session.handle(request.body);
    serde_json::to_string(&Packet {
        response: Envelope {
            id: request.id,
            body: response,
        },
        events: events
            .into_iter()
            .map(|body| Envelope { id: 0, body })
            .collect(),
    })
    .map_err(|_| "Cannot encode kernel response".into())
}
fn js_error(message: &str) -> JsValue {
    #[cfg(target_arch = "wasm32")]
    {
        JsValue::from_str(message)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = message;
        JsValue::NULL
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    #[test]
    fn real_json_dispatch_solves_and_exports_source_dependency_recovery() {
        let mut kernel = Kernel::new(None).unwrap();
        let packet:Value=serde_json::from_str(&kernel.request(&json!({"id":7,"body":{"type":"evaluate","cell_id":"math","source":"solve(x^2==4,x)","dialect":"Modern"}}).to_string()).unwrap()).unwrap();
        assert_eq!(packet["response"]["id"], 7);
        assert_eq!(
            packet["response"]["body"]["output"]["items"][0]["view"]["solutions"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        let packet: Value = serde_json::from_str(
            &kernel
                .request(r#"{"id":8,"body":{"type":"get_notebook_state"}}"#)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            packet["response"]["body"]["state"]["file"]["cells"][0]["source"],
            "solve(x^2==4,x)"
        );
    }
    #[test]
    fn bad_envelopes_have_no_math_side_effects() {
        let mut session = Session::new(Default::default(), None);
        for text in [
            "bad",
            r#"{"id":0,"body":{"type":"run_all"}}"#,
            r#"{"id":18446744073709551615,"body":{"type":"run_all"}}"#,
        ] {
            assert!(dispatch(&mut session, text).is_err());
        }
        assert!(session.notebook.cells.is_empty());
    }
}
