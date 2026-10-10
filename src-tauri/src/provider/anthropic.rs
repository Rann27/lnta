//! Anthropic Messages adapter per FR-PRV-01, FR-PRV-03, and PRD §6.1.
//!
//! Sends ephemeral prompt cache control on the stable system prompt block,
//! streams SSE deltas, and maps token usage including cache creation and read tokens.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::retry::RetryConfig;
use super::types::{
    FinishReason, Provider, ProviderOutcome, ProviderRequest, ProviderResponse, TokenUsage,
};

/// Configuration for the Anthropic Messages API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub anthropic_version: String,
    pub extra_headers: HashMap<String, String>,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.anthropic.com".to_string(),
            api_key: None,
            anthropic_version: "2023-06-01".to_string(),
            extra_headers: HashMap::new(),
        }
    }
}

/// Cache control specification for Anthropic prompt caching.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnthropicCacheControl {
    #[serde(rename = "type")]
    pub control_type: String,
}

impl Default for AnthropicCacheControl {
    fn default() -> Self {
        Self {
            control_type: "ephemeral".to_string(),
        }
    }
}

/// Structured system block with optional cache control breakpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnthropicSystemBlock<'a> {
    #[serde(rename = "type")]
    pub block_type: &'a str,
    pub text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<AnthropicCacheControl>,
}

/// Message element sent in the `messages` array.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnthropicMessage<'a> {
    pub role: &'a str,
    pub content: &'a str,
}

/// Request body for `POST /v1/messages`.
#[derive(Debug, Serialize)]
pub struct AnthropicRequestBody<'a> {
    pub model: &'a str,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub system: Vec<AnthropicSystemBlock<'a>>,
    pub messages: Vec<AnthropicMessage<'a>>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

// Internal SSE deserialization structures.

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: AnthropicStartMessage },
    #[serde(rename = "content_block_start")]
    ContentBlockStart,
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { delta: AnthropicDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop,
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: Option<AnthropicMessageDelta>,
        usage: Option<AnthropicDeltaUsage>,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "error")]
    Error { error: AnthropicStreamError },
}

