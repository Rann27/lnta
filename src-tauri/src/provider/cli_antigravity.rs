//! Google Antigravity CLI (`agy`) adapter per FR-PRV-05, Story G2, and PRD §10.5.
//!
//! Antigravity CLI replaced Gemini CLI for Google AI Pro and Ultra subscribers on 2026-06-18;
//! Gemini CLI no longer serves personal subscriptions. Flags follow the headless docs and
//! changelog (checked October 2026):
//! - `--print` without a value, then `--input-format stream-json`: the prompt goes in on stdin
//!   as one `{"event":"user",...}` line, which avoids the Windows command-line length limit;
//!   closing stdin ends the session after the turn
//! - `--output-format stream-json`: the last `result` event carries `response`, `status`,
//!   and `usage`
//! - `--model <slug>` (see `agy models`) and `--effort low|medium|high`
//!
//! Antigravity has no system-prompt flag, so the contract and instruction lead the user
//! message. Headless runs need a login cached by one interactive `agy` session.

use std::time::Duration;

use serde_json::{json, Value};

use crate::error::AppError;
use crate::provider::cli_common::{
    detect_cli_version, execute_cli_command, inspect_quota_or_auth_blocker, resolve_cli_path,
    TempDirGuard, CLI_TIMEOUT,
};
use crate::provider::types::{
    FinishReason, Provider, ProviderOutcome, ProviderRequest, ProviderResponse, TokenUsage,
};

/// Adapter for invoking Antigravity CLI in headless print mode.
#[derive(Debug, Clone, Default)]
pub struct CliAntigravityProvider {
    pub executable: String,
    pub timeout: Duration,
}

impl CliAntigravityProvider {
    pub fn new() -> Self {
        Self {
            executable: "agy".to_string(),
            timeout: CLI_TIMEOUT,
        }
    }

    /// Checks whether Antigravity CLI is installed and returns its detected version string.
    pub async fn detect_version(&self) -> Result<String, AppError> {
        detect_cli_version(&self.executable).await
    }
}

/// One stdin line for `--input-format stream-json`.
pub fn build_stream_input(system_prompt: &str, user_prompt: &str) -> String {
    let content = if system_prompt.trim().is_empty() {
        user_prompt.to_string()
    } else {
        format!("{system_prompt}\n\n{user_prompt}")
    };
    let mut line = json!({ "event": "user", "message": { "content": content } }).to_string();
    line.push('\n');
    line
}

impl Provider for CliAntigravityProvider {
    fn name(&self) -> &'static str {
        "cli_antigravity"
    }

    async fn send_request(
        &self,
        req: &ProviderRequest,
        mut token_cb: Option<&mut (dyn FnMut(&str) + Send)>,
    ) -> Result<ProviderOutcome, AppError> {
        let bin_path = resolve_cli_path(&self.executable).ok_or_else(|| {
            AppError::ProviderFailed {
                reason: format!(
                    "Antigravity CLI binary '{}' not found on PATH. Install Antigravity CLI and sign in once with `agy`.",
                    self.executable
                ),
            }
        })?;

        let temp_guard = TempDirGuard::new()?;
        let timeout_arg = format!("{}m", self.timeout.as_secs().div_ceil(60).max(1));

        let mut args: Vec<&str> = vec![
            "--print",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--print-timeout",
            &timeout_arg,
        ];
        let model_arg = req.model.trim();
        if !model_arg.is_empty() && model_arg != "default" {
            args.push("--model");
            args.push(model_arg);
        }
        if let Some(effort) = req.reasoning_effort.as_deref() {
            args.push("--effort");
            args.push(effort);
        }

        let output = execute_cli_command(
            &bin_path,
            &args,
            &temp_guard.path,
            &build_stream_input(&req.system_prompt, &req.user_prompt),
            // A little longer than the CLI's own limit, so its result event can arrive.
            self.timeout + Duration::from_secs(60),
        )
        .await?;

        if let Some(blocker) = inspect_quota_or_auth_blocker(&output.stdout, &output.stderr) {
            return Ok(blocker);
        }

        let parsed = parse_antigravity_stream(&output.stdout, &req.model);
        let parsed = match (parsed, output.exit_code) {
            (Ok(outcome), _) => outcome,
            (Err(e), Some(code)) if code != 0 => {
                let detail = if output.stderr.trim().is_empty() {
                    e.to_string()
                } else {
                    output.stderr.trim().to_string()
                };
                return Err(AppError::ProviderFailed {
                    reason: format!("Antigravity CLI exited with code {code}: {detail}"),
                });
            }
            (Err(e), _) => return Err(e),
        };

        if let Some(ref mut cb) = token_cb {
            if let Some(text) = parsed.text() {
                cb(text);
            }
        }
        Ok(parsed)
    }
}

