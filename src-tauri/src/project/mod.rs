//! LNTA project management module.

pub mod config;
pub mod discover;
pub mod instruction;
pub mod layout;
pub mod preflight;

use std::path::{Path, PathBuf};

pub use config::ProjectConfig;
pub use discover::{discover_chapters, DiscoveredChapter};
pub use instruction::{resolve_instruction, resolve_instruction_candidates};
pub use layout::ProjectLayout;
pub use preflight::{
    detect_instruction_language, run_preflight, PreflightAction, PreflightEnvironment,
    PreflightItem, PreflightReport, PreflightStatus,
};

use crate::error::AppError;

/// An opened LNTA project instance.
#[derive(Debug, Clone)]
pub struct Project {
    pub layout: ProjectLayout,
    pub config: ProjectConfig,
    pub chapters: Vec<DiscoveredChapter>,
    pub instruction_path: PathBuf,
}

/// Opens an existing project or initializes a new project folder with layout and discovery.
pub fn open_project<P: AsRef<Path>>(root: P, profile: &str) -> Result<Project, AppError> {
    let layout = ProjectLayout::new(root);
    let mut config = layout.ensure_layout(profile)?;

    let chapters = discover_chapters(&layout.root)?;
    let instruction_path = resolve_instruction(&layout.root, profile)?;

    // Store resolved instruction file in config if not already set
    if config.instruction_file.is_none() {
        if let Some(file_name) = instruction_path.file_name() {
            config.instruction_file = Some(file_name.to_string_lossy().to_string());
            let _ = config.save_to_file(&layout.config_file);
        }
    }

    Ok(Project {
        layout,
        config,
        chapters,
        instruction_path,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use std::fs;

    fn create_temp_dir(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("lnta_test_{}_{}", name, uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn test_layout_creation_and_reset() {
        let temp = create_temp_dir("layout");
        fs::write(temp.join("instruction.md"), "# Instruksi").unwrap();

        let layout = ProjectLayout::new(&temp);
        let cfg = layout.ensure_layout("ja-id").unwrap();
        assert_eq!(cfg.profile, "ja-id");

        assert!(layout.lnta_dir.exists());
        assert!(layout.config_file.exists());
        assert!(layout.db_file.exists());
        assert!(layout.template_file.exists());
        assert!(layout.backups_dir.exists());
        assert!(layout.logs_dir.exists());

        // Reset
        layout.reset_project().unwrap();
        assert!(!layout.lnta_dir.exists());
        // User files must remain intact
        assert!(temp.join("instruction.md").exists());

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_numeric_sort_and_exclusions() {
        let temp = create_temp_dir("sort");
        fs::write(temp.join("1_prologue.md"), "# Prologue").unwrap();
        fs::write(temp.join("10_chapter.md"), "# Chapter 10").unwrap();
        fs::write(temp.join("9_chapter.md"), "# Chapter 9").unwrap();
        fs::write(temp.join("2_chapter.md"), "# Chapter 2").unwrap();

        // Non-chapter files to exclude
        fs::write(temp.join("instruction.md"), "instruction").unwrap();
        fs::write(temp.join("instructions.md"), "instructions").unwrap();
        fs::write(temp.join("start.md"), "start").unwrap();
        fs::write(temp.join("glossary.md"), "glossary").unwrap();
        fs::write(temp.join("README.md"), "readme").unwrap();

        // Subfolders should be ignored
        let sub = temp.join("illustrations");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("3_subchapter.md"), "ignored").unwrap();

        let chapters = discover_chapters(&temp).unwrap();
        assert_eq!(chapters.len(), 4);

        // Numeric order: 1, 2, 9, 10
        assert_eq!(chapters[0].prefix, "1");
        assert_eq!(chapters[0].ordinal, 1);
        assert_eq!(chapters[1].prefix, "2");
        assert_eq!(chapters[1].ordinal, 2);
        assert_eq!(chapters[2].prefix, "9");
        assert_eq!(chapters[2].ordinal, 3);
        assert_eq!(chapters[3].prefix, "10");
        assert_eq!(chapters[3].ordinal, 4);

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_duplicate_prefix_error() {
        let temp = create_temp_dir("duplicate");
        fs::write(temp.join("1_first.md"), "first").unwrap();
        fs::write(temp.join("1_second.md"), "second").unwrap();

        let err = discover_chapters(&temp).unwrap_err();
        assert!(matches!(err, AppError::DuplicatePrefix { .. }));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_instruction_resolution_ja_id() {
        let temp = create_temp_dir("instr_id");
        fs::write(temp.join("instruction-id.md"), "id specific").unwrap();
        fs::write(temp.join("instructions.md"), "general instructions").unwrap();
        fs::write(temp.join("instruction.md"), "highest priority").unwrap();
        fs::write(temp.join("instruction-en.md"), "en specific").unwrap();

        let resolved = resolve_instruction(&temp, "ja-id").unwrap();
        assert_eq!(resolved.file_name().unwrap(), "instruction.md");

        // Remove instruction.md; next should be instructions.md
        fs::remove_file(temp.join("instruction.md")).unwrap();
        let resolved2 = resolve_instruction(&temp, "ja-id").unwrap();
        assert_eq!(resolved2.file_name().unwrap(), "instructions.md");

        // Remove instructions.md; next should be instruction-id.md
        fs::remove_file(temp.join("instructions.md")).unwrap();
        let resolved3 = resolve_instruction(&temp, "ja-id").unwrap();
        assert_eq!(resolved3.file_name().unwrap(), "instruction-id.md");

        // instruction-en.md is ignored for ja-id; removing instruction-id.md yields InstructionMissing
        fs::remove_file(temp.join("instruction-id.md")).unwrap();
        let err = resolve_instruction(&temp, "ja-id").unwrap_err();
        assert_eq!(err, AppError::InstructionMissing);

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_instruction_resolution_ja_en() {
        let temp = create_temp_dir("instr_en");
        fs::write(temp.join("instruction-en.md"), "highest for en").unwrap();
        fs::write(temp.join("instructions-en.md"), "second for en").unwrap();
        fs::write(temp.join("instruction.md"), "third for en").unwrap();

        let resolved = resolve_instruction(&temp, "ja-en").unwrap();
        assert_eq!(resolved.file_name().unwrap(), "instruction-en.md");

        fs::remove_file(temp.join("instruction-en.md")).unwrap();
        let resolved2 = resolve_instruction(&temp, "ja-en").unwrap();
        assert_eq!(resolved2.file_name().unwrap(), "instructions-en.md");

        fs::remove_file(temp.join("instructions-en.md")).unwrap();
        let resolved3 = resolve_instruction(&temp, "ja-en").unwrap();
        assert_eq!(resolved3.file_name().unwrap(), "instruction.md");

        let _ = fs::remove_dir_all(&temp);
    }
}
