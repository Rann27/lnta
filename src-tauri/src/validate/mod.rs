//! LNTA segment validation and mechanical fixing pipeline per PRD §5.3, §6.3, and FR-VAL-01..14.

pub mod content;
pub mod coverage;
pub mod fixers;
pub mod glossary_check;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::glossary::GlossaryEntryItem;
use crate::prompt::{ParsedModelOutput, ParsedSegment, RepairIssue};
use crate::raw::RawSegment;

pub use content::{
    contains_residual_japanese, is_residual_japanese_char, validate_segment_content,
    ContentValidationConfig,
};
pub use coverage::{validate_coverage, validate_split_seam};
pub use fixers::{
    apply_mechanical_fixes, fix_bold_markdown, fix_brackets, fix_macrons,
    fix_scene_marker_paragraphs, MechanicalFixReport,
};
pub use glossary_check::validate_glossary_compliance;

/// A segment that has undergone mechanical fixes and validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedSegment {
    pub raw_id: String,
    pub id_start: i32,
    pub id_end: i32,
    pub text: String,
    pub mechanical_fixes: Vec<MechanicalFixReport>,
    pub issues: Vec<String>,
}

/// The aggregated outcome of validating a chunk output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkValidationResult {
    pub is_valid: bool,
    pub title: Option<String>,
    pub segments: Vec<ValidatedSegment>,
    pub repair_issues: Vec<RepairIssue>,
    pub warnings: Vec<String>,
    pub mechanical_fixes: Vec<MechanicalFixReport>,
}

