//! Global settings, roles, fallbacks, and project override merging per FR-SET-01, FR-SET-02, and Story G3.
//!
//! Stores global user settings in the OS app-data directory, manages role model assignments
//! with primary and ordered fallbacks, and computes effective project settings by merging
//! project-level overrides over global settings.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;
use crate::project::config::ProjectConfig;
use crate::provider::registry::ModelEntry;

/// The four roles in the LNTA translation and QA pipeline per PRD §4.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../src/lib/bindings/Role.ts")]
pub enum Role {
    Translator,
    Repair,
    QaReviewer,
    QaFixer,
}

impl Role {
    pub const ALL: [Role; 4] = [
        Role::Translator,
        Role::Repair,
        Role::QaReviewer,
        Role::QaFixer,
    ];
}

/// A reference to a provider and model pair.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/ModelReference.ts")]
pub struct ModelReference {
    pub provider: String,
    pub model_id: String,
}

impl ModelReference {
    pub fn new(provider: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model_id: model_id.into(),
        }
    }
}

/// Configuration for a specific pipeline role per FR-PRV-11 and Story G3.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/RoleConfig.ts")]
pub struct RoleConfig {
    pub primary: ModelReference,
    pub fallbacks: Vec<ModelReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
}

impl RoleConfig {
    pub fn new(primary: ModelReference) -> Self {
        Self {
            primary,
            fallbacks: Vec::new(),
            temperature: None,
            max_output_tokens: None,
            reasoning_effort: None,
        }
    }

    pub fn with_fallbacks(mut self, fallbacks: Vec<ModelReference>) -> Self {
        self.fallbacks = fallbacks;
        self
    }
}

/// Role configurations for all four pipeline roles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/RolesConfig.ts")]
pub struct RolesConfig {
    pub translator: RoleConfig,
    pub repair: RoleConfig,
    pub qa_reviewer: RoleConfig,
    pub qa_fixer: RoleConfig,
}

impl Default for RolesConfig {
    fn default() -> Self {
        let default_primary = ModelReference::new("anthropic", "claude-sonnet-5-5");
        let default_fallback = ModelReference::new("gemini", "gemini-3.8-flash");

        let role_cfg = RoleConfig {
            primary: default_primary,
            fallbacks: vec![default_fallback],
            temperature: None,
            max_output_tokens: None,
            reasoning_effort: None,
        };

        Self {
            translator: role_cfg.clone(),
            repair: role_cfg.clone(),
            qa_reviewer: role_cfg.clone(),
            qa_fixer: role_cfg,
        }
    }
}

impl RolesConfig {
    /// Returns the configuration for a given role.
    pub fn for_role(&self, role: Role) -> &RoleConfig {
        match role {
            Role::Translator => &self.translator,
            Role::Repair => &self.repair,
            Role::QaReviewer => &self.qa_reviewer,
            Role::QaFixer => &self.qa_fixer,
        }
    }

    /// Returns a mutable reference to the configuration for a given role.
    pub fn for_role_mut(&mut self, role: Role) -> &mut RoleConfig {
        match role {
            Role::Translator => &mut self.translator,
            Role::Repair => &mut self.repair,
            Role::QaReviewer => &mut self.qa_reviewer,
            Role::QaFixer => &mut self.qa_fixer,
        }
    }
}

/// Provider configuration entry in global settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/ProviderConfigEntry.ts")]
pub struct ProviderConfigEntry {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub active: bool,
}

/// Global settings stored in the OS app-data directory per FR-SET-01.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/GlobalSettings.ts")]
pub struct GlobalSettings {
    pub theme: String,
    pub ui_language: String,
    pub notifications_enabled: bool,
    pub roles: RolesConfig,
    pub providers: Vec<ProviderConfigEntry>,
    pub custom_models: Vec<ModelEntry>,
    #[serde(default)]
    pub recent_projects: Vec<String>,
}

