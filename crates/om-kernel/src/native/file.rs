//! Trusted TOML storage and adjacent atomic replacement, separate from wire masking.
use super::ConfigError;
use crate::KernelConfig;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static SERIAL: AtomicU64 = AtomicU64::new(0);
fn io(operation: &'static str, error: std::io::Error) -> ConfigError {
    ConfigError::Io {
        operation,
        kind: error.kind(),
    }
}
pub(super) fn decode(text: &str) -> Result<KernelConfig, ConfigError> {
    toml::from_str(text).map_err(|e: toml::de::Error| ConfigError::Toml {
        offset: e.span().map(|s| s.start),
    })
}
pub(super) fn encode(config: &KernelConfig) -> Result<String, ConfigError> {
    let mut value = toml::Value::try_from(config).map_err(|_| ConfigError::Serialize)?;
    let profiles = value
        .get_mut("llm")
        .and_then(|v| v.get_mut("profiles"))
        .and_then(toml::Value::as_array_mut)
        .ok_or(ConfigError::Serialize)?;
    if profiles.len() != config.llm.profiles.len() {
        return Err(ConfigError::Serialize);
    }
    for (value, profile) in profiles.iter_mut().zip(&config.llm.profiles) {
        let table = value.as_table_mut().ok_or(ConfigError::Serialize)?;
        if let Some(key) = &profile.api_key {
            table.insert("api_key".into(), toml::Value::String(key.clone()));
        } else {
            table.remove("api_key");
        }
    }
    toml::to_string_pretty(&value).map_err(|_| ConfigError::Serialize)
}
pub(super) fn read(path: &Path) -> Result<KernelConfig, ConfigError> {
    match fs::read_to_string(path) {
        Ok(text) => decode(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(KernelConfig::default()),
        Err(e) => Err(io("read", e)),
    }
}
pub(super) struct PreparedFile {
    temporary: PathBuf,
    destination: PathBuf,
}
impl PreparedFile {
    pub fn prepare(path: &Path, text: &str) -> Result<Self, ConfigError> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|e| io("create directory", e))?;
        for _ in 0..64 {
            let temp = parent.join(format!(
                ".openmath-config-{}-{}.tmp",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let file = match options.open(&temp) {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(io("create temporary file", e)),
            };
            let prepared = Self {
                temporary: temp,
                destination: path.into(),
            };
            Self::write(file, text)?;
            return Ok(prepared);
        }
        Err(ConfigError::Io {
            operation: "create temporary file",
            kind: std::io::ErrorKind::AlreadyExists,
        })
    }
    fn write(mut file: File, text: &str) -> Result<(), ConfigError> {
        file.write_all(text.as_bytes())
            .map_err(|e| io("write", e))?;
        file.flush().map_err(|e| io("flush", e))?;
        file.sync_all().map_err(|e| io("sync", e))
    }
    pub fn commit(self) -> Result<(), ConfigError> {
        fs::rename(&self.temporary, &self.destination).map_err(|e| io("replace", e))
    }
}
impl Drop for PreparedFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.temporary);
    }
}