#[derive(Debug, Deserialize)]
struct AnthropicStartMessage {
    id: Option<String>,
    model: Option<String>,
    usage: Option<AnthropicStartUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicStartUsage {
    input_tokens: Option<i64>,
    cache_creation_input_tokens: Option<i64>,
    cache_read_input_tokens: Option<i64>,
    output_tokens: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AnthropicDelta {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicMessageDelta {
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicDeltaUsage {
    output_tokens: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamError {
    message: Option<String>,
}

/// Parses an entire recorded Anthropic SSE stream into a typed outcome.
///
/// Incremental text tokens are forwarded to `token_cb` as they appear.
pub fn parse_recorded_anthropic_sse(
    sse_text: &str,
    model_override: Option<&str>,
    mut token_cb: Option<&mut (dyn FnMut(&str) + Send)>,
) -> Result<ProviderOutcome, AppError> {
    let mut accumulated_text = String::new();
    let mut finish_reason = FinishReason::Stop;
    let mut refusal_reason: Option<String> = None;
    let mut usage = TokenUsage::default();
    let mut request_id: Option<String> = None;
    let mut model_name = model_override.unwrap_or("unknown").to_string();

    for line in sse_text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("data:") {
            continue;
        }

        let payload = trimmed["data:".len()..].trim();
        if payload.is_empty() || payload == "[DONE]" {
            continue;
        }

        let event: AnthropicStreamEvent =
            serde_json::from_str(payload).map_err(|e| AppError::Internal {
                message: format!("Failed to parse Anthropic SSE JSON: {e}"),
            })?;

        match event {
            AnthropicStreamEvent::MessageStart { message } => {
                if let Some(id) = message.id {
                    request_id = Some(id);
                }
                if let Some(m) = message.model {
                    model_name = m;
                }
                if let Some(u) = message.usage {
                    usage.input_tokens = u.input_tokens.unwrap_or(0);
                    usage.cache_write_tokens = u.cache_creation_input_tokens.unwrap_or(0);
                    usage.cached_input_tokens = u.cache_read_input_tokens.unwrap_or(0);
                    if let Some(out) = u.output_tokens {
                        usage.output_tokens = out;
                    }
                }
            }
            AnthropicStreamEvent::ContentBlockDelta { delta } => {
                if let Some(text) = delta.text {
                    if !text.is_empty() {
                        if let Some(ref mut cb) = token_cb {
                            cb(&text);
                        }
                        accumulated_text.push_str(&text);
                    }
                }
            }
            AnthropicStreamEvent::MessageDelta {
                delta,
                usage: u_opt,
            } => {
                if let Some(d) = delta {
                    if let Some(reason_str) = d.stop_reason {
                        match reason_str.as_str() {
                            "max_tokens" => finish_reason = FinishReason::Length,
                            "refusal" => {
                                finish_reason = FinishReason::Refusal;
                                refusal_reason = Some("refusal".to_string());
                            }
                            _ => finish_reason = FinishReason::Stop,
                        }
                    }
                }
                if let Some(u) = u_opt {
                    if let Some(out) = u.output_tokens {
                        usage.output_tokens = out;
                    }
                }
            }
            AnthropicStreamEvent::Error { error } => {
                let msg = error
                    .message
                    .unwrap_or_else(|| "Unknown Anthropic stream error".to_string());
                return Err(AppError::ProviderFailed {
                    reason: format!("Anthropic stream error: {msg}"),
                });
            }
            AnthropicStreamEvent::ContentBlockStart
            | AnthropicStreamEvent::ContentBlockStop
            | AnthropicStreamEvent::MessageStop
            | AnthropicStreamEvent::Ping => {}
        }
    }

    if let Some(reason) = refusal_reason {
        return Ok(ProviderOutcome::Refused { reason });
    }

    let response = ProviderResponse {
        text: accumulated_text,
        finish_reason,
        usage,
        provider_request_id: request_id,
        model: model_name,
    };

    if finish_reason == FinishReason::Length {
        Ok(ProviderOutcome::LengthTruncated(response))
    } else {
        Ok(ProviderOutcome::Success(response))
    }
}

/// Anthropic Messages API provider implementation.
pub struct AnthropicProvider {
    config: AnthropicConfig,
    client: reqwest::Client,
    retry_config: RetryConfig,
}

impl AnthropicProvider {
    pub fn new(config: AnthropicConfig, retry_config: Option<RetryConfig>) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            retry_config: retry_config.unwrap_or_default(),
        }
    }

    /// Builds the request body with system prompt caching per PRD §6.1.
    pub fn build_body<'a>(&self, req: &'a ProviderRequest) -> AnthropicRequestBody<'a> {
        let system = if req.system_prompt.is_empty() {
            Vec::new()
        } else {
            vec![AnthropicSystemBlock {
                block_type: "text",
                text: &req.system_prompt,
                cache_control: Some(AnthropicCacheControl::default()),
            }]
        };

        AnthropicRequestBody {
            model: &req.model,
            max_tokens: req.max_output_tokens.unwrap_or(4096),
            system,
            messages: vec![AnthropicMessage {
                role: "user",
                content: &req.user_prompt,
            }],
            stream: true,
            temperature: req.temperature,
        }
    }
}

