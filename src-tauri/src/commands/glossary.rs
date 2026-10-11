//! Glossary CRUD commands per FR-GLO-01..08, Story D1, D2.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;
use crate::glossary::merge::{apply_glossary_merge, compare_glossaries};
use crate::glossary::parse::{infer_tier, parse_glossary, GlossaryEntryItem, GlossaryLine};
use crate::glossary::write::{fix_macrons_in_place, write_glossary_file};
use crate::profiles::load_profile;
use crate::project::config::ProjectConfig;
use crate::project::layout::ProjectLayout;
use crate::store::open_path;
use crate::store::repo::{GlossaryEntry, GlossaryRepo};

/// Input structure for adding or updating a glossary entry.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/GlossaryEntryInput.ts")]
pub struct GlossaryEntryInput {
    pub source: String,
    pub translation: String,
    pub note: Option<String>,
}

/// A conflict item between current project glossary and imported glossary per Story D3.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/GlossaryConflictItem.ts")]
pub struct GlossaryConflictItem {
    pub source: String,
    pub current_translation: String,
    pub current_note: Option<String>,
    pub incoming_translation: String,
    pub incoming_note: Option<String>,
}

/// Preview of glossary import with added items and conflicts per Story D3.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/GlossaryImportPreview.ts")]
pub struct GlossaryImportPreview {
    pub total_incoming: usize,
    pub added_count: usize,
    pub identical_count: usize,
    pub conflicts: Vec<GlossaryConflictItem>,
    pub new_entries: Vec<GlossaryEntryInput>,
}

fn get_gender_keywords(profile_name: &str) -> Vec<String> {
    load_profile(profile_name)
        .map(|p| p.config.gender_keywords)
        .unwrap_or_else(|_| vec!["laki-laki".to_string(), "perempuan".to_string()])
}

#[tauri::command]
pub fn list_glossary(path: String) -> Result<Vec<GlossaryEntry>, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    if !layout.db_file.exists() {
        return Ok(Vec::new());
    }

    let conn = open_path(&layout.db_file)?;
    let repo = GlossaryRepo::new(&conn);
    let entries = repo.list_all()?;

    if entries.is_empty() && layout.glossary_file.exists() {
        let content = fs::read_to_string(&layout.glossary_file).unwrap_or_default();
        let config = if layout.config_file.exists() {
            ProjectConfig::load_from_file(&layout.config_file).unwrap_or_default()
        } else {
            ProjectConfig::default()
        };

        let gender_keywords = get_gender_keywords(&config.profile);
        let file = parse_glossary(&content, &gender_keywords);
        let mut synced = Vec::new();
        for (idx, line) in file.lines.iter().enumerate() {
            if let GlossaryLine::Entry(ref item) = line {
                let note = if item.note.is_empty() {
                    None
                } else {
                    Some(item.note.clone())
                };
                let entry = GlossaryEntry {
                    id: format!("glo_{}", uuid::Uuid::new_v4()),
                    source: item.source.clone(),
                    translation: item.translation.clone(),
                    note,
                    tier: item.tier,
                    origin: "manual".to_string(),
                    first_chapter_id: None,
                    occurrences: 0,
                    status: "active".to_string(),
                    file_line: Some((idx + 1) as i32),
                };
                let _ = repo.insert(&entry);
                synced.push(entry);
            }
        }
        return Ok(synced);
    }

    Ok(entries)
}

#[tauri::command]
pub fn add_glossary_entry(
    path: String,
    entry: GlossaryEntryInput,
) -> Result<GlossaryEntry, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    let config = if layout.config_file.exists() {
        ProjectConfig::load_from_file(&layout.config_file)?
    } else {
        ProjectConfig::default()
    };

    let content = if layout.glossary_file.exists() {
        fs::read_to_string(&layout.glossary_file).unwrap_or_default()
    } else {
        String::new()
    };

    let gender_keywords = get_gender_keywords(&config.profile);
    let mut file = parse_glossary(&content, &gender_keywords);
    let note_str = entry.note.clone().unwrap_or_default();
    let tier = infer_tier(&note_str, &gender_keywords);

    let item = GlossaryEntryItem {
        source: entry.source.clone(),
        translation: entry.translation.clone(),
        note: note_str,
        tier,
        line_index: file.lines.len(),
        has_list_prefix: false,
        list_prefix_char: None,
    };

    file.lines.push(GlossaryLine::Entry(item));
    write_glossary_file(&layout.glossary_file, &file, Some(&layout.backups_dir))?;

    let conn = open_path(&layout.db_file)?;
    let repo = GlossaryRepo::new(&conn);

    let db_entry = GlossaryEntry {
        id: format!("glo_{}", uuid::Uuid::new_v4()),
        source: entry.source,
        translation: entry.translation,
        note: entry.note,
        tier,
        origin: "manual".to_string(),
        first_chapter_id: None,
        occurrences: 0,
        status: "active".to_string(),
        file_line: Some(file.lines.len() as i32),
    };

    repo.insert(&db_entry)?;
    Ok(db_entry)
}

