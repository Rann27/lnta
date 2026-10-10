//! Glossary CRUD commands per FR-GLO-01..08, Story D1, D2.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;
use crate::glossary::parse::{infer_tier, parse_glossary, GlossaryEntryItem, GlossaryLine};
use crate::glossary::write::write_glossary_file;
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
