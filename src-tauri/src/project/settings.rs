//! Global settings, roles, fallbacks, and project override merging per FR-SET-01, FR-SET-02, and Story G3.
//!
//! Stores global user settings in the OS app-data directory, manages role model assignments
//! with primary and ordered fallbacks, and computes effective project settings by merging
//! project-level overrides over global settings.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::project::config::ProjectConfig;
use crate::provider::registry::ModelEntry;

/// The four roles in the LNTA translation and QA pipeline per PRD §4.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RolesConfig {
    pub translator: RoleConfig,
    pub repair: RoleConfig,
    pub qa_reviewer: RoleConfig,
    pub qa_fixer: RoleConfig,
}

impl Default for RolesConfig {
    fn default() -> Self {
        let default_primary = ModelReference::new("anthropic", "claude-3-7-sonnet-20250219");
        let default_fallback = ModelReference::new("openai_compat", "gpt-4o");

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderConfigEntry {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub active: bool,
}

/// Global settings stored in the OS app-data directory per FR-SET-01.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalSettings {
    pub theme: String,
    pub ui_language: String,
    pub notifications_enabled: bool,
    pub roles: RolesConfig,
    pub providers: Vec<ProviderConfigEntry>,
    pub custom_models: Vec<ModelEntry>,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            ui_language: "id".to_string(),
            notifications_enabled: true,
            roles: RolesConfig::default(),
            providers: vec![
                ProviderConfigEntry {
                    id: "anthropic".to_string(),
                    name: "Anthropic".to_string(),
                    provider_type: "anthropic".to_string(),
                    base_url: "https://api.anthropic.com".to_string(),
                    active: true,
                },
                ProviderConfigEntry {
                    id: "gemini".to_string(),
                    name: "Google Gemini".to_string(),
                    provider_type: "gemini".to_string(),
                    base_url: "https://generativelanguage.googleapis.com".to_string(),
                    active: true,
                },
                ProviderConfigEntry {
                    id: "openai_compat".to_string(),
                    name: "OpenAI Compatible".to_string(),
                    provider_type: "openai_compat".to_string(),
                    base_url: "https://api.openai.com/v1".to_string(),
                    active: true,
                },
            ],
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
        serde_json::from_str(&content).map_err(|e| AppError::Internal {
            message: format!("Failed to parse global settings: {e}"),
        })
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
            assert_eq!(cfg.primary.model_id, "claude-3-7-sonnet-20250219");
            assert_eq!(cfg.fallbacks.len(), 1);
            assert_eq!(cfg.fallbacks[0].provider, "openai_compat");
            assert_eq!(cfg.fallbacks[0].model_id, "gpt-4o");
        }
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
            "claude-3-7-sonnet-20250219"
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