impl Provider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    async fn send_request(
        &self,
        req: &ProviderRequest,
        token_cb: Option<&mut (dyn FnMut(&str) + Send)>,
    ) -> Result<ProviderOutcome, AppError> {
        let url = format!("{}/v1/messages", self.config.base_url.trim_end_matches('/'));
        let body = self.build_body(req);

        let mut attempt = 1;
        let mut last_error: Option<AppError> = None;

        while attempt <= self.retry_config.max_attempts {
            let mut request_builder = self
                .client
                .post(&url)
                .header("anthropic-version", &self.config.anthropic_version)
                .header("content-type", "application/json");

            if let Some(ref key) = self.config.api_key {
                request_builder = request_builder.header("x-api-key", key);
            }

            for (k, v) in &self.config.extra_headers {
                request_builder = request_builder.header(k, v);
            }

            let response_result = request_builder.json(&body).send().await;

            match response_result {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        let text = resp.text().await.map_err(|e| AppError::ProviderFailed {
                            reason: format!("Failed to read Anthropic stream: {e}"),
                        })?;
                        return parse_recorded_anthropic_sse(&text, Some(&req.model), token_cb);
                    }

                    let retry_after_secs = resp
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u64>().ok());

                    let error_body = resp.text().await.unwrap_or_default();
                    let app_err = AppError::ProviderFailed {
                        reason: format!("HTTP {} from {url}: {error_body}", status.as_u16()),
                    };

                    if super::retry::is_retryable_status(status.as_u16())
                        && attempt < self.retry_config.max_attempts
                    {
                        let delay = super::retry::calculate_retry_delay(
                            attempt,
                            retry_after_secs,
                            &self.retry_config,
                        );
                        tokio::time::sleep(delay).await;
                        attempt += 1;
                        last_error = Some(app_err);
                        continue;
                    }

                    return Err(app_err);
                }
                Err(e) => {
                    let app_err = AppError::ProviderFailed {
                        reason: format!("Anthropic connection failed: {e}"),
                    };

                    if attempt < self.retry_config.max_attempts {
                        let delay =
                            super::retry::calculate_retry_delay(attempt, None, &self.retry_config);
                        tokio::time::sleep(delay).await;
                        attempt += 1;
                        last_error = Some(app_err);
                        continue;
                    }

                    return Err(app_err);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AppError::ProviderFailed {
            reason: "Retries exhausted".to_string(),
        }))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_carries_cache_control() {
        let provider = AnthropicProvider::new(AnthropicConfig::default(), None);
        let req = ProviderRequest {
            system_prompt: "Contract and instruction text".to_string(),
            user_prompt: "Chapter segments".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            temperature: Some(0.2),
            max_output_tokens: Some(4096),
            reasoning_effort: None,
        };

        let body = provider.build_body(&req);
        assert_eq!(body.system.len(), 1);
        assert_eq!(body.system[0].text, "Contract and instruction text");
        assert_eq!(
            body.system[0].cache_control,
            Some(AnthropicCacheControl {
                control_type: "ephemeral".to_string()
            })
        );
    }

    #[test]
    fn test_parse_recorded_anthropic_sse_with_cache_usage() {
        let fixture = r#"
event: message_start
data: {"type":"message_start","message":{"id":"msg_013Zva2CMHLNnx","type":"message","role":"assistant","content":[],"model":"claude-3-5-sonnet-20241022","stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":120,"cache_creation_input_tokens":2500,"cache_read_input_tokens":1800,"output_tokens":1}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"<t>Judul</t>\n"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"<s id=\"1\">Teks terjemahan</s>"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":42}}

event: message_stop
data: {"type":"message_stop"}
"#;

        let mut streamed_tokens = Vec::new();
        let mut cb = |token: &str| {
            streamed_tokens.push(token.to_string());
        };

        let outcome = parse_recorded_anthropic_sse(fixture, None, Some(&mut cb)).unwrap();
        assert!(outcome.is_success());

        if let ProviderOutcome::Success(resp) = outcome {
            assert_eq!(resp.text, "<t>Judul</t>\n<s id=\"1\">Teks terjemahan</s>");
            assert_eq!(resp.finish_reason, FinishReason::Stop);
            assert_eq!(
                resp.provider_request_id.as_deref(),
                Some("msg_013Zva2CMHLNnx")
            );
            assert_eq!(resp.model, "claude-3-5-sonnet-20241022");
            assert_eq!(resp.usage.input_tokens, 120);
            assert_eq!(resp.usage.cache_write_tokens, 2500);
            assert_eq!(resp.usage.cached_input_tokens, 1800);
            assert_eq!(resp.usage.output_tokens, 42);
        } else {
            panic!("Expected ProviderOutcome::Success");
        }

        assert_eq!(streamed_tokens.len(), 2);
    }

    #[test]
    fn test_parse_recorded_anthropic_sse_max_tokens_truncated() {
        let fixture = r#"
event: message_start
data: {"type":"message_start","message":{"id":"msg_trunc","type":"message","role":"assistant","content":[],"model":"claude-3-5-sonnet-20241022","stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":50,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":1}}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"<s id=\"1\">Terkutub"}}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"max_tokens","stop_sequence":null},"usage":{"output_tokens":4096}}

event: message_stop
data: {"type":"message_stop"}
"#;

        let outcome = parse_recorded_anthropic_sse(fixture, None, None).unwrap();
        match outcome {
            ProviderOutcome::LengthTruncated(resp) => {
                assert_eq!(resp.text, "<s id=\"1\">Terkutub");
                assert_eq!(resp.finish_reason, FinishReason::Length);
                assert_eq!(resp.usage.output_tokens, 4096);
            }
            _ => panic!("Expected LengthTruncated"),
        }
    }

    #[test]
    fn test_parse_recorded_anthropic_sse_refusal() {
        let fixture = r#"
event: message_start
data: {"type":"message_start","message":{"id":"msg_ref","type":"message","role":"assistant","content":[],"model":"claude-3-5-sonnet-20241022","stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":0}}}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"refusal","stop_sequence":null},"usage":{"output_tokens":0}}

event: message_stop
data: {"type":"message_stop"}
"#;

        let outcome = parse_recorded_anthropic_sse(fixture, None, None).unwrap();
        match outcome {
            ProviderOutcome::Refused { reason } => {
                assert_eq!(reason, "refusal");
            }
            _ => panic!("Expected Refused"),
        }
    }
}
