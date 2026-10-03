//! Actual named routing and target-specific credential resolution, never prompt data.
use super::{Feature, Session};
use crate::protocol::Request;
use om_llm::Profile;
#[cfg(feature = "native")]
use om_llm::Target;
impl Session {
    pub(super) fn llm_profile(
        &self,
        request: &Request,
        feature: Feature,
    ) -> Result<Profile, String> {
        if feature != Feature::TestProfile && !self.config.llm.enabled {
            return Err(self.localized("AI 功能已关闭", "AI features are disabled"));
        }
        let name = match request {
            Request::LlmTestProfile { profile, .. } => profile,
            _ => match feature {
                Feature::Translate => &self.config.llm.translate,
                Feature::Explain => &self.config.llm.explain,
                Feature::Complete => &self.config.llm.complete,
                Feature::Chat => &self.config.llm.chat,
                Feature::Fix => &self.config.llm.fix,
                Feature::TestProfile => return Err("Missing test profile".into()),
            },
        };
        if name.is_empty() {
            return Err(self.localized(
                "此 AI 功能未配置模型",
                "No profile configured for this AI feature",
            ));
        }
        let draft = match request {
            Request::LlmTestProfile {
                config: Some(config),
                ..
            } => Some(config),
            _ => None,
        };
        let p = if let Some(config) = draft {
            if &config.name != name {
                return Err("Draft profile name does not match".into());
            }
            config
        } else {
            let mut matches = self.config.llm.profiles.iter().filter(|p| &p.name == name);
            let p = matches.next().ok_or_else(|| {
                self.localized("找不到模型配置", "Configured profile was not found")
            })?;
            if matches.next().is_some() {
                return Err("Ambiguous profile name".into());
            }
            p
        };
        if feature == Feature::Chat && !p.supports_tools {
            return Err(self.localized(
                "助手模型需要支持工具调用",
                "Assistant profile must support tool calls",
            ));
        }
        let api_key = if draft.is_some() && p.api_key.as_deref() == Some("***") {
            let mut matches = self
                .config
                .llm
                .profiles
                .iter()
                .filter(|old| old.name == p.name);
            let old = matches
                .next()
                .ok_or("A masked draft requires an existing profile")?;
            if matches.next().is_some() {
                return Err("Ambiguous profile name".into());
            }
            old.api_key.clone()
        } else {
            p.api_key.clone()
        };
        #[cfg(feature = "native")]
        let api_key = if self.llm.target == Target::Native {
            (if draft.is_some() {
                self.resolved_draft_key(p)
            } else {
                self.resolved_profile_key(name)
            })
            .map_err(|e| e.to_string())?
            .map(|k| k.into_string())
        } else {
            api_key
        };
        if api_key.as_deref() == Some("***") {
            return Err(self.localized("需要实际 API 密钥", "An actual API key is required"));
        }
        Ok(Profile {
            name: p.name.clone(),
            kind: p.kind,
            base_url: p.base_url.clone(),
            model: p.model.clone(),
            api_key,
            temperature: p.temperature,
            max_tokens: p.max_tokens,
            supports_tools: p.supports_tools,
            supports_json_mode: p.supports_json_mode,
            timeout_ms: p.timeout_ms,
            extra_headers: p.extra_headers.clone(),
            extra_body: p.extra_body.clone(),
        })
    }
}
