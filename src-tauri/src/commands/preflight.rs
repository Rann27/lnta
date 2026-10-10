//! Preflight verification commands per FR-PRJ-08 and Story A2.

use std::path::Path;

use crate::error::AppError;
use crate::project::preflight::{run_preflight_on_folder, PreflightReport};
use crate::project::settings::GlobalSettings;

#[tauri::command]
pub fn run_preflight(path: String) -> Result<PreflightReport, AppError> {
    let p = Path::new(&path);
    if !p.exists() || !p.is_dir() {
        return Err(AppError::FileNotFound {
            path: format!("Directory not found: {path}"),
        });
    }

    let global_settings = GlobalSettings::load_from_app_data().unwrap_or_default();
    let report = run_preflight_on_folder(p, &global_settings);
    Ok(report)
}
