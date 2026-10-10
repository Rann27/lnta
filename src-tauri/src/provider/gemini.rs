//! Google Gemini adapter per FR-PRV-01, FR-PRV-04, and PRD §6.1.
//!
//! Streams `streamGenerateContent` via SSE, configures the most permissive safety thresholds
//! (`BLOCK_NONE`) to prevent unnecessary blocking on light-novel fiction, and maps cached token usage.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::retry::RetryConfig;
use super::types::{
    FinishReason, Provider, ProviderOutcome, ProviderRequest, ProviderResponse, TokenUsage,
};

/// Configuration for the Google Gemini API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub extra_headers: HashMap<String, String>,
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            base_url: "https://generativelanguage.googleapis.com".to_string(),
            api_key: None,
            extra_headers: HashMap::new(),
        }
    }
}

/// Safety setting for Gemini content filtering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeminiSafetySetting {
    pub category: &'static str,
    pub threshold: &'static str,
}

/// Permissive safety settings setting thresholds to `BLOCK_NONE` across all standard categories
/// per FR-PRV-04.
pub fn permissive_safety_settings() -> Vec<GeminiSafetySetting> {
    vec![
        GeminiSafetySetting {
            category: "HARM_CATEGORY_HARASSMENT",
            threshold: "BLOCK_NONE",
        },
        GeminiSafetySetting {
            category: "HARM_CATEGORY_HATE_SPEECH",
            threshold: "BLOCK_NONE",
        },
        GeminiSafetySetting {
            category: "HARM_CATEGORY_SEXUALLY_EXPLICIT",
            threshold: "BLOCK_NONE",
        },
        GeminiSafetySetting {
            category: "HARM_CATEGORY_DANGEROUS_CONTENT",
            threshold: "BLOCK_NONE",
        },
        GeminiSafetySetting {
            category: "HARM_CATEGORY_CIVIC_INTEGRITY",
            threshold: "BLOCK_NONE",
        },
    ]
}

/// Text part for Gemini requests.
#[derive(Debug, Serialize)]
pub struct GeminiPart<'a> {
    pub text: &'a str,
}

/// Content object with role and parts.
#[derive(Debug, Serialize)]
pub struct GeminiContent<'a> {
    pub role: &'a str,
    pub parts: Vec<GeminiPart<'a>>,
}

/// System instruction container.
#[derive(Debug, Serialize)]
pub struct GeminiSystemInstruction<'a> {
    pub parts: Vec<GeminiPart<'a>>,
}

/// Generation configuration parameters.
#[derive(Debug, Serialize)]
pub struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(rename = "maxOutputTokens", skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
}

/// Request body for `POST /v1beta/models/{model}:streamGenerateContent?alt=sse`.
#[derive(Debug, Serialize)]
pub struct GeminiRequestBody<'a> {
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<GeminiSystemInstruction<'a>>,
    pub contents: Vec<GeminiContent<'a>>,
    #[serde(rename = "safetySettings")]
    pub safety_settings: Vec<GeminiSafetySetting>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GeminiGenerationConfig>,
}

// Internal SSE deserialization structures.

