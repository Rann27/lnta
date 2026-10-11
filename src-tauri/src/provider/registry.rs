//! LLM Model Registry per FR-PRV-10, FR-PRV-11, and Story G3.
//!
//! Stores known models, context windows, max output tokens, token pricing, the request
//! parameters each model accepts, and value sources (provider metadata, user supplied, or
//! default registry). Also supports fetching model lists from provider endpoints.
//!
//! Catalogue checked against the providers' own model and pricing pages in October 2026.
//! Where a provider did not publish a figure, pricing is left empty (the UI says "no price")
//! and output limits use a conservative value, which only makes chunks smaller.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;

/// Source of the model's configuration values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../src/lib/bindings/ValueSource.ts")]
pub enum ValueSource {
    #[default]
    DefaultRegistry,
    ProviderMetadata,
    UserSupplied,
}

/// Token pricing per 1,000,000 tokens in USD.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/ModelPricing.ts")]
pub struct ModelPricing {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_per_million: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_input_per_million: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_per_million: Option<f64>,
}

/// Entry representing a specific model within the registry.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/ModelEntry.ts")]
pub struct ModelEntry {
    pub provider: String,
    pub model_id: String,
    pub display_name: String,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub pricing: ModelPricing,
    pub value_source: ValueSource,
    pub supports_reasoning: bool,
    /// Reasoning effort values the model accepts; requests with any other value drop it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reasoning_efforts: Option<Vec<String>>,
    /// The provider's default effort, shown as the starting choice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub default_reasoning_effort: Option<String>,
    /// `Some(false)` when the model rejects or ignores `temperature` (most reasoning models).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub supports_temperature: Option<bool>,
    /// Temperature the provider recommends, offered as the default in role settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub recommended_temperature: Option<f32>,
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

    /// Finds a model by ID alone. Catalogue IDs are unique across providers, apart from the
    /// CLI placeholder `default`, which carries no parameter rules anyway.
    pub fn find_by_model_id(&self, model_id: &str) -> Option<&ModelEntry> {
        if model_id == "default" {
            return None;
        }
        self.models.iter().find(|m| m.model_id == model_id)
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

/// Request parameters for one catalogue entry.
struct Params {
    efforts: &'static [&'static str],
    default_effort: Option<&'static str>,
    temperature: Option<bool>,
    recommended_temperature: Option<f32>,
}

const CLAUDE_EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];
const GPT_EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];
const GEMINI_LEVELS: &[&str] = &["low", "medium", "high"];
const NO_EFFORTS: &[&str] = &[];

/// Thinking models that ignore sampling settings and take an effort level.
fn reasoning(efforts: &'static [&'static str], default_effort: &'static str) -> Params {
    Params {
        efforts,
        default_effort: Some(default_effort),
        temperature: Some(false),
        recommended_temperature: None,
    }
}

/// Models where nothing beyond the prompt is sent; the provider defaults apply.
fn plain() -> Params {
    Params {
        efforts: NO_EFFORTS,
        default_effort: None,
        temperature: None,
        recommended_temperature: None,
    }
}

/// Builds a catalogue entry. Prices are USD per million tokens: input, cached input, output.
#[allow(clippy::too_many_arguments)]
fn entry(
    provider: &str,
    model_id: &str,
    display_name: &str,
    context_window: u32,
    max_output_tokens: u32,
    price: Option<(f64, Option<f64>, f64)>,
    supports_reasoning: bool,
    params: Params,
) -> ModelEntry {
    ModelEntry {
        provider: provider.to_string(),
        model_id: model_id.to_string(),
        display_name: display_name.to_string(),
        context_window,
        max_output_tokens,
        pricing: price
            .map(|(input, cached, output)| ModelPricing {
                input_per_million: Some(input),
                cached_input_per_million: cached,
                output_per_million: Some(output),
            })
            .unwrap_or_default(),
        value_source: ValueSource::DefaultRegistry,
        supports_reasoning,
        reasoning_efforts: (!params.efforts.is_empty())
            .then(|| params.efforts.iter().map(|e| e.to_string()).collect()),
        default_reasoning_effort: params.default_effort.map(str::to_string),
        supports_temperature: params.temperature,
        recommended_temperature: params.recommended_temperature,
    }
}

