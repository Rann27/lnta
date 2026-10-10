//! Provider and API key management commands per FR-PRV-06, FR-SET-01, Story G1.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;
use crate::project::settings::{GlobalSettings, ProviderConfigEntry};
use crate::provider::registry::fetch_models_from_provider;
use crate::secrets;

/// Result of testing a provider connection per Story G1.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/ProviderTestResult.ts")]
pub struct ProviderTestResult {
    pub success: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
}

#[tauri::command]
pub fn list_providers() -> Result<Vec<ProviderConfigEntry>, AppError> {
    let settings = GlobalSettings::load_from_app_data().unwrap_or_default();
    Ok(settings.providers)
}

#[tauri::command]
pub fn save_provider(entry: ProviderConfigEntry) -> Result<(), AppError> {
    let mut settings = GlobalSettings::load_from_app_data().unwrap_or_default();
    if let Some(existing) = settings.providers.iter_mut().find(|p| p.id == entry.id) {
        *existing = entry;
    } else {
        settings.providers.push(entry);
    }
    settings.save_to_app_data()
}

#[tauri::command]
pub fn delete_provider(id: String) -> Result<(), AppError> {
    let mut settings = GlobalSettings::load_from_app_data().unwrap_or_default();
    settings.providers.retain(|p| p.id != id);
    let _ = secrets::delete_api_key(&id);
    settings.save_to_app_data()
}

#[tauri::command]
pub fn set_api_key(provider_id: String, key: String) -> Result<(), AppError> {
    secrets::set_api_key(&provider_id, &key)
}

#[tauri::command]
pub fn has_api_key(provider_id: String) -> Result<bool, AppError> {
    let key = secrets::get_api_key(&provider_id)?;
    Ok(key.is_some())
}

#[tauri::command]
pub async fn test_provider(provider_id: String) -> Result<ProviderTestResult, AppError> {
    let settings = GlobalSettings::load_from_app_data().unwrap_or_default();
    let provider = settings
        .providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| AppError::NotFound {
            entity: "provider".to_string(),
            id: provider_id.clone(),
        })?;

    let api_key = secrets::get_api_key(&provider_id).unwrap_or(None);
    let start = std::time::Instant::now();

    match fetch_models_from_provider(
        &provider.provider_type,
        &provider.base_url,
        api_key.as_deref(),
    )
    .await
    {
        Ok(_) => Ok(ProviderTestResult {
            success: true,
            latency_ms: start.elapsed().as_millis() as u64,
            error: None,
        }),
        Err(e) => Ok(ProviderTestResult {
            success: false,
            latency_ms: start.elapsed().as_millis() as u64,
            error: Some(e.to_string()),
        }),
    }
}
