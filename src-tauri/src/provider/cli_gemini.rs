//! Google Gemini CLI adapter per FR-PRV-05, Story G2, and PRD §10.5.
//!
//! Note on tool/flag verification:
//! Flags verified against `--help` for installed `@google/gemini-cli` version 0.63.0:
//! - `-p ""` (headless prompt mode; prompts piped via stdin)
//! - `--output-format json` (or `-o json`)
//! - `--approval-mode plan` (read-only mode; tool execution disabled)
//! - `--sandbox` (isolates subprocess environment)
//! - `--skip-trust` (suppresses interactive folder trust prompt in temporary directories)
//! - `-m <model>` (explicit model selection)
//!
//! Individual Google account logins may be rejected by Gemini CLI with
//! `IneligibleTierError: This client is no longer supported for Gemini Code Assist for individuals`.
//! In such events, the adapter reports a clear authentication blocker. API-key based CLI usage remains supported.

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

/// Adapter for invoking Google Gemini CLI in headless, read-only mode.
#[derive(Debug, Clone, Default)]
pub struct CliGeminiProvider {
    pub executable: String,
    pub timeout: Duration,
}

impl CliGeminiProvider {
    pub fn new() -> Self {
        Self {
            executable: "gemini".to_string(),
            timeout: Duration::from_secs(180),
        }
    }

    /// Checks whether the Gemini CLI is installed and returns its detected version.
    pub async fn detect_version(&self) -> Result<String, AppError> {
        detect_cli_version(&self.executable).await
    }
}

impl Provider for CliGeminiProvider {
    fn name(&self) -> &'static str {
        "cli_gemini"
    }

    async fn send_request(
        &self,
        req: &ProviderRequest,
        mut token_cb: Option<&mut (dyn FnMut(&str) + Send)>,
    ) -> Result<ProviderOutcome, AppError> {
        let bin_path = resolve_cli_path(&self.executable).ok_or_else(|| {
            AppError::ProviderFailed {
                reason: format!(
                    "Gemini CLI binary '{}' not found on PATH. Ensure @google/gemini-cli is installed.",
                    self.executable
                ),
            }
        })?;

        let temp_guard = TempDirGuard::new()?;

        let mut args: Vec<&str> = vec![
            "-p",
            "",
            "--output-format",
            "json",
            "--approval-mode",
            "plan",
            "--sandbox",
            "--skip-trust",
        ];

        let model_arg = req.model.trim();
        if !model_arg.is_empty() && model_arg != "default" {
            args.push("-m");
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
                    reason: format!("Gemini CLI exited with code {code}: {err_msg}"),
                });
            }
        }

        let parsed = parse_gemini_cli_output(&output.stdout, &req.model)?;

        if let Some(ref mut cb) = token_cb {
            if let Some(text) = parsed.text() {
                cb(text);
            }
        }

        Ok(parsed)
    }
}