/// A provider LNTA knows out of the box: (id, display name, type, base URL).
/// OpenAI-compatible APIs share one adapter; the base URL picks the service.
const BUILTIN_PROVIDERS: &[(&str, &str, &str, &str)] = &[
    (
        "anthropic",
        "Anthropic (Claude)",
        "anthropic",
        "https://api.anthropic.com",
    ),
    (
        "openai",
        "OpenAI (GPT)",
        "openai_compat",
        "https://api.openai.com/v1",
    ),
    (
        "gemini",
        "Google (Gemini)",
        "gemini",
        "https://generativelanguage.googleapis.com",
    ),
    ("xai", "xAI (Grok)", "openai_compat", "https://api.x.ai/v1"),
    (
        "deepseek",
        "DeepSeek",
        "openai_compat",
        "https://api.deepseek.com",
    ),
    (
        "zai",
        "Z.ai (GLM)",
        "openai_compat",
        "https://api.z.ai/api/paas/v4",
    ),
    (
        "qwen",
        "Alibaba (Qwen)",
        "openai_compat",
        "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
    ),
    (
        "minimax",
        "MiniMax",
        "openai_compat",
        "https://api.minimax.io/v1",
    ),
    (
        "xiaomi",
        "Xiaomi (MiMo)",
        "openai_compat",
        "https://api.xiaomimimo.com/v1",
    ),
    (
        "moonshot",
        "Moonshot (Kimi)",
        "openai_compat",
        "https://api.moonshot.ai/v1",
    ),
    (
        "mistral",
        "Mistral",
        "openai_compat",
        "https://api.mistral.ai/v1",
    ),
    (
        "openrouter",
        "OpenRouter (gateway)",
        "openai_compat",
        "https://openrouter.ai/api/v1",
    ),
    (
        "cli_claude",
        "Claude Code CLI (langganan Claude)",
        "cli_claude",
        "local://claude",
    ),
    (
        "cli_codex",
        "Codex CLI (langganan ChatGPT)",
        "cli_codex",
        "local://codex",
    ),
    (
        "cli_antigravity",
        "Antigravity CLI (langganan Google AI)",
        "cli_antigravity",
        "local://agy",
    ),
];

/// Built-in providers in catalogue order.
pub fn builtin_providers() -> Vec<ProviderConfigEntry> {
    BUILTIN_PROVIDERS
        .iter()
        .map(|(id, name, provider_type, base_url)| ProviderConfigEntry {
            id: id.to_string(),
            name: name.to_string(),
            provider_type: provider_type.to_string(),
            base_url: base_url.to_string(),
            active: true,
        })
        .collect()
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            ui_language: "id".to_string(),
            notifications_enabled: true,
            roles: RolesConfig::default(),
            recent_projects: Vec::new(),
            providers: builtin_providers(),
            custom_models: Vec::new(),
        }
    }
}

