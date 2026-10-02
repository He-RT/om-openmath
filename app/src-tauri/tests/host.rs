//! The native owner thread and actual loopback HTTP never expose internal auth to UI.
use om_desktop::KernelHost;
use om_kernel::{KernelConfig, native::*};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method},
};
static SERIAL: AtomicUsize = AtomicUsize::new(0);
#[derive(Default)]
struct Credentials(Mutex<BTreeMap<String, String>>);
impl CredentialProvider for Credentials {
    fn environment(&self, _: &str) -> Result<Option<String>, CredentialError> {
        Ok(None)
    }
    fn load(&self, name: &str) -> Result<Option<String>, CredentialError> {
        Ok(self.0.lock().unwrap().get(name).cloned())
    }
    fn store(&self, name: &str, key: Option<&str>) -> Result<(), CredentialError> {
        let mut c = self.0.lock().unwrap();
        if let Some(key) = key {
            c.insert(name.into(), key.into());
        } else {
            c.remove(name);
        }
        Ok(())
    }
}
struct Folder(PathBuf);
impl Folder {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "om-host-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn store(folder: &Folder, credentials: Arc<Credentials>) -> ConfigStore {
    ConfigStore::new(folder.0.join("config.toml"), credentials, KeyStorage::Vault)
}
async fn request(host: &KernelHost, id: u64, body: Value) -> Value {
    serde_json::from_str(
        &host
            .request(json!({"id":id,"body":body}).to_string())
            .await
            .unwrap(),
    )
    .unwrap()
}
#[tokio::test]
async fn actual_thread_math_envelopes_and_named_secret_commands_are_real_and_masked() {
    let folder = Folder::new();
    let credentials = Arc::new(Credentials::default());
    let host = KernelHost::new(store(&folder, credentials.clone())).unwrap();
    let result = request(
        &host,
        42,
        json!({"type":"evaluate","cell_id":"a","source":"solve(x^2==4,x)","dialect":"Modern"}),
    )
    .await;
    assert_eq!(result["id"], 42);
    assert_eq!(
        result["body"]["output"]["items"][0]["view"]["solutions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    host.secret("deepseek".into(), Some("synthetic-host-secret".into()))
        .await
        .unwrap();
    assert_eq!(
        credentials
            .0
            .lock()
            .unwrap()
            .get("deepseek")
            .map(String::as_str),
        Some("synthetic-host-secret")
    );
    let config = request(&host, 43, json!({"type":"get_config"})).await;
    assert_eq!(
        config["body"]["config"]["llm"]["profiles"][0]["api_key"],
        "***"
    );
    assert!(!config.to_string().contains("synthetic-host-secret"));
    host.secret("deepseek".into(), None).await.unwrap();
    assert!(credentials.0.lock().unwrap().is_empty());
    assert!(
        host.secret("missing".into(), Some("private".into()))
            .await
            .is_err()
    );
    assert!(host.request("invalid".into()).await.is_err());
}
#[tokio::test]
async fn local_native_llm_is_driven_and_internal_transport_never_enters_public_response_or_events()
{
    let server = MockServer::start().await;
    let body = format!(
        "data: {}\n\ndata: [DONE]\n\n",
        json!({"choices":[{"index":0,"delta":{"content":"pong"},"finish_reason":"stop"}]})
    );
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer synthetic-host-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .expect(1)
        .mount(&server)
        .await;
    let folder = Folder::new();
    let credentials = Arc::new(Credentials::default());
    credentials
        .0
        .lock()
        .unwrap()
        .insert("deepseek".into(), "synthetic-host-secret".into());
    let store = store(&folder, credentials);
    let mut config = KernelConfig::default();
    config.llm.profiles[0].base_url = server.uri();
    store.save(&config).unwrap();
    let host = KernelHost::new(store).unwrap();
    let events = Arc::new(Mutex::new(Vec::<String>::new()));
    let out = events.clone();
    host.subscribe(Arc::new(move |event| out.lock().unwrap().push(event)))
        .unwrap();
    let result = request(
        &host,
        1,
        json!({"type":"llm_test_profile","request_id":"actual","profile":"deepseek"}),
    )
    .await;
    assert_eq!(result["body"]["type"], "llm_started");
    assert_eq!(result["body"]["http"], Value::Null);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if events
                .lock()
                .unwrap()
                .iter()
                .any(|e| e.contains("llm_done"))
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
        !events
            .iter()
            .any(|e| e.contains("synthetic-host-secret") || e.contains("llm_http"))
    );
    assert!(
        events
            .iter()
            .any(|e| e.contains("llm_profile_test") && e.contains("pong"))
    );
    for event in events.iter() {
        let e: Value = serde_json::from_str(event).unwrap();
        assert_eq!(e["id"], 0);
    }
}
