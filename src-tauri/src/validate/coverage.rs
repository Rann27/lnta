//! Coverage and structure validators per FR-VAL-01, FR-VAL-09, FR-VAL-13, and FR-VAL-14.

use std::collections::HashSet;

use crate::prompt::{ParsedSegment, RepairIssue};
use crate::raw::{RawSegment, SegmentKind};

/// Validates segment ID coverage, ordering, and range constraints.
///
/// Rules:
/// - FR-VAL-01: Every expected text segment ID in chunk must appear exactly once in ascending order.
/// - FR-VAL-09: Range segments (`<s id="a-b">`) must not contain more than one dialogue segment.
/// - FR-VAL-14: If chunk is part 1 and contains a title segment, `<t>` must be present.
pub fn validate_coverage(
    parsed_segments: &[ParsedSegment],
    parsed_title: Option<&str>,
    chunk_segments: &[RawSegment],
    part_index: i32,
) -> (Vec<RepairIssue>, Vec<String>) {
    let mut repair_issues = Vec::new();
    let mut warnings = Vec::new();

    // 1. Title validation (FR-VAL-14)
    let has_title_segment = chunk_segments.iter().any(|s| s.kind == SegmentKind::Title);
    if part_index == 1 && has_title_segment && parsed_title.is_none() {
        repair_issues.push(RepairIssue {
            id: 0,
            reason: "judul bab hilang pada bagian pertama".to_string(),
        });
    }

    // Collect all expected text segment IDs (Dialogue and Narration)
    let expected_text_segments: Vec<&RawSegment> = chunk_segments
        .iter()
        .filter(|s| s.kind.sent_to_model() && s.kind != SegmentKind::Title)
        .collect();

    let expected_ids: HashSet<i32> = expected_text_segments.iter().map(|s| s.idx).collect();

    // Track which expected IDs were covered by parsed segments
    let mut covered_ids: HashSet<i32> = HashSet::new();
    let mut last_end_id = -1;

    for seg in parsed_segments {
        let seg_ids = seg.ids();

        // Check ascending order
        if seg.id_start <= last_end_id {
            warnings.push(format!(
                "Segmen {} tidak berurutan naik (setelah segmen {})",
                seg.raw_id, last_end_id
            ));
        }
        last_end_id = seg.id_end;

        // Check for foreign IDs
        for &id in &seg_ids {
            if !expected_ids.contains(&id) {
                warnings.push(format!("ID asing {} dalam output model", id));
            } else if !covered_ids.insert(id) {
                repair_issues.push(RepairIssue {
                    id,
                    reason: format!("id {} muncul lebih dari satu kali", id),
                });
            }
        }

        // FR-VAL-09: Range segment cannot contain more than one dialogue
        if seg.is_range() {
            let dialogue_count = chunk_segments
                .iter()
                .filter(|s| {
                    s.idx >= seg.id_start && s.idx <= seg.id_end && s.kind == SegmentKind::Dialogue
                })
                .count();

            if dialogue_count > 1 {
                repair_issues.push(RepairIssue {
                    id: seg.id_start,
                    reason: format!(
                        "rentang {} memuat {} segmen dialog (maksimal 1)",
                        seg.raw_id, dialogue_count
                    ),
                });
            }
        }
    }

    // Check for missing expected IDs (FR-VAL-01)
    let mut missing_ids: Vec<i32> = expected_ids
        .iter()
        .filter(|id| !covered_ids.contains(id))
        .copied()
        .collect();
    missing_ids.sort_unstable();

    for id in missing_ids {
        repair_issues.push(RepairIssue {
            id,
            reason: format!("segmen {} hilang dari output model", id),
        });
    }

    (repair_issues, warnings)
}

