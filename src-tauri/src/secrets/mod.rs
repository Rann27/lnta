//! OS Credential Manager secrets storage and header redaction per FR-PRV-06, NFR-07, and PRD §8.7.

use keyring::Entry;

use crate::error::AppError;

const KEYRING_SERVICE: &str = "lnta";

/// Stores an API key securely in the OS keyring (Windows Credential Manager).
pub fn set_api_key(provider_id: &str, key: &str) -> Result<(), AppError> {
    let entry = Entry::new(KEYRING_SERVICE, provider_id).map_err(|e| AppError::Internal {
        message: format!("Keyring entry error: {e}"),
    })?;

    entry.set_password(key).map_err(|e| AppError::Internal {
        message: format!("Failed to set credential for {provider_id}: {e}"),
    })?;

    Ok(())
}

/// Retrieves an API key from the OS keyring.
/// Returns Ok(None) if no credential exists for this provider.
pub fn get_api_key(provider_id: &str) -> Result<Option<String>, AppError> {
    let entry = Entry::new(KEYRING_SERVICE, provider_id).map_err(|e| AppError::Internal {
        message: format!("Keyring entry error: {e}"),
    })?;

    match entry.get_password() {
        Ok(pass) => Ok(Some(pass)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Internal {
            message: format!("Failed to get credential for {provider_id}: {e}"),
        }),
    }
}

/// Deletes an API key from the OS keyring.
pub fn delete_api_key(provider_id: &str) -> Result<(), AppError> {
    let entry = Entry::new(KEYRING_SERVICE, provider_id).map_err(|e| AppError::Internal {
        message: format!("Keyring entry error: {e}"),
    })?;

    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Internal {
            message: format!("Failed to delete credential for {provider_id}: {e}"),
        }),
    }
}

/// Redacts sensitive HTTP header values in logs per NFR-07.
///
/// Specifically redacts:
/// - `authorization`
/// - `x-api-key`
/// - `x-goog-api-key`
/// - `api-key`
pub fn redact_header_value(header_name: &str, header_value: &str) -> String {
    let name_lower = header_name.to_ascii_lowercase();
    match name_lower.as_str() {
        "authorization" | "x-api-key" | "x-goog-api-key" | "api-key" => "[REDACTED]".to_string(),
        _ => header_value.to_string(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_header_redaction_per_nfr_07() {
        assert_eq!(
            redact_header_value("Authorization", "Bearer sk-1234567890"),
            "[REDACTED]"
        );
        assert_eq!(
            redact_header_value("x-api-key", "anthropic-key-abcde"),
            "[REDACTED]"
        );
        assert_eq!(
            redact_header_value("X-Goog-Api-Key", "gemini-key-xyz"),
            "[REDACTED]"
        );
        assert_eq!(
            redact_header_value("api-key", "azure-key-secret"),
            "[REDACTED]"
        );

        // Safe headers must not be redacted
        assert_eq!(
            redact_header_value("Content-Type", "application/json"),
            "application/json"
        );
        assert_eq!(
            redact_header_value("Accept", "text/event-stream"),
            "text/event-stream"
        );
    }
}