/// Returns the built-in model catalogue per PRD §4.4 and §7.1.
pub fn default_known_models() -> Vec<ModelEntry> {
    const M: u32 = 1_048_576;
    let gemini = |default_level: &'static str| Params {
        efforts: GEMINI_LEVELS,
        default_effort: Some(default_level),
        temperature: Some(true),
        // Google: keep Gemini 3 at 1.0; lower values can loop.
        recommended_temperature: Some(1.0),
    };

    vec![
        // Anthropic: adaptive thinking, steered by effort; 1M context, 128K output.
        entry(
            "anthropic",
            "claude-fable-5-1",
            "Claude Fable 5.1",
            1_000_000,
            128_000,
            Some((10.0, Some(0.25), 50.0)),
            true,
            reasoning(CLAUDE_EFFORTS, "high"),
        ),
        entry(
            "anthropic",
            "claude-opus-5-5",
            "Claude Opus 5.5",
            1_000_000,
            128_000,
            Some((4.0, Some(0.20), 20.0)),
            true,
            reasoning(CLAUDE_EFFORTS, "medium"),
        ),
        entry(
            "anthropic",
            "claude-sonnet-5-5",
            "Claude Sonnet 5.5",
            1_000_000,
            128_000,
            Some((2.0, Some(0.10), 10.0)),
            true,
            reasoning(CLAUDE_EFFORTS, "high"),
        ),
        entry(
            "anthropic",
            "claude-haiku-5-5",
            "Claude Haiku 5.5",
            1_000_000,
            128_000,
            Some((0.10, Some(0.01), 0.50)),
            true,
            reasoning(CLAUDE_EFFORTS, "medium"),
        ),
        // OpenAI
        entry(
            "openai",
            "gpt-6-astra",
            "GPT-6 Astra",
            1_050_000,
            128_000,
            Some((10.0, Some(1.0), 50.0)),
            true,
            reasoning(GPT_EFFORTS, "medium"),
        ),
        entry(
            "openai",
            "gpt-6.1-sol",
            "GPT-6.1 Sol",
            1_050_000,
            128_000,
            Some((2.0, Some(0.10), 10.0)),
            true,
            reasoning(GPT_EFFORTS, "medium"),
        ),
        entry(
            "openai",
            "gpt-6-luna",
            "GPT-6 Luna",
            1_050_000,
            128_000,
            Some((0.10, Some(0.01), 0.50)),
            true,
            reasoning(&["none", "low", "medium", "high", "xhigh", "max"], "low"),
        ),
        entry(
            "openai",
            "gpt-5.6-terra",
            "GPT-5.6 Terra",
            1_050_000,
            128_000,
            Some((2.0, Some(0.20), 12.0)),
            true,
            reasoning(GPT_EFFORTS, "medium"),
        ),
        // Google Gemini (prices at or under 200K prompt tokens)
        entry(
            "gemini",
            "gemini-3.1-pro-preview",
            "Gemini 3.1 Pro (preview)",
            M,
            65_536,
            Some((2.0, Some(0.20), 12.0)),
            true,
            gemini("high"),
        ),
        entry(
            "gemini",
            "gemini-3.8-flash",
            "Gemini 3.8 Flash",
            M,
            65_536,
            Some((0.75, Some(0.075), 3.75)),
            true,
            gemini("high"),
        ),
        entry(
            "gemini",
            "gemini-3.6-flash",
            "Gemini 3.6 Flash",
            M,
            65_536,
            Some((0.75, Some(0.075), 3.75)),
            true,
            gemini("high"),
        ),
        entry(
            "gemini",
            "gemini-3.5-flash-lite",
            "Gemini 3.5 Flash-Lite",
            M,
            65_536,
            Some((0.30, Some(0.03), 2.50)),
            true,
            gemini("low"),
        ),
        // xAI Grok (prices under 200K prompt tokens)
        entry(
            "xai",
            "grok-4.7",
            "Grok 4.7",
            500_000,
            32_768,
            Some((2.0, Some(0.50), 6.0)),
            true,
            plain(),
        ),
        entry(
            "xai",
            "grok-4.3",
            "Grok 4.3",
            1_000_000,
            32_768,
            Some((1.25, Some(0.20), 2.50)),
            true,
            plain(),
        ),
        entry(
            "xai",
            "grok-4.20-0309-non-reasoning",
            "Grok 4.20 (non-reasoning)",
            1_000_000,
            32_768,
            Some((1.25, Some(0.20), 2.50)),
            false,
            plain(),
        ),
        // DeepSeek (peak-hour prices; off-peak is half). Thinking is the default mode.
        entry(
            "deepseek",
            "deepseek-v4-pro",
            "DeepSeek V4 Pro",
            1_000_000,
            384_000,
            Some((1.32, Some(0.044), 3.96)),
            true,
            plain(),
        ),
        entry(
            "deepseek",
            "deepseek-flash",
            "DeepSeek V4.1 Flash",
            1_000_000,
            384_000,
            Some((0.30, Some(0.006), 1.20)),
            true,
            plain(),
        ),
        // Z.ai GLM (thinking always on)
        entry(
            "zai",
            "glm-5.3",
            "GLM-5.3",
            M,
            131_072,
            Some((1.40, Some(0.26), 4.40)),
            true,
            plain(),
        ),
        entry(
            "zai",
            "glm-5.3-flash",
            "GLM-5.3 Flash",
            200_000,
            32_768,
            Some((0.15, Some(0.03), 0.50)),
            true,
            plain(),
        ),
        // Alibaba Qwen, international endpoint. Official prices not confirmed.
        entry(
            "qwen",
            "qwen3.8-max",
            "Qwen3.8 Max",
            1_000_000,
            32_768,
            None,
            true,
            plain(),
        ),
        entry(
            "qwen",
            "qwen3.7-plus",
            "Qwen3.7 Plus",
            1_000_000,
            32_768,
            None,
            true,
            plain(),
        ),
        entry(
            "qwen",
            "qwen3.8-flash",
            "Qwen3.8 Flash",
            1_000_000,
            32_768,
            None,
            true,
            plain(),
        ),
        // MiniMax. Official prices not confirmed.
        entry(
            "minimax",
            "MiniMax-M3",
            "MiniMax M3",
            524_288,
            32_768,
            None,
            true,
            plain(),
        ),
        // Xiaomi MiMo. Official prices not confirmed.
        entry(
            "xiaomi",
            "mimo-v2.5-pro",
            "MiMo V2.5 Pro",
            1_000_000,
            32_768,
            None,
            true,
            plain(),
        ),
        entry(
            "xiaomi",
            "mimo-v2.6-pro",
            "MiMo V2.6 Pro",
            1_000_000,
            32_768,
            None,
            true,
            plain(),
        ),
        // Moonshot Kimi. K3 fixes its own sampling settings.
        entry(
            "moonshot",
            "kimi-k3",
            "Kimi K3",
            M,
            32_768,
            Some((3.0, Some(0.30), 15.0)),
            true,
            Params {
                temperature: Some(false),
                ..plain()
            },
        ),
        entry(
            "moonshot",
            "kimi-k2.6",
            "Kimi K2.6",
            262_144,
            32_768,
            Some((0.95, Some(0.16), 4.0)),
            true,
            plain(),
        ),
        // Mistral. Aliases follow the newest release; prices not confirmed.
        entry(
            "mistral",
            "mistral-large-latest",
            "Mistral Large (latest)",
            128_000,
            32_768,
            None,
            false,
            plain(),
        ),
        entry(
            "mistral",
            "mistral-medium-latest",
            "Mistral Medium (latest)",
            128_000,
            32_768,
            None,
            false,
            plain(),
        ),
        // Subscription CLIs: the CLI picks the model when "default" is chosen.
        entry(
            "cli_claude",
            "default",
            "Claude Code: model default",
            1_000_000,
            128_000,
            None,
            true,
            Params {
                temperature: Some(false),
                ..reasoning(CLAUDE_EFFORTS, "high")
            },
        ),
        entry(
            "cli_claude",
            "opus",
            "Claude Code: Opus",
            1_000_000,
            128_000,
            None,
            true,
            reasoning(CLAUDE_EFFORTS, "medium"),
        ),
        entry(
            "cli_claude",
            "sonnet",
            "Claude Code: Sonnet",
            1_000_000,
            128_000,
            None,
            true,
            reasoning(CLAUDE_EFFORTS, "high"),
        ),
        entry(
            "cli_claude",
            "fable",
            "Claude Code: Fable",
            1_000_000,
            128_000,
            None,
            true,
            reasoning(CLAUDE_EFFORTS, "high"),
        ),
        entry(
            "cli_codex",
            "default",
            "Codex: model default",
            1_050_000,
            128_000,
            None,
            true,
            reasoning(GPT_EFFORTS, "medium"),
        ),
        entry(
            "cli_antigravity",
            "default",
            "Antigravity: model default",
            M,
            65_536,
            None,
            true,
            reasoning(GEMINI_LEVELS, "high"),
        ),
        entry(
            "cli_antigravity",
            "gemini-3.1-pro-high",
            "Antigravity: Gemini 3.1 Pro (high)",
            M,
            65_536,
            None,
            true,
            plain(),
        ),
        entry(
            "cli_antigravity",
            "gemini-3.8-flash-medium",
            "Antigravity: Gemini 3.8 Flash (medium)",
            M,
            65_536,
            None,
            true,
            plain(),
        ),
    ]
}

