//! Common provider types, traits, usage accounting, and typed outcomes per FR-PRV-01 and FR-PRV-09.

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Reasons why a model completed its response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    Refusal,
    Error,
}

/// Token usage accounting per FR-PRV-01 and FR-USG-01.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub cache_write_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_tokens: i64,
}

/// A request sent to an LLM provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderRequest {
    pub system_prompt: String,
    pub user_prompt: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<u32>,
    pub reasoning_effort: Option<String>,
}

/// A successful response from a provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub text: String,
    pub finish_reason: FinishReason,
    pub usage: TokenUsage,
    pub provider_request_id: Option<String>,
    pub model: String,
}

/// Typed outcome of a provider invocation per FR-PRV-09 and PRD §6.3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderOutcome {
    Success(ProviderResponse),
    Refused { reason: String },
    ContentFiltered { reason: String },
    LengthTruncated(ProviderResponse),
}

impl ProviderOutcome {
    /// Returns true if the call produced a complete, successful translation.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    /// Extracts the response text if available.
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Success(r) | Self::LengthTruncated(r) => Some(&r.text),
            Self::Refused { .. } | Self::ContentFiltered { .. } => None,
        }
    }

    /// Extracts usage if available.
    pub fn usage(&self) -> Option<&TokenUsage> {
        match self {
            Self::Success(r) | Self::LengthTruncated(r) => Some(&r.usage),
            Self::Refused { .. } | Self::ContentFiltered { .. } => None,
        }
    }
}

/// Common trait implemented by all model provider adapters.
pub trait Provider: Send + Sync {
    /// Returns the provider identifier (e.g. "openai_compat", "anthropic", "gemini", "mock").
    fn name(&self) -> &'static str;

    /// Sends a prompt to the provider with an optional streaming text callback.
    fn send_request(
        &self,
        req: &ProviderRequest,
        token_cb: Option<&mut (dyn FnMut(&str) + Send)>,
    ) -> impl std::future::Future<Output = Result<ProviderOutcome, AppError>> + Send;
}
