use std::fs;
use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::project::config::ProjectConfig;
use crate::store::schema::open_path;

/// Represents paths within an active LNTA project folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLayout {
    pub root: PathBuf,
    pub lnta_dir: PathBuf,
    pub config_file: PathBuf,
    pub db_file: PathBuf,
    pub template_file: PathBuf,
    pub backups_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub glossary_file: PathBuf,
    pub docx_output: PathBuf,
    pub illustrations_dir: PathBuf,
}

impl ProjectLayout {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        let root = root.as_ref().to_path_buf();
        let lnta_dir = root.join(".lnta");
        let config_file = lnta_dir.join("project.toml");
        let db_file = lnta_dir.join("lnta.db");
        let template_file = lnta_dir.join("template.docx");
        let backups_dir = lnta_dir.join("backups");
        let logs_dir = lnta_dir.join("logs");
        let glossary_file = root.join("glossary.md");
        let docx_output = root.join("Translations.docx");
        let illustrations_dir = root.join("illustrations");

        Self {
            root,
            lnta_dir,
            config_file,
            db_file,
            template_file,
            backups_dir,
            logs_dir,
            glossary_file,
            docx_output,
            illustrations_dir,
        }
    }

    /// Ensures the `.lnta/` directory structure and database exist.
    pub fn ensure_layout(&self, profile: &str) -> Result<ProjectConfig, AppError> {
        if !self.root.exists() {
            return Err(AppError::FileNotFound {
                path: self.root.display().to_string(),
            });
        }

        fs::create_dir_all(&self.lnta_dir).map_err(|e| AppError::PermissionDenied {
            path: format!("{}: {e}", self.lnta_dir.display()),
        })?;

        fs::create_dir_all(&self.backups_dir).map_err(|e| AppError::PermissionDenied {
            path: format!("{}: {e}", self.backups_dir.display()),
        })?;

        fs::create_dir_all(&self.logs_dir).map_err(|e| AppError::PermissionDenied {
            path: format!("{}: {e}", self.logs_dir.display()),
        })?;

        // Initialize SQLite store with all migrations
        let _conn = open_path(&self.db_file)?;

        // Ensure project.toml exists
        let config = if self.config_file.exists() {
            ProjectConfig::load_from_file(&self.config_file)?
        } else {
            let default_cfg = ProjectConfig::for_profile(profile);
            default_cfg.save_to_file(&self.config_file)?;
            default_cfg
        };

        // Ensure template.docx exists (copied from Translations.docx per PRD Story E1 / line 405, or minimal default)
        let needs_template = !self.template_file.exists()
            || fs::metadata(&self.template_file)
                .map(|m| m.len() == 0)
                .unwrap_or(true);
        if needs_template {
            if self.docx_output.exists() {
                let _ = fs::copy(&self.docx_output, &self.template_file);
            } else if let Ok(default_bytes) =
                crate::docx::generate_default_template("Heading1").to_bytes()
            {
                let _ = fs::write(&self.template_file, default_bytes);
            }
        }

        Ok(config)
    }

    /// Resets the project by removing `.lnta/` while leaving user files intact (FR-PRJ-10).
    pub fn reset_project(&self) -> Result<(), AppError> {
        if self.lnta_dir.exists() {
            fs::remove_dir_all(&self.lnta_dir).map_err(|e| AppError::PermissionDenied {
                path: format!("{}: {e}", self.lnta_dir.display()),
            })?;
        }
        Ok(())
    }
}