// Deserialization structures for provider models API endpoints

#[derive(Debug, Deserialize)]
struct OpenAiModelItem {
    id: String,
    name: Option<String>,
    /// OpenRouter, Moonshot, and some others report the window here.
    context_length: Option<u32>,
    context_window: Option<u32>,
    pricing: Option<OpenRouterPricing>,
    top_provider: Option<OpenRouterTopProvider>,
}

/// OpenRouter prices are USD per single token, as strings.
#[derive(Debug, Deserialize)]
struct OpenRouterPricing {
    prompt: Option<String>,
    completion: Option<String>,
    input_cache_read: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterTopProvider {
    max_completion_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModelItem>,
}

#[derive(Debug, Deserialize)]
struct AnthropicModelItem {
    id: String,
    display_name: Option<String>,
    max_input_tokens: Option<u32>,
    max_tokens: Option<u32>,
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
    #[serde(rename = "supportedGenerationMethods")]
    supported_generation_methods: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct GeminiModelsResponse {
    models: Option<Vec<GeminiModelItem>>,
}

fn per_token_to_per_million(value: Option<&String>) -> Option<f64> {
    value
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|v| *v >= 0.0)
        .map(|v| (v * 1_000_000.0 * 10_000.0).round() / 10_000.0)
}

/// Parses an OpenAI-compatible `/models` JSON response into ModelEntry items for `provider`.
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
        .map(|item| {
            let pricing = item
                .pricing
                .as_ref()
                .map(|p| ModelPricing {
                    input_per_million: per_token_to_per_million(p.prompt.as_ref()),
                    cached_input_per_million: per_token_to_per_million(p.input_cache_read.as_ref()),
                    output_per_million: per_token_to_per_million(p.completion.as_ref()),
                })
                .unwrap_or_default();
            ModelEntry {
                provider: provider.to_string(),
                display_name: item.name.clone().unwrap_or_else(|| item.id.clone()),
                model_id: item.id,
                context_window: item
                    .context_length
                    .or(item.context_window)
                    .unwrap_or(128_000),
                // Unknown limits stay conservative so chunk plans never overrun them.
                max_output_tokens: item
                    .top_provider
                    .and_then(|t| t.max_completion_tokens)
                    .unwrap_or(16_384),
                pricing,
                value_source: ValueSource::ProviderMetadata,
                ..Default::default()
            }
        })
        .collect())
}

