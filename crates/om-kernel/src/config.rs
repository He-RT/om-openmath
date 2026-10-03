//! Shared configuration schema and pure API-key masking support.
use om_llm::ProviderKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;
/// Preferred UI and message language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
#[serde(rename_all = "snake_case")]
pub enum Language {
    /// Follow the host language.
    Auto,
    /// Simplified Chinese.
    #[serde(rename = "zh-CN")]
    ZhCn,
    /// English.
    En,
}

/// ConfigDialect values accepted by the wire protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
#[serde(rename_all = "snake_case")]
pub enum ConfigDialect {
    /// Auto.
    Auto,
    /// Modern.
    Modern,
    /// Wolfram.
    Wolfram,
}

/// Constants values accepted by the wire protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
#[serde(rename_all = "snake_case")]
pub enum Constants {
    /// Math.
    Math,
    /// Strict.
    Strict,
}

/// Kernel configuration shared by native settings and the JSON protocol.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
#[serde(default)]
pub struct KernelConfig {
    /// Terminal-only preferences, absent from legacy/default configuration.
    #[serde(default, skip_serializing_if = "CliConfig::is_default")]
    #[ts(optional, as = "Option<CliConfig>")]
    pub cli: CliConfig,
    /// Evaluation and presentation settings.
    pub general: GeneralConfig,
    /// Profile routing and provider settings.
    pub llm: LlmConfig,
}
/// Nonblocking terminal AI history hints are explicitly opt-in.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
#[serde(default)]
pub struct CliConfig {
    /// Allow a background FIM cache instead of history hints after first-use confirmation.
    pub ai_hints: bool,
}
impl CliConfig {
    fn is_default(&self) -> bool {
        !self.ai_hints
    }
}

/// General settings from PLAN section 10.8.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
#[serde(default)]
pub struct GeneralConfig {
    /// Preferred language.
    pub language: Language,
    /// Default input dialect.
    pub dialect: ConfigDialect,
    /// Treatment of e and i aliases.
    pub constants: Constants,
    /// Enable dependency-based notebook semantics.
    pub reactive: bool,
    /// Recompute affected cells.
    pub auto_run_dependents: bool,
    /// Record and display solver steps.
    pub show_steps: bool,
    /// Offer supported solver visualizations.
    pub auto_plot: bool,
    /// Evaluation deadline in milliseconds.
    #[ts(type = "number")]
    pub eval_timeout_ms: u64,
}

/// Feature routing and configured provider profiles.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
#[serde(default)]
pub struct LlmConfig {
    /// Enable LLM features.
    pub enabled: bool,
    /// Translation profile; empty disables the feature.
    pub translate: String,
    /// Explanation profile.
    pub explain: String,
    /// Assistant profile.
    pub chat: String,
    /// Completion profile.
    pub complete: String,
    /// Error repair profile.
    pub fix: String,
    /// Allow sending notebook source context.
    pub send_context: bool,
    /// Editable named provider profiles.
    pub profiles: Vec<ProfileConfig>,
}

