//! LNTA QA scanning and inspection module.
//!
//! Provides pure-core automated checks across translated volumes per
//! PRD §5.5, FR-QA-01..02, and Appendix B.

pub mod common;
pub mod fix_ai;
pub mod fix_mechanical;
pub mod fixlog;
pub mod frequency;
pub mod hotspots;
pub mod ja_en;
pub mod ja_id;
pub mod re;
pub mod review;

pub use common::{QaFinding, SegmentForQa};
pub use fix_ai::{
    apply_ai_fixes_to_db, build_qa_fix_prompt, parse_qa_fix_response, validate_fix_replacement,
    QaFixItem, QaFixPromptArgs, QaFixResponseItem,
};
pub use fix_mechanical::{
    apply_mechanical_fixes_to_text, AppliedMechanicalFix, MechanicalFixOptions,
};
pub use fixlog::{
    apply_mechanical_fixes_to_volume, undo_batch, undo_batch_and_rerender, undo_fix,
    undo_fix_and_rerender, undo_run, undo_run_and_rerender, ApplyMechanicalResult,
};
pub use frequency::{analyze_phrase_frequency, FrequencyItem, FrequencyReport};
pub use hotspots::{
    select_all_hotspots, select_boundary_hotspots, select_frequency_hotspots,
    select_homophone_hotspots, select_late_volume_hotspots, select_long_run_hotspots,
    select_names_hotspots, select_seam_hotspots, select_single_character_hotspots,
    select_wordplay_hotspots, Hotspot, HotspotPair,
};
pub use ja_en::CharacterGender;
pub use review::{
    apply_rename_to_volume, apply_review_fix, build_qa_review_prompt, build_qa_summary_log,
    load_volume_segments_for_qa, parse_qa_review_response, parse_ref_str, QaLoopResult,
    QaReviewArgs, QaReviewResponseItem,
};

/// Runs a full QA scan for the `ja-id` profile over all chapters and segments in a volume.
///
/// Returns all findings with chapter and segment IDs, plus the phrase frequency report.
pub fn scan_ja_id(chapters: &[Vec<SegmentForQa>]) -> (Vec<QaFinding>, FrequencyReport) {
    let mut findings = Vec::new();
    let all_segments: Vec<SegmentForQa> = chapters.iter().flatten().cloned().collect();

    // 1. Per-segment checks
    for seg in &all_segments {
        // Common checks
        findings.extend(common::check_residual_source_and_quotes(seg));
        findings.extend(common::check_macrons(seg));
        findings.extend(common::check_markup_bugs(seg));
        findings.extend(common::check_quotes(seg));
        findings.extend(common::check_run_together_words(seg));
        findings.extend(common::check_punctuation_faults(seg));
        findings.extend(common::check_repeated_letters(seg));

        // ja-id specific checks
        findings.extend(ja_id::check_redundant_attribution(seg));
        findings.extend(ja_id::check_flattened_sibling_terms(seg));
        findings.extend(ja_id::check_genre_terminology(seg));
        findings.extend(ja_id::check_non_standard_spelling(seg));
        findings.extend(ja_id::check_misplaced_hyphen(seg));
        findings.extend(ja_id::check_lowercase_acronyms(seg));
    }

    // 2. Volume-wide cross-segment checks
    findings.extend(common::check_duplicate_paragraphs(&all_segments));
    findings.extend(common::check_name_variants(&all_segments));
    findings.extend(common::check_style_drift(&all_segments));
    findings.extend(common::check_corrections_that_correct_nothing(
        &all_segments,
    ));
    findings.extend(ja_id::check_pronoun_consistency(&all_segments));

    // 3. Metronome openers across chapters (Q05)
    findings.extend(ja_id::check_metronome_openers(chapters));

    // 4. Phrase frequency analysis (Q21)
    let frequency_report = analyze_phrase_frequency(&all_segments);

    (findings, frequency_report)
}