#[tauri::command]
pub fn update_glossary_entry(path: String, entry: GlossaryEntryInput) -> Result<(), AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    let config = if layout.config_file.exists() {
        ProjectConfig::load_from_file(&layout.config_file)?
    } else {
        ProjectConfig::default()
    };

    let content = if layout.glossary_file.exists() {
        fs::read_to_string(&layout.glossary_file).unwrap_or_default()
    } else {
        String::new()
    };

    let gender_keywords = get_gender_keywords(&config.profile);
    let mut file = parse_glossary(&content, &gender_keywords);
    let note_str = entry.note.clone().unwrap_or_default();
    let tier = infer_tier(&note_str, &gender_keywords);

    for line in &mut file.lines {
        if let GlossaryLine::Entry(ref mut item) = line {
            if item.source == entry.source {
                item.translation = entry.translation.clone();
                item.note = note_str.clone();
                item.tier = tier;
                break;
            }
        }
    }

    write_glossary_file(&layout.glossary_file, &file, Some(&layout.backups_dir))?;

    if layout.db_file.exists() {
        let conn = open_path(&layout.db_file)?;
        let repo = GlossaryRepo::new(&conn);
        if let Some(mut existing) = repo.get_by_source(&entry.source)? {
            existing.translation = entry.translation;
            existing.note = entry.note;
            existing.tier = tier;
            repo.update(&existing)?;
        }
    }

    Ok(())
}

/// Replaces macrons with digraphs in every glossary entry (FR-GLO-06), keeping a backup.
/// Returns how many entries changed.
#[tauri::command]
pub fn fix_glossary_macrons(path: String) -> Result<usize, AppError> {
    let layout = ProjectLayout::new(Path::new(&path));
    if !layout.glossary_file.exists() {
        return Ok(0);
    }
    let config = if layout.config_file.exists() {
        ProjectConfig::load_from_file(&layout.config_file)?
    } else {
        ProjectConfig::default()
    };
    let content = fs::read_to_string(&layout.glossary_file).unwrap_or_default();
    let mut file = parse_glossary(&content, &get_gender_keywords(&config.profile));
    let changed = fix_macrons_in_place(&mut file);
    if changed == 0 {
        return Ok(0);
    }
    write_glossary_file(&layout.glossary_file, &file, Some(&layout.backups_dir))?;

    if layout.db_file.exists() {
        let conn = open_path(&layout.db_file)?;
        let repo = GlossaryRepo::new(&conn);
        for line in &file.lines {
            if let GlossaryLine::Entry(item) = line {
                if let Some(mut existing) = repo.get_by_source(&item.source)? {
                    let note = (!item.note.is_empty()).then(|| item.note.clone());
                    if existing.translation != item.translation || existing.note != note {
                        existing.translation = item.translation.clone();
                        existing.note = note;
                        repo.update(&existing)?;
                    }
                }
            }
        }
    }
    Ok(changed)
}

#[tauri::command]
pub fn delete_glossary_entry(path: String, source: String) -> Result<(), AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    let config = if layout.config_file.exists() {
        ProjectConfig::load_from_file(&layout.config_file)?
    } else {
        ProjectConfig::default()
    };

    let content = if layout.glossary_file.exists() {
        fs::read_to_string(&layout.glossary_file).unwrap_or_default()
    } else {
        String::new()
    };

    let gender_keywords = get_gender_keywords(&config.profile);
    let mut file = parse_glossary(&content, &gender_keywords);
    file.lines.retain(|line| match line {
        GlossaryLine::Entry(item) => item.source != source,
        GlossaryLine::NonEntry(_) => true,
    });

    write_glossary_file(&layout.glossary_file, &file, Some(&layout.backups_dir))?;

    if layout.db_file.exists() {
        let conn = open_path(&layout.db_file)?;
        let repo = GlossaryRepo::new(&conn);
        if let Some(existing) = repo.get_by_source(&source)? {
            repo.delete(&existing.id)?;
        }
    }

    Ok(())
}

