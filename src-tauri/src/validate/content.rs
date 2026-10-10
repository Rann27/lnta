//! Segment content validation per FR-VAL-02, 03, 04, 07, 08, 09, 10.

use crate::prompt::RepairIssue;
use crate::raw::{is_ideograph, is_kana, RawSegment, SegmentKind};

/// Configuration for content validation checks.
#[derive(Debug, Clone)]
pub struct ContentValidationConfig {
    pub min_length_ratio: f64,
    pub max_length_ratio: f64,
    pub critical_short_ratio: f64,
    pub scene_markers: Vec<char>,
}

impl Default for ContentValidationConfig {
    fn default() -> Self {
        Self {
            min_length_ratio: 0.4,
            max_length_ratio: 2.5,
            critical_short_ratio: 0.2,
            scene_markers: vec!['◆', '◇', '★', '☆', '◈', '■', '□'],
        }
    }
}

/// Checks if a character is a residual Japanese character or symbol.
///
/// Per FR-VAL-04:
/// - Hiragana, Katakana, Kanji
/// - Leftover ruby brackets: 《 or 》
/// - Japanese quotes: 「」 or 『』
pub fn is_residual_japanese_char(c: char) -> bool {
    is_kana(c)
        || is_ideograph(c)
        || c == '《'
        || c == '》'
        || c == '「'
        || c == '」'
        || c == '『'
        || c == '』'
        || c == 'ー'
        || c == 'ッ'
        || c == 'っ'
}

/// Checks if a string contains residual Japanese characters.
pub fn contains_residual_japanese(text: &str) -> bool {
    text.chars().any(is_residual_japanese_char)
}

/// Checks if a paragraph opens with a quotation mark.
pub fn starts_with_quote(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with('"')
        || trimmed.starts_with('“')
        || trimmed.starts_with('”')
        || trimmed.starts_with('\'')
        || trimmed.starts_with('‘')
        || trimmed.starts_with('’')
        || trimmed.starts_with('«')
        || trimmed.starts_with('「')
        || trimmed.starts_with('『')
}