/// Validates seam boundary between split parts per FR-VAL-13.
///
/// Ensures 3 last segments of part k and 3 first segments of part k+1
/// appear exactly once in the combined output sequence.
pub fn validate_split_seam(
    combined_ids: &[i32],
    part_k_last_3: &[i32],
    part_k1_first_3: &[i32],
) -> Vec<String> {
    let mut seam_issues = Vec::new();

    for &id in part_k_last_3.iter().chain(part_k1_first_3.iter()) {
        let count = combined_ids.iter().filter(|&&x| x == id).count();
        if count == 0 {
            seam_issues.push(format!("Segmen seam {} hilang pada batas split", id));
        } else if count > 1 {
            seam_issues.push(format!(
                "Segmen seam {} terduplikasi ({count} kali) pada batas split",
                id
            ));
        }
    }

    seam_issues
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn make_raw_segment(idx: i32, kind: SegmentKind) -> RawSegment {
        RawSegment {
            idx,
            kind,
            src_text: format!("Teks {idx}"),
            line_start: idx,
            line_end: idx,
            image_file: None,
        }
    }

    fn make_parsed_segment(raw_id: &str, start: i32, end: i32) -> ParsedSegment {
        ParsedSegment {
            raw_id: raw_id.to_string(),
            id_start: start,
            id_end: end,
            text: format!("Terjemahan {raw_id}"),
        }
    }

    #[test]
    fn test_fr_val_01_positive_and_negative() {
        let chunk = vec![
            make_raw_segment(1, SegmentKind::Narration),
            make_raw_segment(2, SegmentKind::Dialogue),
            make_raw_segment(3, SegmentKind::Narration),
        ];

        // Positive: all expected IDs present exactly once
        let good_parsed = vec![
            make_parsed_segment("1", 1, 1),
            make_parsed_segment("2", 2, 2),
            make_parsed_segment("3", 3, 3),
        ];
        let (repairs, _) = validate_coverage(&good_parsed, None, &chunk, 2);
        assert!(repairs.is_empty());

        // Negative: segment 2 missing
        let bad_parsed = vec![
            make_parsed_segment("1", 1, 1),
            make_parsed_segment("3", 3, 3),
        ];
        let (repairs_bad, _) = validate_coverage(&bad_parsed, None, &chunk, 2);
        assert_eq!(repairs_bad.len(), 1);
        assert_eq!(repairs_bad[0].id, 2);
        assert!(repairs_bad[0].reason.contains("hilang"));
    }

    #[test]
    fn test_fr_val_09_multi_dialogue_range_positive_and_negative() {
        let chunk = vec![
            make_raw_segment(10, SegmentKind::Narration),
            make_raw_segment(11, SegmentKind::Dialogue),
            make_raw_segment(12, SegmentKind::Dialogue),
            make_raw_segment(13, SegmentKind::Narration),
        ];

        // Positive: range 10-11 covers only 1 dialogue
        let good_range = vec![
            make_parsed_segment("10-11", 10, 11),
            make_parsed_segment("12", 12, 12),
            make_parsed_segment("13", 13, 13),
        ];
        let (repairs_good, _) = validate_coverage(&good_range, None, &chunk, 2);
        assert!(repairs_good.is_empty());

        // Negative: range 11-12 covers 2 dialogues
        let bad_range = vec![
            make_parsed_segment("10", 10, 10),
            make_parsed_segment("11-12", 11, 12),
            make_parsed_segment("13", 13, 13),
        ];
        let (repairs_bad, _) = validate_coverage(&bad_range, None, &chunk, 2);
        assert_eq!(repairs_bad.len(), 1);
        assert_eq!(repairs_bad[0].id, 11);
        assert!(repairs_bad[0].reason.contains("dialog"));
    }

    #[test]
    fn test_fr_val_14_title_presence_positive_and_negative() {
        let chunk = vec![
            make_raw_segment(0, SegmentKind::Title),
            make_raw_segment(1, SegmentKind::Narration),
        ];
        let parsed = vec![make_parsed_segment("1", 1, 1)];

        // Positive: part 1 with title
        let (repairs_good, _) = validate_coverage(&parsed, Some("Judul"), &chunk, 1);
        assert!(repairs_good.is_empty());

        // Negative: part 1 without title
        let (repairs_bad, _) = validate_coverage(&parsed, None, &chunk, 1);
        assert_eq!(repairs_bad.len(), 1);
        assert_eq!(repairs_bad[0].id, 0);
        assert!(repairs_bad[0].reason.contains("judul"));
    }

    #[test]
    fn test_fr_val_13_split_seam_positive_and_negative() {
        // Positive: all 3 seam items on each side appear exactly once
        let combined = vec![7, 8, 9, 10, 11, 12];
        let part_k_last = vec![7, 8, 9];
        let part_k1_first = vec![10, 11, 12];
        let issues = validate_split_seam(&combined, &part_k_last, &part_k1_first);
        assert!(issues.is_empty());

        // Negative: 10 missing, 9 duplicated
        let broken = vec![7, 8, 9, 9, 11, 12];
        let issues_broken = validate_split_seam(&broken, &part_k_last, &part_k1_first);
        assert_eq!(issues_broken.len(), 2);
    }
}