/// Validates a parsed model output against source chunk segments and glossary.
///
/// Workflow:
/// 1. Applies mechanical fixes (quotes, bold, macrons, marker-only paragraphs) to title and segments.
/// 2. Validates segment ID coverage, ordering, and range rules (FR-VAL-01, 09, 14).
/// 3. Validates segment content (emptiness, length ratio, residual Japanese, asterisks, quotes, scene markers).
/// 4. Validates glossary compliance (FR-VAL-11).
/// 5. Deduplicates and merges repair issues per segment ID.
pub fn validate_chunk(
    model_output: &ParsedModelOutput,
    chunk_segments: &[RawSegment],
    selected_glossary: &[&GlossaryEntryItem],
    part_index: i32,
    config: &ContentValidationConfig,
) -> ChunkValidationResult {
    let mut all_fixes = Vec::new();
    let mut all_warnings = Vec::new();
    let mut raw_repair_issues = Vec::new();

    // 1. Process Title if present
    let mut fixed_title: Option<String> = None;
    if let Some(ref raw_t) = model_output.title {
        let (fixed_t, title_fixes) =
            apply_mechanical_fixes(raw_t, selected_glossary, &config.scene_markers);
        all_fixes.extend(title_fixes);

        if contains_residual_japanese(&fixed_t) {
            raw_repair_issues.push(RepairIssue {
                id: 0,
                reason: "judul bab memuat sisa aksara Jepang".to_string(),
            });
        }
        fixed_title = Some(fixed_t);
    }

    // 2. Apply mechanical fixes to all parsed segments
    let mut fixed_parsed_segments = Vec::new();
    let mut validated_segments = Vec::new();

    for seg in &model_output.segments {
        let (fixed_text, seg_fixes) =
            apply_mechanical_fixes(&seg.text, selected_glossary, &config.scene_markers);
        all_fixes.extend(seg_fixes.clone());

        fixed_parsed_segments.push(ParsedSegment {
            raw_id: seg.raw_id.clone(),
            id_start: seg.id_start,
            id_end: seg.id_end,
            text: fixed_text.clone(),
        });

        validated_segments.push(ValidatedSegment {
            raw_id: seg.raw_id.clone(),
            id_start: seg.id_start,
            id_end: seg.id_end,
            text: fixed_text,
            mechanical_fixes: seg_fixes,
            issues: Vec::new(),
        });
    }

    // 3. Coverage validation (FR-VAL-01, 09, 14)
    let (coverage_repairs, coverage_warnings) = validate_coverage(
        &fixed_parsed_segments,
        fixed_title.as_deref(),
        chunk_segments,
        part_index,
    );
    raw_repair_issues.extend(coverage_repairs);
    all_warnings.extend(coverage_warnings);

    // 4. Content validation (FR-VAL-02, 03, 04, 07, 08, 09, 10)
    for seg in &fixed_parsed_segments {
        let source_segments: Vec<&RawSegment> = chunk_segments
            .iter()
            .filter(|s| s.idx >= seg.id_start && s.idx <= seg.id_end)
            .collect();

        let (content_repairs, content_warnings) = validate_segment_content(
            seg.id_start,
            &seg.raw_id,
            &seg.text,
            &source_segments,
            config,
        );
        raw_repair_issues.extend(content_repairs);
        all_warnings.extend(content_warnings);
    }

    // 5. Glossary compliance validation (FR-VAL-11)
    let (glossary_repairs, glossary_warnings) =
        validate_glossary_compliance(chunk_segments, &fixed_parsed_segments, selected_glossary);
    raw_repair_issues.extend(glossary_repairs);
    all_warnings.extend(glossary_warnings);

    // 6. Merge and deduplicate repair issues by segment ID
    let mut issue_map: BTreeMap<i32, Vec<String>> = BTreeMap::new();
    for issue in raw_repair_issues {
        issue_map.entry(issue.id).or_default().push(issue.reason);
    }

    let mut consolidated_repair_issues = Vec::new();
    for (id, reasons) in issue_map {
        let joined_reason = reasons.join("; ");
        consolidated_repair_issues.push(RepairIssue {
            id,
            reason: joined_reason.clone(),
        });

        // Record issues on the corresponding validated segment
        for vseg in &mut validated_segments {
            if vseg.id_start == id {
                vseg.issues.push(joined_reason.clone());
            }
        }
    }

    let is_valid = consolidated_repair_issues.is_empty();

    ChunkValidationResult {
        is_valid,
        title: fixed_title,
        segments: validated_segments,
        repair_issues: consolidated_repair_issues,
        warnings: all_warnings,
        mechanical_fixes: all_fixes,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::raw::SegmentKind;

    fn make_raw_segment(idx: i32, kind: SegmentKind, src: &str) -> RawSegment {
        RawSegment {
            idx,
            kind,
            src_text: src.to_string(),
            line_start: idx,
            line_end: idx,
            image_file: None,
        }
    }

    fn make_parsed(raw_id: &str, id_start: i32, id_end: i32, text: &str) -> ParsedSegment {
        ParsedSegment {
            raw_id: raw_id.to_string(),
            id_start,
            id_end,
            text: text.to_string(),
        }
    }

    #[test]
    fn test_validate_chunk_end_to_end_clean() {
        let chunk_raw = vec![
            make_raw_segment(0, SegmentKind::Title, "第1話　旅立ち"),
            make_raw_segment(1, SegmentKind::Dialogue, "「おはよう」"),
            make_raw_segment(2, SegmentKind::Narration, "朝日が昇る。"),
        ];

        let model_output = ParsedModelOutput {
            title: Some("Bab 1: Keberangkatan".to_string()),
            segments: vec![
                make_parsed("1", 1, 1, "\"Selamat pagi.\""),
                make_parsed("2", 2, 2, "Matahari pagi mulai terbit."),
            ],
            glossary_proposals: Vec::new(),
            echoed_context: Vec::new(),
            duplicate_segments: Vec::new(),
            model_comments: Vec::new(),
            auto_closed_final_tag: false,
        };

        let result = validate_chunk(
            &model_output,
            &chunk_raw,
            &[],
            1,
            &ContentValidationConfig::default(),
        );

        assert!(result.is_valid);
        assert!(result.repair_issues.is_empty());
        assert_eq!(result.segments.len(), 2);
    }

    #[test]
    fn test_validate_chunk_applies_mechanical_fixes_and_flags_repairs() {
        let chunk_raw = vec![
            make_raw_segment(1, SegmentKind::Dialogue, "「おはよう」"),
            make_raw_segment(2, SegmentKind::Narration, "歩き続けた。"),
        ];

        let model_output = ParsedModelOutput {
            title: None,
            // Segment 1 has 「」 which must be mechanically fixed, plus bold **teks**
            // Segment 2 has residual Japanese (kanji) which must trigger repair
            segments: vec![
                make_parsed("1", 1, 1, "「**Selamat pagi**, Satō!」"),
                make_parsed("2", 2, 2, "Ia terus berjalan bersama 健太."),
            ],
            glossary_proposals: Vec::new(),
            echoed_context: Vec::new(),
            duplicate_segments: Vec::new(),
            model_comments: Vec::new(),
            auto_closed_final_tag: false,
        };

        let result = validate_chunk(
            &model_output,
            &chunk_raw,
            &[],
            2,
            &ContentValidationConfig::default(),
        );

        // Segment 1 should be mechanically fixed
        assert_eq!(result.segments[0].text, "\"Selamat pagi, Satou!\"");
        assert!(!result.mechanical_fixes.is_empty());

        // Segment 2 has residual Japanese, so chunk is invalid and needs repair
        assert!(!result.is_valid);
        assert_eq!(result.repair_issues.len(), 1);
        assert_eq!(result.repair_issues[0].id, 2);
        assert!(result.repair_issues[0]
            .reason
            .contains("sisa aksara Jepang"));
    }
}
