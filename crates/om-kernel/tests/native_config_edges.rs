//! Native transaction/lifecycle regressions never invoke the live environment or vault.
#![cfg(feature = "native")]
use om_kernel::{KernelConfig, Session, native::*, protocol::*};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Dir(PathBuf);
impl Dir {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "openmath-config-edges-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn path(&self) -> PathBuf {
        self.0.join("config.toml")
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[derive(Default)]
struct Vault {
    data: Mutex<BTreeMap<String, String>>,
    calls: AtomicUsize,
    fail_at: AtomicUsize,
    after_write: AtomicBool,
    fail_all: AtomicBool,
    access_error: AtomicBool,
    encoding_error: AtomicBool,
    unavailable: AtomicBool,
}
impl CredentialProvider for Vault {
    fn environment(&self, _: &str) -> Result<Option<String>, CredentialError> {
        if self.encoding_error.load(Ordering::Relaxed) {
            Err(CredentialError::Encoding)
        } else {
            Ok(None)
        }
    }
    fn load(&self, p: &str) -> Result<Option<String>, CredentialError> {
        if self.unavailable.load(Ordering::Relaxed) {
            Err(CredentialError::Unavailable)
        } else if self.access_error.load(Ordering::Relaxed) {
            Err(CredentialError::Access)
        } else {
            Ok(self.data.lock().unwrap().get(p).cloned())
        }
    }
    fn store(&self, p: &str, key: Option<&str>) -> Result<(), CredentialError> {
        let n = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
        let fail =
            self.fail_all.load(Ordering::Relaxed) || self.fail_at.load(Ordering::Relaxed) == n;
        if fail && !self.after_write.load(Ordering::Relaxed) {
            return Err(CredentialError::Access);
        }
        {
            let mut data = self.data.lock().unwrap();
            if let Some(key) = key {
                data.insert(p.into(), key.into());
            } else {
                data.remove(p);
            }
        }
        if fail {
            Err(CredentialError::Access)
        } else {
            Ok(())
        }
    }
}
fn session(dir: &Dir, v: Arc<Vault>) -> Session {
    Session::new_native(ConfigStore::new(dir.path(), v, KeyStorage::Vault), None).unwrap()
}
#[test]
fn provider_that_mutates_then_fails_is_restored_before_reporting_the_update_failure() {
    let d = Dir::new();
    let v = Arc::new(Vault::default());
    let mut s = session(&d, v.clone());
    v.fail_at.store(1, Ordering::Relaxed);
    v.after_write.store(true, Ordering::Relaxed);
    let before = serde_json::to_value(&s.config).unwrap();
    let mut config = s.config.clone();
    config.llm.profiles[0].api_key = Some("partial-write-secret".into());
    assert!(matches!(
        s.handle(Request::SetConfig { config }).0,
        Response::Error { .. }
    ));
    assert!(v.data.lock().unwrap().is_empty());
    assert_eq!(serde_json::to_value(&s.config).unwrap(), before);
    assert!(!d.path().exists());
    assert_eq!(std::fs::read_dir(&d.0).unwrap().count(), 0);
}
#[test]
fn later_vault_failure_restores_earlier_keys_and_explicit_rollback_failure_is_honest() {
    let d = Dir::new();
    let v = Arc::new(Vault::default());
    let mut s = session(&d, v.clone());
    v.data
        .lock()
        .unwrap()
        .insert("deepseek".into(), "old-secret".into());
    v.fail_at.store(2, Ordering::Relaxed);
    let mut config = s.config.clone();
    for p in &mut config.llm.profiles {
        p.api_key = Some("new-secret".into());
    }
    assert!(matches!(
        s.handle(Request::SetConfig { config }).0,
        Response::Error { .. }
    ));
    assert_eq!(
        v.data.lock().unwrap().get("deepseek").map(String::as_str),
        Some("old-secret")
    );
    assert!(!v.data.lock().unwrap().contains_key("deepseek-fim"));
    assert!(!d.path().exists());
    v.calls.store(0, Ordering::Relaxed);
    v.fail_at.store(0, Ordering::Relaxed);
    v.fail_all.store(true, Ordering::Relaxed);
    v.after_write.store(true, Ordering::Relaxed);
    let mut config = s.config.clone();
    config.llm.profiles[0].api_key = Some("failed-rollback-secret".into());
    let Response::Error { message } = s.handle(Request::SetConfig { config }).0 else {
        panic!()
    };
    assert!(message.contains("rollback"));
    assert!(!message.contains("failed-rollback-secret"));
}
#[test]
fn masks_empty_keys_removal_and_rename_preserve_only_the_correct_named_credentials() {
    let d = Dir::new();
    let v = Arc::new(Vault::default());
    let mut s = session(&d, v.clone());
    let mut config = s.config.clone();
    config.llm.profiles[0].api_key = Some(String::new());
    config.llm.profiles[1].api_key = Some("second-key".into());
    assert!(matches!(
        s.handle(Request::SetConfig { config }).0,
        Response::Ok
    ));
    assert_eq!(
        s.resolved_profile_key("deepseek")
            .unwrap()
            .unwrap()
            .as_str(),
        ""
    );
    let wire = serde_json::to_string(&s.handle(Request::GetConfig).0).unwrap();
    let Response::Config { mut config } = serde_json::from_str(&wire).unwrap() else {
        panic!()
    };
    config.llm.profiles[0].name = "renamed".into();
    config.llm.profiles.remove(1);
    assert!(matches!(
        s.handle(Request::SetConfig { config }).0,
        Response::Ok
    ));
    assert!(v.data.lock().unwrap().is_empty());
    assert!(s.resolved_profile_key("renamed").unwrap().is_none());
    assert!(s.resolved_profile_key("deepseek").is_err());
}
#[test]
fn locked_nonunicode_and_unavailable_providers_keep_the_declared_resolution_policy() {
    let d = Dir::new();
    let v = Arc::new(Vault::default());
    let store = ConfigStore::new(d.path(), v.clone(), KeyStorage::Vault);
    let mut profile = KernelConfig::default().llm.profiles.remove(0);
    profile.api_key = Some("raw-fallback".into());
    v.access_error.store(true, Ordering::Relaxed);
    assert!(matches!(
        store.resolve_key(&profile),
        Err(ConfigError::Credential(CredentialError::Access))
    ));
    v.access_error.store(false, Ordering::Relaxed);
    v.encoding_error.store(true, Ordering::Relaxed);
    assert!(matches!(
        store.resolve_key(&profile),
        Err(ConfigError::Credential(CredentialError::Encoding))
    ));
    v.encoding_error.store(false, Ordering::Relaxed);
    v.unavailable.store(true, Ordering::Relaxed);
    assert_eq!(
        store.resolve_key(&profile).unwrap().unwrap().as_str(),
        "raw-fallback"
    );
    let mut s = Session::new_native(store, None).unwrap();
    let mut config = s.config.clone();
    config.general.show_steps = false;
    assert!(matches!(
        s.handle(Request::SetConfig { config }).0,
        Response::Ok
    ));
    let old = std::fs::read_to_string(d.path()).unwrap();
    let mut config = s.config.clone();
    config.llm.profiles[0].api_key = Some("not-written-secret".into());
    assert!(matches!(
        s.handle(Request::SetConfig { config }).0,
        Response::Error { .. }
    ));
    assert_eq!(std::fs::read_to_string(d.path()).unwrap(), old);
}
#[test]
fn invalid_updates_and_nonunicode_files_do_not_modify_durable_or_runtime_state() {
    let d = Dir::new();
    let v = Arc::new(Vault::default());
    let mut s = session(&d, v.clone());
    let mut config = s.config.clone();
    config.llm.profiles[0].temperature = f32::NAN;
    assert!(matches!(
        s.handle(Request::SetConfig { config }).0,
        Response::Error { .. }
    ));
    assert!(!d.path().exists());
    assert_eq!(v.calls.load(Ordering::Relaxed), 0);
    let store = ConfigStore::new(d.path(), v, KeyStorage::Vault);
    std::fs::write(d.path(), [0xff, 0xfe]).unwrap();
    assert!(store.load().is_err());
    assert_eq!(std::fs::read(d.path()).unwrap(), [0xff, 0xfe]);
}
