//! Chapter and segment query commands per FR-REV-01, Story B1.

use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;
use crate::project::layout::ProjectLayout;
use crate::store::open_path;
use crate::store::repo::{
    Chapter, ChapterRepo, Segment, SegmentRepo, Translation, TranslationRepo,
};

/// Full chapter detail including segments and translations.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/ChapterDetail.ts")]
pub struct ChapterDetail {
    pub chapter: Chapter,
    pub segments: Vec<Segment>,
    pub translations: Vec<Translation>,
}

#[tauri::command]
pub fn list_chapters(path: String) -> Result<Vec<Chapter>, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    if !layout.db_file.exists() {
        return Ok(Vec::new());
    }
    let conn = open_path(&layout.db_file)?;
    ChapterRepo::new(&conn).list_all()
}

#[tauri::command]
pub fn get_chapter(path: String, chapter_id: String) -> Result<ChapterDetail, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    let conn = open_path(&layout.db_file)?;

    let chapter = ChapterRepo::new(&conn)
        .get_by_id(&chapter_id)?
        .ok_or_else(|| AppError::NotFound {
            entity: "chapter".to_string(),
            id: chapter_id.clone(),
        })?;

    let segments = SegmentRepo::new(&conn).list_by_chapter(&chapter_id)?;
    let translations = TranslationRepo::new(&conn).get_current_by_chapter(&chapter_id)?;

    Ok(ChapterDetail {
        chapter,
        segments,
        translations,
    })
}

#[tauri::command]
pub fn list_segments(path: String, chapter_id: String) -> Result<Vec<Segment>, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    if !layout.db_file.exists() {
        return Ok(Vec::new());
    }
    let conn = open_path(&layout.db_file)?;
    SegmentRepo::new(&conn).list_by_chapter(&chapter_id)
}

#[tauri::command]
pub fn list_translations(path: String, chapter_id: String) -> Result<Vec<Translation>, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    if !layout.db_file.exists() {
        return Ok(Vec::new());
    }
    let conn = open_path(&layout.db_file)?;
    TranslationRepo::new(&conn).get_current_by_chapter(&chapter_id)
}