impl GlobalSettings {
    /// Loads global settings from a specific JSON file path.
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self, AppError> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path_ref).map_err(|e| AppError::Internal {
            message: format!("Failed to read settings from {}: {e}", path_ref.display()),
        })?;
        let mut settings: Self =
            serde_json::from_str(&content).map_err(|e| AppError::Internal {
                message: format!("Failed to parse global settings: {e}"),
            })?;
        if settings.rename_legacy_openai_entry() {
            // The key follows the entry; a failure only means re-entering it once.
            if let Ok(Some(key)) = crate::secrets::get_api_key("openai_compat") {
                if crate::secrets::set_api_key("openai", &key).is_ok() {
                    let _ = crate::secrets::delete_api_key("openai_compat");
                }
            }
        }
        settings.add_missing_builtin_providers();
        Ok(settings)
    }

    /// Older settings named the OpenAI entry `openai_compat`. Renames it to `openai`, along
    /// with role and custom-model references, so a saved choice keeps working.
    /// Returns true when it renamed something (the caller moves the stored key).
    pub fn rename_legacy_openai_entry(&mut self) -> bool {
        let is_legacy = |p: &ProviderConfigEntry| {
            p.id == "openai_compat"
                && p.base_url.trim_end_matches('/') == "https://api.openai.com/v1"
        };
        if !self.providers.iter().any(is_legacy) || self.providers.iter().any(|p| p.id == "openai")
        {
            return false;
        }
        for p in self.providers.iter_mut().filter(|p| is_legacy(p)) {
            p.id = "openai".to_string();
            p.name = "OpenAI (GPT)".to_string();
        }
        for role in Role::ALL {
            let cfg = self.roles.for_role_mut(role);
            for model in std::iter::once(&mut cfg.primary).chain(cfg.fallbacks.iter_mut()) {
                if model.provider == "openai_compat" {
                    model.provider = "openai".to_string();
                }
            }
        }
        for model in &mut self.custom_models {
            if model.provider == "openai_compat" {
                model.provider = "openai".to_string();
            }
        }
        true
    }

    /// Adds built-in providers that an older settings file does not list yet, and retires
    /// entries that can no longer work. User-added and edited entries are kept as they are.
    pub fn add_missing_builtin_providers(&mut self) {
        // Gemini CLI stopped serving personal Google AI subscriptions on 2026-06-18;
        // Antigravity CLI replaces it.
        self.providers.retain(|p| p.provider_type != "cli_gemini");
        for builtin in builtin_providers() {
            if !self.providers.iter().any(|p| p.id == builtin.id) {
                self.providers.push(builtin);
            }
        }
    }

    /// Saves global settings atomically to a specific JSON file path.
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<(), AppError> {
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            fs::create_dir_all(parent).map_err(|e| AppError::Internal {
                message: format!(
                    "Failed to create settings directory {}: {e}",
                    parent.display()
                ),
            })?;
        }

        let json = serde_json::to_string_pretty(self).map_err(|e| AppError::Internal {
            message: format!("Failed to serialize global settings: {e}"),
        })?;

        let temp_path = path_ref.with_extension("tmp");
        fs::write(&temp_path, json).map_err(|e| AppError::Internal {
            message: format!("Failed to write temp settings {}: {e}", temp_path.display()),
        })?;

        fs::rename(&temp_path, path_ref).map_err(|e| AppError::Internal {
            message: format!("Failed to replace settings {}: {e}", path_ref.display()),
        })
    }

    /// Returns the OS-appropriate app-data settings path for LNTA.
    pub fn app_data_settings_path() -> Result<PathBuf, AppError> {
        let base_dir = if let Ok(app_data) = std::env::var("APPDATA") {
            PathBuf::from(app_data)
        } else if let Ok(user_profile) = std::env::var("USERPROFILE") {
            PathBuf::from(user_profile).join("AppData").join("Roaming")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config")
        } else {
            return Err(AppError::Internal {
                message: "Unable to determine OS AppData directory".to_string(),
            });
        };

        Ok(base_dir.join("lnta").join("settings.json"))
    }

    /// Loads global settings from the OS app-data directory.
    pub fn load_from_app_data() -> Result<Self, AppError> {
        let path = Self::app_data_settings_path()?;
        Self::load_from_path(path)
    }

    /// Saves global settings to the OS app-data directory.
    pub fn save_to_app_data(&self) -> Result<(), AppError> {
        let path = Self::app_data_settings_path()?;
        self.save_to_path(path)
    }
}

/// Optional project-level overrides per FR-SET-02.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProjectSettingsOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<RolesConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_threshold_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_k_segments: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_full_glossary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strip_title_brackets: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalize_ruby: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_treatment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_token_limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_cost_limit: Option<f64>,
}

/// Computed effective settings for an active project run, combining global settings
/// with project-level configuration and overrides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/EffectiveSettings.ts")]
pub struct EffectiveSettings {
    pub profile: String,
    pub target_language: String,
    pub instruction_file: Option<String>,
    pub roles: RolesConfig,
    pub chunk_threshold_lines: usize,
    pub context_k_segments: usize,
    pub send_full_glossary: bool,
    pub title_source: String,
    pub strip_title_brackets: bool,
    pub normalize_ruby: bool,
    pub template_treatment: String,
    #[ts(type = "number | null")]
    pub budget_token_limit: Option<u64>,
    pub budget_cost_limit: Option<f64>,
    pub theme: String,
    pub ui_language: String,
    pub notifications_enabled: bool,
}

