//! Step C Hotspot review formatting, parsing, rename handling, and rescan loop.
//!
//! Implements FR-QA-07..10, Appendix C, and `qa-contract.md`:
//! - Request and response formats for `<qa_review kind="...">`
//! - Parse `<fix ref="...">`, `<rename source="..." to="..."/>`, `<suspect token="..." reason="..."/>`, `<none/>`
//! - `<rename>` updates `glossary.md` and database `glossary_entry`, and normalizes the volume
//! - `<suspect>` triggers a homophone batch review
//! - Re-validation of review fixes via `validate_fix_replacement`
//! - Rescan loop: max 2 rounds
//! - Summary log line builder from the fix log

use std::path::Path;

use regex::Regex;
use rusqlite::Connection;
use uuid::Uuid;

use crate::error::AppError;
use crate::glossary::{parse_glossary, write_glossary_file, GlossaryEntryItem, GlossaryLine};
use crate::qa::common::SegmentForQa;
use crate::qa::fix_ai::validate_fix_replacement;
use crate::qa::hotspots::HotspotPair;
use crate::store::repo::{
    Chapter, ChapterRepo, Fix, FixRepo, GlossaryEntry, GlossaryRepo, TranslationRepo,
};

/// Arguments for building a `<qa_review>` prompt per `qa-contract.md`.
#[derive(Debug, Clone)]
pub struct QaReviewArgs<'a> {
    pub kind: &'a str,
    pub directive: &'a str,
    pub glossary_text: &'a str,
    pub pairs: &'a [HotspotPair],
}

/// Builds the user prompt for a `<qa_review>` call per `qa-contract.md`.
pub fn build_qa_review_prompt(args: &QaReviewArgs) -> String {
    let mut out = String::new();

    if !args.glossary_text.trim().is_empty() {
        out.push_str("<reference_glossary>\n");
        out.push_str(args.glossary_text.trim());
        out.push_str("\n</reference_glossary>\n");
    }

    out.push_str(&format!("<qa_review kind=\"{}\">\n", args.kind));
    out.push_str(&format!(
        "<directive>{}</directive>\n",
        args.directive.trim()
    ));

    for pair in args.pairs {
        out.push_str(&format!(
            "<pair ref=\"{}\"><source>{}</source><target>{}</target></pair>\n",
            pair.ref_str(),
            pair.source,
            pair.target
        ));
    }

    out.push_str("</qa_review>");
    out
}

/// An item parsed from a `<qa_review>` AI response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QaReviewResponseItem {
    Fix { ref_str: String, text: String },
    Rename { source: String, to: String },
    Suspect { token: String, reason: String },
    None,
}

