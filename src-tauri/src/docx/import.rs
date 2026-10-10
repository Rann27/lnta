//! Frozen chapter import and docx heading mapping per Story A4 and FR-QA-11.
//!
//! When an existing `Translations.docx` has pre-translated chapters, LNTA:
//! 1. Parses docx headings using the document's Heading 1 style.
//! 2. Proposes a sequential heading-to-chapter mapping for user confirmation.
//! 3. Marks confirmed chapters as `frozen` in SQLite, keeping their content
//!    verbatim as the opening prefix of the final document.
//! 4. Populates translation records with `origin = "frozen"` so frozen chapters
//!    are scanned read-only during QA without automatic modification.

use std::collections::HashMap;
use std::io::{Cursor, Read};

use regex::Regex;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use zip::ZipArchive;

use crate::error::AppError;
use crate::project::discover::DiscoveredChapter;
use crate::store::repo::{ChapterRepo, Segment, SegmentRepo, TranslationRepo};

/// Heading extracted from a pre-existing `Translations.docx`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/ExtractedDocxHeading.ts")]
pub struct ExtractedDocxHeading {
    pub index: usize,
    pub heading_text: String,
    pub paragraph_count: usize,
    pub paragraphs: Vec<String>,
}

/// Proposed or confirmed mapping between a raw chapter file and an imported docx heading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/FrozenChapterMapping.ts")]
pub struct FrozenChapterMapping {
    pub raw_chapter_prefix: String,
    pub raw_chapter_file: String,
    pub raw_chapter_title: String,
    pub docx_heading: String,
    pub paragraph_count: usize,
    pub is_frozen: bool,
}

/// Extracts headings and their associated paragraphs from docx bytes.
pub fn extract_headings_from_docx_bytes(
    bytes: &[u8],
) -> Result<Vec<ExtractedDocxHeading>, AppError> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| AppError::InvalidTemplate {
        reason: format!("Failed to read docx zip archive: {e}"),
    })?;

    let styles_xml = {
        let mut f = archive
            .by_name("word/styles.xml")
            .map_err(|e| AppError::InvalidTemplate {
                reason: format!("word/styles.xml not found in docx: {e}"),
            })?;
        let mut s = String::new();
        f.read_to_string(&mut s)
            .map_err(|e| AppError::InvalidTemplate {
                reason: format!("Failed to read styles.xml: {e}"),
            })?;
        s
    };

    let heading1_style_id = super::styles::find_heading1_style_id(&styles_xml);

    let document_xml = {
        let mut f =
            archive
                .by_name("word/document.xml")
                .map_err(|e| AppError::InvalidTemplate {
                    reason: format!("word/document.xml not found in docx: {e}"),
                })?;
        let mut s = String::new();
        f.read_to_string(&mut s)
            .map_err(|e| AppError::InvalidTemplate {
                reason: format!("Failed to read document.xml: {e}"),
            })?;
        s
    };

    extract_headings_from_document_xml(&document_xml, &heading1_style_id)
}

/// Parses `<w:body>` paragraphs from `word/document.xml` grouping them under Heading 1 paragraphs.
pub fn extract_headings_from_document_xml(
    document_xml: &str,
    heading1_style_id: &str,
) -> Result<Vec<ExtractedDocxHeading>, AppError> {
    #[allow(clippy::unwrap_used)]
    let p_re = Regex::new(r"(?s)<w:p\b[^>]*>(.*?)</w:p>").unwrap();
    #[allow(clippy::unwrap_used)]
    let t_re = Regex::new(r"<w:t[^>]*>(.*?)</w:t>").unwrap();
    #[allow(clippy::unwrap_used)]
    let style_re = Regex::new(r#"<w:pStyle\s+[^>]*w:val="([^"]+)""#).unwrap();

    let mut headings = Vec::new();
    let mut current_heading: Option<ExtractedDocxHeading> = None;

    for caps in p_re.captures_iter(document_xml) {
        let p_content = match caps.get(1) {
            Some(m) => m.as_str(),
            None => continue,
        };

        let style_val = style_re
            .captures(p_content)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str());

        let mut text_buf = String::new();
        for t_caps in t_re.captures_iter(p_content) {
            if let Some(tm) = t_caps.get(1) {
                text_buf.push_str(tm.as_str());
            }
        }
        let trimmed_text = text_buf.trim().to_string();

        let is_heading = if let Some(st) = style_val {
            st.eq_ignore_ascii_case(heading1_style_id)
                || st.eq_ignore_ascii_case("heading 1")
                || st.eq_ignore_ascii_case("heading1")
                || st.eq_ignore_ascii_case("judul1")
                || st.eq_ignore_ascii_case("judul 1")
        } else {
            false
        };

        if is_heading {
            if let Some(h) = current_heading.take() {
                headings.push(h);
            }
            current_heading = Some(ExtractedDocxHeading {
                index: headings.len() + 1,
                heading_text: trimmed_text,
                paragraph_count: 0,
                paragraphs: Vec::new(),
            });
        } else if !trimmed_text.is_empty() {
            if let Some(ref mut h) = current_heading {
                h.paragraphs.push(trimmed_text);
                h.paragraph_count += 1;
            }
        }
    }

    if let Some(h) = current_heading {
        headings.push(h);
    }

    Ok(headings)
}

