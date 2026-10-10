//! OpenAI-compatible Chat Completions adapter per FR-PRV-01, FR-PRV-02, and FR-PRV-09.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::retry::RetryConfig;
use super::types::{
    FinishReason, Provider, ProviderOutcome, ProviderRequest, ProviderResponse, TokenUsage,
};

/// Configuration for an OpenAI-compatible endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiCompatConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub extra_headers: HashMap<String, String>,
}

impl Default for OpenAiCompatConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: None,
            extra_headers: HashMap::new(),
        }
    }
}

/// JSON payload sent to `/chat/completions`.
#[derive(Debug, Serialize)]
struct ChatCompletionRequestMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequestBody<'a> {
    model: &'a str,
    messages: Vec<ChatCompletionRequestMessage<'a>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<&'a str>,
    stream_options: ChatCompletionStreamOptions,
}

#[derive(Debug, Serialize)]
struct ChatCompletionStreamOptions {
    include_usage: bool,
}

/// SSE Chunk JSON deserialization structures.
#[derive(Debug, Deserialize)]
pub struct OpenAiPromptTokensDetails {
    pub cached_tokens: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAiCompletionTokensDetails {
    pub reasoning_tokens: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAiUsage {
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub prompt_tokens_details: Option<OpenAiPromptTokensDetails>,
    pub completion_tokens_details: Option<OpenAiCompletionTokensDetails>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAiDelta {
    pub content: Option<String>,
    pub refusal: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAiChoice {
    pub delta: Option<OpenAiDelta>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAiChunk {
    pub id: Option<String>,
    pub model: Option<String>,
    pub choices: Option<Vec<OpenAiChoice>>,
    pub usage: Option<OpenAiUsage>,
}

/// Parses a single SSE line into an optional `OpenAiChunk`.
pub fn parse_sse_line(line: &str) -> Result<Option<OpenAiChunk>, AppError> {
    let trimmed = line.trim();
    if !trimmed.starts_with("data:") {
        return Ok(None);
    }

    let payload = trimmed["data:".len()..].trim();
    if payload.is_empty() || payload == "[DONE]" {
        return Ok(None);
    }

    let chunk = serde_json::from_str::<OpenAiChunk>(payload).map_err(|e| AppError::Internal {
        message: format!("Failed to parse SSE JSON chunk: {e}"),
    })?;

    Ok(Some(chunk))
}

/// Parses an entire recorded SSE stream into a typed `ProviderOutcome`.
///
/// Calls `token_cb` incrementally for each non-empty content delta.
pub fn parse_recorded_sse_stream(
    sse_text: &str,
    model_override: Option<&str>,
    mut token_cb: Option<&mut (dyn FnMut(&str) + Send)>,
) -> Result<ProviderOutcome, AppError> {
    let mut accumulated_text = String::new();
    let mut finish_reason = FinishReason::Stop;
    let mut refusal_reason: Option<String> = None;
    let mut content_filter_reason: Option<String> = None;
    let mut usage = TokenUsage::default();
    let mut request_id: Option<String> = None;
    let mut model_name = model_override.unwrap_or("unknown").to_string();

    for line in sse_text.lines() {
        if let Some(chunk) = parse_sse_line(line)? {
            if let Some(id) = chunk.id {
                request_id = Some(id);
            }
            if let Some(m) = chunk.model {
                model_name = m;
            }

            if let Some(choices) = chunk.choices {
                for choice in choices {
                    if let Some(reason_str) = choice.finish_reason {
                        match reason_str.as_str() {
                            "length" => finish_reason = FinishReason::Length,
                            "content_filter" => {
                                finish_reason = FinishReason::ContentFilter;
                                content_filter_reason = Some("content_filter".to_string());
                            }
                            "refusal" => {
                                finish_reason = FinishReason::Refusal;
                                refusal_reason = Some("refusal".to_string());
                            }
                            _ => finish_reason = FinishReason::Stop,
                        }
                    }

                    if let Some(delta) = choice.delta {
                        if let Some(refusal) = delta.refusal {
                            refusal_reason = Some(refusal);
                            finish_reason = FinishReason::Refusal;
                        }

                        if let Some(content) = delta.content {
                            if !content.is_empty() {
                                if let Some(ref mut cb) = token_cb {
                                    cb(&content);
                                }
                                accumulated_text.push_str(&content);
                            }
                        }
                    }
                }
            }

            if let Some(u) = chunk.usage {
                usage.input_tokens = u.prompt_tokens.unwrap_or(0);
                usage.output_tokens = u.completion_tokens.unwrap_or(0);
                if let Some(details) = u.prompt_tokens_details {
                    usage.cached_input_tokens = details.cached_tokens.unwrap_or(0);
                }
                if let Some(details) = u.completion_tokens_details {
                    usage.reasoning_tokens = details.reasoning_tokens.unwrap_or(0);
                }
            }
        }
    }

    if let Some(reason) = refusal_reason {
        return Ok(ProviderOutcome::Refused { reason });
    }

    if let Some(reason) = content_filter_reason {
        return Ok(ProviderOutcome::ContentFiltered { reason });
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

/// OpenAI-compatible provider implementation.
pub struct OpenAiCompatProvider {
    config: OpenAiCompatConfig,
    client: reqwest::Client,
    retry_config: RetryConfig,
}

impl OpenAiCompatProvider {
    pub fn new(config: OpenAiCompatConfig, retry_config: RetryConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            retry_config,
        }
    }

    pub fn retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }
}

impl Provider for OpenAiCompatProvider {
    fn name(&self) -> &'static str {
        "openai_compat"
    }

    async fn send_request(
        &self,
        req: &ProviderRequest,
        token_cb: Option<&mut (dyn FnMut(&str) + Send)>,
    ) -> Result<ProviderOutcome, AppError> {
        let endpoint = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );

        let messages = vec![
            ChatCompletionRequestMessage {
                role: "system",
                content: &req.system_prompt,
            },
            ChatCompletionRequestMessage {
                role: "user",
                content: &req.user_prompt,
            },
        ];

        let body = ChatCompletionRequestBody {
            model: &req.model,
            messages,
            stream: true,
            temperature: req.temperature,
            max_tokens: req.max_output_tokens,
            reasoning_effort: req.reasoning_effort.as_deref(),
            stream_options: ChatCompletionStreamOptions {
                include_usage: true,
            },
        };

        let mut request_builder = self.client.post(&endpoint).json(&body);

        if let Some(ref key) = self.config.api_key {
            request_builder = request_builder.header("Authorization", format!("Bearer {key}"));
        }

        for (k, v) in &self.config.extra_headers {
            request_builder = request_builder.header(k, v);
        }

        let resp = request_builder
            .send()
            .await
            .map_err(|e| AppError::ProviderFailed {
                reason: format!("HTTP request error: {e}"),
            })?;

        let status = resp.status();
        if !status.is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            return Err(AppError::ProviderFailed {
                reason: format!("HTTP {} from {endpoint}: {error_text}", status.as_u16()),
            });
        }

        let full_sse_text = resp.text().await.map_err(|e| AppError::ProviderFailed {
            reason: format!("Failed to read SSE response stream: {e}"),
        })?;

        parse_recorded_sse_stream(&full_sse_text, Some(&req.model), token_cb)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_recorded_sse_stream_success_and_usage() {
        let sse_data = r#"data: {"id":"chatcmpl-01","model":"gpt-4o","choices":[{"index":0,"delta":{"content":"<s id=\"1\">"},"finish_reason":null}]}
data: {"id":"chatcmpl-01","choices":[{"index":0,"delta":{"content":"Pagi itu"},"finish_reason":null}]}
data: {"id":"chatcmpl-01","choices":[{"index":0,"delta":{"content":"</s>"},"finish_reason":null}]}
data: {"id":"chatcmpl-01","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":150,"completion_tokens":25,"prompt_tokens_details":{"cached_tokens":100},"completion_tokens_details":{"reasoning_tokens":10}}}
data: [DONE]"#;

        let mut tokens_received = Vec::new();
        let mut cb = |token: &str| {
            tokens_received.push(token.to_string());
        };

        let outcome = parse_recorded_sse_stream(sse_data, Some("gpt-4o"), Some(&mut cb)).unwrap();

        assert!(outcome.is_success());
        if let ProviderOutcome::Success(resp) = outcome {
            assert_eq!(resp.text, "<s id=\"1\">Pagi itu</s>");
            assert_eq!(resp.finish_reason, FinishReason::Stop);
            assert_eq!(resp.usage.input_tokens, 150);
            assert_eq!(resp.usage.output_tokens, 25);
            assert_eq!(resp.usage.cached_input_tokens, 100);
            assert_eq!(resp.usage.reasoning_tokens, 10);
            assert_eq!(resp.provider_request_id.as_deref(), Some("chatcmpl-01"));
        } else {
            panic!("Expected Success outcome");
        }

        assert_eq!(tokens_received, vec!["<s id=\"1\">", "Pagi itu", "</s>"]);
    }

    #[test]
    fn test_parse_recorded_sse_stream_length_truncated() {
        let sse_data = r#"data: {"id":"chatcmpl-02","choices":[{"index":0,"delta":{"content":"Teks terpotong"},"finish_reason":null}]}
data: {"id":"chatcmpl-02","choices":[{"index":0,"delta":{},"finish_reason":"length"}],"usage":{"prompt_tokens":50,"completion_tokens":10}}
data: [DONE]"#;

        let outcome = parse_recorded_sse_stream(sse_data, None, None).unwrap();
        match outcome {
            ProviderOutcome::LengthTruncated(resp) => {
                assert_eq!(resp.text, "Teks terpotong");
                assert_eq!(resp.finish_reason, FinishReason::Length);
            }
            _ => panic!("Expected LengthTruncated"),
        }
    }

    #[test]
    fn test_parse_recorded_sse_stream_refusal() {
        let sse_data = r#"data: {"id":"chatcmpl-03","choices":[{"index":0,"delta":{"refusal":"Maaf, saya menolak menerjemahkan teks ini."},"finish_reason":"refusal"}]}
data: [DONE]"#;

        let outcome = parse_recorded_sse_stream(sse_data, None, None).unwrap();
        match outcome {
            ProviderOutcome::Refused { reason } => {
                assert!(reason.contains("menolak"));
            }
            _ => panic!("Expected Refused"),
        }
    }

    #[test]
    fn test_parse_recorded_sse_stream_content_filter() {
        let sse_data = r#"data: {"id":"chatcmpl-04","choices":[{"index":0,"delta":{},"finish_reason":"content_filter"}]}
data: [DONE]"#;

        let outcome = parse_recorded_sse_stream(sse_data, None, None).unwrap();
        match outcome {
            ProviderOutcome::ContentFiltered { reason } => {
                assert_eq!(reason, "content_filter");
            }
            _ => panic!("Expected ContentFiltered"),
        }
    }
}
