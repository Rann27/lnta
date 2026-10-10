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
    /// Answers every segment id found in the request's `<chapter>` block, so a whole
    /// volume can run offline. Dialogue lines keep 「」 to exercise the mechanical fixer.
    Echo,
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
    echo_when_empty: bool,
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
            echo_when_empty: false,
        }
    }

    /// A mock that answers with `MockStep::Echo` whenever its queue is empty.
    pub fn new_echo() -> Self {
        Self {
            echo_when_empty: true,
            ..Self::new()
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
                if self.echo_when_empty {
                    MockStep::Echo
                } else {
                    MockStep::simple_success("<s id=\"1\">Default mock response</s>")
                }
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
            MockStep::Echo => {
                let text = echo_response(&req.user_prompt);
                if let Some(ref mut cb) = token_cb {
                    cb(&text);
                }
                Ok(ProviderOutcome::Success(ProviderResponse {
                    text,
                    finish_reason: FinishReason::Stop,
                    usage: TokenUsage {
                        input_tokens: (req.system_prompt.len() + req.user_prompt.len()) as i64 / 3,
                        output_tokens: 40,
                        ..Default::default()
                    },
                    provider_request_id: Some("mock-echo-id".to_string()),
                    model: req.model.clone(),
                }))
            }
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

/// Builds a contract-shaped answer for every `<t>` and `<s id>` inside `<chapter>`.
fn echo_response(user_prompt: &str) -> String {
    let chapter = user_prompt
        .find("<chapter")
        .map(|i| &user_prompt[i..])
        .unwrap_or("");
    let mut out = String::new();
    for line in chapter.lines() {
        let line = line.trim();
        if line.starts_with("<t>") {
            out.push_str(
                "<t>Judul Bab</t>
",
            );
        } else if let Some(rest) = line.strip_prefix("<s id=\"") {
            let Some((id, body)) = rest.split_once("\">") else {
                continue;
            };
            let src = body.trim_end_matches("</s>");
            // Real translations run longer than the Japanese source; keep the length ratio
            // validator satisfied by padding to roughly 1.5x the source character count.
            let target_len = src.chars().count() * 3 / 2;
            let mut filler = String::new();
            while filler.chars().count() < target_len {
                filler.push_str(" kata");
            }
            if src.starts_with('「') || src.starts_with('『') {
                out.push_str(&format!(
                    "<s id=\"{id}\">「Dialog {id}{filler}.」</s>
"
                ));
            } else {
                out.push_str(&format!(
                    "<s id=\"{id}\">Narasi segmen {id}{filler}.</s>
"
                ));
            }
        }
    }
    out.push_str(
        "<glossary>
</glossary>
",
    );
    out
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