/// Parses an Anthropic `/v1/models` JSON response into ModelEntry items for `provider`.
pub fn parse_anthropic_models_response(
    json_str: &str,
    provider: &str,
) -> Result<Vec<ModelEntry>, AppError> {
    let res: AnthropicModelsResponse =
        serde_json::from_str(json_str).map_err(|e| AppError::Internal {
            message: format!("Failed to parse Anthropic models JSON: {e}"),
        })?;

    Ok(res
        .data
        .into_iter()
        .map(|item| ModelEntry {
            provider: provider.to_string(),
            display_name: item.display_name.unwrap_or_else(|| item.id.clone()),
            model_id: item.id,
            context_window: item.max_input_tokens.unwrap_or(200_000),
            max_output_tokens: item.max_tokens.unwrap_or(16_384),
            value_source: ValueSource::ProviderMetadata,
            ..Default::default()
        })
        .collect())
}

/// Parses a Google Gemini `/v1beta/models` JSON response into ModelEntry items for `provider`.
/// Embedding, image, and audio-only models are skipped.
pub fn parse_gemini_models_response(
    json_str: &str,
    provider: &str,
) -> Result<Vec<ModelEntry>, AppError> {
    let res: GeminiModelsResponse =
        serde_json::from_str(json_str).map_err(|e| AppError::Internal {
            message: format!("Failed to parse Gemini models JSON: {e}"),
        })?;

    Ok(res
        .models
        .unwrap_or_default()
        .into_iter()
        .filter(|item| {
            item.supported_generation_methods
                .as_ref()
                .is_none_or(|m| m.iter().any(|g| g == "generateContent"))
        })
        .map(|item| {
            let model_id = item.name.trim_start_matches("models/").to_string();
            ModelEntry {
                provider: provider.to_string(),
                display_name: item.display_name.unwrap_or_else(|| model_id.clone()),
                model_id,
                context_window: item.input_token_limit.unwrap_or(1_000_000),
                max_output_tokens: item.output_token_limit.unwrap_or(8192),
                value_source: ValueSource::ProviderMetadata,
                ..Default::default()
            }
        })
        .collect())
}