/// Validates the content of a single translated segment against its corresponding source segments.
pub fn validate_segment_content(
    id_start: i32,
    raw_id: &str,
    target_text: &str,
    source_segments: &[&RawSegment],
    config: &ContentValidationConfig,
) -> (Vec<RepairIssue>, Vec<String>) {
    let mut repair_issues = Vec::new();
    let mut warnings = Vec::new();

    // 1. Source / Target emptiness (FR-VAL-02)
    let total_src_len: usize = source_segments
        .iter()
        .map(|s| s.src_text.trim().len())
        .sum();
    let tgt_trimmed = target_text.trim();

    if total_src_len > 0 && tgt_trimmed.is_empty() {
        repair_issues.push(RepairIssue {
            id: id_start,
            reason: format!("segmen {} menghasilkan terjemahan kosong", raw_id),
        });
        return (repair_issues, warnings);
    }

    // 2. Length ratio (FR-VAL-03)
    if total_src_len >= 20 {
        let tgt_char_count = tgt_trimmed.chars().count() as f64;
        let src_char_count = total_src_len as f64;
        let ratio = tgt_char_count / src_char_count;

        if ratio < config.critical_short_ratio {
            repair_issues.push(RepairIssue {
                id: id_start,
                reason: format!(
                    "rasio panjang terjemahan segmen {} terlalu pendek ({:.2} < {:.2})",
                    raw_id, ratio, config.critical_short_ratio
                ),
            });
        } else if ratio < config.min_length_ratio || ratio > config.max_length_ratio {
            warnings.push(format!(
                "Rasio panjang segmen {} di luar pita ({:.2}, diharapkan {:.2}..{:.2})",
                raw_id, ratio, config.min_length_ratio, config.max_length_ratio
            ));
        }
    }

    // 3. Residual Japanese characters (FR-VAL-04)
    if contains_residual_japanese(tgt_trimmed) {
        repair_issues.push(RepairIssue {
            id: id_start,
            reason: format!("segmen {} memuat sisa aksara Jepang", raw_id),
        });
    }

    // 4. Paragraph-level checks: asterisks, quotes, scene markers
    let paragraphs: Vec<&str> = tgt_trimmed.split("\n\n").collect();

    for (para_idx, para) in paragraphs.iter().enumerate() {
        let para_trimmed = para.trim();
        if para_trimmed.is_empty() {
            continue;
        }

        // FR-VAL-07: Odd count of asterisks (*)
        let asterisk_count = para_trimmed.chars().filter(|&c| c == '*').count();
        if asterisk_count % 2 != 0 {
            repair_issues.push(RepairIssue {
                id: id_start,
                reason: format!(
                    "paragraf {} pada segmen {} memuat jumlah tanda bintang ganjil ({asterisk_count})",
                    para_idx + 1, raw_id
                ),
            });
        }

        // FR-VAL-10: Odd count of double quotes (")
        // Count ASCII double quotes or typographic quote pairs
        let ascii_quotes = para_trimmed.chars().filter(|&c| c == '"').count();
        let curly_open = para_trimmed.chars().filter(|&c| c == '“').count();
        let curly_close = para_trimmed.chars().filter(|&c| c == '”').count();

        if ascii_quotes % 2 != 0 || curly_open != curly_close {
            repair_issues.push(RepairIssue {
                id: id_start,
                reason: format!(
                    "paragraf {} pada segmen {} memuat tanda petik ganda tidak seimbang",
                    para_idx + 1,
                    raw_id
                ),
            });
        }

        // FR-VAL-08: Scene markers inside text
        let has_scene_markers = para_trimmed
            .chars()
            .any(|c| config.scene_markers.contains(&c));
        if has_scene_markers {
            repair_issues.push(RepairIssue {
                id: id_start,
                reason: format!("segmen {} memuat penanda adegan di dalam teks", raw_id),
            });
        }
    }

    // 5. FR-VAL-09: Dialogue opening quotation
    // If ANY of the source segments was a dialogue segment:
    let is_dialogue = source_segments
        .iter()
        .any(|s| s.kind == SegmentKind::Dialogue);
    if is_dialogue && !starts_with_quote(tgt_trimmed) {
        repair_issues.push(RepairIssue {
            id: id_start,
            reason: format!("segmen dialog {} tidak dibuka dengan tanda petik", raw_id),
        });
    }

    (repair_issues, warnings)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn make_src(idx: i32, kind: SegmentKind, text: &str) -> RawSegment {
        RawSegment {
            idx,
            kind,
            src_text: text.to_string(),
            line_start: idx,
            line_end: idx,
            image_file: None,
        }
    }

    #[test]
    fn test_fr_val_02_empty_translation_positive_and_negative() {
        let src = make_src(1, SegmentKind::Narration, "朝が来た。");
        let cfg = ContentValidationConfig::default();

        // Positive
        let (repairs_pos, _) = validate_segment_content(1, "1", "Pagi telah tiba.", &[&src], &cfg);
        assert!(repairs_pos.is_empty());

        // Negative: empty target
        let (repairs_neg, _) = validate_segment_content(1, "1", "   ", &[&src], &cfg);
        assert_eq!(repairs_neg.len(), 1);
        assert!(repairs_neg[0].reason.contains("kosong"));
    }

    #[test]
    fn test_fr_val_03_length_ratio_positive_and_negative() {
        let long_src = make_src(
            1,
            SegmentKind::Narration,
            "部屋の中に柔らかな光が差し込んでいた。とても静かな朝だった。", // 31 chars
        );
        let cfg = ContentValidationConfig::default();

        // Positive normal ratio
        let (repairs_pos, warnings_pos) = validate_segment_content(
            1,
            "1",
            "Cahaya lembut masuk ke dalam kamar. Pagi itu begitu hening.",
            &[&long_src],
            &cfg,
        );
        assert!(repairs_pos.is_empty());
        assert!(warnings_pos.is_empty());

        // Negative: critically short ratio (< 0.2)
        let (repairs_neg, _) = validate_segment_content(1, "1", "Ya.", &[&long_src], &cfg);
        assert_eq!(repairs_neg.len(), 1);
        assert!(repairs_neg[0].reason.contains("terlalu pendek"));
    }

    #[test]
    fn test_fr_val_04_residual_japanese_positive_and_negative() {
        let src = make_src(1, SegmentKind::Narration, "歩き続けた。");
        let cfg = ContentValidationConfig::default();

        // Positive
        let (repairs_pos, _) =
            validate_segment_content(1, "1", "Ia terus berjalan.", &[&src], &cfg);
        assert!(repairs_pos.is_empty());

        // Negative: residual kanji/kana or ruby brackets
        let (repairs_neg1, _) =
            validate_segment_content(1, "1", "Ia terus berjalan bersama 健太.", &[&src], &cfg);
        assert_eq!(repairs_neg1.len(), 1);
        assert!(repairs_neg1[0].reason.contains("sisa aksara Jepang"));

        let (repairs_neg2, _) =
            validate_segment_content(1, "1", "Ia menatap《Kenta》.", &[&src], &cfg);
        assert_eq!(repairs_neg2.len(), 1);
        assert!(repairs_neg2[0].reason.contains("sisa aksara Jepang"));
    }

    #[test]
    fn test_fr_val_07_odd_asterisks_positive_and_negative() {
        let src = make_src(1, SegmentKind::Narration, "足音が響いた。");
        let cfg = ContentValidationConfig::default();

        // Positive: even asterisks (*italic*)
        let (repairs_pos, _) =
            validate_segment_content(1, "1", "Suara *tap tap* terdengar.", &[&src], &cfg);
        assert!(repairs_pos.is_empty());

        // Negative: odd asterisks
        let (repairs_neg, _) =
            validate_segment_content(1, "1", "Suara *tap tap terdengar.", &[&src], &cfg);
        assert_eq!(repairs_neg.len(), 1);
        assert!(repairs_neg[0].reason.contains("tanda bintang"));
    }

    #[test]
    fn test_fr_val_08_scene_marker_inside_text_positive_and_negative() {
        let src = make_src(1, SegmentKind::Narration, "星が見えた。");
        let cfg = ContentValidationConfig::default();

        // Positive
        let (repairs_pos, _) =
            validate_segment_content(1, "1", "Bintang-bintang terlihat terang.", &[&src], &cfg);
        assert!(repairs_pos.is_empty());

        // Negative: embedded marker ☆
        let (repairs_neg, _) = validate_segment_content(
            1,
            "1",
            "Bintang-bintang terlihat ☆ di langit.",
            &[&src],
            &cfg,
        );
        assert_eq!(repairs_neg.len(), 1);
        assert!(repairs_neg[0].reason.contains("penanda adegan"));
    }

    #[test]
    fn test_fr_val_09_dialogue_opening_quote_positive_and_negative() {
        let src = make_src(2, SegmentKind::Dialogue, "「おはよう」");
        let cfg = ContentValidationConfig::default();

        // Positive
        let (repairs_pos, _) = validate_segment_content(2, "2", "\"Selamat pagi.\"", &[&src], &cfg);
        assert!(repairs_pos.is_empty());

        // Negative: missing opening quote
        let (repairs_neg, _) = validate_segment_content(2, "2", "Selamat pagi.", &[&src], &cfg);
        assert_eq!(repairs_neg.len(), 1);
        assert!(repairs_neg[0].reason.contains("dialog"));
    }

    #[test]
    fn test_fr_val_10_unbalanced_quotes_positive_and_negative() {
        let src = make_src(1, SegmentKind::Narration, "「はい」");
        let cfg = ContentValidationConfig::default();

        // Positive: balanced quotes
        let (repairs_pos, _) = validate_segment_content(1, "1", "\"Ya, tentu.\"", &[&src], &cfg);
        assert!(repairs_pos.is_empty());

        // Negative: odd quotes
        let (repairs_neg, _) = validate_segment_content(1, "1", "\"Ya, tentu.", &[&src], &cfg);
        assert_eq!(repairs_neg.len(), 1);
        assert!(repairs_neg[0].reason.contains("tanda petik ganda"));
    }
}
