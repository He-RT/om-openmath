//! Direct interruption/cancellation and owner shutdown work independently of the queue.
use om_desktop::KernelHost;
use om_kernel::{KernelConfig, native::*};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
static SERIAL: AtomicUsize = AtomicUsize::new(0);
struct NoCredentials;
impl CredentialProvider for NoCredentials {
    fn environment(&self, _: &str) -> Result<Option<String>, CredentialError> {
        Ok(None)
    }
    fn load(&self, _: &str) -> Result<Option<String>, CredentialError> {
        Ok(None)
    }
    fn store(&self, _: &str, _: Option<&str>) -> Result<(), CredentialError> {
        Ok(())
    }
}
struct Folder(PathBuf);
impl Folder {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "om-host-interrupt-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn store(folder: &Folder) -> ConfigStore {
    ConfigStore::new(
        folder.0.join("config.toml"),
        Arc::new(NoCredentials),
        KeyStorage::Vault,
    )
}
#[tokio::test]
async fn direct_interrupt_stops_running_real_cas_and_next_request_recovers() {
    let folder = Folder::new();
    let host = Arc::new(KernelHost::new(store(&folder)).unwrap());
    let running = host.clone();
    let task = tokio::spawn(async move {
        running.request(json!({"id":1,"body":{"type":"evaluate","cell_id":"slow","source":"Factorial[1000000]","dialect":"Wolfram"}}).to_string()).await
    });
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    host.interrupt();
    let result: Value = serde_json::from_str(
        &tokio::time::timeout(std::time::Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert!(
        result["body"]["output"]["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["tag"] == "interrupted"),
        "{result}"
    );
    let next:Value=serde_json::from_str(&host.request(json!({"id":2,"body":{"type":"evaluate","cell_id":"recovered","source":"2+2","dialect":"Modern"}}).to_string()).await.unwrap()).unwrap();
    assert_eq!(next["body"]["output"]["items"][0]["input_form"], "4");
}
#[tokio::test]
async fn native_llm_cancel_aborts_waiting_http_and_shutdown_joins_idle_owner() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_secs(2))
                .set_body_string("data: [DONE]\n\n"),
        )
        .mount(&server)
        .await;
    let folder = Folder::new();
    let store = store(&folder);
    let mut config = KernelConfig::default();
    config.llm.profiles[0].base_url = server.uri();
    store.save(&config).unwrap();
    let host = KernelHost::new(store).unwrap();
    let events = Arc::new(Mutex::new(Vec::<String>::new()));
    let out = events.clone();
    host.subscribe(Arc::new(move |event| out.lock().unwrap().push(event)))
        .unwrap();
    host.request(
        json!({"id":1,"body":{"type":"llm_test_profile","request_id":"slow","profile":"deepseek"}})
            .to_string(),
    )
    .await
    .unwrap();
    host.request(json!({"id":2,"body":{"type":"llm_cancel","request_id":"slow"}}).to_string())
        .await
        .unwrap();
    assert!(
        events
            .lock()
            .unwrap()
            .iter()
            .any(|e| e.contains("llm_error"))
    );
    let now = std::time::Instant::now();
    drop(host);
    assert!(now.elapsed() < std::time::Duration::from_secs(1));
}

#[tokio::test]
async fn queued_byte_acknowledgement_cannot_turn_an_expired_profile_request_into_success() {
    let server = MockServer::start().await;
    let body = format!(
        "data: {}\n\ndata: [DONE]\n\n",
        json!({"choices":[{"index":0,"delta":{"content":"pong"},"finish_reason":"stop"}]})
    );
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_millis(50))
                .set_body_string(body),
        )
        .mount(&server)
        .await;
    let folder = Folder::new();
    let store = store(&folder);
    let mut config = KernelConfig::default();
    config.general.eval_timeout_ms = 250;
    config.llm.profiles[0].base_url = server.uri();
    config.llm.profiles[0].timeout_ms = 100;
    store.save(&config).unwrap();
    let host = KernelHost::new(store).unwrap();
    let events = Arc::new(Mutex::new(Vec::<String>::new()));
    let out = events.clone();
    host.subscribe(Arc::new(move |event| out.lock().unwrap().push(event)))
        .unwrap();
    host.request(json!({"id":1,"body":{"type":"llm_test_profile","request_id":"deadline","profile":"deepseek"}}).to_string()).await.unwrap();
    host.request(json!({"id":2,"body":{"type":"evaluate","cell_id":"busy","source":"Factorial[1000000]","dialect":"Wolfram"}}).to_string()).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if events
                .lock()
                .unwrap()
                .iter()
                .any(|e| e.contains("llm_error") || e.contains("llm_done"))
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let events = events.lock().unwrap();
    assert!(
        events
            .iter()
            .any(|e| e.contains("llm_error") && e.contains("timed out")),
        "{events:?}"
    );
    assert!(!events.iter().any(|e| e.contains("llm_done")));
}