impl EffectiveSettings {
    /// Merges global settings and project configuration, with project settings taking precedence.
    pub fn merge(
        global: &GlobalSettings,
        project: &ProjectConfig,
        overrides: Option<&ProjectSettingsOverrides>,
    ) -> Self {
        let roles = overrides
            .and_then(|o| o.roles.clone())
            .unwrap_or_else(|| global.roles.clone());

        let chunk_threshold_lines = overrides
            .and_then(|o| o.chunk_threshold_lines)
            .unwrap_or(project.chunk_threshold_lines);

        let context_k_segments = overrides
            .and_then(|o| o.context_k_segments)
            .unwrap_or(project.context_k_segments);

        let send_full_glossary = overrides
            .and_then(|o| o.send_full_glossary)
            .unwrap_or(project.send_full_glossary);

        let title_source = overrides
            .and_then(|o| o.title_source.clone())
            .unwrap_or_else(|| project.title_source.clone());

        let strip_title_brackets = overrides
            .and_then(|o| o.strip_title_brackets)
            .unwrap_or(project.strip_title_brackets);

        let normalize_ruby = overrides
            .and_then(|o| o.normalize_ruby)
            .unwrap_or(project.normalize_ruby);

        let template_treatment = overrides
            .and_then(|o| o.template_treatment.clone())
            .unwrap_or_else(|| project.template_treatment.clone());

        let budget_token_limit = overrides
            .and_then(|o| o.budget_token_limit)
            .or(project.budget_token_limit);

        let budget_cost_limit = overrides
            .and_then(|o| o.budget_cost_limit)
            .or(project.budget_cost_limit);

        Self {
            profile: project.profile.clone(),
            target_language: project.target_language.clone(),
            instruction_file: project.instruction_file.clone(),
            roles,
            chunk_threshold_lines,
            context_k_segments,
            send_full_glossary,
            title_source,
            strip_title_brackets,
            normalize_ruby,
            template_treatment,
            budget_token_limit,
            budget_cost_limit,
            theme: global.theme.clone(),
            ui_language: global.ui_language.clone(),
            notifications_enabled: global.notifications_enabled,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_roles_config_defaults_cover_four_roles() {
        let roles = RolesConfig::default();
        for role in Role::ALL {
            let cfg = roles.for_role(role);
            assert_eq!(cfg.primary.provider, "anthropic");
            assert_eq!(cfg.primary.model_id, "claude-sonnet-5-5");
            assert_eq!(cfg.fallbacks.len(), 1);
            assert_eq!(cfg.fallbacks[0].provider, "gemini");
            assert_eq!(cfg.fallbacks[0].model_id, "gemini-3.8-flash");
        }
    }

    #[test]
    fn test_default_providers_cover_the_catalogue() {
        let ids: Vec<String> = GlobalSettings::default()
            .providers
            .into_iter()
            .map(|p| p.id)
            .collect();
        for id in [
            "anthropic",
            "openai",
            "gemini",
            "xai",
            "deepseek",
            "zai",
            "qwen",
            "minimax",
            "xiaomi",
            "moonshot",
            "openrouter",
            "cli_claude",
            "cli_codex",
            "cli_antigravity",
        ] {
            assert!(ids.iter().any(|i| i == id), "{id}");
        }
    }

    #[test]
    fn test_old_settings_gain_new_providers_and_lose_dead_ones() {
        let mut settings = GlobalSettings {
            providers: vec![
                ProviderConfigEntry {
                    id: "openai_compat".to_string(),
                    name: "OpenAI Compatible".to_string(),
                    provider_type: "openai_compat".to_string(),
                    base_url: "https://api.openai.com/v1".to_string(),
                    active: true,
                },
                ProviderConfigEntry {
                    id: "my_gemini_cli".to_string(),
                    name: "Gemini CLI".to_string(),
                    provider_type: "cli_gemini".to_string(),
                    base_url: "local://gemini".to_string(),
                    active: true,
                },
                ProviderConfigEntry {
                    id: "local_llm".to_string(),
                    name: "LM Studio".to_string(),
                    provider_type: "openai_compat".to_string(),
                    base_url: "http://127.0.0.1:1234/v1".to_string(),
                    active: true,
                },
            ],
            ..GlobalSettings::default()
        };
        settings.roles.translator.primary = ModelReference::new("openai_compat", "gpt-6.1-sol");
        assert!(settings.rename_legacy_openai_entry());
        settings.add_missing_builtin_providers();
        let ids: Vec<&str> = settings.providers.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"local_llm"));
        assert_eq!(ids.iter().filter(|i| **i == "openai").count(), 1);
        assert!(!ids.contains(&"openai_compat"));
        assert!(!ids.contains(&"my_gemini_cli"));
        assert_eq!(settings.roles.translator.primary.provider, "openai");
        // A second pass changes nothing.
        assert!(!settings.rename_legacy_openai_entry());
    }

