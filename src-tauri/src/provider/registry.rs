//! LLM Model Registry per FR-PRV-10, FR-PRV-11, and Story G3.
//!
//! Stores known models, context windows, max output tokens, token pricing,
//! and value sources (provider metadata, user supplied, or default registry).
//! Also supports fetching model lists from provider endpoints.

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Source of the model's configuration values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueSource {
    DefaultRegistry,
    ProviderMetadata,
    UserSupplied,
}

/// Token pricing per 1,000,000 tokens in USD.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ModelPricing {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_per_million: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_input_per_million: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_per_million: Option<f64>,
}

/// Entry representing a specific model within the registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelEntry {
    pub provider: String,
    pub model_id: String,
    pub display_name: String,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub pricing: ModelPricing,
    pub value_source: ValueSource,
    pub supports_reasoning: bool,
}

/// Model registry containing known and user-configured models.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelRegistry {
    pub models: Vec<ModelEntry>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self {
            models: default_known_models(),
        }
    }
}

impl ModelRegistry {
    /// Creates a new model registry populated with default known models.
    pub fn new() -> Self {
        Self::default()
    }

    /// Finds a model by its provider and model ID.
    pub fn get(&self, provider: &str, model_id: &str) -> Option<&ModelEntry> {
        self.models
            .iter()
            .find(|m| m.provider == provider && m.model_id == model_id)
    }

    /// Inserts or updates a model entry.
    pub fn register_or_update(&mut self, entry: ModelEntry) {
        if let Some(existing) = self
            .models
            .iter_mut()
            .find(|m| m.provider == entry.provider && m.model_id == entry.model_id)
        {
            *existing = entry;
        } else {
            self.models.push(entry);
        }
    }

    /// Lists all models in the registry.
    pub fn list_models(&self) -> &[ModelEntry] {
        &self.models
    }

    /// Lists models filtered by provider.
    pub fn list_by_provider(&self, provider: &str) -> Vec<&ModelEntry> {
        self.models
            .iter()
            .filter(|m| m.provider == provider)
            .collect()
    }
}

/// Returns the default list of well-known models per PRD §4.4 and §7.1.
pub fn default_known_models() -> Vec<ModelEntry> {
    vec![
        ModelEntry {
            provider: "anthropic".to_string(),
            model_id: "claude-3-7-sonnet-20250219".to_string(),
            display_name: "Claude 3.7 Sonnet".to_string(),
            context_window: 200_000,
            max_output_tokens: 8192,
            pricing: ModelPricing {
                input_per_million: Some(3.0),
                cached_input_per_million: Some(0.30),
                output_per_million: Some(15.0),
            },
            value_source: ValueSource::DefaultRegistry,
            supports_reasoning: true,
        },
        ModelEntry {
            provider: "anthropic".to_string(),
            model_id: "claude-3-5-sonnet-20241022".to_string(),
            display_name: "Claude 3.5 Sonnet".to_string(),
            context_window: 200_000,
            max_output_tokens: 8192,
            pricing: ModelPricing {
                input_per_million: Some(3.0),
                cached_input_per_million: Some(0.30),
                output_per_million: Some(15.0),
            },
            value_source: ValueSource::DefaultRegistry,
            supports_reasoning: false,
        },
        ModelEntry {
            provider: "anthropic".to_string(),
            model_id: "claude-3-5-haiku-20241022".to_string(),
            display_name: "Claude 3.5 Haiku".to_string(),
            context_window: 200_000,
            max_output_tokens: 8192,
            pricing: ModelPricing {
                input_per_million: Some(0.80),
                cached_input_per_million: Some(0.08),
                output_per_million: Some(4.0),
            },
            value_source: ValueSource::DefaultRegistry,
            supports_reasoning: false,
        },
        ModelEntry {
            provider: "gemini".to_string(),
            model_id: "gemini-1.5-pro".to_string(),
            display_name: "Gemini 1.5 Pro".to_string(),
            context_window: 2_000_000,
            max_output_tokens: 8192,
            pricing: ModelPricing {
                input_per_million: Some(1.25),
                cached_input_per_million: Some(0.3125),
                output_per_million: Some(5.0),
            },
            value_source: ValueSource::DefaultRegistry,
            supports_reasoning: false,
        },
        ModelEntry {
            provider: "gemini".to_string(),
            model_id: "gemini-1.5-flash".to_string(),
            display_name: "Gemini 1.5 Flash".to_string(),
            context_window: 1_000_000,
            max_output_tokens: 8192,
            pricing: ModelPricing {
                input_per_million: Some(0.075),
                cached_input_per_million: Some(0.01875),
                output_per_million: Some(0.30),
            },
            value_source: ValueSource::DefaultRegistry,
            supports_reasoning: false,
        },
        ModelEntry {
            provider: "gemini".to_string(),
            model_id: "gemini-2.0-flash".to_string(),
            display_name: "Gemini 2.0 Flash".to_string(),
            context_window: 1_000_000,
            max_output_tokens: 8192,
            pricing: ModelPricing {
                input_per_million: Some(0.10),
                cached_input_per_million: Some(0.025),
                output_per_million: Some(0.40),
            },
            value_source: ValueSource::DefaultRegistry,
            supports_reasoning: false,
        },
        ModelEntry {
            provider: "openai_compat".to_string(),
            model_id: "gpt-4o".to_string(),
            display_name: "GPT-4o".to_string(),
            context_window: 128_000,
            max_output_tokens: 16384,
            pricing: ModelPricing {
                input_per_million: Some(2.50),
                cached_input_per_million: Some(1.25),
                output_per_million: Some(10.0),
            },
            value_source: ValueSource::DefaultRegistry,
            supports_reasoning: false,
        },
        ModelEntry {
            provider: "openai_compat".to_string(),
            model_id: "gpt-4o-mini".to_string(),
            display_name: "GPT-4o mini".to_string(),
            context_window: 128_000,
            max_output_tokens: 16384,
            pricing: ModelPricing {
                input_per_million: Some(0.15),
                cached_input_per_million: Some(0.075),
                output_per_million: Some(0.60),
            },
            value_source: ValueSource::DefaultRegistry,
            supports_reasoning: false,
        },
    ]
}