/// Parses tags from a `<qa_review>` AI response.
pub fn parse_qa_review_response(response: &str) -> Vec<QaReviewResponseItem> {
    let mut results = Vec::new();

    // 1. Match <fix ref="...">...</fix>
    if let Ok(fix_re) = Regex::new(r#"(?is)<fix\s+ref="([^"]+)">(.*?)</fix>"#) {
        for cap in fix_re.captures_iter(response) {
            let ref_str = cap.get(1).map_or("", |m| m.as_str()).to_string();
            let text = cap.get(2).map_or("", |m| m.as_str()).trim().to_string();
            results.push(QaReviewResponseItem::Fix { ref_str, text });
        }
    }

    // 2. Match <rename source="..." to="..."/> (tolerant of self-closing or not)
    if let Ok(rename_re) = Regex::new(r#"(?is)<rename\s+source="([^"]+)"\s+to="([^"]+)"\s*/?>"#) {
        for cap in rename_re.captures_iter(response) {
            let source = cap.get(1).map_or("", |m| m.as_str()).to_string();
            let to = cap.get(2).map_or("", |m| m.as_str()).to_string();
            results.push(QaReviewResponseItem::Rename { source, to });
        }
    }

    // 3. Match <suspect token="..." reason="..."/>
    if let Ok(suspect_re) =
        Regex::new(r#"(?is)<suspect\s+token="([^"]+)"\s+reason="([^"]+)"\s*/?>"#)
    {
        for cap in suspect_re.captures_iter(response) {
            let token = cap.get(1).map_or("", |m| m.as_str()).to_string();
            let reason = cap.get(2).map_or("", |m| m.as_str()).to_string();
            results.push(QaReviewResponseItem::Suspect { token, reason });
        }
    }

    // 4. Match <none/>
    if let Ok(none_re) = Regex::new(r#"(?is)<none\s*/?>"#) {
        if none_re.is_match(response) && results.is_empty() {
            results.push(QaReviewResponseItem::None);
        }
    }

    results
}

/// Parses a segment reference string such as "4:41" or "4:41-43" into
/// `(chapter_ordinal, seg_from, seg_to)`.
pub fn parse_ref_str(ref_str: &str) -> Option<(i32, i32, i32)> {
    let parts: Vec<&str> = ref_str.split(':').collect();
    if parts.len() != 2 {
        return None;
    }

    let chap_ord: i32 = parts[0].trim().parse().ok()?;
    let seg_part = parts[1].trim();

    if seg_part.contains('-') {
        let seg_range: Vec<&str> = seg_part.split('-').collect();
        if seg_range.len() == 2 {
            let seg_from: i32 = seg_range[0].trim().parse().ok()?;
            let seg_to: i32 = seg_range[1].trim().parse().ok()?;
            Some((chap_ord, seg_from, seg_to))
        } else {
            None
        }
    } else {
        let seg_idx: i32 = seg_part.parse().ok()?;
        Some((chap_ord, seg_idx, seg_idx))
    }
}

/// Applies a single validated `<fix ref="...">` from a hotspot review to the database.
pub fn apply_review_fix(
    conn: &Connection,
    ref_str: &str,
    text: &str,
    profile: &str,
    chapters: &[Chapter],
) -> Result<Option<Fix>, AppError> {
    // 1. Re-validate replacement text per FR-QA-05
    if validate_fix_replacement(text, profile).is_err() {
        return Ok(None);
    }

    // 2. Parse segment reference
    let Some((chap_ord, seg_from, seg_to)) = parse_ref_str(ref_str) else {
        return Ok(None);
    };

    let Some(chapter) = chapters.iter().find(|c| c.ordinal == chap_ord) else {
        return Ok(None);
    };

    let trans_repo = TranslationRepo::new(conn);
    let fix_repo = FixRepo::new(conn);

    let current_trans = trans_repo
        .get_current_by_chapter(&chapter.id)?
        .into_iter()
        .find(|t| t.seg_from == seg_from && t.seg_to == seg_to);

    let old_trans_id = current_trans.as_ref().map(|t| t.id.clone());

    let new_trans =
        trans_repo.insert_version(&chapter.id, seg_from, seg_to, text, "qa_review", None)?;

    let fix = Fix {
        id: format!("fix_{}", Uuid::new_v4().simple()),
        finding_id: None,
        translation_id_before: old_trans_id,
        translation_id_after: Some(new_trans.id),
        origin: "ai".to_string(),
        call_id: None,
        applied_at: chrono::Utc::now().to_rfc3339(),
        reverted_at: None,
    };
    fix_repo.insert(&fix)?;

    Ok(Some(fix))
}

/// Applies a `<rename source="..." to="..."/>` action:
/// 1. Updates database `glossary_entry` table
/// 2. Updates `glossary.md` file if project_dir is provided
/// 3. Normalizes all occurrences of the term across current volume translations
pub fn apply_rename_to_volume(
    conn: &Connection,
    project_dir: Option<&Path>,
    source: &str,
    to: &str,
) -> Result<Vec<Fix>, AppError> {
    let glossary_repo = GlossaryRepo::new(conn);
    let trans_repo = TranslationRepo::new(conn);
    let fix_repo = FixRepo::new(conn);
    let chap_repo = ChapterRepo::new(conn);

    // 1. Update DB glossary_entry
    let existing = glossary_repo.get_by_source(source)?;
    let old_target = match existing {
        Some(mut entry) => {
            let prev = entry.translation.clone();
            entry.translation = to.to_string();
            glossary_repo.update(&entry)?;
            prev
        }
        None => {
            let new_entry = GlossaryEntry {
                id: format!("glo_{}", Uuid::new_v4().simple()),
                source: source.to_string(),
                translation: to.to_string(),
                note: None,
                tier: 1,
                origin: "ai_qa_rename".to_string(),
                first_chapter_id: None,
                occurrences: 0,
                status: "approved".to_string(),
                file_line: None,
            };
            glossary_repo.insert(&new_entry)?;
            String::new()
        }
    };

    // 2. Update glossary.md file if project_dir is provided
    if let Some(p_dir) = project_dir {
        let glossary_path = p_dir.join("glossary.md");
        if glossary_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&glossary_path) {
                let mut glo_file = parse_glossary(&content, &[]);
                let mut found = false;
                for line in &mut glo_file.lines {
                    if let GlossaryLine::Entry(ref mut item) = line {
                        if item.source == source {
                            item.translation = to.to_string();
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    glo_file.lines.push(GlossaryLine::Entry(GlossaryEntryItem {
                        source: source.to_string(),
                        translation: to.to_string(),
                        note: String::new(),
                        tier: 1,
                        has_list_prefix: false,
                        list_prefix_char: None,
                        line_index: 0,
                    }));
                }
                let backup_dir = p_dir.join(".lnta").join("backups");
                let _ = write_glossary_file(&glossary_path, &glo_file, Some(&backup_dir));
            }
        }
    }

    // 3. Normalize all occurrences in volume translations
    let mut applied_fixes = Vec::new();
    let chapters = chap_repo.list_all()?;

    for chap in chapters {
        let current_translations = trans_repo.get_current_by_chapter(&chap.id)?;
        for trans in current_translations {
            let mut modified_text = trans.text.clone();
            let mut changed = false;

            // Replace occurrences of old_target if non-empty and different from `to`
            if !old_target.is_empty() && old_target != to && modified_text.contains(&old_target) {
                modified_text = modified_text.replace(&old_target, to);
                changed = true;
            }

            // Replace occurrences of source if directly present in target
            if modified_text.contains(source) {
                modified_text = modified_text.replace(source, to);
                changed = true;
            }

            if changed && modified_text != trans.text {
                let new_trans = trans_repo.insert_version(
                    &chap.id,
                    trans.seg_from,
                    trans.seg_to,
                    &modified_text,
                    "rename",
                    None,
                )?;

                let fix = Fix {
                    id: format!("fix_{}", Uuid::new_v4().simple()),
                    finding_id: None,
                    translation_id_before: Some(trans.id),
                    translation_id_after: Some(new_trans.id),
                    origin: "rename".to_string(),
                    call_id: None,
                    applied_at: chrono::Utc::now().to_rfc3339(),
                    reverted_at: None,
                };
                fix_repo.insert(&fix)?;
                applied_fixes.push(fix);
            }
        }
    }

    Ok(applied_fixes)
}

/// Builds the final one-line summary log string from the fix log counts.
///
/// ID: `QA selesai: {M} perbaikan otomatis, {A} perbaikan AI, {R} temuan tersisa`
/// EN: `QA complete: {M} automatic fixes, {A} AI fixes, {R} remaining findings`
pub fn build_qa_summary_log(
    mechanical_count: usize,
    ai_count: usize,
    remaining_findings: usize,
    is_indonesian: bool,
) -> String {
    if is_indonesian {
        format!(
            "QA selesai: {mechanical_count} perbaikan otomatis, {ai_count} perbaikan AI, {remaining_findings} temuan tersisa"
        )
    } else {
        format!(
            "QA complete: {mechanical_count} automatic fixes, {ai_count} AI fixes, {remaining_findings} remaining findings"
        )
    }
}

/// Helper function to load all volume chapters as SegmentForQa slices.
pub fn load_volume_segments_for_qa(
    conn: &Connection,
) -> Result<(Vec<Chapter>, Vec<Vec<SegmentForQa>>), AppError> {
    let chap_repo = ChapterRepo::new(conn);
    let trans_repo = TranslationRepo::new(conn);
    let seg_repo = crate::store::repo::SegmentRepo::new(conn);

    let chapters = chap_repo.list_all()?;
    let mut chapter_segments = Vec::new();

    for chap in &chapters {
        let segs = seg_repo.list_by_chapter(&chap.id)?;
        let current_trans = trans_repo.get_current_by_chapter(&chap.id)?;

        // One QA unit per translation: a merged range `12-14` becomes a single unit carrying
        // all three sources. Copying its text onto every covered segment would make the
        // duplicate-paragraph check fire on every merge.
        let mut segs_for_qa = Vec::new();
        for s in &segs {
            match current_trans
                .iter()
                .find(|t| t.seg_from <= s.idx && s.idx <= t.seg_to)
            {
                Some(t) if t.seg_from == s.idx => {
                    let src_text = segs
                        .iter()
                        .filter(|o| o.idx >= t.seg_from && o.idx <= t.seg_to)
                        .map(|o| o.src_text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                    segs_for_qa.push(SegmentForQa {
                        segment_id: s.id.clone(),
                        chapter_id: chap.id.clone(),
                        idx: s.idx,
                        src_text,
                        tgt_text: t.text.clone(),
                    });
                }
                Some(_) => {}
                None => segs_for_qa.push(SegmentForQa {
                    segment_id: s.id.clone(),
                    chapter_id: chap.id.clone(),
                    idx: s.idx,
                    src_text: s.src_text.clone(),
                    tgt_text: String::new(),
                }),
            }
        }
        chapter_segments.push(segs_for_qa);
    }

    Ok((chapters, chapter_segments))
}

/// Result of running the full QA loop over a volume.
#[derive(Debug, Clone)]
pub struct QaLoopResult {
    pub rounds_run: usize,
    pub mechanical_fixes_count: usize,
    pub ai_fixes_count: usize,
    pub remaining_findings_count: usize,
    pub summary_log: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::store::open_in_memory;
    use crate::store::repo::{Chapter, ChapterRepo, TranslationRepo};

    #[test]
    fn test_build_qa_review_prompt_format() {
        let pairs = vec![
            HotspotPair {
                chapter_ordinal: 4,
                seg_idx: 41,
                seg_from: 41,
                seg_to: 41,
                chapter_id: "c4".to_string(),
                source: "空が青い。".to_string(),
                target: "Langit begitu biru.".to_string(),
            },
            HotspotPair {
                chapter_ordinal: 4,
                seg_idx: 42,
                seg_from: 42,
                seg_to: 42,
                chapter_id: "c4".to_string(),
                source: "風が吹いた。".to_string(),
                target: "Angin bertiup lembut.".to_string(),
            },
        ];

        let args = QaReviewArgs {
            kind: "boundary",
            directive: "Periksa konsistensi batas bab.",
            glossary_text: "健太 === Kenta",
            pairs: &pairs,
        };

        let prompt = build_qa_review_prompt(&args);
        assert!(prompt.contains("<reference_glossary>\n健太 === Kenta\n</reference_glossary>"));
        assert!(prompt.contains("<qa_review kind=\"boundary\">"));
        assert!(prompt.contains("<directive>Periksa konsistensi batas bab.</directive>"));
        assert!(prompt.contains(
            "<pair ref=\"4:41\"><source>空が青い。</source><target>Langit begitu biru.</target></pair>"
        ));
        assert!(prompt.contains(
            "<pair ref=\"4:42\"><source>風が吹いた。</source><target>Angin bertiup lembut.</target></pair>"
        ));
        assert!(prompt.ends_with("</qa_review>"));
    }

    #[test]
    fn test_parse_qa_review_response() {
        let response = r#"
        <fix ref="4:42">
        Angin sepoi bertiup pelan.
        </fix>
        <rename source="健太" to="Kenta"/>
        <suspect token="え" reason="dibaca sebagai 絵 (gambar), seharusnya interjeksi"/>
        <none/>
        "#;

        let parsed = parse_qa_review_response(response);
        assert_eq!(parsed.len(), 3); // Fix, Rename, Suspect (none ignored because not empty)
        assert_eq!(
            parsed[0],
            QaReviewResponseItem::Fix {
                ref_str: "4:42".to_string(),
                text: "Angin sepoi bertiup pelan.".to_string(),
            }
        );
        assert_eq!(
            parsed[1],
            QaReviewResponseItem::Rename {
                source: "健太".to_string(),
                to: "Kenta".to_string(),
            }
        );
        assert_eq!(
            parsed[2],
            QaReviewResponseItem::Suspect {
                token: "え".to_string(),
                reason: "dibaca sebagai 絵 (gambar), seharusnya interjeksi".to_string(),
            }
        );

        let none_only = "<none/>";
        let parsed_none = parse_qa_review_response(none_only);
        assert_eq!(parsed_none, vec![QaReviewResponseItem::None]);
    }

    #[test]
    fn test_apply_review_fix_to_db() {
        let conn = open_in_memory().unwrap();
        let chap_repo = ChapterRepo::new(&conn);
        let trans_repo = TranslationRepo::new(&conn);

        let chap = Chapter {
            id: "c4".to_string(),
            ordinal: 4,
            file_name: "04_test.md".to_string(),
            prefix: "4".to_string(),
            title_src: "第4話".to_string(),
            title_tgt: Some("Bab 4".to_string()),
            src_hash: "hash".to_string(),
            line_count: 50,
            char_count: 500,
            status: "done".to_string(),
            frozen: false,
            parts_planned: 1,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        chap_repo.insert(&chap).unwrap();

        trans_repo
            .insert_version("c4", 42, 42, "Teks lama yang salah.", "model", None)
            .unwrap();

        let fix = apply_review_fix(
            &conn,
            "4:42",
            "Teks baru yang sudah diperbaiki.",
            "ja-id",
            &[chap],
        )
        .unwrap();

        assert!(fix.is_some());
        let current = trans_repo.get_current_by_chapter("c4").unwrap();
        assert_eq!(current[0].version, 2);
        assert_eq!(current[0].text, "Teks baru yang sudah diperbaiki.");
    }

    #[test]
    fn test_apply_rename_to_volume() {
        let conn = open_in_memory().unwrap();
        let chap_repo = ChapterRepo::new(&conn);
        let trans_repo = TranslationRepo::new(&conn);
        let glo_repo = GlossaryRepo::new(&conn);

        let chap = Chapter {
            id: "c1".to_string(),
            ordinal: 1,
            file_name: "01_test.md".to_string(),
            prefix: "1".to_string(),
            title_src: "第1話".to_string(),
            title_tgt: Some("Bab 1".to_string()),
            src_hash: "hash".to_string(),
            line_count: 50,
            char_count: 500,
            status: "done".to_string(),
            frozen: false,
            parts_planned: 1,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        chap_repo.insert(&chap).unwrap();

        // Initial glossary entry
        glo_repo
            .insert(&GlossaryEntry {
                id: "g1".to_string(),
                source: "健太".to_string(),
                translation: "Kenda".to_string(), // typo in initial translation
                note: None,
                tier: 1,
                origin: "user".to_string(),
                first_chapter_id: None,
                occurrences: 1,
                status: "approved".to_string(),
                file_line: None,
            })
            .unwrap();

        trans_repo
            .insert_version("c1", 1, 1, "Kenda tersenyum cerah.", "model", None)
            .unwrap();

        let fixes = apply_rename_to_volume(&conn, None, "健太", "Kenta").unwrap();
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].origin, "rename");

        // Check updated translation
        let current = trans_repo.get_current_by_chapter("c1").unwrap();
        assert_eq!(current[0].text, "Kenta tersenyum cerah.");

        // Check updated glossary entry
        let updated_entry = glo_repo.get_by_source("健太").unwrap().unwrap();
        assert_eq!(updated_entry.translation, "Kenta");
    }

    #[test]
    fn test_build_qa_summary_log() {
        let id_log = build_qa_summary_log(14, 5, 2, true);
        assert_eq!(
            id_log,
            "QA selesai: 14 perbaikan otomatis, 5 perbaikan AI, 2 temuan tersisa"
        );

        let en_log = build_qa_summary_log(14, 5, 2, false);
        assert_eq!(
            en_log,
            "QA complete: 14 automatic fixes, 5 AI fixes, 2 remaining findings"
        );
    }
}