/// Generates a mapping proposal matching raw chapters to docx headings sequentially per Story A4.
pub fn propose_frozen_chapter_mappings(
    docx_headings: &[ExtractedDocxHeading],
    raw_chapters: &[DiscoveredChapter],
) -> Vec<FrozenChapterMapping> {
    let mut mappings = Vec::new();

    for (i, raw_ch) in raw_chapters.iter().enumerate() {
        if i < docx_headings.len() {
            let dh = &docx_headings[i];
            mappings.push(FrozenChapterMapping {
                raw_chapter_prefix: raw_ch.prefix.clone(),
                raw_chapter_file: raw_ch.file_name.clone(),
                raw_chapter_title: raw_ch.title_candidate.clone(),
                docx_heading: dh.heading_text.clone(),
                paragraph_count: dh.paragraph_count,
                is_frozen: true,
            });
        } else {
            mappings.push(FrozenChapterMapping {
                raw_chapter_prefix: raw_ch.prefix.clone(),
                raw_chapter_file: raw_ch.file_name.clone(),
                raw_chapter_title: raw_ch.title_candidate.clone(),
                docx_heading: String::new(),
                paragraph_count: 0,
                is_frozen: false,
            });
        }
    }

    mappings
}

/// Applies confirmed frozen chapter mappings to SQLite per Story A4 and FR-QA-11.
pub fn apply_frozen_mappings_to_db(
    conn: &Connection,
    mappings: &[FrozenChapterMapping],
    extracted_headings: &[ExtractedDocxHeading],
) -> Result<(), AppError> {
    let chapter_repo = ChapterRepo::new(conn);
    let segment_repo = SegmentRepo::new(conn);
    let trans_repo = TranslationRepo::new(conn);

    let heading_map: HashMap<usize, &ExtractedDocxHeading> =
        extracted_headings.iter().enumerate().collect();

    for (idx, m) in mappings.iter().enumerate() {
        let existing = chapter_repo.get_by_prefix(&m.raw_chapter_prefix)?;
        let chap = match existing {
            Some(mut c) => {
                if m.is_frozen {
                    c.frozen = true;
                    c.status = "frozen".to_string();
                    c.title_tgt = Some(m.docx_heading.clone());
                } else {
                    c.frozen = false;
                    c.status = "pending".to_string();
                }
                c.updated_at = chrono::Utc::now().to_rfc3339();
                chapter_repo.update_title_tgt(&c.id, &m.docx_heading)?;
                chapter_repo.update_status(&c.id, &c.status)?;
                let _ = conn.execute(
                    "UPDATE chapter SET frozen = ?1 WHERE id = ?2",
                    rusqlite::params![if c.frozen { 1 } else { 0 }, c.id],
                );
                c
            }
            None => continue,
        };

        if m.is_frozen {
            if let Some(h) = heading_map.get(&idx) {
                // Populate segments with imported paragraphs so QA can scan read-only
                let segments = segment_repo.list_by_chapter(&chap.id)?;
                let text_segs: Vec<&Segment> = segments
                    .iter()
                    .filter(|s| s.kind == "narration" || s.kind == "dialogue")
                    .collect();

                for (s_idx, s) in text_segs.iter().enumerate() {
                    let trans_text = if s_idx < h.paragraphs.len() {
                        &h.paragraphs[s_idx]
                    } else if !h.paragraphs.is_empty() {
                        h.paragraphs.last().map(|s| s.as_str()).unwrap_or("")
                    } else {
                        &s.src_text
                    };

                    let _ = trans_repo
                        .insert_version(&chap.id, s.idx, s.idx, trans_text, "frozen", None);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::store::{open_in_memory, Chapter};
    use std::path::PathBuf;

    #[test]
    fn test_extract_headings_from_document_xml() {
        let xml = r#"
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Bab 1: Awal Mula</w:t></w:r></w:p>
    <w:p><w:r><w:t>Ini adalah paragraf pertama bab satu.</w:t></w:r></w:p>
    <w:p><w:r><w:t>Ini adalah paragraf kedua bab satu.</w:t></w:r></w:p>
    <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Bab 2: Pertemuan</w:t></w:r></w:p>
    <w:p><w:r><w:t>Paragraf pembuka bab dua.</w:t></w:r></w:p>
  </w:body>
</w:document>
"#;

        let headings = extract_headings_from_document_xml(xml, "Heading1").unwrap();
        assert_eq!(headings.len(), 2);

        assert_eq!(headings[0].heading_text, "Bab 1: Awal Mula");
        assert_eq!(headings[0].paragraph_count, 2);
        assert_eq!(
            headings[0].paragraphs[0],
            "Ini adalah paragraf pertama bab satu."
        );

        assert_eq!(headings[1].heading_text, "Bab 2: Pertemuan");
        assert_eq!(headings[1].paragraph_count, 1);
        assert_eq!(headings[1].paragraphs[0], "Paragraf pembuka bab dua.");
    }

    #[test]
    fn test_propose_frozen_chapter_mappings() {
        let headings = vec![ExtractedDocxHeading {
            index: 1,
            heading_text: "Bab 1: Pembuka".to_string(),
            paragraph_count: 5,
            paragraphs: vec!["Teks".to_string()],
        }];

        let raw = vec![
            DiscoveredChapter {
                path: PathBuf::from("01_ch1.md"),
                file_name: "01_ch1.md".to_string(),
                prefix: "1".to_string(),
                prefix_num: 1,
                ordinal: 1,
                title_candidate: "第1話".to_string(),
            },
            DiscoveredChapter {
                path: PathBuf::from("02_ch2.md"),
                file_name: "02_ch2.md".to_string(),
                prefix: "2".to_string(),
                prefix_num: 2,
                ordinal: 2,
                title_candidate: "第2話".to_string(),
            },
        ];

        let proposed = propose_frozen_chapter_mappings(&headings, &raw);
        assert_eq!(proposed.len(), 2);

        // Chapter 1 is frozen with mapped heading
        assert!(proposed[0].is_frozen);
        assert_eq!(proposed[0].docx_heading, "Bab 1: Pembuka");
        assert_eq!(proposed[0].paragraph_count, 5);

        // Chapter 2 is unfrozen pending
        assert!(!proposed[1].is_frozen);
        assert_eq!(proposed[1].docx_heading, "");
    }

    #[test]
    fn test_apply_frozen_mappings_to_db() {
        let mut conn = open_in_memory().unwrap();
        crate::store::migrate(&mut conn).unwrap();

        let chap_repo = ChapterRepo::new(&conn);
        let ch = Chapter {
            id: "c1".to_string(),
            ordinal: 1,
            file_name: "01_ch1.md".to_string(),
            prefix: "1".to_string(),
            title_src: "第1話".to_string(),
            title_tgt: None,
            src_hash: "hash".to_string(),
            line_count: 10,
            char_count: 50,
            status: "pending".to_string(),
            frozen: false,
            parts_planned: 1,
            updated_at: "now".to_string(),
        };
        chap_repo.insert(&ch).unwrap();

        let mappings = vec![FrozenChapterMapping {
            raw_chapter_prefix: "1".to_string(),
            raw_chapter_file: "01_ch1.md".to_string(),
            raw_chapter_title: "第1話".to_string(),
            docx_heading: "Bab 1: Terjemahan Beku".to_string(),
            paragraph_count: 1,
            is_frozen: true,
        }];

        let headings = vec![ExtractedDocxHeading {
            index: 1,
            heading_text: "Bab 1: Terjemahan Beku".to_string(),
            paragraph_count: 1,
            paragraphs: vec!["Teks beku".to_string()],
        }];

        apply_frozen_mappings_to_db(&conn, &mappings, &headings).unwrap();

        let updated = chap_repo.get_by_prefix("1").unwrap().unwrap();
        assert!(updated.frozen);
        assert_eq!(updated.status, "frozen");
        assert_eq!(updated.title_tgt.as_deref(), Some("Bab 1: Terjemahan Beku"));
    }
}
