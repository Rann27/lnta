//! Fix log management, application of mechanical fixes, and versioned undo.
//!
//! Implements FR-QA-03, FR-QA-06, and Story F2:
//! - Mechanical fixes applied as new translation versions linked in `fix` table
//! - Full undo support per fix, per batch, and per run
//! - Automatic re-render of Translations.docx after undo

use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::engine::chapter::render_docx_after_chapter;
use crate::engine::events::{EventEmitter, RecordingEventEmitter};
use crate::error::AppError;
use crate::project::layout::ProjectLayout;
use crate::project::settings::EffectiveSettings;
use crate::qa::fix_mechanical::{apply_mechanical_fixes_to_text, MechanicalFixOptions};
use crate::store::repo::{ChapterRepo, Finding, FindingRepo, Fix, FixRepo, TranslationRepo};

/// Result of applying mechanical fixes to a volume.
#[derive(Debug, Clone, Default)]
pub struct ApplyMechanicalResult {
    pub applied_count: usize,
    pub created_fixes: Vec<Fix>,
}

/// Applies mechanical fixes to all active translations across all non-frozen chapters.
///
/// Each changed segment gets a new `Translation` version in SQLite with origin="fix_mechanical",
/// and a corresponding `Fix` record linking the previous and new version.
pub fn apply_mechanical_fixes_to_volume(
    conn: &Connection,
    run_id: Option<&str>,
    profile: &str,
    glossary_terms: &[&str],
) -> Result<ApplyMechanicalResult, AppError> {
    let chapter_repo = ChapterRepo::new(conn);
    let trans_repo = TranslationRepo::new(conn);
    let finding_repo = FindingRepo::new(conn);
    let fix_repo = FixRepo::new(conn);
    let run_repo = crate::store::repo::RunRepo::new(conn);

    let valid_run_id = if let Some(r) = run_id {
        if run_repo.get_by_id(r).unwrap_or(None).is_some() {
            Some(r.to_string())
        } else {
            None
        }
    } else {
        None
    };

    let all_chapters = chapter_repo.list_all()?;
    let mut created_fixes = Vec::new();

    for chap in &all_chapters {
        if chap.frozen {
            continue;
        }

        let current_translations = trans_repo.get_current_by_chapter(&chap.id)?;

        for tr in current_translations {
            let is_manual = tr.origin == "manual";
            let opts = MechanicalFixOptions {
                profile,
                is_manual_edit: is_manual,
                glossary_terms,
                majority_curly_quotes: None,
                majority_ellipsis: if profile == "ja-en" {
                    Some("...")
                } else {
                    None
                },
            };

            let (new_text, applied) = apply_mechanical_fixes_to_text(&tr.text, &opts);
            if !applied.is_empty() && new_text != tr.text {
                // 1. Create finding record
                let finding_id = format!("find_{}", Uuid::new_v4().simple());
                let first_code = applied[0].check_code.clone();
                let finding = Finding {
                    id: finding_id.clone(),
                    run_id: valid_run_id.clone(),
                    check_code: first_code,
                    chapter_id: Some(chap.id.clone()),
                    segment_id: None,
                    severity: "warning".to_string(),
                    detail_json: serde_json::json!({
                        "original": tr.text,
                        "fixed": new_text,
                        "checks": applied.iter().map(|a| &a.check_code).collect::<Vec<_>>(),
                    })
                    .to_string(),
                    status: "fixed".to_string(),
                };
                finding_repo.insert_batch(&[finding])?;

                // 2. Add new translation version
                let new_tr = trans_repo.insert_version(
                    &chap.id,
                    tr.seg_from,
                    tr.seg_to,
                    &new_text,
                    "fix_mechanical",
                    None,
                )?;

                // 3. Insert fix record
                let fix = Fix {
                    id: format!("fix_{}", Uuid::new_v4().simple()),
                    finding_id: Some(finding_id),
                    translation_id_before: Some(tr.id.clone()),
                    translation_id_after: Some(new_tr.id.clone()),
                    origin: "mechanical".to_string(),
                    call_id: None,
                    applied_at: chrono::Utc::now().to_rfc3339(),
                    reverted_at: None,
                };
                fix_repo.insert(&fix)?;
                created_fixes.push(fix);
            }
        }
    }

    Ok(ApplyMechanicalResult {
        applied_count: created_fixes.len(),
        created_fixes,
    })
}

