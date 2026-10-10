//! AI QA fix formatting, parsing, re-validation, and database recording.
//!
//! Implements FR-QA-04, FR-QA-05, and `qa-contract.md` prompt/output format:
//! - `<qa_fix check="<code>">` prompt generation with `<directive>`, `<source>`, `<target>`, `<neighbors>`
//! - Response parser for `<fix id="...">text</fix>` and `<keep id="..."/>`
//! - Re-validation of AI fixes against quality rules (rejected on violations)
//! - Atomic commit of approved AI fixes to `translation` and `fix` tables

use regex::Regex;
use rusqlite::Connection;
use uuid::Uuid;

use crate::error::AppError;
use crate::store::repo::{FindingRepo, Fix, FixRepo, TranslationRepo};

/// A single item sent to the AI QA Fixer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaFixItem {
    pub id: String,
    pub ref_str: String, // e.g. "4:41"
    pub chapter_id: String,
    pub seg_from: i32,
    pub seg_to: i32,
    pub source: String,
    pub target: String,
    pub prev_target: Option<String>,
    pub next_target: Option<String>,
    pub finding_id: Option<String>,
}

/// Arguments for building a `<qa_fix>` prompt.
#[derive(Debug, Clone)]
pub struct QaFixPromptArgs<'a> {
    pub check_code: &'a str,
    pub directive: &'a str,
    pub glossary_text: &'a str,
    pub items: &'a [QaFixItem],
}

/// Builds the user prompt for an AI QA fix call per `qa-contract.md`.
pub fn build_qa_fix_prompt(args: &QaFixPromptArgs) -> String {
    let mut out = String::new();

    if !args.glossary_text.trim().is_empty() {
        out.push_str("<reference_glossary>\n");
        out.push_str(args.glossary_text.trim());
        out.push_str("\n</reference_glossary>\n");
    }

    out.push_str(&format!("<qa_fix check=\"{}\">\n", args.check_code));
    out.push_str(&format!(
        "<directive>{}</directive>\n",
        args.directive.trim()
    ));

    for it in args.items {
        out.push_str(&format!("<item id=\"{}\" ref=\"{}\">\n", it.id, it.ref_str));
        out.push_str(&format!("<source>{}</source>\n", it.source));
        out.push_str(&format!("<target>{}</target>\n", it.target));
        out.push_str("<neighbors readonly=\"true\">");
        out.push_str(&format!(
            "<prev>{}</prev>",
            it.prev_target.as_deref().unwrap_or("")
        ));
        out.push_str(&format!(
            "<next>{}</next>",
            it.next_target.as_deref().unwrap_or("")
        ));
        out.push_str("</neighbors>\n");
        out.push_str("</item>\n");
    }

    out.push_str("</qa_fix>");
    out
}

/// An item parsed from an AI QA Fix response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QaFixResponseItem {
    Fix { id: String, text: String },
    Keep { id: String },
}

/// Parses `<fix id="...">...</fix>` and `<keep id="..."/>` from an AI response.
pub fn parse_qa_fix_response(response: &str) -> Vec<QaFixResponseItem> {
    let mut results = Vec::new();

    // Match <fix id="...">content</fix> (case-insensitive tags, single or multiline)
    if let Ok(fix_re) = Regex::new(r#"(?is)<fix\s+id="([^"]+)">(.*?)</fix>"#) {
        for cap in fix_re.captures_iter(response) {
            let id = cap.get(1).map_or("", |m| m.as_str()).to_string();
            let text = cap.get(2).map_or("", |m| m.as_str()).trim().to_string();
            results.push(QaFixResponseItem::Fix { id, text });
        }
    }

    // Match <keep id="..."/>
    if let Ok(keep_re) = Regex::new(r#"(?is)<keep\s+id="([^"]+)"\s*/>"#) {
        for cap in keep_re.captures_iter(response) {
            let id = cap.get(1).map_or("", |m| m.as_str()).to_string();
            results.push(QaFixResponseItem::Keep { id });
        }
    }

    results
}