/// Inspects an incoming glossary file or project folder, comparing entries against current project.
#[tauri::command]
pub fn inspect_glossary_import(
    target_path: String,
    source_path: String,
) -> Result<GlossaryImportPreview, AppError> {
    let target_p = Path::new(&target_path);
    let target_layout = ProjectLayout::new(target_p);
    let config = if target_layout.config_file.exists() {
        ProjectConfig::load_from_file(&target_layout.config_file).unwrap_or_default()
    } else {
        ProjectConfig::default()
    };
    let gender_keywords = get_gender_keywords(&config.profile);

    let target_content = if target_layout.glossary_file.exists() {
        fs::read_to_string(&target_layout.glossary_file).unwrap_or_default()
    } else {
        String::new()
    };
    let target_file = parse_glossary(&target_content, &gender_keywords);

    // Resolve source file: can be directory containing glossary.md or direct markdown file
    let source_p = Path::new(&source_path);
    let resolved_source_file = if source_p.is_dir() {
        source_p.join("glossary.md")
    } else {
        source_p.to_path_buf()
    };

    if !resolved_source_file.exists() {
        return Err(AppError::FileNotFound {
            path: resolved_source_file.display().to_string(),
        });
    }

    let source_content =
        fs::read_to_string(&resolved_source_file).map_err(|e| AppError::Internal {
            message: format!("Failed to read source glossary: {e}"),
        })?;

    let source_file = parse_glossary(&source_content, &gender_keywords);
    let comparison = compare_glossaries(&target_file, &source_file);

    let conflicts: Vec<GlossaryConflictItem> = comparison
        .conflicts
        .into_iter()
        .map(|c| GlossaryConflictItem {
            source: c.source,
            current_translation: c.current_translation,
            current_note: if c.current_note.is_empty() {
                None
            } else {
                Some(c.current_note)
            },
            incoming_translation: c.incoming_translation,
            incoming_note: if c.incoming_note.is_empty() {
                None
            } else {
                Some(c.incoming_note)
            },
        })
        .collect();

    let new_entries: Vec<GlossaryEntryInput> = comparison
        .added
        .into_iter()
        .map(|a| GlossaryEntryInput {
            source: a.source,
            translation: a.translation,
            note: if a.note.is_empty() {
                None
            } else {
                Some(a.note)
            },
        })
        .collect();

    Ok(GlossaryImportPreview {
        total_incoming: source_file.entries().len(),
        added_count: new_entries.len(),
        identical_count: comparison.identical.len(),
        conflicts,
        new_entries,
    })
}