// Deserialization structures for provider models API endpoints

#[derive(Debug, Deserialize)]
struct OpenAiModelItem {
    id: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModelItem>,
}

#[derive(Debug, Deserialize)]
struct AnthropicModelItem {
    id: String,
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicModelsResponse {
    data: Vec<AnthropicModelItem>,
}

#[derive(Debug, Deserialize)]
struct GeminiModelItem {
    name: String,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    #[serde(rename = "inputTokenLimit")]
    input_token_limit: Option<u32>,
    #[serde(rename = "outputTokenLimit")]
    output_token_limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GeminiModelsResponse {
    models: Option<Vec<GeminiModelItem>>,
}

/// Parses an OpenAI-compatible `/models` JSON response into ModelEntry items.
pub fn parse_openai_models_response(
    json_str: &str,
    provider: &str,
) -> Result<Vec<ModelEntry>, AppError> {
    let res: OpenAiModelsResponse =
        serde_json::from_str(json_str).map_err(|e| AppError::Internal {
            message: format!("Failed to parse OpenAI models JSON: {e}"),
        })?;

    Ok(res
        .data
        .into_iter()
        .map(|item| ModelEntry {
            provider: provider.to_string(),
            model_id: item.id.clone(),
            display_name: item.id,
            context_window: 128_000,
            max_output_tokens: 4096,
            pricing: ModelPricing::default(),
            value_source: ValueSource::ProviderMetadata,
            supports_reasoning: false,
        })
        .collect())
}

/// Parses an Anthropic `/v1/models` JSON response into ModelEntry items.
pub fn parse_anthropic_models_response(json_str: &str) -> Result<Vec<ModelEntry>, AppError> {
    let res: AnthropicModelsResponse =
        serde_json::from_str(json_str).map_err(|e| AppError::Internal {
            message: format!("Failed to parse Anthropic models JSON: {e}"),
        })?;

    Ok(res
        .data
        .into_iter()
        .map(|item| {
            let display = item.display_name.unwrap_or_else(|| item.id.clone());
            ModelEntry {
                provider: "anthropic".to_string(),
                model_id: item.id,
                display_name: display,
                context_window: 200_000,
                max_output_tokens: 8192,
                pricing: ModelPricing::default(),
                value_source: ValueSource::ProviderMetadata,
                supports_reasoning: false,
            }
        })
        .collect())
}

/// Parses a Google Gemini `/v1beta/models` JSON response into ModelEntry items.
pub fn parse_gemini_models_response(json_str: &str) -> Result<Vec<ModelEntry>, AppError> {
    let res: GeminiModelsResponse =
        serde_json::from_str(json_str).map_err(|e| AppError::Internal {
            message: format!("Failed to parse Gemini models JSON: {e}"),
        })?;

    let items = res.models.unwrap_or_default();
    Ok(items
        .into_iter()
        .map(|item| {
            let model_id = item.name.trim_start_matches("models/").to_string();
            let display = item.display_name.unwrap_or_else(|| model_id.clone());
            ModelEntry {
                provider: "gemini".to_string(),
                model_id,
                display_name: display,
                context_window: item.input_token_limit.unwrap_or(1_000_000),
                max_output_tokens: item.output_token_limit.unwrap_or(8192),
                pricing: ModelPricing::default(),
                value_source: ValueSource::ProviderMetadata,
                supports_reasoning: false,
            }
        })
        .collect())
}

/// Asynchronously fetches the available models list from a provider endpoint.
pub async fn fetch_models_from_provider(
    provider_type: &str,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<Vec<ModelEntry>, AppError> {
    let client = reqwest::Client::new();
    let base = base_url.trim_end_matches('/');

    match provider_type {
        "anthropic" => {
            let url = format!("{base}/v1/models");
            let mut req = client.get(&url).header("anthropic-version", "2023-06-01");
            if let Some(key) = api_key {
                req = req.header("x-api-key", key);
            }
            let resp = req.send().await.map_err(|e| AppError::ProviderFailed {
                reason: format!("Failed to fetch Anthropic models: {e}"),
            })?;
            let text = resp.text().await.map_err(|e| AppError::ProviderFailed {
                reason: format!("Failed to read Anthropic models response: {e}"),
            })?;
            parse_anthropic_models_response(&text)
        }
        "gemini" => {
            let url = format!("{base}/v1beta/models");
            let mut req = client.get(&url);
            if let Some(key) = api_key {
                req = req.header("x-goog-api-key", key);
            }
            let resp = req.send().await.map_err(|e| AppError::ProviderFailed {
                reason: format!("Failed to fetch Gemini models: {e}"),
            })?;
            let text = resp.text().await.map_err(|e| AppError::ProviderFailed {
                reason: format!("Failed to read Gemini models response: {e}"),
            })?;
            parse_gemini_models_response(&text)
        }
        _ => {
            // Default to OpenAI-compatible `/models`
            let url = format!("{base}/models");
            let mut req = client.get(&url);
            if let Some(key) = api_key {
                req = req.header("Authorization", format!("Bearer {key}"));
            }
            let resp = req.send().await.map_err(|e| AppError::ProviderFailed {
                reason: format!("Failed to fetch OpenAI models: {e}"),
            })?;
            let text = resp.text().await.map_err(|e| AppError::ProviderFailed {
                reason: format!("Failed to read OpenAI models response: {e}"),
            })?;
            parse_openai_models_response(&text, provider_type)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registry_has_standard_models() {
        let registry = ModelRegistry::new();
        assert!(!registry.list_models().is_empty());

        let claude = registry.get("anthropic", "claude-3-7-sonnet-20250219");
        assert!(claude.is_some());
        let claude = claude.unwrap();
        assert_eq!(claude.context_window, 200_000);
        assert_eq!(claude.max_output_tokens, 8192);
        assert_eq!(claude.value_source, ValueSource::DefaultRegistry);
        assert_eq!(claude.pricing.input_per_million, Some(3.0));

        let gpt4 = registry.get("openai_compat", "gpt-4o");
        assert!(gpt4.is_some());
        let gemini = registry.get("gemini", "gemini-1.5-pro");
        assert!(gemini.is_some());
    }

    #[test]
    fn test_register_or_update_model() {
        let mut registry = ModelRegistry::new();
        let custom_model = ModelEntry {
            provider: "openai_compat".to_string(),
            model_id: "deepseek-v3".to_string(),
            display_name: "DeepSeek V3".to_string(),
            context_window: 64_000,
            max_output_tokens: 8192,
            pricing: ModelPricing {
                input_per_million: Some(0.14),
                cached_input_per_million: Some(0.014),
                output_per_million: Some(0.28),
            },
            value_source: ValueSource::UserSupplied,
            supports_reasoning: false,
        };

        registry.register_or_update(custom_model.clone());
        let fetched = registry.get("openai_compat", "deepseek-v3").unwrap();
        assert_eq!(fetched.display_name, "DeepSeek V3");
        assert_eq!(fetched.value_source, ValueSource::UserSupplied);

        // Update existing
        let mut updated = custom_model;
        updated.max_output_tokens = 16384;
        registry.register_or_update(updated);
        let fetched2 = registry.get("openai_compat", "deepseek-v3").unwrap();
        assert_eq!(fetched2.max_output_tokens, 16384);
    }

    #[test]
    fn test_parse_openai_models_response() {
        let json = r#"{"data": [{"id": "gpt-4o"}, {"id": "gpt-4o-mini"}]}"#;
        let models = parse_openai_models_response(json, "openai_compat").unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].model_id, "gpt-4o");
        assert_eq!(models[0].value_source, ValueSource::ProviderMetadata);
    }

    #[test]
    fn test_parse_anthropic_models_response() {
        let json = r#"{"data": [{"id": "claude-3-5-sonnet-20241022", "display_name": "Claude 3.5 Sonnet"}]}"#;
        let models = parse_anthropic_models_response(json).unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model_id, "claude-3-5-sonnet-20241022");
        assert_eq!(models[0].display_name, "Claude 3.5 Sonnet");
        assert_eq!(models[0].context_window, 200_000);
    }

    #[test]
    fn test_parse_gemini_models_response() {
        let json = r#"{"models": [{"name": "models/gemini-1.5-pro-latest", "displayName": "Gemini 1.5 Pro", "inputTokenLimit": 2097152, "outputTokenLimit": 8192}]}"#;
        let models = parse_gemini_models_response(json).unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model_id, "gemini-1.5-pro-latest");
        assert_eq!(models[0].display_name, "Gemini 1.5 Pro");
        assert_eq!(models[0].context_window, 2097152);
        assert_eq!(models[0].max_output_tokens, 8192);
    }
}
