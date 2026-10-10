use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use thiserror::Error;

/// Core application error enum.
///
/// Per AGENTS.md §8.3, every error variant serializes to `{ code: string, params: object }`,
/// where `code` is an i18n key under the `errors.*` namespace.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AppError {
    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Instruction file missing")]
    InstructionMissing,

    #[error("Duplicate chapter prefix: {prefix}")]
    DuplicatePrefix { prefix: String },

    #[error("Invalid docx template: {reason}")]
    InvalidTemplate { reason: String },

    #[error("Docx file is locked")]
    DocxLocked,

    #[error("Provider failed: {reason}")]
    ProviderFailed { reason: String },

    #[error("Failed to parse model output: {reason}")]
    ParseFailed { reason: String },

    #[error("Segment validation failed: {reason}")]
    ValidationFailed { reason: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("Database error: {message}")]
    DatabaseError { message: String },

    #[error("Entity not found: {entity} ({id})")]
    NotFound { entity: String, id: String },

    #[error("Database migration {version} failed: {message}")]
    MigrationError { version: u32, message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl AppError {
    /// Returns the i18n key under `errors.*`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::FileNotFound { .. } => "errors.file_not_found",
            Self::InstructionMissing => "errors.instruction_missing",
            Self::DuplicatePrefix { .. } => "errors.duplicate_prefix",
            Self::InvalidTemplate { .. } => "errors.invalid_template",
            Self::DocxLocked => "errors.docx_locked",
            Self::ProviderFailed { .. } => "errors.provider_failed",
            Self::ParseFailed { .. } => "errors.parse_failed",
            Self::ValidationFailed { .. } => "errors.validation_failed",
            Self::PermissionDenied { .. } => "errors.permission_denied",
            Self::DatabaseError { .. } => "errors.database_error",
            Self::NotFound { .. } => "errors.not_found",
            Self::MigrationError { .. } => "errors.migration_failed",
            Self::Internal { .. } => "errors.unknown",
        }
    }

    /// Returns the parameters object for i18n interpolation.
    pub fn params(&self) -> serde_json::Value {
        match self {
            Self::FileNotFound { path } => serde_json::json!({ "path": path }),
            Self::InstructionMissing => serde_json::json!({}),
            Self::DuplicatePrefix { prefix } => serde_json::json!({ "prefix": prefix }),
            Self::InvalidTemplate { reason } => serde_json::json!({ "reason": reason }),
            Self::DocxLocked => serde_json::json!({}),
            Self::ProviderFailed { reason } => serde_json::json!({ "reason": reason }),
            Self::ParseFailed { reason } => serde_json::json!({ "reason": reason }),
            Self::ValidationFailed { reason } => serde_json::json!({ "reason": reason }),
            Self::PermissionDenied { path } => serde_json::json!({ "path": path }),
            Self::DatabaseError { message } => serde_json::json!({ "message": message }),
            Self::NotFound { entity, id } => serde_json::json!({ "entity": entity, "id": id }),
            Self::MigrationError { version, message } => {
                serde_json::json!({ "version": version, "message": message })
            }
            Self::Internal { message } => serde_json::json!({ "message": message }),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("params", &self.params())?;
        state.end()
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        Self::DatabaseError {
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_error_serialization() {
        let err = AppError::FileNotFound {
            path: "test/path.txt".to_string(),
        };

        let val = serde_json::to_value(&err).unwrap();
        assert_eq!(val["code"], "errors.file_not_found");
        assert_eq!(val["params"]["path"], "test/path.txt");

        let locked = AppError::DocxLocked;
        let locked_val = serde_json::to_value(&locked).unwrap();
        assert_eq!(locked_val["code"], "errors.docx_locked");
        assert_eq!(locked_val["params"], serde_json::json!({}));
    }

    #[test]
    fn test_from_rusqlite() {
        let sqlite_err = rusqlite::Error::QueryReturnedNoRows;
        let app_err = AppError::from(sqlite_err);
        assert_eq!(app_err.code(), "errors.database_error");
    }
}