/// Persistable provider options; real keys never appear in protocol serialization.
#[derive(Clone, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
#[serde(default)]
pub struct ProfileConfig {
    /// UI authentication intent; false permits explicitly configured keyless remote services.
    #[serde(
        default = "requires_key_default",
        skip_serializing_if = "requires_key_is_default"
    )]
    #[ts(optional, type = "boolean")]
    pub requires_api_key: bool,
    /// Unique routing name.
    pub name: String,
    /// Provider protocol.
    pub kind: ProviderKind,
    /// User-configured endpoint base.
    pub base_url: String,
    /// Editable provider model name.
    pub model: String,
    /// Environment variable name for native key lookup.
    pub api_key_env: Option<String>,
    /// Submitted key; serialization always replaces a present value with ***.
    #[serde(with = "masked_api_key")]
    #[ts(as = "Option<String>")]
    pub api_key: Option<String>,
    /// Sampling temperature.
    pub temperature: f32,
    /// Output token limit.
    pub max_tokens: u32,
    /// Explicit tool capability.
    pub supports_tools: bool,
    /// Explicit JSON mode capability.
    pub supports_json_mode: bool,
    /// HTTP deadline in milliseconds.
    #[ts(type = "number")]
    pub timeout_ms: u64,
    /// Additional configured headers.
    pub extra_headers: BTreeMap<String, String>,
    /// Optional explicit provider parameters, omitted when unused.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[ts(optional, type = "Record<string, unknown>")]
    pub extra_body: BTreeMap<String, serde_json::Value>,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            language: Language::Auto,
            dialect: ConfigDialect::Auto,
            constants: Constants::Math,
            reactive: true,
            auto_run_dependents: true,
            show_steps: true,
            auto_plot: true,
            eval_timeout_ms: 30_000,
        }
    }
}
fn requires_key_default() -> bool {
    true
}
fn requires_key_is_default(value: &bool) -> bool {
    *value
}
impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            requires_api_key: true,
            name: String::new(),
            kind: ProviderKind::OpenaiChat,
            base_url: String::new(),
            model: String::new(),
            api_key_env: None,
            api_key: None,
            temperature: 0.2,
            max_tokens: 1024,
            supports_tools: false,
            supports_json_mode: false,
            timeout_ms: 60_000,
            extra_headers: BTreeMap::new(),
            extra_body: BTreeMap::new(),
        }
    }
}
impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            translate: "deepseek".into(),
            explain: "deepseek".into(),
            chat: "deepseek".into(),
            complete: "deepseek-fim".into(),
            fix: "deepseek".into(),
            send_context: true,
            profiles: vec![
                ProfileConfig {
                    name: "deepseek".into(),
                    kind: ProviderKind::OpenaiChat,
                    base_url: "https://api.deepseek.com/v1".into(),
                    model: "deepseek-flash".into(),
                    extra_body: BTreeMap::from([(
                        "thinking".into(),
                        serde_json::json!({"type":"disabled"}),
                    )]),
                    api_key_env: Some("DEEPSEEK_API_KEY".into()),
                    supports_tools: true,
                    supports_json_mode: true,
                    ..ProfileConfig::default()
                },
                ProfileConfig {
                    name: "deepseek-fim".into(),
                    kind: ProviderKind::OpenaiFim,
                    base_url: "https://api.deepseek.com/beta".into(),
                    model: "deepseek-flash".into(),
                    api_key_env: Some("DEEPSEEK_API_KEY".into()),
                    max_tokens: 64,
                    ..ProfileConfig::default()
                },
            ],
        }
    }
}
impl KernelConfig {
    /// Resolve submitted *** sentinels against current profiles by name.
    /// Missing or null keys clear the value; unmatched masked profiles have no key.
    /// Call before installing a frontend-submitted configuration in a Session.
    pub fn merge_redacted_keys(&mut self, current: &Self) {
        for profile in &mut self.llm.profiles {
            if profile.api_key.as_deref() == Some("***") {
                profile.api_key = current
                    .llm
                    .profiles
                    .iter()
                    .find(|old| old.name == profile.name)
                    .and_then(|old| old.api_key.clone());
            }
        }
    }
}
mod masked_api_key {
    use serde::{Deserialize, Serialize};

    pub fn serialize<S: serde::Serializer>(
        key: &Option<String>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        key.as_ref().map(|_| "***").serialize(serializer)
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<String>, D::Error> {
        Option::<String>::deserialize(deserializer)
    }
}
impl std::fmt::Debug for ProfileConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProfileConfig")
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("api_key_env", &self.api_key_env)
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .field("temperature", &self.temperature)
            .field("max_tokens", &self.max_tokens)
            .field("supports_tools", &self.supports_tools)
            .field("supports_json_mode", &self.supports_json_mode)
            .field("timeout_ms", &self.timeout_ms)
            .field(
                "extra_headers",
                &self.extra_headers.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}
