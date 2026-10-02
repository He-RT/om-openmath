//! Explicit live smoke harness: accepts a key from stdin, never persists or prints it.
#![forbid(unsafe_code)]
#![cfg(feature = "native")]
use om_kernel::{KernelConfig, Session, protocol::*};
use om_llm::{Target, drive_native_http, native_client};
use om_num::ctx::Clock;
use std::{io, sync::Arc, time::Instant};
struct Time(Instant);
impl Clock for Time {
    fn now_ms(&self) -> f64 {
        self.0.elapsed().as_secs_f64() * 1000.0
    }
}
#[tokio::main]
async fn main() {
    let mut key = String::new();
    if io::stdin().read_line(&mut key).is_err() {
        return;
    }
    let mut config = KernelConfig::default();
    config.llm.send_context = false;
    for p in &mut config.llm.profiles {
        p.api_key = Some(key.trim().into());
        p.api_key_env = None;
        p.timeout_ms = 30_000;
    }
    key.clear();
    config.llm.profiles[0].max_tokens = 256;
    let mut session = Session::new(config, Some(Arc::new(Time(Instant::now()))));
    if session.set_llm_target(Target::Native).is_err() {
        return;
    }
    let client = match native_client() {
        Ok(c) => c,
        Err(_) => return,
    };
    for (id,request) in [
        ("probe",Request::LlmTestProfile{request_id:"probe".into(),profile:"deepseek".into()}),
        ("translate",Request::LlmTranslate{request_id:"translate".into(),text:"Solve x squared equals 4 over the real numbers.".into(),cell_id:None}),
        ("complete",Request::LlmComplete{request_id:"complete".into(),prefix:"solve(x^2 == 4, ".into(),suffix:")".into(),dialect:Dialect::Modern}),
        ("chat",Request::LlmChat{request_id:"chat".into(),messages:vec![ChatMessage{role:Role::User,content:"Use the solve tool to solve x^2 == 4 over the reals and cite the returned CAS result.".into(),tool_calls:vec![],tool_call_id:None}]}),
    ] {
        let started=Instant::now();let (response,_)=session.handle(request);
        let Response::LlmStarted {http:Some(mut http),..}=response else{println!("{id}: start failed");continue;};
        let mut round=0;let mut suggestions=0;let mut tools=0;let mut delta=String::new();
        loop {
            round+=1;let Some(cancel)=session.llm_cancellation_handle(id)else{break;};
            let timeout=session.llm_http_timeout(id).unwrap_or(30_000);
            let mut events=vec![];
            let (status,error)=drive_native_http(&http,timeout,&client,&cancel.token(),|status,bytes|{events.extend(session.llm_http_bytes(id,status,bytes).1);session.llm_cancellation_handle(id).is_some()}).await;
            events.extend(session.handle(Request::LlmHttpEnd{request_id:id.into(),status,error}).1);
            let mut next=None;let mut failed=false;
            for event in events {
                match event {
                    Event::LlmHttp{http,..}=>next=Some(http),
                    Event::LlmDelta{text,..}=>delta.push_str(&text),
                    Event::LlmSuggestion{suggestion,..}=>{suggestions+=1;println!("{id}: checked_source={}",suggestion.wolfram);}
                    Event::LlmToolCall{name,result_summary,..}=>{tools+=1;println!("{id}: actual_tool={name}; result={result_summary}");}
                    Event::LlmProfileTest{response,latency_ms,first_byte_ms,..}=>println!("{id}: reply={response}; latency_ms={latency_ms:?}; first_byte_ms={first_byte_ms:?}"),
                    Event::LlmError{message,..}=>{failed=true;println!("{id}: failure={message}");}
                    _=>{}
                }
            }
            if let Some(request)=next{http=request;}else{println!("{id}: status={status}; failed={failed}; rounds={round}; suggestions={suggestions}; tools={tools}; latency_ms={:.0}; text={delta}",started.elapsed().as_secs_f64()*1000.0);break;}
        }
    }
}