/// Applies imported entries to target project glossary, writing to disk and updating database.
#[tauri::command]
pub fn apply_glossary_import(
    target_path: String,
    entries_to_apply: Vec<GlossaryEntryInput>,
) -> Result<usize, AppError> {
    let target_p = Path::new(&target_path);
    let target_layout = ProjectLayout::new(target_p);
    let config = if target_layout.config_file.exists() {
        ProjectConfig::load_from_file(&target_layout.config_file).unwrap_or_default()
    } else {
        ProjectConfig::default()
    };
    let gender_keywords = get_gender_keywords(&config.profile);

    let target_content = if target_layout.glossary_file.exists() {
        fs::read_to_string(&target_layout.glossary_file).unwrap_or_default()
    } else {
        String::new()
    };
    let mut target_file = parse_glossary(&target_content, &gender_keywords);

    let items: Vec<GlossaryEntryItem> = entries_to_apply
        .into_iter()
        .map(|e| {
            let note_str = e.note.unwrap_or_default();
            let tier = infer_tier(&note_str, &gender_keywords);
            GlossaryEntryItem {
                source: e.source,
                translation: e.translation,
                note: note_str,
                tier,
                line_index: 0,
                has_list_prefix: false,
                list_prefix_char: None,
            }
        })
        .collect();

    apply_glossary_merge(&mut target_file, &items, &gender_keywords);
    write_glossary_file(
        &target_layout.glossary_file,
        &target_file,
        Some(&target_layout.backups_dir),
    )?;

    if target_layout.db_file.exists() {
        let conn = open_path(&target_layout.db_file)?;
        let repo = GlossaryRepo::new(&conn);
        for item in &items {
            let note = if item.note.is_empty() {
                None
            } else {
                Some(item.note.clone())
            };
            if let Some(mut existing) = repo.get_by_source(&item.source)? {
                existing.translation = item.translation.clone();
                existing.note = note;
                existing.tier = item.tier;
                repo.update(&existing)?;
            } else {
                let entry = GlossaryEntry {
                    id: format!("glo_{}", uuid::Uuid::new_v4()),
                    source: item.source.clone(),
                    translation: item.translation.clone(),
                    note,
                    tier: item.tier,
                    origin: "imported".to_string(),
                    first_chapter_id: None,
                    occurrences: 0,
                    status: "active".to_string(),
                    file_line: Some(item.line_index as i32),
                };
                repo.insert(&entry)?;
            }
        }
    }

    Ok(items.len())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_inspect_and_apply_glossary_import() {
        let target_dir =
            std::env::temp_dir().join(format!("lnta_test_glo_tgt_{}", uuid::Uuid::new_v4()));
        let source_dir =
            std::env::temp_dir().join(format!("lnta_test_glo_src_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&target_dir).unwrap();
        std::fs::create_dir_all(&source_dir).unwrap();

        let target_layout = ProjectLayout::new(&target_dir);
        target_layout.ensure_layout("ja-id").unwrap();

        let source_layout = ProjectLayout::new(&source_dir);
        source_layout.ensure_layout("ja-id").unwrap();

        // Target glossary
        let target_content = "佐藤 === Satou === laki-laki\n美咲 === Misaki === perempuan; adik\n";
        std::fs::write(&target_layout.glossary_file, target_content).unwrap();

        // Source glossary
        let source_content = "佐藤 === Satou === laki-laki\n美咲 === Misaki === perempuan; saudari\n健太 === Kenta === laki-laki; teman\n";
        std::fs::write(&source_layout.glossary_file, source_content).unwrap();

        let preview = inspect_glossary_import(
            target_dir.to_str().unwrap().to_string(),
            source_dir.to_str().unwrap().to_string(),
        )
        .unwrap();

        assert_eq!(preview.total_incoming, 3);
        assert_eq!(preview.identical_count, 1);
        assert_eq!(preview.added_count, 1);
        assert_eq!(preview.conflicts.len(), 1);

        assert_eq!(preview.conflicts[0].source, "美咲");
        assert_eq!(preview.new_entries[0].source, "健太");

        // Apply merge with incoming choice for conflict
        let entries_to_apply = vec![
            preview.new_entries[0].clone(),
            GlossaryEntryInput {
                source: "美咲".to_string(),
                translation: "Misaki".to_string(),
                note: Some("perempuan; saudari".to_string()),
            },
        ];

        let applied =
            apply_glossary_import(target_dir.to_str().unwrap().to_string(), entries_to_apply)
                .unwrap();

        assert_eq!(applied, 2);

        // Check updated file
        let updated = std::fs::read_to_string(&target_layout.glossary_file).unwrap();
        assert!(updated.contains("健太 === Kenta"));
        assert!(updated.contains("perempuan; saudari"));

        // Check database
        let conn = open_path(&target_layout.db_file).unwrap();
        let repo = GlossaryRepo::new(&conn);
        let kenta = repo.get_by_source("健太").unwrap().unwrap();
        assert_eq!(kenta.translation, "Kenta");
        assert_eq!(kenta.tier, 1);

        let _ = std::fs::remove_dir_all(&target_dir);
        let _ = std::fs::remove_dir_all(&source_dir);
    }

    #[test]
    fn test_fix_glossary_macrons_rewrites_file() {
        let dir = std::env::temp_dir().join(format!("lnta_test_glo_mac_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("glossary.md"),
            "\u{592a}\u{90ce} === Tar\u{014d} === laki-laki\n",
        )
        .unwrap();
        let path = dir.to_string_lossy().to_string();

        assert_eq!(fix_glossary_macrons(path.clone()).unwrap(), 1);
        let text = std::fs::read_to_string(dir.join("glossary.md")).unwrap();
        assert!(text.contains("Tarou"), "{text}");
        assert_eq!(fix_glossary_macrons(path).unwrap(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
