//! OS credential access is isolated and never exposes provider error payloads.
use super::ConfigError;
use crate::config::ProfileConfig;

/// A credential lookup/update failure with no secret-bearing backend details.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CredentialError {
    /// No supported/available credential store.
    #[error("credential store is unavailable")]
    Unavailable,
    /// Access or another backend operation failed.
    #[error("credential access failed")]
    Access,
    /// A configured value was not Unicode text.
    #[error("credential value is not Unicode text")]
    Encoding,
}
/// Host-injectable environment and named vault operations; tests use synthetic providers.
pub trait CredentialProvider: Send + Sync {
    /// Read an explicitly configured environment variable without logging its value.
    fn environment(&self, name: &str) -> Result<Option<String>, CredentialError>;
    /// Read the openmath service entry for this profile name.
    fn load(&self, profile: &str) -> Result<Option<String>, CredentialError>;
    /// Set Some (including empty), or delete None, for this profile entry.
    fn store(&self, profile: &str, key: Option<&str>) -> Result<(), CredentialError>;
}
/// The platform's actual process environment and keyring v1 store.
pub struct NativeCredentials;
fn error(error: keyring::Error) -> CredentialError {
    match error {
        keyring::Error::NoDefaultStore => CredentialError::Unavailable,
        keyring::Error::BadEncoding(_) => CredentialError::Encoding,
        _ => CredentialError::Access,
    }
}
impl CredentialProvider for NativeCredentials {
    fn environment(&self, name: &str) -> Result<Option<String>, CredentialError> {
        match std::env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => Err(CredentialError::Encoding),
        }
    }
    fn load(&self, profile: &str) -> Result<Option<String>, CredentialError> {
        match keyring::Entry::new("openmath", profile)
            .map_err(error)?
            .get_password()
        {
            Ok(key) => Ok(Some(key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(error(e)),
        }
    }
    fn store(&self, profile: &str, key: Option<&str>) -> Result<(), CredentialError> {
        let entry = keyring::Entry::new("openmath", profile).map_err(error)?;
        if let Some(key) = key {
            entry.set_password(key).map_err(error)
        } else {
            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(e) => Err(error(e)),
            }
        }
    }
}
/// A runtime-only credential with redacted formatting and no Serialize implementation.
#[derive(Clone)]
pub struct SecretKey(pub(crate) String);
impl SecretKey {
    /// Actual value for an explicitly requested transport operation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// Consume the runtime credential when constructing an actual provider request.
    pub fn into_string(self) -> String {
        self.0
    }
}
impl std::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretKey(\"***\")")
    }
}
pub(super) fn resolve(
    profile: &ProfileConfig,
    provider: &dyn CredentialProvider,
) -> Result<Option<SecretKey>, ConfigError> {
    if let Some(name) = &profile.api_key_env
        && let Some(value) = provider.environment(name)?
    {
        return Ok(Some(SecretKey(value)));
    }
    match provider.load(&profile.name) {
        Ok(Some(value)) => return Ok(Some(SecretKey(value))),
        Err(CredentialError::Unavailable) | Ok(None) => {}
        Err(e) => return Err(e.into()),
    }
    if profile.api_key.as_deref() == Some("***") {
        return Err(ConfigError::StoredMask);
    }
    Ok(profile.api_key.clone().map(SecretKey))
}
