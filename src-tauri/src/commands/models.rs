//! Model registry commands per FR-PRV-10, Story G3.

use crate::error::AppError;
use crate::project::settings::GlobalSettings;
use crate::provider::registry::{
    fetch_models_from_provider, ModelEntry, ModelRegistry, ValueSource,
};

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

/// Fetches the models a configured provider offers now, with its stored API key, and keeps
/// the new ones in the settings so they appear in model pickers. Catalogue entries keep their
/// curated limits and parameters; entries the user registered by hand are not overwritten.
#[tauri::command]
pub async fn fetch_provider_models(provider_id: String) -> Result<Vec<ModelEntry>, AppError> {
    let settings = GlobalSettings::load_from_app_data().unwrap_or_default();
    let provider = settings
        .providers
        .iter()
        .find(|p| p.id == provider_id)
        .cloned()
        .ok_or_else(|| AppError::NotFound {
            entity: "provider".to_string(),
            id: provider_id.clone(),
        })?;
    let api_key = crate::secrets::get_api_key(&provider_id).unwrap_or(None);
    let fetched = fetch_models_from_provider(
        &provider.id,
        &provider.provider_type,
        &provider.base_url,
        api_key.as_deref(),
    )
    .await?;

    // Reload before writing: the fetch can take a while and settings may have changed.
    let mut settings = GlobalSettings::load_from_app_data().unwrap_or_default();
    merge_fetched_models(&mut settings.custom_models, &fetched);
    settings.save_to_app_data()?;
    Ok(fetched)
}

/// Adds fetched models that the catalogue does not cover, refreshing earlier fetches.
fn merge_fetched_models(custom: &mut Vec<ModelEntry>, fetched: &[ModelEntry]) {
    let catalogue = ModelRegistry::new();
    for model in fetched {
        if catalogue.get(&model.provider, &model.model_id).is_some() {
            continue;
        }
        match custom
            .iter_mut()
            .find(|m| m.provider == model.provider && m.model_id == model.model_id)
        {
            Some(existing) if existing.value_source == ValueSource::UserSupplied => {}
            Some(existing) => *existing = model.clone(),
            None => custom.push(model.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fetched(provider: &str, id: &str) -> ModelEntry {
        ModelEntry {
            provider: provider.to_string(),
            model_id: id.to_string(),
            display_name: id.to_string(),
            value_source: ValueSource::ProviderMetadata,
            ..Default::default()
        }
    }

    #[test]
    fn merge_keeps_catalogue_and_hand_entries() {
        let mut hand = fetched("zai", "glm-x");
        hand.value_source = ValueSource::UserSupplied;
        hand.display_name = "Mine".to_string();
        let mut custom = vec![hand];

        merge_fetched_models(
            &mut custom,
            &[
                fetched("zai", "glm-5.3"),
                fetched("zai", "glm-x"),
                fetched("zai", "glm-new"),
            ],
        );

        assert_eq!(custom.len(), 2);
        assert_eq!(custom[0].display_name, "Mine");
        assert_eq!(custom[1].model_id, "glm-new");
    }
}