#[derive(Debug, Deserialize)]
pub struct GeminiResponseChunk {
    pub candidates: Option<Vec<GeminiCandidate>>,
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: Option<GeminiUsageMetadata>,
    #[serde(rename = "promptFeedback")]
    pub prompt_feedback: Option<GeminiPromptFeedback>,
    #[serde(rename = "modelVersion")]
    pub model_version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiPromptFeedback {
    #[serde(rename = "blockReason")]
    pub block_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiCandidate {
    pub content: Option<GeminiCandidateContent>,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
    #[serde(rename = "safetyRatings")]
    pub safety_ratings: Option<Vec<GeminiSafetyRating>>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiCandidateContent {
    pub parts: Option<Vec<GeminiCandidatePart>>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiCandidatePart {
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiSafetyRating {
    pub category: Option<String>,
    pub probability: Option<String>,
    pub blocked: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    pub prompt_token_count: Option<i64>,
    #[serde(rename = "candidatesTokenCount")]
    pub candidates_token_count: Option<i64>,
    #[serde(rename = "cachedContentTokenCount")]
    pub cached_content_token_count: Option<i64>,
}

/// Parses an entire recorded Gemini SSE stream into a typed outcome.
///
/// Incremental text tokens are forwarded to `token_cb` as they appear.
pub fn parse_recorded_gemini_sse(
    sse_text: &str,
    model_override: Option<&str>,
    mut token_cb: Option<&mut (dyn FnMut(&str) + Send)>,
) -> Result<ProviderOutcome, AppError> {
    let mut accumulated_text = String::new();
    let mut finish_reason = FinishReason::Stop;
    let mut content_filter_reason: Option<String> = None;
    let mut usage = TokenUsage::default();
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

        let chunk: GeminiResponseChunk =
            serde_json::from_str(payload).map_err(|e| AppError::Internal {
                message: format!("Failed to parse Gemini SSE JSON: {e}"),
            })?;

        if let Some(mv) = chunk.model_version {
            model_name = mv;
        }

        if let Some(pf) = chunk.prompt_feedback {
            if let Some(reason) = pf.block_reason {
                return Ok(ProviderOutcome::ContentFiltered {
                    reason: format!("Prompt blocked: {reason}"),
                });
            }
        }

        if let Some(candidates) = chunk.candidates {
            for candidate in candidates {
                if let Some(reason_str) = candidate.finish_reason {
                    match reason_str.as_str() {
                        "MAX_TOKENS" => finish_reason = FinishReason::Length,
                        "SAFETY" | "RECITATION" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "SPII" => {
                            finish_reason = FinishReason::ContentFilter;
                            content_filter_reason = Some(reason_str.clone());
                        }
                        _ => finish_reason = FinishReason::Stop,
                    }
                }

                if let Some(ratings) = candidate.safety_ratings {
                    for rating in ratings {
                        if rating.blocked.unwrap_or(false) {
                            finish_reason = FinishReason::ContentFilter;
                            content_filter_reason =
                                Some("Candidate blocked by safety rating".to_string());
                        }
                    }
                }

                if let Some(content) = candidate.content {
                    if let Some(parts) = content.parts {
                        for part in parts {
                            if let Some(text) = part.text {
                                if !text.is_empty() {
                                    if let Some(ref mut cb) = token_cb {
                                        cb(&text);
                                    }
                                    accumulated_text.push_str(&text);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(u) = chunk.usage_metadata {
            usage.input_tokens = u.prompt_token_count.unwrap_or(0);
            usage.output_tokens = u.candidates_token_count.unwrap_or(0);
            usage.cached_input_tokens = u.cached_content_token_count.unwrap_or(0);
        }
    }

    if let Some(reason) = content_filter_reason {
        return Ok(ProviderOutcome::ContentFiltered { reason });
    }

    let response = ProviderResponse {
        text: accumulated_text,
        finish_reason,
        usage,
        provider_request_id: None,
        model: model_name,
    };

    if finish_reason == FinishReason::Length {
        Ok(ProviderOutcome::LengthTruncated(response))
    } else {
        Ok(ProviderOutcome::Success(response))
    }
}

/// Google Gemini API provider implementation.
pub struct GeminiProvider {
    config: GeminiConfig,
    client: reqwest::Client,
    retry_config: RetryConfig,
}

impl GeminiProvider {
    pub fn new(config: GeminiConfig, retry_config: Option<RetryConfig>) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            retry_config: retry_config.unwrap_or_default(),
        }
    }

    /// Builds the request body with permissive safety settings per FR-PRV-04.
    pub fn build_body<'a>(&self, req: &'a ProviderRequest) -> GeminiRequestBody<'a> {
        let system_instruction = if req.system_prompt.is_empty() {
            None
        } else {
            Some(GeminiSystemInstruction {
                parts: vec![GeminiPart {
                    text: &req.system_prompt,
                }],
            })
        };

        let generation_config = if req.temperature.is_some() || req.max_output_tokens.is_some() {
            Some(GeminiGenerationConfig {
                temperature: req.temperature,
                max_output_tokens: req.max_output_tokens,
            })
        } else {
            None
        };

        GeminiRequestBody {
            system_instruction,
            contents: vec![GeminiContent {
                role: "user",
                parts: vec![GeminiPart {
                    text: &req.user_prompt,
                }],
            }],
            safety_settings: permissive_safety_settings(),
            generation_config,
        }
    }
}

impl Provider for GeminiProvider {
    fn name(&self) -> &'static str {
        "gemini"
    }

    async fn send_request(
        &self,
        req: &ProviderRequest,
        token_cb: Option<&mut (dyn FnMut(&str) + Send)>,
    ) -> Result<ProviderOutcome, AppError> {
        let url = format!(
            "{}/v1beta/models/{}:streamGenerateContent?alt=sse",
            self.config.base_url.trim_end_matches('/'),
            req.model
        );
        let body = self.build_body(req);

        let mut attempt = 1;
        let mut last_error: Option<AppError> = None;

        while attempt <= self.retry_config.max_attempts {
            let mut request_builder = self
                .client
                .post(&url)
                .header("content-type", "application/json");

            if let Some(ref key) = self.config.api_key {
                request_builder = request_builder.header("x-goog-api-key", key);
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
                            reason: format!("Failed to read Gemini stream: {e}"),
                        })?;
                        return parse_recorded_gemini_sse(&text, Some(&req.model), token_cb);
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
                        reason: format!("Gemini connection failed: {e}"),
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
    fn test_permissive_safety_settings_contain_block_none() {
        let settings = permissive_safety_settings();
        assert_eq!(settings.len(), 5);
        for setting in &settings {
            assert_eq!(setting.threshold, "BLOCK_NONE");
        }
    }

    #[test]
    fn test_parse_recorded_gemini_sse_with_cached_tokens() {
        let fixture = r#"
data: {"candidates":[{"content":{"parts":[{"text":"<t>Pagi</t>\n"}],"role":"model"},"finishReason":"STOP","index":0}],"modelVersion":"gemini-1.5-pro"}

data: {"candidates":[{"content":{"parts":[{"text":"<s id=\"1\">Matahari terbit.</s>"}],"role":"model"},"finishReason":"STOP","index":0}],"usageMetadata":{"promptTokenCount":1500,"candidatesTokenCount":25,"totalTokenCount":1525,"cachedContentTokenCount":1200},"modelVersion":"gemini-1.5-pro"}
"#;

        let mut streamed_tokens = Vec::new();
        let mut cb = |token: &str| {
            streamed_tokens.push(token.to_string());
        };

        let outcome = parse_recorded_gemini_sse(fixture, None, Some(&mut cb)).unwrap();
        assert!(outcome.is_success());

        if let ProviderOutcome::Success(resp) = outcome {
            assert_eq!(resp.text, "<t>Pagi</t>\n<s id=\"1\">Matahari terbit.</s>");
            assert_eq!(resp.finish_reason, FinishReason::Stop);
            assert_eq!(resp.model, "gemini-1.5-pro");
            assert_eq!(resp.usage.input_tokens, 1500);
            assert_eq!(resp.usage.output_tokens, 25);
            assert_eq!(resp.usage.cached_input_tokens, 1200);
        } else {
            panic!("Expected ProviderOutcome::Success");
        }

        assert_eq!(streamed_tokens.len(), 2);
    }

    #[test]
    fn test_parse_recorded_gemini_sse_length_truncated() {
        let fixture = r#"
data: {"candidates":[{"content":{"parts":[{"text":"<s id=\"1\">Terpotong"}],"role":"model"},"finishReason":"MAX_TOKENS","index":0}],"usageMetadata":{"promptTokenCount":100,"candidatesTokenCount":8192,"totalTokenCount":8292},"modelVersion":"gemini-1.5-flash"}
"#;

        let outcome = parse_recorded_gemini_sse(fixture, None, None).unwrap();
        match outcome {
            ProviderOutcome::LengthTruncated(resp) => {
                assert_eq!(resp.text, "<s id=\"1\">Terpotong");
                assert_eq!(resp.finish_reason, FinishReason::Length);
                assert_eq!(resp.usage.output_tokens, 8192);
            }
            _ => panic!("Expected LengthTruncated"),
        }
    }

    #[test]
    fn test_parse_recorded_gemini_sse_safety_content_filter() {
        let fixture = r#"
data: {"candidates":[{"finishReason":"SAFETY","index":0,"safetyRatings":[{"category":"HARM_CATEGORY_SEXUALLY_EXPLICIT","probability":"HIGH","blocked":true}]}],"modelVersion":"gemini-1.5-flash"}
"#;

        let outcome = parse_recorded_gemini_sse(fixture, None, None).unwrap();
        match outcome {
            ProviderOutcome::ContentFiltered { reason } => {
                assert!(reason.contains("SAFETY") || reason.contains("blocked"));
            }
            _ => panic!("Expected ContentFiltered"),
        }
    }

    #[test]
    fn test_parse_recorded_gemini_sse_prompt_feedback_blocked() {
        let fixture = r#"
data: {"promptFeedback":{"blockReason":"SAFETY"}}
"#;

        let outcome = parse_recorded_gemini_sse(fixture, None, None).unwrap();
        match outcome {
            ProviderOutcome::ContentFiltered { reason } => {
                assert!(reason.contains("SAFETY"));
            }
            _ => panic!("Expected ContentFiltered"),
        }
    }
}
