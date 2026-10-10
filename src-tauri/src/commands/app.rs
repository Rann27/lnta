//! Application info command per FR-SET-03.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;

/// Basic metadata about the running application.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/AppInfo.ts")]
pub struct AppInfo {
    pub version: String,
    pub license: String,
    pub repo_url: String,
}

#[tauri::command]
pub fn get_app_info() -> Result<AppInfo, AppError> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        license: "MIT + Commons Clause".to_string(),
        repo_url: "https://github.com/Rann27/lnta".to_string(),
    })
}