/// Re-validates an AI fix replacement text against quality rules (FR-QA-05).
///
/// Returns Err with the violation reason if the fix introduces new errors.
pub fn validate_fix_replacement(replacement: &str, _profile: &str) -> Result<(), String> {
    let trimmed = replacement.trim();
    if trimmed.is_empty() {
        return Err("Replacement text cannot be empty".to_string());
    }

    // 1. Must not contain residual Japanese kana or kanji
    for ch in trimmed.chars() {
        if ('\u{3040}'..='\u{30ff}').contains(&ch) || ('\u{4e00}'..='\u{9fff}').contains(&ch) {
            return Err(format!(
                "Replacement contains residual Japanese character '{ch}'"
            ));
        }
    }

    // 2. Must not contain untranslated corner brackets
    if trimmed.contains('「') || trimmed.contains('」') {
        return Err("Replacement contains residual corner brackets 「」".to_string());
    }

    // 3. Must not have unbalanced double quotes
    let quote_count = trimmed.chars().filter(|&c| c == '"').count();
    if quote_count % 2 != 0 {
        return Err("Replacement contains unbalanced double quotes".to_string());
    }

    // 4. Must not contain markdown bold **
    if trimmed.contains("**") {
        return Err("Replacement contains markdown bold ** (italics only)".to_string());
    }

    Ok(())
}

