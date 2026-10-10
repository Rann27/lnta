//! Model registry commands per FR-PRV-10, Story G3.

use crate::error::AppError;
use crate::project::settings::GlobalSettings;
use crate::provider::registry::{fetch_models_from_provider, ModelEntry, ModelRegistry};

#[tauri::command]
pub fn list_models() -> Result<Vec<ModelEntry>, AppError> {
    let mut registry = ModelRegistry::new();
    let settings = GlobalSettings::load_from_app_data().unwrap_or_default();
    for custom in settings.custom_models {
        registry.register_or_update(custom);
    }
    Ok(registry.list_models().to_vec())
}

#[tauri::command]
pub fn register_model(entry: ModelEntry) -> Result<(), AppError> {
    let mut settings = GlobalSettings::load_from_app_data().unwrap_or_default();
    if let Some(existing) = settings
        .custom_models
        .iter_mut()
        .find(|m| m.provider == entry.provider && m.model_id == entry.model_id)
    {
        *existing = entry;
    } else {
        settings.custom_models.push(entry);
    }
    settings.save_to_app_data()
}

#[tauri::command]
pub async fn fetch_provider_models(
    provider_type: String,
    base_url: String,
    api_key: Option<String>,
) -> Result<Vec<ModelEntry>, AppError> {
    fetch_models_from_provider(&provider_type, &base_url, api_key.as_deref()).await
}