/// Fetches the models a provider currently offers. `provider_id` names the configured
/// provider entry; `provider_type` picks the API dialect.
pub async fn fetch_models_from_provider(
    provider_id: &str,
    provider_type: &str,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<Vec<ModelEntry>, AppError> {
    let client = reqwest::Client::new();
    let base = base_url.trim_end_matches('/');

    let read = |resp: reqwest::Response| async move {
        let status = resp.status();
        let text = resp.text().await.map_err(|e| AppError::ProviderFailed {
            reason: format!("Failed to read models response: {e}"),
        })?;
        if !status.is_success() {
            return Err(AppError::ProviderFailed {
                reason: format!("Models request failed with HTTP {status}: {text}"),
            });
        }
        Ok(text)
    };

    match provider_type {
        "anthropic" => {
            let mut req = client
                .get(format!("{base}/v1/models?limit=1000"))
                .header("anthropic-version", "2023-06-01");
            if let Some(key) = api_key {
                req = req.header("x-api-key", key);
            }
            let resp = req.send().await.map_err(|e| AppError::ProviderFailed {
                reason: format!("Failed to fetch Anthropic models: {e}"),
            })?;
            parse_anthropic_models_response(&read(resp).await?, provider_id)
        }
        "gemini" => {
            let mut req = client.get(format!("{base}/v1beta/models?pageSize=1000"));
            if let Some(key) = api_key {
                req = req.header("x-goog-api-key", key);
            }
            let resp = req.send().await.map_err(|e| AppError::ProviderFailed {
                reason: format!("Failed to fetch Gemini models: {e}"),
            })?;
            parse_gemini_models_response(&read(resp).await?, provider_id)
        }
        "cli_claude" | "cli_codex" | "cli_antigravity" => {
            let bin = crate::provider::cli_common::cli_binary_for(provider_type);
            let ver = crate::provider::cli_common::detect_cli_version(bin).await?;
            Ok(default_known_models()
                .into_iter()
                .filter(|m| m.provider == provider_type)
                .map(|mut m| {
                    m.provider = provider_id.to_string();
                    if m.model_id == "default" {
                        m.display_name = format!("{} ({ver})", m.display_name);
                    }
                    m
                })
                .collect())
        }
        _ => {
            let mut req = client.get(format!("{base}/models"));
            if let Some(key) = api_key {
                req = req
                    .header("Authorization", format!("Bearer {key}"))
                    .header("api-key", key);
            }
            let resp = req.send().await.map_err(|e| AppError::ProviderFailed {
                reason: format!("Failed to fetch models: {e}"),
            })?;
            parse_openai_models_response(&read(resp).await?, provider_id)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registry_has_current_models() {
        let registry = ModelRegistry::new();
        let opus = registry.get("anthropic", "claude-opus-5-5").unwrap();
        assert_eq!(opus.context_window, 1_000_000);
        assert_eq!(opus.max_output_tokens, 128_000);
        assert_eq!(opus.pricing.input_per_million, Some(4.0));
        assert_eq!(opus.supports_temperature, Some(false));
        assert_eq!(opus.default_reasoning_effort.as_deref(), Some("medium"));

        assert!(registry.get("openai", "gpt-6.1-sol").is_some());
        let flash = registry.get("gemini", "gemini-3.8-flash").unwrap();
        assert_eq!(flash.recommended_temperature, Some(1.0));
        for provider in [
            "xai", "deepseek", "zai", "qwen", "minimax", "xiaomi", "moonshot",
        ] {
            assert!(
                !registry.list_by_provider(provider).is_empty(),
                "{provider}"
            );
        }
        // Retired models are gone.
        assert!(registry.find_by_model_id("gemini-2.0-flash").is_none());
        assert!(registry.find_by_model_id("gpt-4o").is_none());
    }

    #[test]
    fn test_catalogue_ids_are_unique_apart_from_cli_default() {
        let models = default_known_models();
        let mut seen = std::collections::HashSet::new();
        for m in &models {
            if m.model_id != "default" {
                assert!(seen.insert(m.model_id.clone()), "duplicate {}", m.model_id);
            }
        }
    }

    #[test]
    fn test_register_or_update_model() {
        let mut registry = ModelRegistry::new();
        let custom_model = ModelEntry {
            provider: "deepseek".to_string(),
            model_id: "deepseek-custom".to_string(),
            display_name: "DeepSeek Custom".to_string(),
            context_window: 64_000,
            max_output_tokens: 8192,
            value_source: ValueSource::UserSupplied,
            ..Default::default()
        };

        registry.register_or_update(custom_model.clone());
        let fetched = registry.get("deepseek", "deepseek-custom").unwrap();
        assert_eq!(fetched.display_name, "DeepSeek Custom");

        let mut updated = custom_model;
        updated.max_output_tokens = 16384;
        registry.register_or_update(updated);
        assert_eq!(
            registry
                .get("deepseek", "deepseek-custom")
                .unwrap()
                .max_output_tokens,
            16384
        );
    }

    #[test]
    fn test_parse_openai_models_response() {
        let json =
            r#"{"data": [{"id": "glm-5.3"}, {"id": "glm-5.3-flash", "context_length": 200000}]}"#;
        let models = parse_openai_models_response(json, "zai").unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].provider, "zai");
        assert_eq!(models[1].context_window, 200_000);
        assert_eq!(models[0].value_source, ValueSource::ProviderMetadata);
    }

    #[test]
    fn test_parse_openrouter_pricing_and_limits() {
        let json = r#"{"data": [{"id": "z-ai/glm-5.3", "name": "Z.ai: GLM 5.3",
            "context_length": 1048576,
            "pricing": {"prompt": "0.0000014", "completion": "0.0000044", "input_cache_read": "0.00000026"},
            "top_provider": {"max_completion_tokens": 131072}}]}"#;
        let models = parse_openai_models_response(json, "openrouter").unwrap();
        let m = &models[0];
        assert_eq!(m.display_name, "Z.ai: GLM 5.3");
        assert_eq!(m.max_output_tokens, 131_072);
        assert_eq!(m.pricing.input_per_million, Some(1.4));
        assert_eq!(m.pricing.output_per_million, Some(4.4));
        assert_eq!(m.pricing.cached_input_per_million, Some(0.26));
    }

    #[test]
    fn test_parse_anthropic_models_response() {
        let json = r#"{"data": [{"id": "claude-sonnet-5-5", "display_name": "Claude Sonnet 5.5", "max_input_tokens": 1000000, "max_tokens": 128000}]}"#;
        let models = parse_anthropic_models_response(json, "anthropic").unwrap();
        assert_eq!(models[0].model_id, "claude-sonnet-5-5");
        assert_eq!(models[0].context_window, 1_000_000);
        assert_eq!(models[0].max_output_tokens, 128_000);
    }

    #[test]
    fn test_parse_gemini_models_response_skips_non_text_models() {
        let json = r#"{"models": [
            {"name": "models/gemini-3.8-flash", "displayName": "Gemini 3.8 Flash", "inputTokenLimit": 1048576, "outputTokenLimit": 65536, "supportedGenerationMethods": ["generateContent"]},
            {"name": "models/text-embedding-005", "supportedGenerationMethods": ["embedContent"]}
        ]}"#;
        let models = parse_gemini_models_response(json, "gemini").unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model_id, "gemini-3.8-flash");
        assert_eq!(models[0].max_output_tokens, 65_536);
    }
}