/// Applies validated AI fixes to the database, creating new translation versions
/// and linking them in the `fix` table.
pub fn apply_ai_fixes_to_db(
    conn: &Connection,
    items: &[QaFixItem],
    responses: &[QaFixResponseItem],
    profile: &str,
) -> Result<Vec<Fix>, AppError> {
    let trans_repo = TranslationRepo::new(conn);
    let fix_repo = FixRepo::new(conn);
    let finding_repo = FindingRepo::new(conn);

    let mut applied_fixes = Vec::new();

    for resp in responses {
        if let QaFixResponseItem::Fix { id, text } = resp {
            let Some(item) = items.iter().find(|it| it.id == *id) else {
                continue;
            };

            // Re-validate per FR-QA-05
            if validate_fix_replacement(text, profile).is_err() {
                // Reject fix on new violation
                continue;
            }

            // Find current translation for this segment
            let current_trans = trans_repo
                .get_current_by_chapter(&item.chapter_id)?
                .into_iter()
                .find(|t| t.seg_from == item.seg_from && t.seg_to == item.seg_to);

            let old_trans_id = current_trans.as_ref().map(|t| t.id.clone());

            // Add new translation version
            let new_trans = trans_repo.insert_version(
                &item.chapter_id,
                item.seg_from,
                item.seg_to,
                text,
                "fix_ai",
                None,
            )?;

            let valid_finding_id = match &item.finding_id {
                Some(fid) => {
                    let exists: bool = conn
                        .query_row(
                            "SELECT EXISTS(SELECT 1 FROM finding WHERE id = ?1)",
                            rusqlite::params![fid],
                            |r| r.get(0),
                        )
                        .unwrap_or(false);
                    if exists {
                        Some(fid.clone())
                    } else {
                        None
                    }
                }
                None => None,
            };

            // Record fix
            let fix = Fix {
                id: format!("fix_{}", Uuid::new_v4().simple()),
                finding_id: valid_finding_id.clone(),
                translation_id_before: old_trans_id,
                translation_id_after: Some(new_trans.id),
                origin: "ai".to_string(),
                call_id: None,
                applied_at: chrono::Utc::now().to_rfc3339(),
                reverted_at: None,
            };
            fix_repo.insert(&fix)?;

            if let Some(ref fid) = valid_finding_id {
                let _ = finding_repo.update_status(fid, "fixed");
            }

            applied_fixes.push(fix);
        }
    }

    Ok(applied_fixes)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::store::open_in_memory;
    use crate::store::repo::Chapter;
    use crate::store::repo::ChapterRepo;

    #[test]
    fn test_build_qa_fix_prompt_format() {
        let items = vec![QaFixItem {
            id: "f1".to_string(),
            ref_str: "4:41".to_string(),
            chapter_id: "c4".to_string(),
            seg_from: 41,
            seg_to: 41,
            source: "健太は走った。".to_string(),
            target: "Kenta berkata begitu.".to_string(),
            prev_target: Some("Sebelumnya.".to_string()),
            next_target: Some("Sesudahnya.".to_string()),
            finding_id: Some("find1".to_string()),
        }];

        let args = QaFixPromptArgs {
            check_code: "Q04",
            directive: "Hapus atribusi mubazir.",
            glossary_text: "健太 === Kenta === laki-laki",
            items: &items,
        };

        let prompt = build_qa_fix_prompt(&args);
        assert!(prompt.contains("<reference_glossary>"));
        assert!(prompt.contains("<qa_fix check=\"Q04\">"));
        assert!(prompt.contains("<directive>Hapus atribusi mubazir.</directive>"));
        assert!(prompt.contains("<item id=\"f1\" ref=\"4:41\">"));
        assert!(prompt.contains("<source>健太は走った。</source>"));
        assert!(prompt.contains("<neighbors readonly=\"true\"><prev>Sebelumnya.</prev><next>Sesudahnya.</next></neighbors>"));
        assert!(prompt.ends_with("</qa_fix>"));
    }

    #[test]
    fn test_parse_qa_fix_response() {
        let response = r#"
        <fix id="f1">
        Kenta berlari cepat menuju gerbang.
        </fix>
        <keep id="f2"/>
        "#;

        let parsed = parse_qa_fix_response(response);
        assert_eq!(parsed.len(), 2);
        assert_eq!(
            parsed[0],
            QaFixResponseItem::Fix {
                id: "f1".to_string(),
                text: "Kenta berlari cepat menuju gerbang.".to_string(),
            }
        );
        assert_eq!(
            parsed[1],
            QaFixResponseItem::Keep {
                id: "f2".to_string(),
            }
        );
    }

    #[test]
    fn test_validate_fix_replacement() {
        // Valid
        assert!(validate_fix_replacement("Dia mengangguk perlahan.", "ja-id").is_ok());

        // Rejects residual Japanese
        assert!(validate_fix_replacement("Dia mengangguk はい.", "ja-id").is_err());

        // Rejects corner brackets
        assert!(validate_fix_replacement("「Halo」", "ja-id").is_err());

        // Rejects unbalanced quotes
        assert!(validate_fix_replacement("\"Halo", "ja-id").is_err());

        // Rejects markdown bold
        assert!(validate_fix_replacement("**Tebal**", "ja-id").is_err());
    }

    #[test]
    fn test_apply_ai_fixes_to_db() {
        let conn = open_in_memory().unwrap();
        let chap_repo = ChapterRepo::new(&conn);
        let trans_repo = TranslationRepo::new(&conn);
        let finding_repo = FindingRepo::new(&conn);

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

        finding_repo
            .insert_batch(&[crate::store::repo::Finding {
                id: "find_1".to_string(),
                run_id: None,
                check_code: "Q04".to_string(),
                chapter_id: Some("c1".to_string()),
                segment_id: None,
                severity: "warning".to_string(),
                detail_json: "{}".to_string(),
                status: "open".to_string(),
            }])
            .unwrap();

        trans_repo
            .insert_version("c1", 1, 1, "Kenta berkata begitu.", "model", None)
            .unwrap();

        let items = vec![QaFixItem {
            id: "f1".to_string(),
            ref_str: "1:1".to_string(),
            chapter_id: "c1".to_string(),
            seg_from: 1,
            seg_to: 1,
            source: "健太は言った。".to_string(),
            target: "Kenta berkata begitu.".to_string(),
            prev_target: None,
            next_target: None,
            finding_id: Some("find_1".to_string()),
        }];

        let responses = vec![QaFixResponseItem::Fix {
            id: "f1".to_string(),
            text: "\"Ayo pergi,\" ajak Kenta.".to_string(),
        }];

        let applied = apply_ai_fixes_to_db(&conn, &items, &responses, "ja-id").unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].origin, "ai");

        let cur = trans_repo.get_current_by_chapter("c1").unwrap();
        assert_eq!(cur[0].version, 2);
        assert_eq!(cur[0].text, "\"Ayo pergi,\" ajak Kenta.");
    }
}
