//! OpenAI Codex CLI adapter per FR-PRV-05, Story G2, and PRD §10.5.
//!
//! Note on tool/flag verification:
//! Flags verified against `--help` for installed `codex-cli` version 0.162.0:
//! - `exec` (non-interactive subcommand)
//! - `--skip-git-repo-check` (permits running inside ephemeral non-git working directory)
//! - `--sandbox read-only` (read-only filesystem permissions; tools restricted)
//! - `--ephemeral` (prevents storing session history on disk)
//! - `--json` (emits structured event stream as JSON Lines)
//! - `-C <DIR>` (specifies isolated temporary directory)
//! - `-m <MODEL>` (explicit model selection)
//! - Prompt piped via stdin when prompt positional argument is omitted.

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

/// Adapter for invoking Codex CLI non-interactively in read-only sandbox mode.
#[derive(Debug, Clone, Default)]
pub struct CliCodexProvider {
    pub executable: String,
    pub timeout: Duration,
}

impl CliCodexProvider {
    pub fn new() -> Self {
        Self {
            executable: "codex".to_string(),
            timeout: Duration::from_secs(180),
        }
    }

    /// Checks whether Codex CLI is installed and returns its detected version string.
    pub async fn detect_version(&self) -> Result<String, AppError> {
        detect_cli_version(&self.executable).await
    }
}

impl Provider for CliCodexProvider {
    fn name(&self) -> &'static str {
        "cli_codex"
    }

    async fn send_request(
        &self,
        req: &ProviderRequest,
        mut token_cb: Option<&mut (dyn FnMut(&str) + Send)>,
    ) -> Result<ProviderOutcome, AppError> {
        let bin_path =
            resolve_cli_path(&self.executable).ok_or_else(|| AppError::ProviderFailed {
                reason: format!(
                    "Codex CLI binary '{}' not found on PATH. Ensure codex-cli is installed.",
                    self.executable
                ),
            })?;

        let temp_guard = TempDirGuard::new()?;
        let temp_dir_str = temp_guard.path.to_string_lossy().to_string();

        let mut args: Vec<&str> = vec![
            "exec",
            "--skip-git-repo-check",
            "--sandbox",
            "read-only",
            "--ephemeral",
            "--json",
            "-C",
            &temp_dir_str,
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
                    reason: format!("Codex CLI exited with code {code}: {err_msg}"),
                });
            }
        }

        let parsed = parse_codex_cli_output(&output.stdout, &req.model)?;

        if let Some(ref mut cb) = token_cb {
            if let Some(text) = parsed.text() {
                cb(text);
            }
        }

        Ok(parsed)
    }
}

/// Parses JSON Lines output from Codex CLI (`codex exec --json`).
pub fn parse_codex_cli_output(
    jsonl_stdout: &str,
    requested_model: &str,
) -> Result<ProviderOutcome, AppError> {
    let mut accumulated_text = String::new();
    let mut usage = TokenUsage::default();
    let mut saw_agent_message = false;

    for line in jsonl_stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let val: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue, // Skip any non-JSON status banner lines
        };

        // Check for error items or blockers
        if let Some(err_msg) = val.pointer("/item/message").and_then(|v| v.as_str()) {
            if let Some(blocker) = inspect_quota_or_auth_blocker(err_msg, "") {
                return Ok(blocker);
            }
        }

        let event_type = val.get("type").and_then(|v| v.as_str()).unwrap_or("");

        // Check for agent_message completion
        if event_type == "item.completed" {
            let item_type = val.pointer("/item/type").and_then(|v| v.as_str());
            if item_type == Some("agent_message") {
                if let Some(text) = val.pointer("/item/text").and_then(|v| v.as_str()) {
                    accumulated_text.push_str(text);
                    saw_agent_message = true;
                }
            }
        }

        // Check for turn.completed token usage
        if event_type == "turn.completed" {
            if let Some(u) = val.get("usage") {
                usage.input_tokens = u.get("input_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                usage.cached_input_tokens = u
                    .get("cached_input_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                usage.output_tokens = u.get("output_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                usage.reasoning_tokens = u
                    .get("reasoning_output_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
            }
        }
    }

    if !saw_agent_message && accumulated_text.is_empty() {
        // Fall back to checking if the output was a single JSON object with a response field
        if let Ok(single_val) = serde_json::from_str::<Value>(jsonl_stdout.trim()) {
            if let Some(text) = single_val.get("response").and_then(|v| v.as_str()) {
                return Ok(ProviderOutcome::Success(ProviderResponse {
                    text: text.to_string(),
                    finish_reason: FinishReason::Stop,
                    usage,
                    provider_request_id: None,
                    model: requested_model.to_string(),
                }));
            }
        }

        return Err(AppError::ParseFailed {
            reason: "Codex CLI did not produce any completed agent_message in JSONL output"
                .to_string(),
        });
    }

    Ok(ProviderOutcome::Success(ProviderResponse {
        text: accumulated_text,
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
    fn test_parse_codex_cli_jsonl_output() {
        let jsonl = r#"
{"type":"thread.started","thread_id":"01a12664-1c4e-7a30-99d2-e375be8cc07a"}
{"type":"turn.started"}
{"type":"item.completed","item":{"id":"item_2","type":"agent_message","text":"<1>Terjemahan dari Codex</1>"}}
{"type":"turn.completed","usage":{"input_tokens":1200,"cached_input_tokens":300,"cache_write_input_tokens":0,"output_tokens":85,"reasoning_output_tokens":20}}
"#;

        let outcome = parse_codex_cli_output(jsonl, "o3-mini").unwrap();
        assert!(outcome.is_success());
        let res = match outcome {
            ProviderOutcome::Success(r) => r,
            _ => panic!("Expected success"),
        };
        assert_eq!(res.text, "<1>Terjemahan dari Codex</1>");
        assert_eq!(res.usage.input_tokens, 1200);
        assert_eq!(res.usage.cached_input_tokens, 300);
        assert_eq!(res.usage.output_tokens, 85);
        assert_eq!(res.usage.reasoning_tokens, 20);
        assert_eq!(res.finish_reason, FinishReason::Stop);
    }

    #[test]
    fn test_parse_codex_cli_quota_blocker() {
        let jsonl = r#"
{"type":"item.completed","item":{"id":"item_0","type":"error","message":"Rate limit exceeded: 429 Too Many Requests"}}
"#;

        let outcome = parse_codex_cli_output(jsonl, "o3-mini").unwrap();
        match outcome {
            ProviderOutcome::Refused { reason } => {
                assert!(reason.contains("Quota / Rate-limit"));
            }
            _ => panic!("Expected refusal on rate limit"),
        }
    }

    #[test]
    fn test_parse_codex_cli_missing_message() {
        let jsonl = r#"
{"type":"thread.started","thread_id":"01a12664-1c4e-7a30-99d2-e375be8cc07a"}
{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":0}}
"#;
        let res = parse_codex_cli_output(jsonl, "o3-mini");
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_missing_codex_reports_error() {
        let provider = CliCodexProvider {
            executable: "non_existent_codex_binary_98765".to_string(),
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
