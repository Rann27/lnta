//! Retry logic with exponential backoff, jitter, and Retry-After support per FR-PRV-07.

use std::time::Duration;

/// Configuration for network call retries.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 500,
            max_delay_ms: 30000,
            backoff_factor: 2.0,
        }
    }
}

/// Checks whether an HTTP status code indicates a retryable failure (429, 5xx).
pub fn is_retryable_status(status_code: u16) -> bool {
    status_code == 429 || (500..=599).contains(&status_code)
}

/// Checks whether a reqwest error is a retryable network issue (timeout, connection dropped).
pub fn is_retryable_reqwest_error(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect()
}

/// Computes the retry delay in milliseconds honoring Retry-After or exponential backoff with jitter.
pub fn calculate_retry_delay(
    attempt: u32,
    retry_after_seconds: Option<u64>,
    config: &RetryConfig,
) -> Duration {
    if let Some(secs) = retry_after_seconds {
        let ms = (secs * 1000).min(config.max_delay_ms);
        return Duration::from_millis(ms);
    }

    // Exponential backoff: initial * factor^(attempt - 1)
    let base = if attempt <= 1 {
        config.initial_delay_ms as f64
    } else {
        (config.initial_delay_ms as f64) * config.backoff_factor.powi((attempt - 1) as i32)
    };

    let clamped = base.min(config.max_delay_ms as f64);

    // Simple deterministic pseudo-jitter (+/- 10%) based on attempt count
    let jitter_factor = 1.0 + ((((attempt * 17) % 21) as f64 - 10.0) / 100.0);
    let final_ms = (clamped * jitter_factor)
        .max(100.0)
        .min(config.max_delay_ms as f64);

    Duration::from_millis(final_ms as u64)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_retryable_status_codes() {
        assert!(is_retryable_status(429));
        assert!(is_retryable_status(500));
        assert!(is_retryable_status(502));
        assert!(is_retryable_status(503));
        assert!(is_retryable_status(504));

        assert!(!is_retryable_status(200));
        assert!(!is_retryable_status(400));
        assert!(!is_retryable_status(401));
        assert!(!is_retryable_status(403));
        assert!(!is_retryable_status(404));
    }

    #[test]
    fn test_retry_after_honored() {
        let cfg = RetryConfig::default();
        let delay = calculate_retry_delay(1, Some(10), &cfg);
        assert_eq!(delay.as_secs(), 10);
    }

    #[test]
    fn test_exponential_backoff_increases() {
        let cfg = RetryConfig {
            max_attempts: 5,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_factor: 2.0,
        };

        let delay1 = calculate_retry_delay(1, None, &cfg);
        let delay2 = calculate_retry_delay(2, None, &cfg);
        let delay3 = calculate_retry_delay(3, None, &cfg);

        assert!(delay2 > delay1);
        assert!(delay3 > delay2);
    }
}
