//! QA findings and fix-log commands per FR-QA-03..06, Story F1, Story F2.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;
use crate::project::config::ProjectConfig;
use crate::project::layout::ProjectLayout;
use crate::project::settings::{EffectiveSettings, GlobalSettings};
use crate::qa::fixlog::{undo_batch_and_rerender, undo_fix_and_rerender, undo_run_and_rerender};
use crate::store::open_path;
use crate::store::repo::{ChapterRepo, SegmentRepo};

/// A single finding mapped for UI display.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/QaFindingItem.ts")]
pub struct QaFindingItem {
    pub id: String,
    pub check_code: String,
    pub chapter_id: Option<String>,
    pub chapter_ordinal: Option<i32>,
    pub segment_id: Option<String>,
    pub seg_idx: Option<i32>,
    pub severity: String,
    pub message: String,
    pub matched_text: String,
    pub status: String,
}

/// A group of findings by check code.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/QaCheckGroup.ts")]
pub struct QaCheckGroup {
    pub check_code: String,
    pub name: String,
    pub count: usize,
    pub findings: Vec<QaFindingItem>,
}

/// Summary of all QA findings in a volume.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/QaFindingsSummary.ts")]
pub struct QaFindingsSummary {
    pub total_findings: usize,
    pub remaining_findings: usize,
    pub groups: Vec<QaCheckGroup>,
    pub findings: Vec<QaFindingItem>,
}

/// A single fix log item with diff text before and after.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/QaFixLogItem.ts")]
pub struct QaFixLogItem {
    pub id: String,
    pub finding_id: Option<String>,
    pub check_code: Option<String>,
    pub chapter_id: Option<String>,
    pub chapter_ordinal: Option<i32>,
    pub seg_idx: Option<i32>,
    pub origin: String,
    pub applied_at: String,
    pub reverted_at: Option<String>,
    pub text_before: String,
    pub text_after: String,
}

#[tauri::command]
pub fn get_qa_summary(path: String) -> Result<QaFindingsSummary, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    if !layout.db_file.exists() {
        return Ok(QaFindingsSummary {
            total_findings: 0,
            remaining_findings: 0,
            groups: Vec::new(),
            findings: Vec::new(),
        });
    }

    let conn = open_path(&layout.db_file)?;
    let chap_repo = ChapterRepo::new(&conn);
    let seg_repo = SegmentRepo::new(&conn);

    let chapters = chap_repo.list_all()?;
    let chapter_map: HashMap<String, i32> =
        chapters.iter().map(|c| (c.id.clone(), c.ordinal)).collect();

    let mut stmt = conn.prepare(
        "SELECT id, run_id, check_code, chapter_id, segment_id, severity, detail_json, status
         FROM finding ORDER BY rowid ASC",
    )?;

    let mut all_findings = Vec::new();
    let rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let _run_id: Option<String> = row.get(1)?;
        let check_code: String = row.get(2)?;
        let chapter_id: Option<String> = row.get(3)?;
        let segment_id: Option<String> = row.get(4)?;
        let severity: String = row.get(5)?;
        let detail_json: String = row.get(6)?;
        let status: String = row.get(7)?;

        Ok((
            id,
            check_code,
            chapter_id,
            segment_id,
            severity,
            detail_json,
            status,
        ))
    })?;

    for r in rows {
        let (id, check_code, chapter_id, segment_id, severity, detail_json, status) = r?;

        let chapter_ordinal = chapter_id
            .as_ref()
            .and_then(|cid| chapter_map.get(cid).copied());
        let seg_idx = if let Some(ref sid) = segment_id {
            seg_repo.get_by_id(sid)?.map(|s| s.idx)
        } else {
            None
        };

        // Parse detail_json for message / matched text if JSON is present
        let mut message = String::new();
        let mut matched_text = String::new();
        // Checks write {"msg", "matched"}; older rows used the long key names.
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&detail_json) {
            let text = |keys: [&str; 2]| {
                keys.iter()
                    .find_map(|k| val.get(*k).and_then(|v| v.as_str()))
                    .unwrap_or_default()
                    .to_string()
            };
            message = text(["msg", "message"]);
            matched_text = text(["matched", "matched_text"]);
        }

        all_findings.push(QaFindingItem {
            id,
            check_code,
            chapter_id,
            chapter_ordinal,
            segment_id,
            seg_idx,
            severity,
            message,
            matched_text,
            status,
        });
    }

    let total_findings = all_findings.len();
    let remaining_findings = all_findings.iter().filter(|f| f.status != "fixed").count();

    // Group findings by check_code
    let mut groups_map: HashMap<String, Vec<QaFindingItem>> = HashMap::new();
    for f in &all_findings {
        groups_map
            .entry(f.check_code.clone())
            .or_default()
            .push(f.clone());
    }

    let mut groups: Vec<QaCheckGroup> = groups_map
        .into_iter()
        .map(|(code, list)| QaCheckGroup {
            name: code.clone(),
            check_code: code,
            count: list.len(),
            findings: list,
        })
        .collect();
    groups.sort_by(|a, b| a.check_code.cmp(&b.check_code));

    Ok(QaFindingsSummary {
        total_findings,
        remaining_findings,
        groups,
        findings: all_findings,
    })
}

