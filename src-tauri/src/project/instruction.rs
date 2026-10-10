use std::fs;
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// Resolves the candidate instruction files for a project root according to the profile order in PRD §4.2.
pub fn resolve_instruction_candidates<P: AsRef<Path>>(
    root: P,
    profile: &str,
) -> Result<Vec<PathBuf>, AppError> {
    let root_path = root.as_ref();
    if !root_path.exists() {
        return Err(AppError::FileNotFound {
            path: root_path.display().to_string(),
        });
    }

    let entries = fs::read_dir(root_path).map_err(|e| AppError::PermissionDenied {
        path: format!("{}: {e}", root_path.display()),
    })?;

    let mut instruction_files: Vec<(String, PathBuf)> = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| AppError::Internal {
            message: format!("Failed to read dir entry: {e}"),
        })?;

        if !entry.file_type().is_ok_and(|ft| ft.is_file()) {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let lower = name.to_lowercase();

        if lower.starts_with("instruction") && lower.ends_with(".md") {
            instruction_files.push((lower, entry.path()));
        }
    }

    if instruction_files.is_empty() {
        return Err(AppError::InstructionMissing);
    }

    let mut ranked: Vec<(i32, PathBuf)> = Vec::new();

    if profile == "ja-en" {
        // Candidate order per PRD §4.2 for ja-en:
        // 1. instruction-en.md
        // 2. instructions-en.md
        // 3. instruction.md
        // 4. instructions.md
        // 5. instruction*.md
        for (lower, path) in instruction_files {
            let rank = if lower == "instruction-en.md" {
                1
            } else if lower == "instructions-en.md" {
                2
            } else if lower == "instruction.md" {
                3
            } else if lower == "instructions.md" {
                4
            } else {
                5
            };
            ranked.push((rank, path));
        }
    } else {
        // Candidate order per PRD §4.2 for ja-id:
        // 1. instruction.md
        // 2. instructions.md
        // 3. instruction-id.md
        // 4. instruction*.md except *-en.md
        for (lower, path) in instruction_files {
            if lower.ends_with("-en.md") {
                // Excluded from ja-id
                continue;
            }

            let rank = if lower == "instruction.md" {
                1
            } else if lower == "instructions.md" {
                2
            } else if lower == "instruction-id.md" {
                3
            } else {
                4
            };
            ranked.push((rank, path));
        }
    }

    if ranked.is_empty() {
        return Err(AppError::InstructionMissing);
    }

    // Sort by rank ascending
    ranked.sort_by_key(|item| item.0);

    Ok(ranked.into_iter().map(|item| item.1).collect())
}

/// Resolves the single highest-priority instruction file for the project root and profile.
pub fn resolve_instruction<P: AsRef<Path>>(root: P, profile: &str) -> Result<PathBuf, AppError> {
    let candidates = resolve_instruction_candidates(root, profile)?;
    candidates
        .into_iter()
        .next()
        .ok_or(AppError::InstructionMissing)
}
