//! Docx file persistence, locked file detection, external backups, and atomic writes.

use std::path::{Path, PathBuf};

use chrono::Utc;
use uuid::Uuid;

use crate::error::AppError;

use super::body::{inject_chapters_into_body, RenderChapter};
use super::styles::find_heading1_style_id;
use super::template::DocxPackage;
use super::template_content::TemplateTreatment;

/// Checks whether a target file is currently locked by Word or another process.
/// On Windows, Word opens documents with exclusive sharing modes, yielding ERROR_SHARING_VIOLATION (32)
/// or PermissionDenied.
pub fn check_file_locked(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Ok(());
    }

    match std::fs::OpenOptions::new().write(true).open(path) {
        Ok(_) => Ok(()),
        Err(e) => {
            // Check for Windows error code 32 (ERROR_SHARING_VIOLATION) or PermissionDenied
            let is_locked =
                e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(32);

            if is_locked {
                Err(AppError::DocxLocked)
            } else {
                Err(AppError::Internal {
                    message: format!("Failed to access file {}: {e}", path.display()),
                })
            }
        }
    }
}

/// Backs up `Translations.docx` to the backups directory before overwriting.
pub fn backup_docx_file(file_path: &Path, backup_dir: &Path) -> Result<Option<PathBuf>, AppError> {
    if !file_path.exists() {
        return Ok(None);
    }

    std::fs::create_dir_all(backup_dir).map_err(|e| AppError::Internal {
        message: format!(
            "Failed to create backup directory {}: {e}",
            backup_dir.display()
        ),
    })?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let backup_name = format!("Translations_{timestamp}.docx");
    let backup_path = backup_dir.join(backup_name);

    std::fs::copy(file_path, &backup_path).map_err(|e| AppError::Internal {
        message: format!("Failed to backup docx to {}: {e}", backup_path.display()),
    })?;

    Ok(Some(backup_path))
}

/// Renders chapters into a docx package with a specific `TemplateTreatment` and writes it atomically to `target_path`.
pub fn render_and_write_docx_with_treatment(
    template_bytes: &[u8],
    chapters: &[RenderChapter],
    target_path: &Path,
    backup_dir: Option<&Path>,
    treatment: TemplateTreatment,
) -> Result<String, AppError> {
    // 1. Check if locked
    check_file_locked(target_path)?;

    // 2. Load template package
    let mut package = DocxPackage::from_bytes(template_bytes)?;

    // 3. Find Heading 1 style from word/styles.xml
    let styles_xml = package
        .get_text_entry("word/styles.xml")
        .unwrap_or_default();
    let heading_style_id = find_heading1_style_id(&styles_xml);

    // 4. Update word/document.xml
    let doc_xml = package
        .get_text_entry("word/document.xml")
        .unwrap_or_default();
    let new_doc_xml = inject_chapters_into_body(&doc_xml, chapters, &heading_style_id, treatment);
    package.set_text_entry("word/document.xml", &new_doc_xml);

    // 5. Serialize package to bytes
    let output_bytes = package.to_bytes()?;

    // 6. Backup existing file if requested
    if let Some(b_dir) = backup_dir {
        backup_docx_file(target_path, b_dir)?;
    }

    // 7. Atomic write: write to temp file in same directory, then rename
    let parent_dir = target_path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent_dir).map_err(|e| AppError::Internal {
        message: format!("Failed to create directory {}: {e}", parent_dir.display()),
    })?;

    let temp_name = format!("docx_{}.tmp", Uuid::new_v4().simple());
    let temp_path = parent_dir.join(temp_name);

    std::fs::write(&temp_path, &output_bytes).map_err(|e| AppError::Internal {
        message: format!("Failed to write temp docx {}: {e}", temp_path.display()),
    })?;

    std::fs::rename(&temp_path, target_path).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        AppError::Internal {
            message: format!(
                "Failed to replace {} with temp docx: {e}",
                target_path.display()
            ),
        }
    })?;

    Ok(new_doc_xml)
}

/// Renders chapters into a docx package with default `TemplateTreatment::KeepFrontMatter` and writes it atomically to `target_path`.
///
/// If `target_path` is locked, returns `AppError::DocxLocked` without modifying anything.
pub fn render_and_write_docx(
    template_bytes: &[u8],
    chapters: &[RenderChapter],
    target_path: &Path,
    backup_dir: Option<&Path>,
) -> Result<String, AppError> {
    render_and_write_docx_with_treatment(
        template_bytes,
        chapters,
        target_path,
        backup_dir,
        TemplateTreatment::KeepFrontMatter,
    )
}

/// Atomically writes a raw DocxPackage to `target_path`.
pub fn write_docx_package_atomically(
    package: &DocxPackage,
    target_path: &Path,
    backup_dir: Option<&Path>,
) -> Result<(), AppError> {
    check_file_locked(target_path)?;
    let output_bytes = package.to_bytes()?;

    if let Some(b_dir) = backup_dir {
        backup_docx_file(target_path, b_dir)?;
    }

    let parent_dir = target_path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent_dir).map_err(|e| AppError::Internal {
        message: format!("Failed to create directory {}: {e}", parent_dir.display()),
    })?;

    let temp_name = format!("docx_{}.tmp", Uuid::new_v4().simple());
    let temp_path = parent_dir.join(temp_name);

    std::fs::write(&temp_path, &output_bytes).map_err(|e| AppError::Internal {
        message: format!("Failed to write temp docx {}: {e}", temp_path.display()),
    })?;

    std::fs::rename(&temp_path, target_path).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        AppError::Internal {
            message: format!(
                "Failed to replace {} with temp docx: {e}",
                target_path.display()
            ),
        }
    })?;

    Ok(())
}
