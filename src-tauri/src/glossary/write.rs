//! Glossary serialization, atomic writes, backups, and in-place macron fixes per FR-GLO-06 and FR-GLO-08.

use std::path::{Path, PathBuf};

use chrono::Utc;
use uuid::Uuid;

use crate::error::AppError;

use super::parse::{convert_macrons, GlossaryFile, GlossaryLine};

/// Serializes a `GlossaryFile` to a string, preserving non-entry lines, formatting, and order.
pub fn serialize_glossary(file: &GlossaryFile) -> String {
    let mut lines_out = Vec::with_capacity(file.lines.len());

    for line in &file.lines {
        match line {
            GlossaryLine::NonEntry(text) => lines_out.push(text.clone()),
            GlossaryLine::Entry(item) => {
                let prefix = if item.has_list_prefix {
                    let p_char = item.list_prefix_char.unwrap_or('-');
                    format!("{p_char} ")
                } else {
                    String::new()
                };

                let formatted = if item.note.is_empty() {
                    format!("{prefix}{} === {}", item.source, item.translation)
                } else {
                    format!(
                        "{prefix}{} === {} === {}",
                        item.source, item.translation, item.note
                    )
                };
                lines_out.push(formatted);
            }
        }
    }

    lines_out.join("\n")
}

/// Converts macrons to digraphs in all entries of a `GlossaryFile` in place per FR-GLO-06.
/// Returns the number of entries modified.
pub fn fix_macrons_in_place(file: &mut GlossaryFile) -> usize {
    let mut modified_count = 0;

    for line in &mut file.lines {
        if let GlossaryLine::Entry(ref mut item) = line {
            let (new_trans, mod_trans) = convert_macrons(&item.translation);
            let (new_note, mod_note) = convert_macrons(&item.note);

            if mod_trans || mod_note {
                item.translation = new_trans;
                item.note = new_note;
                modified_count += 1;
            }
        }
    }

    modified_count
}

/// Creates a timestamped backup copy of a file before overwriting it.
pub fn backup_glossary_file(
    file_path: &Path,
    backup_dir: &Path,
) -> Result<Option<PathBuf>, AppError> {
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
    let backup_name = format!("glossary_{timestamp}.md");
    let backup_path = backup_dir.join(backup_name);

    std::fs::copy(file_path, &backup_path).map_err(|e| AppError::Internal {
        message: format!(
            "Failed to backup glossary to {}: {e}",
            backup_path.display()
        ),
    })?;

    Ok(Some(backup_path))
}

/// Atomically writes a `GlossaryFile` to disk with an automatic backup per AGENTS.md §8.5.
pub fn write_glossary_file(
    target_path: &Path,
    file: &GlossaryFile,
    backup_dir: Option<&Path>,
) -> Result<(), AppError> {
    // 1. Create backup if target file exists and backup_dir is provided
    if let Some(b_dir) = backup_dir {
        backup_glossary_file(target_path, b_dir)?;
    }

    let parent_dir = target_path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent_dir).map_err(|e| AppError::Internal {
        message: format!(
            "Failed to create parent directory for {}: {e}",
            target_path.display()
        ),
    })?;

    let content = serialize_glossary(file);

    // 2. Atomic write: write to temp file in same directory, then rename
    let temp_name = format!("glossary_{}.tmp", Uuid::new_v4().simple());
    let temp_path = parent_dir.join(temp_name);

    std::fs::write(&temp_path, content.as_bytes()).map_err(|e| AppError::Internal {
        message: format!(
            "Failed to write temp glossary file {}: {e}",
            temp_path.display()
        ),
    })?;

    std::fs::rename(&temp_path, target_path).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        AppError::Internal {
            message: format!(
                "Failed to replace glossary file {} with temp file: {e}",
                target_path.display()
            ),
        }
    })?;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::glossary::parse::parse_glossary;

    #[test]
    fn test_rewrite_preserves_non_entry_lines_and_order() {
        let original =
            "# Character Glossary\n\n- 佐藤 === Satou === laki-laki\n\n// Note\n鈴木 === Suzuki";
        let keywords = vec!["laki-laki".to_string()];
        let g = parse_glossary(original, &keywords);

        let serialized = serialize_glossary(&g);
        assert_eq!(serialized, original);
    }

    #[test]
    fn test_macron_to_digraph_fixes_in_place_with_backup() {
        let temp_dir = std::env::temp_dir().join(format!("lnta_test_glo_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let glo_path = temp_dir.join("glossary.md");
        let backup_dir = temp_dir.join("backups");

        let original = "- 佐藤 === Satō === Tōkyō kara kita";
        std::fs::write(&glo_path, original).unwrap();

        let keywords = vec!["laki-laki".to_string()];
        let mut g = parse_glossary(original, &keywords);

        // Fix macrons in place
        let fixed_count = fix_macrons_in_place(&mut g);
        assert_eq!(fixed_count, 1);

        // Write with backup
        write_glossary_file(&glo_path, &g, Some(&backup_dir)).unwrap();

        // Check content has digraphs
        let updated = std::fs::read_to_string(&glo_path).unwrap();
        assert_eq!(updated, "- 佐藤 === Satou === Toukyou kara kita");

        // Check backup exists and has original content
        let backups: Vec<_> = std::fs::read_dir(&backup_dir).unwrap().flatten().collect();
        assert_eq!(backups.len(), 1);
        let backup_content = std::fs::read_to_string(backups[0].path()).unwrap();
        assert_eq!(backup_content, original);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