/// Runs a full QA scan for the `ja-en` profile over all chapters and segments in a volume.
///
/// Returns all findings with chapter and segment IDs, plus the phrase frequency report.
pub fn scan_ja_en(
    chapters: &[Vec<SegmentForQa>],
    glossary_characters: &[CharacterGender],
) -> (Vec<QaFinding>, FrequencyReport) {
    let mut findings = Vec::new();
    let all_segments: Vec<SegmentForQa> = chapters.iter().flatten().cloned().collect();

    // 1. Per-segment checks
    for seg in &all_segments {
        // Common checks
        findings.extend(common::check_residual_source_and_quotes(seg));
        findings.extend(common::check_macrons(seg));
        findings.extend(common::check_markup_bugs(seg));
        findings.extend(common::check_quotes(seg));
        findings.extend(common::check_run_together_words(seg));
        findings.extend(common::check_punctuation_faults(seg));
        findings.extend(common::check_repeated_letters(seg));

        // ja-en specific checks
        findings.extend(ja_en::check_redundant_attribution(seg));
        findings.extend(ja_en::check_flattened_sibling_terms(seg));
        findings.extend(ja_en::check_romanized_sfx(seg));
        findings.extend(ja_en::check_lowercase_acronyms(seg));
    }

    // 2. Volume-wide cross-segment checks
    findings.extend(common::check_duplicate_paragraphs(&all_segments));
    findings.extend(common::check_name_variants(&all_segments));
    findings.extend(common::check_style_drift(&all_segments));
    findings.extend(common::check_corrections_that_correct_nothing(
        &all_segments,
    ));
    findings.extend(ja_en::check_gender_conflicts(
        &all_segments,
        glossary_characters,
    ));
    findings.extend(ja_en::check_spelling_consistency(&all_segments));
    findings.extend(ja_en::check_ellipsis_consistency(&all_segments));

    // 3. Metronome openers across chapters (Q05)
    findings.extend(ja_en::check_metronome_openers(chapters));

    // 4. Phrase frequency analysis (Q21)
    let frequency_report = analyze_phrase_frequency(&all_segments);

    (findings, frequency_report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_ja_id_scan_end_to_end() {
        let chap1 = vec![
            SegmentForQa {
                segment_id: "s1".to_string(),
                chapter_id: "c1".to_string(),
                idx: 1,
                src_text: "空".to_string(),
                tgt_text: "Langit begitu biru cerah di atas bukit.".to_string(),
            },
            SegmentForQa {
                segment_id: "s2".to_string(),
                chapter_id: "c1".to_string(),
                idx: 2,
                src_text: "dungeon".to_string(),
                tgt_text: "Mereka bersiap memasuki penjara bawah tanah.".to_string(),
            },
            SegmentForQa {
                segment_id: "s3".to_string(),
                chapter_id: "c1".to_string(),
                idx: 3,
                src_text: "resiko".to_string(),
                tgt_text: "Ada resiko besar yang harus dihadapi.".to_string(),
            },
        ];

        let chapters = vec![chap1];
        let (findings, freq) = scan_ja_id(&chapters);

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.check_code == "Q07"));
        assert!(findings.iter().any(|f| f.check_code == "Q17"));
        assert!(!freq.top_words.is_empty());
    }

    #[test]
    fn test_full_ja_en_scan_end_to_end() {
        let chap1 = vec![
            SegmentForQa {
                segment_id: "s1".to_string(),
                chapter_id: "c1".to_string(),
                idx: 1,
                src_text: "空".to_string(),
                tgt_text: "The blue sky stretched above the quiet hill.".to_string(),
            },
            SegmentForQa {
                segment_id: "s2".to_string(),
                chapter_id: "c1".to_string(),
                idx: 2,
                src_text: "ドキドキ".to_string(),
                tgt_text: "Her heart went doki doki as she opened the door.".to_string(),
            },
            SegmentForQa {
                segment_id: "s3".to_string(),
                chapter_id: "c1".to_string(),
                idx: 3,
                src_text: "テレビ".to_string(),
                tgt_text: "He turned on the tv to watch the morning news.".to_string(),
            },
            SegmentForQa {
                segment_id: "s4".to_string(),
                chapter_id: "c1".to_string(),
                idx: 4,
                src_text: "ふふ".to_string(),
                tgt_text: "\"Fufu, what a curious boy,\" she smiled.".to_string(),
            },
        ];

        let chapters = vec![chap1];
        let characters = vec![CharacterGender::new("Misaki", true)];
        let (findings, freq) = scan_ja_en(&chapters, &characters);

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.check_code == "Q08")); // doki doki
        assert!(findings.iter().any(|f| f.check_code == "Q20")); // tv
                                                                 // "fufu" must NOT be flagged
        assert!(!findings.iter().any(|f| f.matched_text.contains("Fufu")));
        assert!(!freq.top_words.is_empty());
    }
}
