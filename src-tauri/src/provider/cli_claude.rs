//! Anthropic Claude Code CLI adapter per FR-PRV-05, Story G2, and PRD §10.5.
//!
//! Note on tool/flag verification:
//! Claude Code CLI flags derived from official Claude Code documentation (v0.2.x+ headless mode):
//! - `-p` (print mode: non-interactive headless invocation; prompts passed via stdin)
//! - `--output-format json` (emits structured JSON payload containing result text and token usage)
//! - `--model <model>` (explicit model selector)
//! - Tools are not triggered during single-turn headless translation runs.

use std::time::Duration;

use serde_json::Value;

use crate::error::AppError;
use crate::provider::cli_common::{
    detect_cli_version, execute_cli_command, inspect_quota_or_auth_blocker, resolve_cli_path,
    TempDirGuard,
};
use crate::provider::types::{
    FinishReason, Provider, ProviderOutcome, ProviderRequest, ProviderResponse, TokenUsage,
};

/// Adapter for invoking Anthropic Claude Code CLI in headless mode.
#[derive(Debug, Clone, Default)]
pub struct CliClaudeProvider {
    pub executable: String,
    pub timeout: Duration,
}

impl CliClaudeProvider {
    pub fn new() -> Self {
        Self {
            executable: "claude".to_string(),
            timeout: Duration::from_secs(180),
        }
    }

    /// Checks whether Claude Code CLI is installed and returns its detected version string.
    pub async fn detect_version(&self) -> Result<String, AppError> {
        detect_cli_version(&self.executable).await
    }
}

impl Provider for CliClaudeProvider {
    fn name(&self) -> &'static str {
        "cli_claude"
    }

    async fn send_request(
        &self,
        req: &ProviderRequest,
        mut token_cb: Option<&mut (dyn FnMut(&str) + Send)>,
    ) -> Result<ProviderOutcome, AppError> {
        let bin_path = resolve_cli_path(&self.executable).ok_or_else(|| {
            AppError::ProviderFailed {
                reason: format!(
                    "Claude Code CLI binary '{}' not found on PATH. Ensure @anthropic-ai/claude-code is installed.",
                    self.executable
                ),
            }
        })?;

        let temp_guard = TempDirGuard::new()?;

        let mut args: Vec<&str> = vec!["-p", "--output-format", "json"];

        let model_arg = req.model.trim();
        if !model_arg.is_empty() && model_arg != "default" {
            args.push("--model");
            args.push(model_arg);
        }

        let full_prompt = if req.system_prompt.trim().is_empty() {
            req.user_prompt.clone()
        } else {
            format!("{}\n\n{}", req.system_prompt, req.user_prompt)
        };

        let output = execute_cli_command(
            &bin_path,
            &args,
            &temp_guard.path,
            &full_prompt,
            self.timeout,
        )
        .await?;

        if let Some(blocker) = inspect_quota_or_auth_blocker(&output.stdout, &output.stderr) {
            return Ok(blocker);
        }

        if let Some(code) = output.exit_code {
            if code != 0 {
                let err_msg = if !output.stderr.trim().is_empty() {
                    output.stderr.trim()
                } else {
                    output.stdout.trim()
                };
                return Err(AppError::ProviderFailed {
                    reason: format!("Claude Code CLI exited with code {code}: {err_msg}"),
                });
            }
        }

        let parsed = parse_claude_cli_output(&output.stdout, &req.model)?;

        if let Some(ref mut cb) = token_cb {
            if let Some(text) = parsed.text() {
                cb(text);
            }
        }

        Ok(parsed)
    }
}

