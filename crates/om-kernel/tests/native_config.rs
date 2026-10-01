//! Native storage uses isolated files and synthetic injectable credentials only.
#![cfg(feature = "native")]
use om_kernel::{KernelConfig, Session, config::*, native::*, protocol::*};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
static SERIAL: AtomicU64 = AtomicU64::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "openmath-config-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn path(&self) -> PathBuf {
        self.0.join("config.toml")
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[derive(Default)]
struct Credentials {
    env: Mutex<BTreeMap<String, String>>,
    vault: Mutex<BTreeMap<String, String>>,
    fail_write: AtomicBool,
    unavailable: AtomicBool,
}
impl CredentialProvider for Credentials {
    fn environment(&self, name: &str) -> Result<Option<String>, CredentialError> {
        Ok(self.env.lock().unwrap().get(name).cloned())
    }
    fn load(&self, profile: &str) -> Result<Option<String>, CredentialError> {
        if self.unavailable.load(Ordering::Relaxed) {
            Err(CredentialError::Unavailable)
        } else {
            Ok(self.vault.lock().unwrap().get(profile).cloned())
        }
    }
    fn store(&self, profile: &str, key: Option<&str>) -> Result<(), CredentialError> {
        if self.fail_write.load(Ordering::Relaxed) || self.unavailable.load(Ordering::Relaxed) {
            return Err(CredentialError::Unavailable);
        }
        let mut v = self.vault.lock().unwrap();
        if let Some(key) = key {
            v.insert(profile.into(), key.into());
        } else {
            v.remove(profile);
        }
        Ok(())
    }
}
fn store(d: &Directory, c: Arc<Credentials>, policy: KeyStorage) -> ConfigStore {
    ConfigStore::new(d.path(), c, policy)
}
#[test]
fn missing_partial_and_complete_toml_preserve_defaults_and_trusted_keys() {
    let d = Directory::new();
    let c = Arc::new(Credentials::default());
    let store = store(&d, c, KeyStorage::Plaintext);
    assert_eq!(
        serde_json::to_value(store.load().unwrap()).unwrap(),
        serde_json::to_value(KernelConfig::default()).unwrap()
    );
    assert!(!d.path().exists());
    std::fs::write(d.path(), "[general]\nlanguage='en'\n[llm]\nenabled=false\n").unwrap();
    let config = store.load().unwrap();
    assert_eq!(config.general.language, Language::En);
    assert!(!config.llm.enabled);
    assert_eq!(config.general.eval_timeout_ms, 30000);
    let mut config = KernelConfig::default();
    config.general.constants = Constants::Strict;
    config.llm.profiles[0].api_key = Some("synthetic-file-secret".into());
    config.llm.profiles[0]
        .extra_headers
        .insert("X-Example".into(), "example".into());
    store.save(&config).unwrap();
    let raw = std::fs::read_to_string(d.path()).unwrap();
    assert!(raw.contains("synthetic-file-secret"));
    assert!(!raw.contains("***"));
    let restored = store.load().unwrap();
    assert_eq!(
        restored.llm.profiles[0].api_key.as_deref(),
        Some("synthetic-file-secret")
    );
    assert_eq!(
        serde_json::to_value(restored).unwrap(),
        serde_json::to_value(config).unwrap()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(d.path()).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
#[test]
fn key_priority_debug_and_unavailable_vault_fallback_use_actual_sources() {
    let d = Directory::new();
    let c = Arc::new(Credentials::default());
    let store = store(&d, c.clone(), KeyStorage::Vault);
    let mut p = KernelConfig::default().llm.profiles.remove(0);
    p.api_key = Some("fallback-secret".into());
    c.vault
        .lock()
        .unwrap()
        .insert(p.name.clone(), "vault-secret".into());
    c.env
        .lock()
        .unwrap()
        .insert("DEEPSEEK_API_KEY".into(), "env-secret".into());
    let key = store.resolve_key(&p).unwrap().unwrap();
    assert_eq!(key.as_str(), "env-secret");
    assert!(!format!("{key:?}").contains("env-secret"));
    c.env.lock().unwrap().clear();
    assert_eq!(
        store.resolve_key(&p).unwrap().unwrap().as_str(),
        "vault-secret"
    );
    c.vault.lock().unwrap().clear();
    assert_eq!(
        store.resolve_key(&p).unwrap().unwrap().as_str(),
        "fallback-secret"
    );
    c.unavailable.store(true, Ordering::Relaxed);
    assert_eq!(
        store.resolve_key(&p).unwrap().unwrap().as_str(),
        "fallback-secret"
    );
    c.env
        .lock()
        .unwrap()
        .insert("DEEPSEEK_API_KEY".into(), String::new());
    assert_eq!(store.resolve_key(&p).unwrap().unwrap().as_str(), "");
}
#[test]
fn bound_session_stores_real_vault_keys_preserves_masks_and_never_persists_env_values() {
    let d = Directory::new();
    let c = Arc::new(Credentials::default());
    let store = store(&d, c.clone(), KeyStorage::Vault);
    let mut s = Session::new_native(store, None).unwrap();
    let mut config = s.config.clone();
    config.llm.profiles[0].api_key = Some("entered-vault-secret".into());
    assert!(matches!(
        s.handle(Request::SetConfig { config }).0,
        Response::Ok
    ));
    assert_eq!(c.vault.lock().unwrap()["deepseek"], "entered-vault-secret");
    assert!(
        !std::fs::read_to_string(d.path())
            .unwrap()
            .contains("entered-vault-secret")
    );
    c.env
        .lock()
        .unwrap()
        .insert("DEEPSEEK_API_KEY".into(), "environment-secret".into());
    let wire = serde_json::to_string(&s.handle(Request::GetConfig).0).unwrap();
    assert!(!wire.contains("environment-secret"));
    assert!(!wire.contains("entered-vault-secret"));
    let Response::Config { mut config } = serde_json::from_str(&wire).unwrap() else {
        panic!()
    };
    assert_eq!(config.llm.profiles[0].api_key.as_deref(), Some("***"));
    config.llm.profiles.reverse();
    config.general.show_steps = false;
    assert!(matches!(
        s.handle(Request::SetConfig { config }).0,
        Response::Ok
    ));
    assert_eq!(c.vault.lock().unwrap()["deepseek"], "entered-vault-secret");
    assert!(
        !std::fs::read_to_string(d.path())
            .unwrap()
            .contains("environment-secret")
    );
    let key = s.resolved_profile_key("deepseek").unwrap().unwrap();
    assert_eq!(key.as_str(), "environment-secret");
    let mut config = s.config.clone();
    config
        .llm
        .profiles
        .iter_mut()
        .find(|p| p.name == "deepseek")
        .unwrap()
        .api_key = None;
    assert!(matches!(
        s.handle(Request::SetConfig { config }).0,
        Response::Ok
    ));
    assert!(!c.vault.lock().unwrap().contains_key("deepseek"));
}
#[test]
fn invalid_files_and_configs_are_sanitized_and_preserved_without_overwrite() {
    let d = Directory::new();
    let c = Arc::new(Credentials::default());
    let store = store(&d, c, KeyStorage::Vault);
    for text in [
        "[llm]\nprofiles=[{name='x',api_key='synthetic-invalid-secret'",
        "[[llm.profiles]]\nname='x'\napi_key='***'",
        "[[llm.profiles]]\nname=''",
        "[[llm.profiles]]\nname='x'\n[[llm.profiles]]\nname='x'",
    ] {
        std::fs::write(d.path(), text).unwrap();
        let error = store.load().unwrap_err();
        assert!(!format!("{error:?}: {error}").contains("synthetic-invalid-secret"));
        assert_eq!(std::fs::read_to_string(d.path()).unwrap(), text);
    }
}
#[test]
fn file_failure_rolls_back_vault_and_session_state_then_recovers() {
    let d = Directory::new();
    let c = Arc::new(Credentials::default());
    let path = d.0.join("blocked");
    let store = ConfigStore::new(path.clone(), c.clone(), KeyStorage::Vault);
    let mut s = Session::new_native(store, None).unwrap();
    std::fs::create_dir(&path).unwrap();
    let before = serde_json::to_value(&s.config).unwrap();
    let mut config = s.config.clone();
    config.general.show_steps = false;
    config.llm.profiles[0].api_key = Some("never-committed-secret".into());
    let Response::Error { message } = s
        .handle(Request::SetConfig {
            config: config.clone(),
        })
        .0
    else {
        panic!()
    };
    assert!(!message.contains("never-committed-secret"));
    assert_eq!(serde_json::to_value(&s.config).unwrap(), before);
    assert!(c.vault.lock().unwrap().is_empty());
    std::fs::remove_dir(&path).unwrap();
    assert!(matches!(
        s.handle(Request::SetConfig { config }).0,
        Response::Ok
    ));
    assert!(!s.config.general.show_steps);
    assert_eq!(
        c.vault.lock().unwrap()["deepseek"],
        "never-committed-secret"
    );
}
#[test]
fn project_path_and_pure_sessions_have_no_implicit_native_io() {
    let expected = directories::ProjectDirs::from("org", "openmath", "OpenMath")
        .unwrap()
        .config_dir()
        .join("config.toml");
    assert_eq!(ConfigStore::system_default().unwrap().path(), expected);
    let mut s = Session::new(Default::default(), None);
    assert!(matches!(
        s.handle(Request::SetConfig {
            config: Default::default()
        })
        .0,
        Response::Ok
    ));
    assert!(s.notebook.cells.is_empty());
}

#[test]
fn trusted_load_save_and_presentation_preserve_existing_credential_sources() {
    let d = Directory::new();
    let c = Arc::new(Credentials::default());
    std::fs::write(
        d.path(),
        "[[llm.profiles]]\nname='legacy'\napi_key='legacy-file-secret'\n",
    )
    .unwrap();
    c.vault
        .lock()
        .unwrap()
        .insert("legacy".into(), "higher-vault-secret".into());
    let store = store(&d, c.clone(), KeyStorage::Vault);
    let mut config = store.load().unwrap();
    let presentation = store.presentation(&config).unwrap();
    assert_eq!(presentation.llm.profiles[0].api_key.as_deref(), Some("***"));
    config.general.show_steps = false;
    store.save(&config).unwrap();
    assert_eq!(
        store.load().unwrap().llm.profiles[0].api_key.as_deref(),
        Some("legacy-file-secret")
    );
    assert_eq!(c.vault.lock().unwrap()["legacy"], "higher-vault-secret");
}

#[test]
fn bound_config_and_credentials_survive_source_notebook_load_and_reload() {
    let d = Directory::new();
    let c = Arc::new(Credentials::default());
    let mut s = Session::new_native(store(&d, c.clone(), KeyStorage::Vault), None).unwrap();
    let mut config = s.config.clone();
    config.general.constants = Constants::Strict;
    config.general.show_steps = false;
    config.general.language = Language::En;
    config.llm.profiles[0].api_key = Some("persisted-key".into());
    assert!(matches!(
        s.handle(Request::SetConfig { config }).0,
        Response::Ok
    ));
    s.handle(Request::Interrupt);
    s.handle(Request::LoadNotebook {
        file: NotebookFile {
            version: 1,
            title: "source-only".into(),
            cells: vec![],
        },
    });
    assert!(s.interrupt_handle().load(Ordering::Relaxed));
    let text = serde_json::to_string(&s.handle(Request::SaveNotebook).0).unwrap();
    assert!(!text.contains("persisted-key"));
    assert!(!text.contains("config"));
    let mut restored = Session::new_native(store(&d, c, KeyStorage::Vault), None).unwrap();
    assert_eq!(restored.config.general.language, Language::En);
    assert_eq!(restored.config.general.constants, Constants::Strict);
    assert!(!restored.config.general.show_steps);
    assert_eq!(
        restored
            .resolved_profile_key("deepseek")
            .unwrap()
            .unwrap()
            .as_str(),
        "persisted-key"
    );
    let response = restored
        .handle(Request::Evaluate {
            cell_id: "s".into(),
            source: "Solve[x^2==4,x]".into(),
            dialect: Dialect::Wolfram,
        })
        .0;
    let Response::Evaluated { output, .. } = response else {
        panic!()
    };
    assert!(matches!(
        output.items[0],
        OutputItem::Solutions { steps: None, .. }
    ));
}