/// Parses standard JSON output produced by Gemini CLI (`--output-format json`).
pub fn parse_gemini_cli_output(
    json_stdout: &str,
    requested_model: &str,
) -> Result<ProviderOutcome, AppError> {
    let trimmed = json_stdout.trim();
    if trimmed.is_empty() {
        return Err(AppError::ParseFailed {
            reason: "Gemini CLI returned empty standard output".to_string(),
        });
    }

    let val: Value = serde_json::from_str(trimmed).map_err(|e| AppError::ParseFailed {
        reason: format!("Failed to parse Gemini CLI JSON response: {e}. Output: {trimmed}"),
    })?;

    // Check if error is contained in JSON payload
    if let Some(err) = val.get("error") {
        let msg = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown Gemini CLI error");
        if let Some(blocker) = inspect_quota_or_auth_blocker(msg, "") {
            return Ok(blocker);
        }
        return Err(AppError::ProviderFailed {
            reason: format!("Gemini CLI returned error: {msg}"),
        });
    }

    // Extract text from known formats:
    // 1. { "response": "..." }
    // 2. { "result": "..." }
    // 3. { "text": "..." }
    // 4. { "candidates": [ { "content": { "parts": [ { "text": "..." } ] } } ] }
    let text = if let Some(r) = val.get("response").and_then(|v| v.as_str()) {
        r.to_string()
    } else if let Some(r) = val.get("result").and_then(|v| v.as_str()) {
        r.to_string()
    } else if let Some(t) = val.get("text").and_then(|v| v.as_str()) {
        t.to_string()
    } else if let Some(parts) = val
        .pointer("/candidates/0/content/parts")
        .and_then(|v| v.as_array())
    {
        let mut buf = String::new();
        for p in parts {
            if let Some(t) = p.get("text").and_then(|v| v.as_str()) {
                buf.push_str(t);
            }
        }
        buf
    } else {
        return Err(AppError::ParseFailed {
            reason: format!("Unrecognized Gemini CLI output schema: {trimmed}"),
        });
    };

    // Extract usage statistics if provided
    let input_tokens = val
        .pointer("/stats/tokens/input")
        .or_else(|| val.pointer("/tokens/input"))
        .or_else(|| val.pointer("/usage/prompt_tokens"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let output_tokens = val
        .pointer("/stats/tokens/output")
        .or_else(|| val.pointer("/tokens/output"))
        .or_else(|| val.pointer("/usage/completion_tokens"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let usage = TokenUsage {
        input_tokens,
        cached_input_tokens: 0,
        cache_write_tokens: 0,
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
    fn test_parse_gemini_cli_normal_response() {
        let sample = r#"{
            "response": "<1>Halo dunia</1>",
            "stats": {
                "tokens": {
                    "input": 120,
                    "output": 15
                }
            }
        }"#;

        let outcome = parse_gemini_cli_output(sample, "gemini-2.0-flash").unwrap();
        assert!(outcome.is_success());
        let res = match outcome {
            ProviderOutcome::Success(r) => r,
            _ => panic!("Expected success"),
        };
        assert_eq!(res.text, "<1>Halo dunia</1>");
        assert_eq!(res.usage.input_tokens, 120);
        assert_eq!(res.usage.output_tokens, 15);
        assert_eq!(res.finish_reason, FinishReason::Stop);
    }

    #[test]
    fn test_parse_gemini_cli_candidates_format() {
        let sample = r#"{
            "candidates": [
                {
                    "content": {
                        "parts": [
                            { "text": "<t>Judul Bab</t>\n<1>Kalimat pertama</1>" }
                        ]
                    }
                }
            ]
        }"#;

        let outcome = parse_gemini_cli_output(sample, "gemini-1.5-pro").unwrap();
        assert!(outcome.is_success());
        assert_eq!(
            outcome.text().unwrap(),
            "<t>Judul Bab</t>\n<1>Kalimat pertama</1>"
        );
        // Unreported tokens default to 0
        assert_eq!(outcome.usage().unwrap().input_tokens, 0);
    }

    #[test]
    fn test_parse_gemini_cli_quota_rejection() {
        let sample = r#"{
            "error": {
                "message": "Resource exhausted: quota exceeded for 15 requests per minute"
            }
        }"#;

        let outcome = parse_gemini_cli_output(sample, "gemini-1.5-flash").unwrap();
        match outcome {
            ProviderOutcome::Refused { reason } => {
                assert!(reason.contains("Quota / Rate-limit"));
            }
            _ => panic!("Expected refused outcome on quota exhaustion"),
        }
    }

    #[test]
    fn test_parse_gemini_cli_malformed_json() {
        let sample = "Not a JSON output";
        let res = parse_gemini_cli_output(sample, "gemini-2.0-flash");
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_missing_cli_reports_error() {
        let provider = CliGeminiProvider {
            executable: "non_existent_gemini_binary_12345".to_string(),
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
