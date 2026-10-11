use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use ts_rs::TS;

use crate::error::AppError;

/// Project configuration stored in `.lnta/project.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../src/lib/bindings/ProjectConfig.ts")]
pub struct ProjectConfig {
    pub profile: String,
    pub target_language: String,
    pub instruction_file: Option<String>,
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
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            profile: "ja-id".to_string(),
            target_language: "id".to_string(),
            instruction_file: None,
            chunk_threshold_lines: 1500,
            context_k_segments: 6,
            send_full_glossary: false,
            title_source: "first_heading_or_filename".to_string(),
            strip_title_brackets: true,
            normalize_ruby: true,
            template_treatment: "copy".to_string(),
            budget_token_limit: None,
            budget_cost_limit: None,
        }
    }
}

impl ProjectConfig {
    pub fn for_profile(profile: &str) -> Self {
        let target_language = if profile == "ja-en" { "en" } else { "id" };
        Self {
            profile: profile.to_string(),
            target_language: target_language.to_string(),
            ..Default::default()
        }
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, AppError> {
        let content = fs::read_to_string(&path).map_err(|e| AppError::FileNotFound {
            path: format!("{}: {e}", path.as_ref().display()),
        })?;
        toml::from_str(&content).map_err(|e| AppError::Internal {
            message: format!("Failed to parse project.toml: {e}"),
        })
    }

    /// Atomically writes config to file (write to temp file then rename).
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), AppError> {
        let path_ref = path.as_ref();
        let parent = path_ref.parent().ok_or_else(|| AppError::Internal {
            message: "Project config path has no parent directory".to_string(),
        })?;

        let serialized = toml::to_string_pretty(self).map_err(|e| AppError::Internal {
            message: format!("Failed to serialize project config: {e}"),
        })?;

        let temp_path = parent.join(format!(".project_toml_tmp_{}", uuid::Uuid::new_v4()));
        fs::write(&temp_path, serialized).map_err(|e| AppError::PermissionDenied {
            path: format!("{}: {e}", temp_path.display()),
        })?;

        fs::rename(&temp_path, path_ref).map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            AppError::PermissionDenied {
                path: format!("{}: {e}", path_ref.display()),
            }
        })?;

        Ok(())
    }
}