#[tauri::command]
pub fn get_qa_fix_log(path: String) -> Result<Vec<QaFixLogItem>, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    if !layout.db_file.exists() {
        return Ok(Vec::new());
    }

    let conn = open_path(&layout.db_file)?;
    let chap_repo = ChapterRepo::new(&conn);
    let chapters = chap_repo.list_all()?;
    let chapter_map: HashMap<String, i32> =
        chapters.iter().map(|c| (c.id.clone(), c.ordinal)).collect();

    let mut stmt = conn.prepare(
        "SELECT f.id, f.finding_id, f.origin, f.applied_at, f.reverted_at,
                tb.text, ta.text, ta.chapter_id, ta.seg_from, fd.check_code
         FROM fix f
         LEFT JOIN translation tb ON f.translation_id_before = tb.id
         LEFT JOIN translation ta ON f.translation_id_after = ta.id
         LEFT JOIN finding fd ON f.finding_id = fd.id
         ORDER BY f.rowid DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let finding_id: Option<String> = row.get(1)?;
        let origin: String = row.get(2)?;
        let applied_at: String = row.get(3)?;
        let reverted_at: Option<String> = row.get(4)?;
        let text_before: Option<String> = row.get(5)?;
        let text_after: Option<String> = row.get(6)?;
        let chapter_id: Option<String> = row.get(7)?;
        let seg_idx: Option<i32> = row.get(8)?;
        let check_code: Option<String> = row.get(9)?;

        Ok((
            id,
            finding_id,
            origin,
            applied_at,
            reverted_at,
            text_before.unwrap_or_default(),
            text_after.unwrap_or_default(),
            chapter_id,
            seg_idx,
            check_code,
        ))
    })?;

    let mut items = Vec::new();
    for r in rows {
        let (
            id,
            finding_id,
            origin,
            applied_at,
            reverted_at,
            text_before,
            text_after,
            chapter_id,
            seg_idx,
            check_code,
        ) = r?;

        let chapter_ordinal = chapter_id
            .as_ref()
            .and_then(|cid| chapter_map.get(cid).copied());

        items.push(QaFixLogItem {
            id,
            finding_id,
            check_code,
            chapter_id,
            chapter_ordinal,
            seg_idx,
            origin,
            applied_at,
            reverted_at,
            text_before,
            text_after,
        });
    }

    Ok(items)
}

#[tauri::command]
pub fn undo_qa_fix(path: String, fix_id: String) -> Result<(), AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    let conn = open_path(&layout.db_file)?;
    let global = GlobalSettings::load_from_app_data().unwrap_or_default();
    let project = ProjectConfig::load_from_file(&layout.config_file).ok();
    let effective = project
        .as_ref()
        .map(|proj| EffectiveSettings::merge(&global, proj, None));

    undo_fix_and_rerender(&conn, &fix_id, Some(&layout), effective.as_ref(), None)?;
    Ok(())
}

#[tauri::command]
pub fn undo_qa_batch(path: String, fix_ids: Vec<String>) -> Result<(), AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    let conn = open_path(&layout.db_file)?;
    let global = GlobalSettings::load_from_app_data().unwrap_or_default();
    let project = ProjectConfig::load_from_file(&layout.config_file).ok();
    let effective = project
        .as_ref()
        .map(|proj| EffectiveSettings::merge(&global, proj, None));

    undo_batch_and_rerender(&conn, &fix_ids, Some(&layout), effective.as_ref(), None)?;
    Ok(())
}

#[tauri::command]
pub fn undo_qa_run(path: String, run_id: String) -> Result<(), AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    let conn = open_path(&layout.db_file)?;
    let global = GlobalSettings::load_from_app_data().unwrap_or_default();
    let project = ProjectConfig::load_from_file(&layout.config_file).ok();
    let effective = project
        .as_ref()
        .map(|proj| EffectiveSettings::merge(&global, proj, None));

    undo_run_and_rerender(&conn, &run_id, Some(&layout), effective.as_ref(), None)?;
    Ok(())
}
