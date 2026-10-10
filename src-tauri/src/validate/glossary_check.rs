//! Glossary compliance verification per FR-VAL-11.

use crate::glossary::GlossaryEntryItem;
use crate::prompt::{ParsedSegment, RepairIssue};
use crate::raw::RawSegment;

/// Verifies that glossary entries present in the source chunk are used in the translation output.
///
/// Per FR-VAL-11:
/// If `{Source}` appears in the chunk's Japanese source text, but `{Translation}` does not
/// appear anywhere in the chunk's target translations, a repair issue is flagged for the
/// affected source segment(s) and a QA warning is emitted.
pub fn validate_glossary_compliance(
    chunk_segments: &[RawSegment],
    parsed_segments: &[ParsedSegment],
    selected_glossary: &[&GlossaryEntryItem],
) -> (Vec<RepairIssue>, Vec<String>) {
    let mut repair_issues = Vec::new();
    let mut warnings = Vec::new();

    // Combine all parsed target text in lowercase for case-insensitive matching
    let mut combined_target = String::new();
    for seg in parsed_segments {
        combined_target.push_str(&seg.text);
        combined_target.push(' ');
    }
    let combined_target_lower = combined_target.to_lowercase();

    for entry in selected_glossary {
        if entry.source.is_empty() || entry.translation.is_empty() {
            continue;
        }

        // Find segments in chunk where entry.source appears
        let mut affected_segments = Vec::new();
        for s in chunk_segments {
            if s.src_text.contains(&entry.source) {
                affected_segments.push(s.idx);
            }
        }

        if affected_segments.is_empty() {
            continue;
        }

        // Check if translation is used anywhere in target output
        let trans_lower = entry.translation.to_lowercase();
        let is_used = combined_target_lower.contains(&trans_lower);

        if !is_used {
            for &seg_id in &affected_segments {
                repair_issues.push(RepairIssue {
                    id: seg_id,
                    reason: format!(
                        "istilah glosarium {} ({}) tidak ditemukan dalam terjemahan",
                        entry.source, entry.translation
                    ),
                });
            }
            warnings.push(format!(
                "Istilah glosarium {} === {} tidak muncul dalam output chunk",
                entry.source, entry.translation
            ));
        }
    }

    (repair_issues, warnings)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::raw::SegmentKind;

    fn make_raw(idx: i32, text: &str) -> RawSegment {
        RawSegment {
            idx,
            kind: SegmentKind::Narration,
            src_text: text.to_string(),
            line_start: idx,
            line_end: idx,
            image_file: None,
        }
    }

    fn make_parsed(raw_id: &str, id: i32, text: &str) -> ParsedSegment {
        ParsedSegment {
            raw_id: raw_id.to_string(),
            id_start: id,
            id_end: id,
            text: text.to_string(),
        }
    }

    fn make_entry(src: &str, trans: &str) -> GlossaryEntryItem {
        GlossaryEntryItem {
            source: src.to_string(),
            translation: trans.to_string(),
            note: "".to_string(),
            tier: 1,
            line_index: 0,
            has_list_prefix: false,
            list_prefix_char: None,
        }
    }

    #[test]
    fn test_fr_val_11_glossary_compliance_positive_and_negative() {
        let entry = make_entry("王都", "Ibukota Kerajaan");
        let glossary = [&entry];

        let chunk = vec![make_raw(1, "佐藤は王都へ向かった。")];

        // Positive: translation "Ibukota Kerajaan" is used
        let good_parsed = vec![make_parsed("1", 1, "Satou pergi menuju Ibukota Kerajaan.")];
        let (repairs_pos, warnings_pos) =
            validate_glossary_compliance(&chunk, &good_parsed, &glossary);
        assert!(repairs_pos.is_empty());
        assert!(warnings_pos.is_empty());

        // Negative: translation not used
        let bad_parsed = vec![make_parsed("1", 1, "Satou pergi menuju kota besar.")];
        let (repairs_neg, warnings_neg) =
            validate_glossary_compliance(&chunk, &bad_parsed, &glossary);
        assert_eq!(repairs_neg.len(), 1);
        assert_eq!(repairs_neg[0].id, 1);
        assert!(repairs_neg[0].reason.contains("Ibukota Kerajaan"));
        assert_eq!(warnings_neg.len(), 1);
    }
}