/// Reverts a single fix by ID, restoring the previous translation version as current.
pub fn undo_fix(conn: &Connection, fix_id: &str) -> Result<Fix, AppError> {
    let fix_repo = FixRepo::new(conn);

    let fix = fix_repo
        .get_by_id(fix_id)?
        .ok_or_else(|| AppError::Internal {
            message: format!("Fix not found: {fix_id}"),
        })?;

    if fix.reverted_at.is_some() {
        return Ok(fix);
    }

    let before_id = fix
        .translation_id_before
        .as_deref()
        .ok_or_else(|| AppError::Internal {
            message: "Fix missing translation_id_before".to_string(),
        })?;
    let after_id = fix
        .translation_id_after
        .as_deref()
        .ok_or_else(|| AppError::Internal {
            message: "Fix missing translation_id_after".to_string(),
        })?;

    // Swap is_current in a transaction
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE translation SET is_current = 0 WHERE id = ?1",
        params![after_id],
    )?;
    tx.execute(
        "UPDATE translation SET is_current = 1 WHERE id = ?1",
        params![before_id],
    )?;

    let now = chrono::Utc::now().to_rfc3339();
    tx.execute(
        "UPDATE fix SET reverted_at = ?1 WHERE id = ?2",
        params![now, fix_id],
    )?;

    if let Some(ref find_id) = fix.finding_id {
        tx.execute(
            "UPDATE finding SET status = 'open' WHERE id = ?1",
            params![find_id],
        )?;
    }
    tx.commit()?;

    let mut updated_fix = fix;
    updated_fix.reverted_at = Some(now);
    Ok(updated_fix)
}

/// Reverts a batch of fixes by ID within a single transaction.
pub fn undo_batch(conn: &Connection, fix_ids: &[String]) -> Result<Vec<Fix>, AppError> {
    let mut reverted = Vec::new();
    for id in fix_ids {
        reverted.push(undo_fix(conn, id)?);
    }
    Ok(reverted)
}

/// Reverts all un-reverted fixes associated with a given run_id.
pub fn undo_run(conn: &Connection, run_id: &str) -> Result<Vec<Fix>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT f.id
         FROM fix f
         JOIN finding fd ON f.finding_id = fd.id
         WHERE fd.run_id = ?1 AND f.reverted_at IS NULL
         ORDER BY f.rowid DESC",
    )?;

    let fix_ids: Vec<String> = stmt
        .query_map([run_id], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;

    undo_batch(conn, &fix_ids)
}

/// Reverts a single fix and triggers docx re-rendering if layout and settings are provided.
pub fn undo_fix_and_rerender(
    conn: &Connection,
    fix_id: &str,
    layout: Option<&ProjectLayout>,
    settings: Option<&EffectiveSettings>,
    event_emitter: Option<&dyn EventEmitter>,
) -> Result<Fix, AppError> {
    let fix = undo_fix(conn, fix_id)?;
    if let (Some(layout), Some(settings)) = (layout, settings) {
        let default_emitter = RecordingEventEmitter::new();
        let emitter = event_emitter.unwrap_or(&default_emitter);
        render_docx_after_chapter(conn, layout, settings, emitter)?;
    }
    Ok(fix)
}

/// Reverts a batch of fixes and triggers docx re-rendering.
pub fn undo_batch_and_rerender(
    conn: &Connection,
    fix_ids: &[String],
    layout: Option<&ProjectLayout>,
    settings: Option<&EffectiveSettings>,
    event_emitter: Option<&dyn EventEmitter>,
) -> Result<Vec<Fix>, AppError> {
    let fixes = undo_batch(conn, fix_ids)?;
    if let (Some(layout), Some(settings)) = (layout, settings) {
        let default_emitter = RecordingEventEmitter::new();
        let emitter = event_emitter.unwrap_or(&default_emitter);
        render_docx_after_chapter(conn, layout, settings, emitter)?;
    }
    Ok(fixes)
}

