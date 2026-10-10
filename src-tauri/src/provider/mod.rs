//! LNTA provider abstraction, adapters, retry mechanism, and mock provider per PRD §7.1 and FR-PRV-01..09.

pub mod anthropic;
pub mod gemini;
pub mod mock;
pub mod openai_compat;
pub mod retry;
pub mod types;

pub use anthropic::{
    parse_recorded_anthropic_sse, AnthropicCacheControl, AnthropicConfig, AnthropicProvider,
    AnthropicSystemBlock,
};
pub use gemini::{
    parse_recorded_gemini_sse, permissive_safety_settings, GeminiConfig, GeminiProvider,
    GeminiSafetySetting,
};
pub use mock::{MockProvider, MockStep};
pub use openai_compat::{
    parse_recorded_sse_stream, parse_sse_line, OpenAiCompatConfig, OpenAiCompatProvider,
};
pub use retry::{calculate_retry_delay, is_retryable_status, RetryConfig};
pub use types::{
    FinishReason, Provider, ProviderOutcome, ProviderRequest, ProviderResponse, TokenUsage,
};
