//! LNTA provider abstraction, adapters, retry mechanism, and mock provider per PRD §7.1 and FR-PRV-01..09.

pub mod anthropic;
pub mod cli_antigravity;
pub mod cli_claude;
pub mod cli_codex;
pub mod cli_common;
pub mod gemini;
pub mod mock;
pub mod openai_compat;
pub mod registry;
pub mod retry;
pub mod types;

use crate::error::AppError;
pub use anthropic::{
    parse_recorded_anthropic_sse, AnthropicCacheControl, AnthropicConfig, AnthropicProvider,
    AnthropicSystemBlock,
};
pub use cli_antigravity::CliAntigravityProvider;
pub use cli_claude::CliClaudeProvider;
pub use cli_codex::CliCodexProvider;
pub use cli_common::{detect_cli_version, resolve_cli_path, TempDirGuard};
pub use gemini::{
    parse_recorded_gemini_sse, permissive_safety_settings, GeminiConfig, GeminiProvider,
    GeminiSafetySetting,
};
pub use mock::{MockProvider, MockStep};
pub use openai_compat::{
    parse_recorded_sse_stream, parse_sse_line, OpenAiCompatConfig, OpenAiCompatProvider,
};
pub use registry::{
    default_known_models, fetch_models_from_provider, ModelEntry, ModelPricing, ModelRegistry,
    ValueSource,
};
pub use retry::{calculate_retry_delay, is_retryable_status, RetryConfig};
pub use types::{
    FinishReason, Provider, ProviderOutcome, ProviderRequest, ProviderResponse, TokenUsage,
};

/// Shapes a request to what the target model accepts, using the built-in catalogue:
/// drops `temperature` where the model rejects it, drops effort values the model does not
/// know, and fills or caps the output limit. Models outside the catalogue pass unchanged.
pub fn fit_request_to_model(req: &ProviderRequest) -> ProviderRequest {
    let mut fitted = req.clone();
    let registry = ModelRegistry::default();
    let Some(entry) = registry.find_by_model_id(&req.model) else {
        return fitted;
    };
    if entry.supports_temperature == Some(false) {
        fitted.temperature = None;
    }
    let effort_ok = match (&entry.reasoning_efforts, &fitted.reasoning_effort) {
        (Some(allowed), Some(effort)) => allowed.iter().any(|a| a == effort),
        (None, Some(_)) => false,
        (_, None) => true,
    };
    if !effort_ok {
        fitted.reasoning_effort = None;
    }
    let cap = entry.max_output_tokens;
    fitted.max_output_tokens = Some(fitted.max_output_tokens.map_or(cap, |m| m.min(cap)));
    fitted
}

/// Unified enum covering all concrete provider implementations for static dispatch.
#[derive(Clone)]
pub enum AnyProvider {
    OpenAiCompat(std::sync::Arc<OpenAiCompatProvider>),
    Anthropic(std::sync::Arc<AnthropicProvider>),
    Gemini(std::sync::Arc<GeminiProvider>),
    CliClaude(std::sync::Arc<CliClaudeProvider>),
    CliAntigravity(std::sync::Arc<CliAntigravityProvider>),
    CliCodex(std::sync::Arc<CliCodexProvider>),
    Mock(std::sync::Arc<MockProvider>),
}

impl Provider for AnyProvider {
    fn name(&self) -> &'static str {
        match self {
            Self::OpenAiCompat(p) => p.name(),
            Self::Anthropic(p) => p.name(),
            Self::Gemini(p) => p.name(),
            Self::CliClaude(p) => p.name(),
            Self::CliAntigravity(p) => p.name(),
            Self::CliCodex(p) => p.name(),
            Self::Mock(p) => p.name(),
        }
    }

    async fn send_request(
        &self,
        req: &ProviderRequest,
        token_cb: Option<&mut (dyn FnMut(&str) + Send)>,
    ) -> Result<ProviderOutcome, AppError> {
        let fitted = fit_request_to_model(req);
        let req = &fitted;
        match self {
            Self::OpenAiCompat(p) => p.send_request(req, token_cb).await,
            Self::Anthropic(p) => p.send_request(req, token_cb).await,
            Self::Gemini(p) => p.send_request(req, token_cb).await,
            Self::CliClaude(p) => p.send_request(req, token_cb).await,
            Self::CliAntigravity(p) => p.send_request(req, token_cb).await,
            Self::CliCodex(p) => p.send_request(req, token_cb).await,
            Self::Mock(p) => p.send_request(req, token_cb).await,
        }
    }
}

#[cfg(test)]
mod fit_tests {
    use super::*;

    fn req(model: &str) -> ProviderRequest {
        ProviderRequest {
            system_prompt: String::new(),
            user_prompt: String::new(),
            model: model.to_string(),
            temperature: Some(0.3),
            max_output_tokens: None,
            reasoning_effort: Some("xhigh".to_string()),
        }
    }

    #[test]
    fn reasoning_models_lose_temperature_and_keep_known_effort() {
        let fitted = fit_request_to_model(&req("claude-opus-5-5"));
        assert_eq!(fitted.temperature, None);
        assert_eq!(fitted.reasoning_effort.as_deref(), Some("xhigh"));
        assert_eq!(fitted.max_output_tokens, Some(128_000));
    }

    #[test]
    fn unknown_effort_values_are_dropped() {
        // Gemini thinking levels stop at "high".
        let fitted = fit_request_to_model(&req("gemini-3.8-flash"));
        assert_eq!(fitted.reasoning_effort, None);
        assert_eq!(fitted.temperature, Some(0.3));
        // Models without an effort setting never receive one.
        assert_eq!(fit_request_to_model(&req("glm-5.3")).reasoning_effort, None);
    }

    #[test]
    fn output_limit_is_capped_and_unknown_models_pass_through() {
        let mut big = req("gemini-3.8-flash");
        big.max_output_tokens = Some(500_000);
        assert_eq!(fit_request_to_model(&big).max_output_tokens, Some(65_536));

        let custom = req("some-local-model");
        assert_eq!(fit_request_to_model(&custom), custom);
    }
}
