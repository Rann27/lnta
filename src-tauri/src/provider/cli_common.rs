//! Common infrastructure for CLI-based model adapters per FR-PRV-05, Story G2, and PRD §10.5.
//!
//! Provides path resolution for global CLI binaries on Windows/Unix,
//! disposable temporary working directory guards, stdin prompt streaming,
//! timeout control, and quota/auth blocker detection.

use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::error::AppError;
use crate::provider::types::ProviderOutcome;

/// Result of executing a CLI subprocess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

/// RAII guard ensuring each CLI invocation executes in a clean, isolated
/// directory that is automatically destroyed upon completion.
pub struct TempDirGuard {
    pub path: PathBuf,
}

impl TempDirGuard {
    pub fn new() -> Result<Self, AppError> {
        let temp_root = std::env::temp_dir();
        let folder_name = format!("lnta_cli_{}", uuid::Uuid::new_v4());
        let path = temp_root.join(folder_name);
        std::fs::create_dir_all(&path).map_err(|e| AppError::Internal {
            message: format!("Failed to create temporary directory for CLI execution: {e}"),
        })?;
        Ok(Self { path })
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        // Best effort cleanup; directory contains no persistent project state
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Resolves the executable path of a command across system PATH entries,
/// taking into account Windows extensions (.cmd, .exe, .bat).
pub fn resolve_cli_path(bin_name: &str) -> Option<PathBuf> {
    if let Ok(meta) = std::fs::metadata(bin_name) {
        if meta.is_file() {
            return Some(PathBuf::from(bin_name));
        }
    }

    let path_var = std::env::var_os("PATH")?;
    let search_dirs = std::env::split_paths(&path_var);

    #[cfg(windows)]
    let candidates = [
        format!("{bin_name}.cmd"),
        format!("{bin_name}.exe"),
        format!("{bin_name}.bat"),
        bin_name.to_string(),
    ];

    #[cfg(not(windows))]
    let candidates = [bin_name.to_string()];

    for dir in search_dirs {
        for candidate in &candidates {
            let full_path = dir.join(candidate);
            if full_path.is_file() {
                return Some(full_path);
            }
        }
    }

    None
}

/// Detects the installed version of a CLI by running `--version`.
pub async fn detect_cli_version(bin_name: &str) -> Result<String, AppError> {
    let resolved = resolve_cli_path(bin_name).ok_or_else(|| AppError::ProviderFailed {
        reason: format!("CLI binary '{bin_name}' is not installed or not found on PATH"),
    })?;

    let temp_guard = TempDirGuard::new()?;

    let mut cmd = Command::new(&resolved);
    cmd.arg("--version");
    cmd.current_dir(&temp_guard.path);

    let output = tokio::time::timeout(Duration::from_secs(10), cmd.output())
        .await
        .map_err(|_| AppError::ProviderFailed {
            reason: format!("Timed out detecting version for '{bin_name}'"),
        })?
        .map_err(|e| AppError::ProviderFailed {
            reason: format!("Failed to execute '{bin_name} --version': {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AppError::ProviderFailed {
            reason: format!("Version detection for '{bin_name}' failed: {stderr}"),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Ok(stderr)
    } else {
        Ok(stdout)
    }
}

/// Executes a CLI command with stdin piping in an isolated directory.
pub async fn execute_cli_command(
    bin_path: &Path,
    args: &[&str],
    cwd: &Path,
    stdin_content: &str,
    timeout: Duration,
) -> Result<CliOutput, AppError> {
    let start = std::time::Instant::now();

    let mut cmd = Command::new(bin_path);
    cmd.args(args);
    cmd.current_dir(cwd);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| AppError::ProviderFailed {
        reason: format!("Failed to spawn CLI process '{:?}': {e}", bin_path),
    })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_content.as_bytes())
            .await
            .map_err(|e| AppError::ProviderFailed {
                reason: format!("Failed to write prompt to CLI stdin: {e}"),
            })?;
        drop(stdin);
    }

    let output = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| AppError::ProviderFailed {
            reason: format!(
                "CLI execution timed out after {} seconds",
                timeout.as_secs()
            ),
        })?
        .map_err(|e| AppError::ProviderFailed {
            reason: format!("Error waiting for CLI output: {e}"),
        })?;

