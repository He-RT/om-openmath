//! Explicit native initialization binds validated durable settings to the shared protocol.
use super::Session;
use crate::{
    native::{ConfigError, ConfigStore, SecretKey},
    protocol::*,
};
use om_num::ctx::Clock;
use std::sync::Arc;
impl Session {
    /// Load and bind an actual native store; pure Session::new remains IO-free.
    pub fn new_native(
        store: ConfigStore,
        clock: Option<Arc<dyn Clock>>,
    ) -> Result<Self, ConfigError> {
        let config = store.load()?;
        let mut session = Self::new(config, clock);
        session.config_store = Some(store);
        Ok(session)
    }
    /// Resolve a configured native routing key without exposing it through config serialization.
    pub fn resolved_profile_key(&self, name: &str) -> Result<Option<SecretKey>, ConfigError> {
        let p = self
            .config
            .llm
            .profiles
            .iter()
            .find(|p| p.name == name)
            .ok_or(ConfigError::MissingProfile)?;
        if let Some(store) = &self.config_store {
            store.resolve_key(p)
        } else if p.api_key.as_deref() == Some("***") {
            Err(ConfigError::StoredMask)
        } else {
            Ok(p.api_key.clone().map(SecretKey))
        }
    }
    pub(super) fn stored_config(&self) -> Response {
        match self
            .config_store
            .as_ref()
            .map(|store| store.presentation(&self.config))
            .transpose()
        {
            Ok(config) => Response::Config {
                config: config.unwrap_or_else(|| self.config.clone()),
            },
            Err(e) => self.config_error(&e),
        }
    }
    pub(super) fn config_error(&self, error: &ConfigError) -> Response {
        self.error(
            "err.config",
            &format!("配置操作失败：{error}"),
            &format!("Configuration operation failed: {error}"),
        )
    }
}
