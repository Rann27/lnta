use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredChapter {
    pub file_name: String,
    pub path: PathBuf,
    pub prefix: String,
    pub prefix_num: u32,
    pub ordinal: i32,
    pub title_candidate: String,
}

const EXCLUDED_PREFIXES: &[&str] = &["instruction", "instructions", "start", "glossary", "readme"];

/// Discovers and numerically sorts chapter markdown files in the project root (FR-PRJ-02, FR-PRJ-03).
pub fn discover_chapters<P: AsRef<Path>>(root: P) -> Result<Vec<DiscoveredChapter>, AppError> {
    let root_path = root.as_ref();
    if !root_path.exists() {
        return Err(AppError::FileNotFound {
            path: root_path.display().to_string(),
        });
    }

    let pattern = Regex::new(r"(?i)^(\d+)[ _\-.]?(.*)\.md$").map_err(|e| AppError::Internal {
        message: format!("Regex compilation failed: {e}"),
    })?;

    let entries = fs::read_dir(root_path).map_err(|e| AppError::PermissionDenied {
        path: format!("{}: {e}", root_path.display()),
    })?;

    let mut raw_chapters = Vec::new();
    let mut prefix_counts: HashMap<u32, Vec<String>> = HashMap::new();

    for entry in entries {
        let entry = entry.map_err(|e| AppError::Internal {
            message: format!("Failed to read dir entry: {e}"),
        })?;

        let file_type = entry.file_type().map_err(|e| AppError::Internal {
            message: format!("Failed to get file type: {e}"),
        })?;

        // Root files only (ignore subfolders like scripts/, illustrations/, .lnta/)
        if !file_type.is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        let lower = file_name.to_lowercase();

        if !lower.ends_with(".md") {
            continue;
        }

        // Check exclusions (instruction, start, glossary, readme)
        let is_excluded = EXCLUDED_PREFIXES
            .iter()
            .any(|&prefix| lower.starts_with(prefix));
        if is_excluded {
            continue;
        }

        if let Some(caps) = pattern.captures(&file_name) {
            let prefix_str = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let title_part = caps.get(2).map_or("", |m| m.as_str()).trim().to_string();

            let prefix_num: u32 = prefix_str.parse().map_err(|_| AppError::Internal {
                message: format!("Failed to parse chapter prefix as number: {prefix_str}"),
            })?;

            prefix_counts
                .entry(prefix_num)
                .or_default()
                .push(file_name.clone());

            raw_chapters.push((file_name, entry.path(), prefix_str, prefix_num, title_part));
        }
    }

    // Check for duplicate prefixes (FR-PRJ-03)
    for (prefix_num, files) in prefix_counts {
        if files.len() > 1 {
            return Err(AppError::DuplicatePrefix {
                prefix: format!("{prefix_num} ({})", files.join(", ")),
            });
        }
    }

    // Sort numerically by prefix_num
    raw_chapters.sort_by_key(|item| item.3);

    let chapters = raw_chapters
        .into_iter()
        .enumerate()
        .map(
            |(i, (file_name, path, prefix, prefix_num, title_candidate))| DiscoveredChapter {
                file_name,
                path,
                prefix,
                prefix_num,
                ordinal: (i + 1) as i32,
                title_candidate,
            },
        )
        .collect();

    Ok(chapters)
}