/// Reverts all fixes for a run and triggers docx re-rendering.
pub fn undo_run_and_rerender(
    conn: &Connection,
    run_id: &str,
    layout: Option<&ProjectLayout>,
    settings: Option<&EffectiveSettings>,
    event_emitter: Option<&dyn EventEmitter>,
) -> Result<Vec<Fix>, AppError> {
    let fixes = undo_run(conn, run_id)?;
    if let (Some(layout), Some(settings)) = (layout, settings) {
        let default_emitter = RecordingEventEmitter::new();
        let emitter = event_emitter.unwrap_or(&default_emitter);
        render_docx_after_chapter(conn, layout, settings, emitter)?;
    }
    Ok(fixes)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::store::open_in_memory;
    use crate::store::repo::{Chapter, Run, RunRepo};

    #[test]
    fn test_apply_mechanical_fixes_and_undo() {
        let conn = open_in_memory().unwrap();
        let chap_repo = ChapterRepo::new(&conn);
        let trans_repo = TranslationRepo::new(&conn);
        let run_repo = RunRepo::new(&conn);

        let run = Run {
            id: "run1".to_string(),
            kind: "full".to_string(),
            status: "done".to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            ended_at: None,
            settings_snapshot: "{}".to_string(),
        };
        run_repo.insert(&run).unwrap();

        let chap = Chapter {
            id: "c1".to_string(),
            ordinal: 1,
            file_name: "01_test.md".to_string(),
            prefix: "1".to_string(),
            title_src: "第1話".to_string(),
            title_tgt: Some("Bab 1".to_string()),
            src_hash: "hash".to_string(),
            line_count: 10,
            char_count: 100,
            status: "done".to_string(),
            frozen: false,
            parts_planned: 1,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        chap_repo.insert(&chap).unwrap();

        // Initial translation with non-standard spelling and brackets
        let initial_text = "「Ada resiko besar saat antri.」";
        let tr = trans_repo
            .insert_version("c1", 1, 1, initial_text, "model", None)
            .unwrap();

        assert_eq!(tr.version, 1);
        assert!(tr.is_current);

        // Apply mechanical fixes
        let res = apply_mechanical_fixes_to_volume(&conn, Some("run1"), "ja-id", &[]).unwrap();
        assert_eq!(res.applied_count, 1);

        let fix = &res.created_fixes[0];
        assert_eq!(fix.translation_id_before.as_deref(), Some(tr.id.as_str()));

        // Check translation table has version 2 marked current
        let current_translations = trans_repo.get_current_by_chapter("c1").unwrap();
        assert_eq!(current_translations.len(), 1);
        assert_eq!(current_translations[0].version, 2);
        assert_eq!(
            current_translations[0].text,
            "\"Ada risiko besar saat antre.\""
        );

        // Undo the fix
        let undone = undo_fix(&conn, &fix.id).unwrap();
        assert!(undone.reverted_at.is_some());

        // Now version 1 is current again
        let restored_translations = trans_repo.get_current_by_chapter("c1").unwrap();
        assert_eq!(restored_translations.len(), 1);
        assert_eq!(restored_translations[0].version, 1);
        assert_eq!(restored_translations[0].text, initial_text);
    }

    #[test]
    fn test_undo_run() {
        let conn = open_in_memory().unwrap();
        let chap_repo = ChapterRepo::new(&conn);
        let trans_repo = TranslationRepo::new(&conn);
        let run_repo = RunRepo::new(&conn);

        let run = Run {
            id: "run_test".to_string(),
            kind: "full".to_string(),
            status: "done".to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            ended_at: None,
            settings_snapshot: "{}".to_string(),
        };
        run_repo.insert(&run).unwrap();

        let chap = Chapter {
            id: "c2".to_string(),
            ordinal: 2,
            file_name: "02_test.md".to_string(),
            prefix: "2".to_string(),
            title_src: "第2話".to_string(),
            title_tgt: Some("Bab 2".to_string()),
            src_hash: "hash2".to_string(),
            line_count: 10,
            char_count: 100,
            status: "done".to_string(),
            frozen: false,
            parts_planned: 1,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        chap_repo.insert(&chap).unwrap();

        trans_repo
            .insert_version("c2", 1, 1, "Dia menonton tv .", "model", None)
            .unwrap();
        trans_repo
            .insert_version("c2", 2, 2, "Ada resiko .", "model", None)
            .unwrap();

        let res = apply_mechanical_fixes_to_volume(&conn, Some("run_test"), "ja-id", &[]).unwrap();
        assert_eq!(res.applied_count, 2);

        // Revert all fixes of run_test
        let reverted = undo_run(&conn, "run_test").unwrap();
        assert_eq!(reverted.len(), 2);

        let currents = trans_repo.get_current_by_chapter("c2").unwrap();
        assert_eq!(currents[0].version, 1);
        assert_eq!(currents[0].text, "Dia menonton tv .");
        assert_eq!(currents[1].version, 1);
        assert_eq!(currents[1].text, "Ada resiko .");
    }
}