/// Parses standard JSON output produced by Claude Code CLI (`claude -p --output-format json`).
pub fn parse_claude_cli_output(
    json_stdout: &str,
    requested_model: &str,
) -> Result<ProviderOutcome, AppError> {
    let trimmed = json_stdout.trim();
    if trimmed.is_empty() {
        return Err(AppError::ParseFailed {
            reason: "Claude Code CLI returned empty standard output".to_string(),
        });
    }

    let val: Value = serde_json::from_str(trimmed).map_err(|e| AppError::ParseFailed {
        reason: format!("Failed to parse Claude Code CLI JSON response: {e}. Output: {trimmed}"),
    })?;

    // Check for explicit error responses
    if let Some(err) = val.get("error") {
        let msg = if let Some(s) = err.as_str() {
            s
        } else {
            err.get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown Claude Code error")
        };

        if let Some(blocker) = inspect_quota_or_auth_blocker(msg, "") {
            return Ok(blocker);
        }

        return Err(AppError::ProviderFailed {
            reason: format!("Claude Code CLI returned error: {msg}"),
        });
    }

    // Extract text response
    let text = if let Some(r) = val.get("result").and_then(|v| v.as_str()) {
        r.to_string()
    } else if let Some(t) = val.get("text").and_then(|v| v.as_str()) {
        t.to_string()
    } else if let Some(c) = val.get("content").and_then(|v| v.as_str()) {
        c.to_string()
    } else {
        return Err(AppError::ParseFailed {
            reason: format!("Unrecognized Claude Code CLI response schema: {trimmed}"),
        });
    };

    // Extract token usage
    let input_tokens = val
        .pointer("/usage/input_tokens")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let cached_input_tokens = val
        .pointer("/usage/cache_read_input_tokens")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let cache_write_tokens = val
        .pointer("/usage/cache_creation_input_tokens")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let output_tokens = val
        .pointer("/usage/output_tokens")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let usage = TokenUsage {
        input_tokens,
        cached_input_tokens,
        cache_write_tokens,
        output_tokens,
        reasoning_tokens: 0,
    };

    Ok(ProviderOutcome::Success(ProviderResponse {
        text,
        finish_reason: FinishReason::Stop,
        usage,
        provider_request_id: None,
        model: requested_model.to_string(),
    }))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_claude_cli_normal_response() {
        let sample = r#"{
            "result": "<1>Halo dari Claude Code</1>",
            "usage": {
                "input_tokens": 450,
                "output_tokens": 30,
                "cache_read_input_tokens": 120,
                "cache_creation_input_tokens": 0
            },
            "cost_usd": 0.0018
        }"#;

        let outcome = parse_claude_cli_output(sample, "claude-3-7-sonnet").unwrap();
        assert!(outcome.is_success());
        let res = match outcome {
            ProviderOutcome::Success(r) => r,
            _ => panic!("Expected success"),
        };
        assert_eq!(res.text, "<1>Halo dari Claude Code</1>");
        assert_eq!(res.usage.input_tokens, 450);
        assert_eq!(res.usage.cached_input_tokens, 120);
        assert_eq!(res.usage.output_tokens, 30);
        assert_eq!(res.finish_reason, FinishReason::Stop);
    }

    #[test]
    fn test_parse_claude_cli_auth_refusal() {
        let sample = r#"{
            "error": "Authentication failed: Please run `claude login` to authenticate"
        }"#;

        let outcome = parse_claude_cli_output(sample, "claude-3-5-sonnet").unwrap();
        match outcome {
            ProviderOutcome::Refused { reason } => {
                assert!(reason.contains("CLI Authentication Blocker"));
            }
            _ => panic!("Expected refusal on auth error"),
        }
    }

    #[test]
    fn test_parse_claude_cli_quota_refusal() {
        let sample = r#"{
            "error": "Credit balance is too low to complete this request"
        }"#;

        let outcome = parse_claude_cli_output(sample, "claude-3-5-sonnet").unwrap();
        match outcome {
            ProviderOutcome::Refused { reason } => {
                assert!(reason.contains("CLI Quota / Rate-limit Blocker"));
            }
            _ => panic!("Expected refusal on quota error"),
        }
    }

    #[tokio::test]
    async fn test_missing_claude_cli_reports_error() {
        let provider = CliClaudeProvider {
            executable: "non_existent_claude_cli_555".to_string(),
            timeout: Duration::from_secs(1),
        };
        let req = ProviderRequest {
            system_prompt: "".to_string(),
            user_prompt: "test".to_string(),
            model: "default".to_string(),
            temperature: None,
            max_output_tokens: None,
            reasoning_effort: None,
        };
        let res = provider.send_request(&req, None).await;
        assert!(res.is_err());
    }
}
