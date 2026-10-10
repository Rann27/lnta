//! Global settings and project configuration commands per FR-SET-01, FR-SET-02.

use std::path::Path;

use crate::error::AppError;
use crate::project::config::ProjectConfig;
use crate::project::layout::ProjectLayout;
use crate::project::settings::GlobalSettings;

#[tauri::command]
pub fn get_global_settings() -> Result<GlobalSettings, AppError> {
    GlobalSettings::load_from_app_data()
}

#[tauri::command]
pub fn save_global_settings(settings: GlobalSettings) -> Result<(), AppError> {
    settings.save_to_app_data()
}

#[tauri::command]
pub fn get_project_config(path: String) -> Result<ProjectConfig, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    if !layout.config_file.exists() {
        return Err(AppError::FileNotFound {
            path: format!("Project config not found: {}", layout.config_file.display()),
        });
    }
    ProjectConfig::load_from_file(&layout.config_file)
}

#[tauri::command]
pub fn save_project_config(path: String, config: ProjectConfig) -> Result<(), AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    config.save_to_file(&layout.config_file)
}
