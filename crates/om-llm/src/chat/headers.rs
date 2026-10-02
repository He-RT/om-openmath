//! Case-insensitive configured header merge, with target-specific Anthropic behavior.
use super::{LlmError, Profile, Target};
use crate::ProviderKind;
use std::collections::BTreeMap;
pub(crate) fn build(p: &Profile, target: Target) -> Result<Vec<(String, String)>, LlmError> {
    let mut headers: BTreeMap<String, (String, String)> = BTreeMap::new();
    let mut put = |name: &str, value: String| {
        headers.insert(name.to_ascii_lowercase(), (name.into(), value));
    };
    put("Content-Type", "application/json".into());
    if p.kind == ProviderKind::Anthropic {
        put("anthropic-version", "2023-06-01".into());
    }
    if let Some(key) = p.api_key.as_ref().filter(|k| !k.is_empty()) {
        if key == "***" {
            return Err(LlmError::Profile("api_key mask"));
        }
        if p.kind == ProviderKind::Anthropic {
            put("x-api-key", key.clone());
        } else {
            put("Authorization", format!("Bearer {key}"));
        }
    }
    for (name, value) in &p.extra_headers {
        put(name, value.clone());
    }
    if p.kind == ProviderKind::Anthropic {
        let name = "anthropic-dangerous-direct-browser-access";
        if target == Target::Browser {
            headers.insert(name.into(), (name.into(), "true".into()));
        } else {
            headers.remove(name);
        }
    }
    if headers.values().any(|(name, value)| {
        name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
            || value.bytes().any(|b| b != b'\t' && (b < 0x20 || b == 0x7f))
    }) {
        return Err(LlmError::Header);
    }
    Ok(headers.into_values().collect())
}
