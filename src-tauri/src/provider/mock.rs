//! Scriptable mock provider for offline integration testing per FR-PRV-01 and AGENTS.md §11.

use std::collections::VecDeque;
use std::sync::Mutex;

use crate::error::AppError;

use super::types::{
    FinishReason, Provider, ProviderOutcome, ProviderRequest, ProviderResponse, TokenUsage,
};

/// A scripted outcome or error returned by `MockProvider`.
#[derive(Debug, Clone)]
pub enum MockStep {
    Success {
        text: String,
        finish: FinishReason,
        usage: TokenUsage,
    },
    Refused {
        reason: String,
    },
    ContentFiltered {
        reason: String,
    },
    LengthTruncated {
        text: String,
        usage: TokenUsage,
    },
    Error(AppError),
    StreamChunks {
        chunks: Vec<String>,
        finish: FinishReason,
        usage: TokenUsage,
    },
}

impl MockStep {
    /// Helper creating a standard successful translation response.
    pub fn simple_success(text: impl Into<String>) -> Self {
        Self::Success {
            text: text.into(),
            finish: FinishReason::Stop,
            usage: TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                ..Default::default()
            },
        }
    }
}

/// A scriptable mock provider implementation.
pub struct MockProvider {
    queue: Mutex<VecDeque<MockStep>>,
    calls: Mutex<Vec<ProviderRequest>>,
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            calls: Mutex::new(Vec::new()),
        }
    }

    /// Appends a scripted step to the response queue.
    pub fn push_step(&self, step: MockStep) {
        if let Ok(mut q) = self.queue.lock() {
            q.push_back(step);
        }
    }

    /// Sets the entire queue of scripted responses.
    pub fn set_steps(&self, steps: impl IntoIterator<Item = MockStep>) {
        if let Ok(mut q) = self.queue.lock() {
            q.clear();
            q.extend(steps);
        }
    }

    /// Returns a snapshot of all requests received by this mock.
    pub fn calls(&self) -> Vec<ProviderRequest> {
        self.calls.lock().map(|c| c.clone()).unwrap_or_default()
    }

    /// Clears the recorded call history.
    pub fn clear_calls(&self) {
        if let Ok(mut c) = self.calls.lock() {
            c.clear();
        }
    }
}

impl Provider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn send_request(
        &self,
        req: &ProviderRequest,
        mut token_cb: Option<&mut (dyn FnMut(&str) + Send)>,
    ) -> Result<ProviderOutcome, AppError> {
        // Record call
        if let Ok(mut calls) = self.calls.lock() {
            calls.push(req.clone());
        }

        // Pop next step
        let step = {
            let mut q = self.queue.lock().map_err(|e| AppError::Internal {
                message: format!("Mock mutex poisoned: {e}"),
            })?;
            q.pop_front().unwrap_or_else(|| {
                // Default fallback if queue is empty
                MockStep::simple_success("<s id=\"1\">Default mock response</s>")
            })
        };

        match step {
            MockStep::Success {
                text,
                finish,
                usage,
            } => {
                if let Some(ref mut cb) = token_cb {
                    cb(&text);
                }
                Ok(ProviderOutcome::Success(ProviderResponse {
                    text,
                    finish_reason: finish,
                    usage,
                    provider_request_id: Some("mock-req-id".to_string()),
                    model: req.model.clone(),
                }))
            }
            MockStep::Refused { reason } => Ok(ProviderOutcome::Refused { reason }),
            MockStep::ContentFiltered { reason } => Ok(ProviderOutcome::ContentFiltered { reason }),
            MockStep::LengthTruncated { text, usage } => {
                if let Some(ref mut cb) = token_cb {
                    cb(&text);
                }
                Ok(ProviderOutcome::LengthTruncated(ProviderResponse {
                    text,
                    finish_reason: FinishReason::Length,
                    usage,
                    provider_request_id: Some("mock-req-id".to_string()),
                    model: req.model.clone(),
                }))
            }
            MockStep::Error(err) => Err(err),
            MockStep::StreamChunks {
                chunks,
                finish,
                usage,
            } => {
                let mut combined = String::new();
                for chunk in chunks {
                    if let Some(ref mut cb) = token_cb {
                        cb(&chunk);
                    }
                    combined.push_str(&chunk);
                }
                let resp = ProviderResponse {
                    text: combined,
                    finish_reason: finish,
                    usage,
                    provider_request_id: Some("mock-stream-id".to_string()),
                    model: req.model.clone(),
                };
                if finish == FinishReason::Length {
                    Ok(ProviderOutcome::LengthTruncated(resp))
                } else {
                    Ok(ProviderOutcome::Success(resp))
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider_scripted_sequence() {
        let mock = MockProvider::new();

        mock.push_step(MockStep::simple_success("<s id=\"1\">Pertama</s>"));
        mock.push_step(MockStep::Refused {
            reason: "Kebijakan konten".to_string(),
        });
        mock.push_step(MockStep::StreamChunks {
            chunks: vec!["<s id=\"2\">".to_string(), "Kedua</s>".to_string()],
            finish: FinishReason::Stop,
            usage: TokenUsage::default(),
        });

        let req = ProviderRequest {
            system_prompt: "Sys".to_string(),
            user_prompt: "User".to_string(),
            model: "mock-model".to_string(),
            temperature: None,
            max_output_tokens: None,
            reasoning_effort: None,
        };

        // Call 1: Success
        let res1 = mock.send_request(&req, None).await.unwrap();
        assert!(res1.is_success());
        assert_eq!(res1.text(), Some("<s id=\"1\">Pertama</s>"));

        // Call 2: Refused
        let res2 = mock.send_request(&req, None).await.unwrap();
        match res2 {
            ProviderOutcome::Refused { reason } => assert!(reason.contains("Kebijakan")),
            _ => panic!("Expected Refused"),
        }

        // Call 3: Streaming chunks
        let mut tokens = Vec::new();
        let mut cb = |t: &str| tokens.push(t.to_string());
        let res3 = mock.send_request(&req, Some(&mut cb)).await.unwrap();
        assert!(res3.is_success());
        assert_eq!(tokens, vec!["<s id=\"2\">", "Kedua</s>"]);

        // Verify call log
        assert_eq!(mock.calls().len(), 3);
    }
}
