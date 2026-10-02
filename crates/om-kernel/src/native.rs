//! Explicit native config persistence; portable Sessions never invoke this IO.
mod credentials;
mod file;
use crate::{KernelConfig, config::ProfileConfig};
pub use credentials::{CredentialError, CredentialProvider, NativeCredentials, SecretKey};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};

/// Secret-safe configuration, filesystem and credential failures.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// This host has no native application config directory.
    #[error("native config directory is unavailable")]
    Directory,
    /// Invalid syntax, with a byte offset but no source excerpt.
    #[error("invalid configuration TOML at byte {offset:?}")]
    Toml {
        /// Source byte offset, when known.
        offset: Option<usize>,
    },
    /// Profile names must be nonempty and unique.
    #[error("profile names must be nonempty and unique")]
    Profiles,
    /// A stored mask cannot be used as a credential.
    #[error("stored api_key mask is not a credential")]
    StoredMask,
    /// Nonfinite settings cannot be transported as real JSON values.
    #[error("profile temperatures must be finite")]
    Temperature,
    /// The trusted config representation could not be serialized.
    #[error("configuration serialization failed")]
    Serialize,
    /// Filesystem failure; the system's payload is deliberately excluded.
    #[error("configuration {operation} failed: {kind:?}")]
    Io {
        /// Failed fixed-name operation.
        operation: &'static str,
        /// Portable failure kind.
        kind: std::io::ErrorKind,
    },
    /// Credential failure, excluding backend payloads.
    #[error(transparent)]
    Credential(#[from] CredentialError),
    /// Multiple resources failed; no atomic success is claimed.
    #[error("credential rollback failed after an unsuccessful configuration update")]
    Rollback,
    /// The requested routing profile is not configured.
    #[error("requested LLM profile is not configured")]
    MissingProfile,
    /// The pure Session has no explicitly bound native store.
    #[error("native credential store is not bound")]
    Unbound,
}
/// Explicit policy for new entered keys; legacy fallback keys are preserved by masks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyStorage {
    /// Store new keys in the actual native vault, omitting them from TOML.
    Vault,
    /// Explicit opt-in to the prescribed plaintext TOML fallback.
    Plaintext,
}
/// Native storage with a concrete path and injectable credential operations.
pub struct ConfigStore {
    path: PathBuf,
    credentials: Arc<dyn CredentialProvider>,
    policy: KeyStorage,
}
impl ConfigStore {
    /// Set/delete one actual named vault entry without persisting a raw fallback key.
    pub fn set_secret(&self, profile: &str, key: Option<&str>) -> Result<(), ConfigError> {
        self.credentials
            .store(profile, key)
            .map_err(ConfigError::from)
    }
    /// Bind a concrete path/provider/policy without reading files or credentials yet.
    pub fn new(
        path: impl Into<PathBuf>,
        credentials: Arc<dyn CredentialProvider>,
        policy: KeyStorage,
    ) -> Self {
        Self {
            path: path.into(),
            credentials,
            policy,
        }
    }
    /// Locate the actual application config path; no file or vault read occurs here.
    pub fn system_default() -> Result<Self, ConfigError> {
        let dirs = directories::ProjectDirs::from("org", "openmath", "OpenMath")
            .ok_or(ConfigError::Directory)?;
        Ok(Self::new(
            dirs.config_dir().join("config.toml"),
            Arc::new(NativeCredentials),
            KeyStorage::Vault,
        ))
    }
    /// Concrete destination used by this store.
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Read actual TOML/defaults, rejecting corrupt masks and invalid profiles atomically.
    pub fn load(&self) -> Result<KernelConfig, ConfigError> {
        let config = file::read(&self.path)?;
        validate(&config, false)?;
        Ok(config)
    }
    /// Persist a trusted config without accepting masked credentials as literal keys.
    pub fn save(&self, config: &KernelConfig) -> Result<(), ConfigError> {
        validate(config, false)?;
        let current = self.load()?;
        let mut submitted = config.clone();
        for profile in &mut submitted.llm.profiles {
            if current
                .llm
                .profiles
                .iter()
                .any(|old| old.name == profile.name && old.api_key == profile.api_key)
            {
                profile.api_key = Some("***".into());
            }
        }
        self.submit(&submitted, &current).map(|_| ())
    }
    /// Commit frontend mask/null/value intents by profile name before installing runtime state.
    pub fn submit(
        &self,
        submitted: &KernelConfig,
        current: &KernelConfig,
    ) -> Result<KernelConfig, ConfigError> {
        validate(submitted, true)?;
        validate(current, false)?;
        let mut config = submitted.clone();
        let mut changes = BTreeMap::new();
        for profile in &mut config.llm.profiles {
            let old = current.llm.profiles.iter().find(|p| p.name == profile.name);
            match profile.api_key.as_deref() {
                Some("***") => profile.api_key = old.and_then(|p| p.api_key.clone()),
                Some(_) => {
                    if self.policy == KeyStorage::Vault {
                        changes.insert(profile.name.clone(), profile.api_key.take());
                    } else {
                        changes.insert(profile.name.clone(), None);
                    }
                }
                None => {
                    changes.insert(profile.name.clone(), None);
                }
            }
        }
        for old in &current.llm.profiles {
            if !config.llm.profiles.iter().any(|p| p.name == old.name) {
                changes.insert(old.name.clone(), None);
            }
        }
        let text = file::encode(&config)?;
        let mut updates = vec![];
        for (name, key) in changes {
            let old = match self.credentials.load(&name) {
                Ok(old) => old,
                Err(CredentialError::Unavailable) if key.is_none() => None,
                Err(e) => return Err(e.into()),
            };
            if old != key {
                updates.push((name, old, key));
            }
        }
        let prepared = file::PreparedFile::prepare(&self.path, &text)?;
        for (applied, (name, _, key)) in updates.iter().enumerate() {
            if let Err(error) = self.credentials.store(name, key.as_deref()) {
                return self.rollback(&updates[..=applied], error.into());
            }
        }
        if let Err(error) = prepared.commit() {
            return self.rollback(&updates, error);
        }
        Ok(config)
    }
    fn rollback<T>(
        &self,
        updates: &[(String, Option<String>, Option<String>)],
        error: ConfigError,
    ) -> Result<T, ConfigError> {
        let mut failed = false;
        for (name, old, _) in updates.iter().rev() {
            if self.credentials.load(name).is_ok_and(|value| value == *old) {
                continue;
            }
            failed |= self.credentials.store(name, old.as_deref()).is_err();
        }
        Err(if failed { ConfigError::Rollback } else { error })
    }
    /// Resolve real credentials in the prescribed environment/vault/fallback order.
    pub fn resolve_key(&self, profile: &ProfileConfig) -> Result<Option<SecretKey>, ConfigError> {
        credentials::resolve(profile, self.credentials.as_ref())
    }
    /// Represent actual credential presence for wire masking without persisting resolved keys.
    pub fn presentation(&self, config: &KernelConfig) -> Result<KernelConfig, ConfigError> {
        let mut result = config.clone();
        for p in &mut result.llm.profiles {
            p.api_key = self.resolve_key(p)?.map(|_| "***".into());
        }
        Ok(result)
    }
}
impl std::fmt::Debug for ConfigStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigStore")
            .field("path", &self.path)
            .field("policy", &self.policy)
            .finish_non_exhaustive()
    }
}
fn validate(config: &KernelConfig, masks: bool) -> Result<(), ConfigError> {
    let mut names = BTreeSet::new();
    for p in &config.llm.profiles {
        if p.name.is_empty() || !names.insert(&p.name) {
            return Err(ConfigError::Profiles);
        }
        if !p.temperature.is_finite() {
            return Err(ConfigError::Temperature);
        }
        if !masks && p.api_key.as_deref() == Some("***") {
            return Err(ConfigError::StoredMask);
        }
    }
    Ok(())
}