/// Parses `--output-format stream-json` output; the last `result` event is the answer.
pub fn parse_antigravity_stream(
    stdout: &str,
    requested_model: &str,
) -> Result<ProviderOutcome, AppError> {
    let result = stdout
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
        .filter(|v| v.get("event").and_then(Value::as_str) == Some("result"))
        .find_map(|v| v.get("result").cloned())
        .ok_or_else(|| AppError::ParseFailed {
            reason: "Antigravity CLI produced no result event".to_string(),
        })?;

    let status = result
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("SUCCESS");
    if status != "SUCCESS" {
        let error = result
            .get("error")
            .map(|e| {
                e.as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| e.to_string())
            })
            .unwrap_or_default();
        return Err(AppError::ProviderFailed {
            reason: format!("Antigravity CLI ended with status {status}: {error}"),
        });
    }

    let count = |key: &str| {
        result
            .pointer(&format!("/usage/{key}"))
            .and_then(Value::as_i64)
            .unwrap_or(0)
    };
    let text = result
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    Ok(ProviderOutcome::Success(ProviderResponse {
        text,
        finish_reason: FinishReason::Stop,
        usage: TokenUsage {
            input_tokens: count("input_tokens"),
            cached_input_tokens: count("cache_read_tokens"),
            cache_write_tokens: 0,
            output_tokens: count("output_tokens"),
            reasoning_tokens: count("thinking_tokens"),
        },
        provider_request_id: result
            .get("conversation_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        model: requested_model.to_string(),
    }))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn stream_input_is_one_user_event_line() {
        let line = build_stream_input("contract", "chapter");
        assert!(line.ends_with('\n'));
        let v: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(v["event"], "user");
        assert_eq!(v["message"]["content"], "contract\n\nchapter");
    }

    #[test]
    fn parses_the_last_result_event() {
        let stdout = concat!(
            r#"{"event":"init","conversation_id":"c1","init":{"cwd":"/tmp"}}"#,
            "\n",
            r#"{"event":"step_update","step_update":{"step_index":0,"state":"ACTIVE","text_delta":"<t>"}}"#,
            "\n",
            r#"{"event":"result","result":{"conversation_id":"c1","status":"SUCCESS","response":"<t>Halo</t>","usage":{"input_tokens":900,"output_tokens":40,"thinking_tokens":12,"cache_read_tokens":300,"total_tokens":952}}}"#,
            "\n"
        );
        let outcome = parse_antigravity_stream(stdout, "default").unwrap();
        let ProviderOutcome::Success(resp) = outcome else {
            panic!("expected success");
        };
        assert_eq!(resp.text, "<t>Halo</t>");
        assert_eq!(resp.usage.input_tokens, 900);
        assert_eq!(resp.usage.cached_input_tokens, 300);
        assert_eq!(resp.usage.reasoning_tokens, 12);
        assert_eq!(resp.provider_request_id.as_deref(), Some("c1"));
    }

    #[test]
    fn failed_status_is_an_error() {
        let stdout =
            r#"{"event":"result","result":{"status":"ERROR","error":"model unavailable"}}"#;
        let err = parse_antigravity_stream(stdout, "default").unwrap_err();
        assert!(err.to_string().contains("model unavailable"));
    }

    #[tokio::test]
    async fn missing_binary_reports_error() {
        let provider = CliAntigravityProvider {
            executable: "non_existent_agy_cli_555".to_string(),
            timeout: Duration::from_secs(1),
        };
        let req = ProviderRequest {
            system_prompt: String::new(),
            user_prompt: "test".to_string(),
            model: "default".to_string(),
            temperature: None,
            max_output_tokens: None,
            reasoning_effort: None,
        };
        assert!(provider.send_request(&req, None).await.is_err());
    }
}
