//! A settings probe uses draft transport data without mutating settings or notebook state.
pub mod support;
use om_kernel::{KernelConfig, Session, protocol::*};
#[test]
fn ephemeral_profile_probe_keeps_config_and_source_unchanged_and_resolves_masks_by_identity() {
    let mut config = KernelConfig::default();
    config.llm.profiles[0].api_key = Some("saved-synthetic-key".into());
    let mut s = Session::new(config.clone(), None);
    let mut draft = config.llm.profiles[0].clone();
    draft.base_url = "https://draft.invalid/v1".into();
    draft.model = "editable-draft".into();
    draft.api_key = Some("new-synthetic-key".into());
    let before = serde_json::to_value(s.handle(Request::GetConfig).0).unwrap();
    let (response, _) = s.handle(Request::LlmTestProfile {
        request_id: "draft".into(),
        profile: draft.name.clone(),
        config: Some(draft.clone()),
    });
    let Response::LlmStarted {
        http: Some(http), ..
    } = response
    else {
        panic!("{response:?}")
    };
    assert_eq!(http.url, "https://draft.invalid/v1/chat/completions");
    assert!(
        http.headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "Bearer new-synthetic-key")
    );
    assert!(http.body.contains("editable-draft"));
    assert_eq!(
        before,
        serde_json::to_value(s.handle(Request::GetConfig).0).unwrap()
    );
    assert!(s.notebook.cells.is_empty());
    draft.api_key = Some("***".into());
    let response = s
        .handle(Request::LlmTestProfile {
            request_id: "mask".into(),
            profile: draft.name.clone(),
            config: Some(draft.clone()),
        })
        .0;
    let Response::LlmStarted {
        http: Some(http), ..
    } = response
    else {
        panic!()
    };
    assert!(
        http.headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "Bearer saved-synthetic-key")
    );
    draft.name = "new-name".into();
    assert!(matches!(
        s.handle(Request::LlmTestProfile {
            request_id: "bad-mask".into(),
            profile: draft.name.clone(),
            config: Some(draft)
        })
        .0,
        Response::Error { .. }
    ));
}

#[cfg(feature = "native")]
#[test]
fn native_draft_key_intents_preserve_env_precedence_without_vault_or_file_writes() {
    use om_kernel::native::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct Credentials(AtomicUsize);
    impl CredentialProvider for Credentials {
        fn environment(&self, name: &str) -> Result<Option<String>, CredentialError> {
            Ok((name == "CURRENT_ENV").then(|| "env-synthetic".into()))
        }
        fn load(&self, _: &str) -> Result<Option<String>, CredentialError> {
            Ok(Some("vault-synthetic".into()))
        }
        fn store(&self, _: &str, _: Option<&str>) -> Result<(), CredentialError> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }
    let credentials = Arc::new(Credentials(AtomicUsize::new(0)));
    let path = std::env::temp_dir()
        .join(format!("openmath-profile-draft-{}", std::process::id()))
        .join("config.toml");
    let store = ConfigStore::new(path.clone(), credentials.clone(), KeyStorage::Vault);
    let mut s = Session::new_native(store, None).unwrap();
    let before = serde_json::to_value(s.handle(Request::GetConfig).0).unwrap();
    let mut draft = s.config.llm.profiles[0].clone();
    draft.api_key = Some("new-synthetic".into());
    draft.api_key_env = None;
    assert_eq!(
        s.resolved_draft_key(&draft).unwrap().unwrap().as_str(),
        "new-synthetic"
    );
    draft.api_key = None;
    assert!(s.resolved_draft_key(&draft).unwrap().is_none());
    draft.api_key = Some("***".into());
    assert_eq!(
        s.resolved_draft_key(&draft).unwrap().unwrap().as_str(),
        "vault-synthetic"
    );
    draft.api_key_env = Some("CURRENT_ENV".into());
    assert_eq!(
        s.resolved_draft_key(&draft).unwrap().unwrap().as_str(),
        "env-synthetic"
    );
    assert_eq!(credentials.0.load(Ordering::Relaxed), 0);
    assert!(!path.exists());
    assert_eq!(
        before,
        serde_json::to_value(s.handle(Request::GetConfig).0).unwrap()
    );
}

#[cfg(feature = "native")]
#[tokio::test]
async fn actual_native_draft_http_uses_new_key_and_returns_real_probe_metrics_without_saving() {
    use om_kernel::native::*;
    use om_llm::{drive_native_http, native_client};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::Instant;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method, path},
    };
    struct Clock(Instant);
    impl om_num::ctx::Clock for Clock {
        fn now_ms(&self) -> f64 {
            self.0.elapsed().as_secs_f64() * 1000.0
        }
    }
    struct Keys(AtomicUsize);
    impl CredentialProvider for Keys {
        fn environment(&self, _: &str) -> Result<Option<String>, CredentialError> {
            Ok(None)
        }
        fn load(&self, _: &str) -> Result<Option<String>, CredentialError> {
            Ok(Some("old-synthetic".into()))
        }
        fn store(&self, _: &str, _: Option<&str>) -> Result<(), CredentialError> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }
    let server = MockServer::start().await;
    let body = "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"actual-ping\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("Authorization", "Bearer new-synthetic"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .expect(1)
        .mount(&server)
        .await;
    let keys = Arc::new(Keys(AtomicUsize::new(0)));
    let location = std::env::temp_dir()
        .join(format!("openmath-draft-http-{}", std::process::id()))
        .join("config.toml");
    let mut s = Session::new_native(
        ConfigStore::new(location.clone(), keys.clone(), KeyStorage::Vault),
        Some(Arc::new(Clock(Instant::now()))),
    )
    .unwrap();
    let before = serde_json::to_value(s.handle(Request::GetConfig).0).unwrap();
    let mut draft = s.config.llm.profiles[0].clone();
    draft.base_url = server.uri();
    draft.api_key_env = None;
    draft.api_key = Some("new-synthetic".into());
    let (started, _) = s.handle(Request::LlmTestProfile {
        request_id: "http-draft".into(),
        profile: draft.name.clone(),
        config: Some(draft),
    });
    let Response::LlmStarted {
        http: Some(http), ..
    } = started
    else {
        panic!()
    };
    let cancel = s.llm_cancellation_handle("http-draft").unwrap();
    let mut events = vec![];
    let (status, error) = drive_native_http(
        &http,
        1000,
        &native_client().unwrap(),
        &cancel.token(),
        |status, bytes| {
            events.extend(s.llm_http_bytes("http-draft", status, bytes).1);
            true
        },
    )
    .await;
    events.extend(
        s.handle(Request::LlmHttpEnd {
            request_id: "http-draft".into(),
            status,
            error,
        })
        .1,
    );
    assert!(events.iter().any(|e|matches!(e,Event::LlmProfileTest{response,latency_ms:Some(_),first_byte_ms:Some(_),..} if response=="actual-ping")));
    assert_eq!(
        before,
        serde_json::to_value(s.handle(Request::GetConfig).0).unwrap()
    );
    assert_eq!(keys.0.load(Ordering::Relaxed), 0);
    assert!(!location.exists());
}