    let duration_ms = start.elapsed().as_millis() as u64;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(CliOutput {
        exit_code: output.status.code(),
        stdout,
        stderr,
        duration_ms,
    })
}

/// Inspects process output and stderr for quota exhaustion, rate limits, or auth refusal
/// to pause the run with an informative blocker per FR-PRV-05 and PRD §10.5.
pub fn inspect_quota_or_auth_blocker(stdout: &str, stderr: &str) -> Option<ProviderOutcome> {
    let combined = format!("{stdout}\n{stderr}").to_lowercase();

    if combined.contains("ineligibletiererror")
        || combined.contains("unsupported_client")
        || combined.contains("error authenticating")
        || combined.contains("not authenticated")
        || combined.contains("login required")
        || combined.contains("gemini login")
        || combined.contains("codex login")
        || combined.contains("claude login")
        || combined.contains("invalid api key")
        || combined.contains("unauthorized")
    {
        let summary = extract_first_meaningful_line(stderr)
            .or_else(|| extract_first_meaningful_line(stdout))
            .unwrap_or_else(|| "Authentication or login required".to_string());
        return Some(ProviderOutcome::Refused {
            reason: format!("CLI Authentication Blocker: {summary}"),
        });
    }

    if combined.contains("quota exceeded")
        || combined.contains("resource_exhausted")
        || combined.contains("rate limit")
        || combined.contains("too many requests")
        || combined.contains("insufficient_quota")
        || combined.contains("429")
        || combined.contains("credit balance is too low")
    {
        let summary = extract_first_meaningful_line(stderr)
            .or_else(|| extract_first_meaningful_line(stdout))
            .unwrap_or_else(|| "Rate limit or quota exhausted".to_string());
        return Some(ProviderOutcome::Refused {
            reason: format!("CLI Quota / Rate-limit Blocker: {summary}"),
        });
    }

    None
}

fn extract_first_meaningful_line(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty()
            && !trimmed.starts_with("Reading prompt")
            && !trimmed.starts_with("Loaded")
        {
            return Some(trimmed.to_string());
        }
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_temp_dir_lifecycle() {
        let path = {
            let guard = TempDirGuard::new().unwrap();
            assert!(guard.path.exists());
            guard.path.clone()
        };
        assert!(!path.exists());
    }

    #[test]
    fn test_inspect_auth_blocker() {
        let stderr = "Error authenticating: IneligibleTierError: This client is no longer supported for Gemini Code Assist for individuals.";
        let outcome = inspect_quota_or_auth_blocker("", stderr);
        assert!(outcome.is_some());
        if let Some(ProviderOutcome::Refused { reason }) = outcome {
            assert!(reason.contains("CLI Authentication Blocker"));
            assert!(reason.contains("IneligibleTierError"));
        } else {
            panic!("Expected Refused outcome");
        }
    }

    #[test]
    fn test_inspect_quota_blocker() {
        let stderr = "status: 429 RESOURCE_EXHAUSTED Quota exceeded for quota metric";
        let outcome = inspect_quota_or_auth_blocker("", stderr);
        assert!(outcome.is_some());
        if let Some(ProviderOutcome::Refused { reason }) = outcome {
            assert!(reason.contains("CLI Quota / Rate-limit Blocker"));
        } else {
            panic!("Expected Refused outcome");
        }
    }

    #[test]
    fn test_inspect_clean_output() {
        let stdout = "{\"result\": \"success\"}";
        let outcome = inspect_quota_or_auth_blocker(stdout, "");
        assert!(outcome.is_none());
    }
}