    #[test]
    fn test_global_settings_save_and_load_roundtrip() {
        let temp_dir =
            std::env::temp_dir().join(format!("lnta_test_settings_{}", uuid::Uuid::new_v4()));
        let settings_path = temp_dir.join("settings.json");

        let mut settings = GlobalSettings {
            theme: "dark".to_string(),
            ui_language: "en".to_string(),
            ..Default::default()
        };
        settings.roles.translator.primary = ModelReference::new("gemini", "gemini-1.5-pro");

        settings.save_to_path(&settings_path).unwrap();
        assert!(settings_path.exists());

        let loaded = GlobalSettings::load_from_path(&settings_path).unwrap();
        assert_eq!(loaded.theme, "dark");
        assert_eq!(loaded.ui_language, "en");
        assert_eq!(loaded.roles.translator.primary.provider, "gemini");
        assert_eq!(loaded.roles.translator.primary.model_id, "gemini-1.5-pro");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_effective_settings_merge_project_overrides_over_global() {
        let global = GlobalSettings {
            theme: "dark".to_string(),
            ui_language: "id".to_string(),
            notifications_enabled: true,
            roles: RolesConfig::default(),
            providers: Vec::new(),
            custom_models: Vec::new(),
            recent_projects: Vec::new(),
        };

        let mut project = ProjectConfig::for_profile("ja-id");
        project.chunk_threshold_lines = 1200;
        project.strip_title_brackets = true;
        project.normalize_ruby = true;

        // Merge with project base
        let effective1 = EffectiveSettings::merge(&global, &project, None);
        assert_eq!(effective1.theme, "dark");
        assert_eq!(effective1.ui_language, "id");
        assert_eq!(effective1.chunk_threshold_lines, 1200);
        assert!(effective1.strip_title_brackets);
        assert!(effective1.normalize_ruby);
        assert_eq!(
            effective1.roles.translator.primary.model_id,
            "claude-sonnet-5-5"
        );

        // Merge with explicit project overrides (e.g. custom roles and strip_title_brackets = false)
        let mut custom_roles = RolesConfig::default();
        custom_roles.translator.primary = ModelReference::new("openai_compat", "deepseek-v3");

        let overrides = ProjectSettingsOverrides {
            roles: Some(custom_roles),
            strip_title_brackets: Some(false),
            normalize_ruby: Some(false),
            chunk_threshold_lines: Some(2000),
            ..Default::default()
        };

        let effective2 = EffectiveSettings::merge(&global, &project, Some(&overrides));
        assert_eq!(effective2.chunk_threshold_lines, 2000);
        assert!(!effective2.strip_title_brackets);
        assert!(!effective2.normalize_ruby);
        assert_eq!(
            effective2.roles.translator.primary.provider,
            "openai_compat"
        );
        assert_eq!(effective2.roles.translator.primary.model_id, "deepseek-v3");
        assert_eq!(effective2.theme, "dark");
    }
}
