//! Common QA scan checks shared across translation profiles.
//!
//! Implements pure-core scan checks per PRD Appendix B and FR-QA-01..02.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use strsim::levenshtein;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QaFinding {
    pub id: String,
    pub check_code: String,
    pub chapter_id: String,
    pub segment_id: String,
    pub severity: String, // "error", "warning", "info"
    pub message: String,
    pub matched_text: String,
    pub detail_json: String,
}

#[derive(Debug, Clone)]
pub struct SegmentForQa {
    pub segment_id: String,
    pub chapter_id: String,
    pub idx: i32,
    pub src_text: String,
    pub tgt_text: String,
}

impl QaFinding {
    pub fn new(
        check_code: &str,
        chapter_id: &str,
        segment_id: &str,
        severity: &str,
        message: &str,
        matched_text: &str,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            check_code: check_code.to_string(),
            chapter_id: chapter_id.to_string(),
            segment_id: segment_id.to_string(),
            severity: severity.to_string(),
            message: message.to_string(),
            matched_text: matched_text.to_string(),
            detail_json: serde_json::json!({
                "matched": matched_text,
                "msg": message,
            })
            .to_string(),
        }
    }
}

/// Q01, Q01a-g: Residual Japanese characters and brackets.
pub fn check_residual_source_and_quotes(seg: &SegmentForQa) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    let text = &seg.tgt_text;

    // Q01a: 「 or 」
    if text.contains('「') || text.contains('」') {
        findings.push(QaFinding::new(
            "Q01a",
            &seg.chapter_id,
            &seg.segment_id,
            "error",
            "Residual Japanese corner brackets 「」",
            "「」",
        ));
    }

    // Q01b: 『 or 』 inside quotes
    if (text.contains('『') || text.contains('』')) && (text.contains('"') || text.contains('“'))
    {
        findings.push(QaFinding::new(
            "Q01b",
            &seg.chapter_id,
            &seg.segment_id,
            "error",
            "Nested 『』 inside quotes",
            "『』",
        ));
    }

    // Q01c: Standalone 『』, 【】, 〈〉, 《》 (info level per PRD Appendix B)
    if text.contains('【')
        || text.contains('】')
        || text.contains('〈')
        || text.contains('〉')
        || text.contains('《')
        || text.contains('》')
    {
        findings.push(QaFinding::new(
            "Q01c",
            &seg.chapter_id,
            &seg.segment_id,
            "info",
            "Japanese brackets 【】, 〈〉, or 《》 present",
            "【】/〈〉/《》",
        ));
    }

    // Q01d: Japanese double ellipsis ……
    if text.contains("……") {
        findings.push(QaFinding::new(
            "Q01d",
            &seg.chapter_id,
            &seg.segment_id,
            "warning",
            "Japanese double ellipsis ……",
            "……",
        ));
    }

    // Q01e: Dramatic bar ─ or ──
    if text.contains('─') {
        findings.push(QaFinding::new(
            "Q01e",
            &seg.chapter_id,
            &seg.segment_id,
            "warning",
            "Japanese dramatic bar ─",
            "─",
        ));
    }

    // Q01f: ッ, っ, ー attached to Latin text
    for (i, c) in text.char_indices() {
        if c == 'ッ' || c == 'っ' || c == 'ー' {
            let before = text[..i].chars().next_back();
            let after = text[i + c.len_utf8()..].chars().next();
            let is_latin = |ch: char| ch.is_ascii_alphabetic();
            if before.is_some_and(is_latin) || after.is_some_and(is_latin) {
                findings.push(QaFinding::new(
                    "Q01f",
                    &seg.chapter_id,
                    &seg.segment_id,
                    "error",
                    "Kana mark attached to Latin text",
                    &c.to_string(),
                ));
                break;
            }
        }
    }

    // Q01g: LaTeX style quotes (`` or '')
    if text.contains("``") || text.contains("''") {
        findings.push(QaFinding::new(
            "Q01g",
            &seg.chapter_id,
            &seg.segment_id,
            "warning",
            "LaTeX style quotes (`` or '')",
            "`` / ''",
        ));
    }

    // Q01: General residual Japanese kana/kanji
    for c in text.chars() {
        if matches!(c, '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}' | '\u{4E00}'..='\u{9FFF}')
        {
            findings.push(QaFinding::new(
                "Q01",
                &seg.chapter_id,
                &seg.segment_id,
                "error",
                "Residual Japanese kana or kanji",
                &c.to_string(),
            ));
            break;
        }
    }

    findings
}

/// Q02: Macron characters (ā, ī, ū, ē, ō and capitals).
pub fn check_macrons(seg: &SegmentForQa) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    let macrons = ['ā', 'ī', 'ū', 'ē', 'ō', 'Ā', 'Ī', 'Ū', 'Ē', 'Ō'];
    for c in seg.tgt_text.chars() {
        if macrons.contains(&c) {
            findings.push(QaFinding::new(
                "Q02",
                &seg.chapter_id,
                &seg.segment_id,
                "error",
                "Macron vowel present (use digraph)",
                &c.to_string(),
            ));
            break;
        }
    }
    findings
}

/// Q03a, Q03b: Markup bugs (**bold** and unclosed *).
pub fn check_markup_bugs(seg: &SegmentForQa) -> Vec<QaFinding> {
    let mut findings = Vec::new();

    // Q03a: ** bold
    if seg.tgt_text.contains("**") {
        findings.push(QaFinding::new(
            "Q03a",
            &seg.chapter_id,
            &seg.segment_id,
            "error",
            "Markdown bold ** is forbidden (italics only)",
            "**",
        ));
    }

    // Q03b: Odd number of *
    let asterisk_count = seg.tgt_text.chars().filter(|&c| c == '*').count();
    if asterisk_count % 2 != 0 {
        findings.push(QaFinding::new(
            "Q03b",
            &seg.chapter_id,
            &seg.segment_id,
            "error",
            "Odd number of asterisks (unclosed italic delimiter)",
            "*",
        ));
    }

    findings
}

/// Q10: Garbage/long repeated letters (5+ identical consecutive letters).
pub fn check_repeated_letters(seg: &SegmentForQa) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    let mut current_char: Option<char> = None;
    let mut count = 0;

    for c in seg.tgt_text.chars() {
        if c.is_ascii_alphabetic() {
            if Some(c.to_ascii_lowercase()) == current_char {
                count += 1;
                if count == 5 {
                    let matched = current_char.unwrap_or(c).to_string().repeat(5);
                    findings.push(QaFinding::new(
                        "Q10",
                        &seg.chapter_id,
                        &seg.segment_id,
                        "warning",
                        "Run of 5+ identical consecutive letters",
                        &matched,
                    ));
                }
            } else {
                current_char = Some(c.to_ascii_lowercase());
                count = 1;
            }
        } else {
            current_char = None;
            count = 0;
        }
    }

    findings
}

/// Q14, Q15: Unbalanced or stacked double quotes.
pub fn check_quotes(seg: &SegmentForQa) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    let text = &seg.tgt_text;

    // Q14: Odd number of straight double quotes
    let quote_count = text.chars().filter(|&c| c == '"').count();
    if quote_count % 2 != 0 {
        findings.push(QaFinding::new(
            "Q14",
            &seg.chapter_id,
            &seg.segment_id,
            "error",
            "Odd number of double quotes (unbalanced dialogue quote)",
            "\"",
        ));
    }

    // Q15: Stacked double quotes ("" or ””)
    if text.contains("\"\"") || text.contains("””") || text.contains("““") {
        findings.push(QaFinding::new(
            "Q15",
            &seg.chapter_id,
            &seg.segment_id,
            "error",
            "Stacked double quotes (simultaneous speech pattern)",
            "\"\"",
        ));
    }

    findings
}

/// Q16: Run-together words (camelCase lowercase then uppercase, or token length > 20).
pub fn check_run_together_words(seg: &SegmentForQa) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    let words: Vec<&str> = seg
        .tgt_text
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .filter(|w| !w.is_empty())
        .collect();

    for word in words {
        // Lowercase directly followed by uppercase
        let mut prev_lower = false;
        let mut has_camel = false;
        for c in word.chars() {
            if c.is_ascii_lowercase() {
                prev_lower = true;
            } else if c.is_ascii_uppercase() && prev_lower {
                has_camel = true;
                break;
            } else {
                prev_lower = false;
            }
        }

        if has_camel {
            findings.push(QaFinding::new(
                "Q16",
                &seg.chapter_id,
                &seg.segment_id,
                "error",
                "Run-together words: lowercase followed by uppercase",
                word,
            ));
            continue;
        }

        // Alphabetic token length > 20
        if word.chars().all(|c| c.is_ascii_alphabetic()) && word.chars().count() > 20 {
            findings.push(QaFinding::new(
                "Q16",
                &seg.chapter_id,
                &seg.segment_id,
                "warning",
                "Excessively long word (> 20 letters)",
                word,
            ));
        }
    }

    findings
}

/// Q19: Punctuation faults (space before punctuation, double spaces, double hyphens).
pub fn check_punctuation_faults(seg: &SegmentForQa) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    let text = &seg.tgt_text;

    // Space before punctuation: ' ,' ' .' ' ;' ' :' ' !' ' ?'
    for punct in [',', '.', ';', ':', '!', '?'] {
        let pattern = format!(" {}", punct);
        if text.contains(&pattern) {
            findings.push(QaFinding::new(
                "Q19",
                &seg.chapter_id,
                &seg.segment_id,
                "warning",
                &format!("Space before punctuation mark '{}'", punct),
                &pattern,
            ));
            break;
        }
    }

    // Double space
    if text.contains("  ") {
        findings.push(QaFinding::new(
            "Q19",
            &seg.chapter_id,
            &seg.segment_id,
            "warning",
            "Double space present",
            "  ",
        ));
    }

    // Double hyphen '--'
    if text.contains("--") {
        findings.push(QaFinding::new(
            "Q19",
            &seg.chapter_id,
            &seg.segment_id,
            "warning",
            "Double hyphen '--' should be em dash",
            "--",
        ));
    }

    findings
}

/// Q23: Duplicate paragraphs across chapters (> 60 chars).
pub fn check_duplicate_paragraphs(segments: &[SegmentForQa]) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    let mut seen: HashMap<String, (String, String)> = HashMap::new(); // text -> (chapter_id, segment_id)

    for seg in segments {
        let clean = seg.tgt_text.trim();
        if clean.chars().count() > 60 {
            if let Some((orig_chap, orig_seg)) = seen.get(clean) {
                // If it belongs to a different segment
                if orig_seg != &seg.segment_id {
                    findings.push(QaFinding::new(
                        "Q23",
                        &seg.chapter_id,
                        &seg.segment_id,
                        "error",
                        &format!(
                            "Duplicate paragraph identical to chapter {} segment {}",
                            orig_chap, orig_seg
                        ),
                        clean,
                    ));
                }
            } else {
                seen.insert(
                    clean.to_string(),
                    (seg.chapter_id.clone(), seg.segment_id.clone()),
                );
            }
        }
    }

    findings
}

/// Q09: Name variants (edit distance 1-2 or vowel differences).
pub fn check_name_variants(segments: &[SegmentForQa]) -> Vec<QaFinding> {
    let mut findings = Vec::new();

    // Collect capitalized words (> 2 chars) with their segment locations
    let mut word_locations: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for seg in segments {
        for word in seg.tgt_text.split_whitespace() {
            let clean: String = word.chars().filter(|c| c.is_alphanumeric()).collect();

            if clean.chars().count() >= 3
                && clean.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && clean.chars().skip(1).all(|c| c.is_ascii_lowercase())
            {
                word_locations
                    .entry(clean)
                    .or_default()
                    .push((seg.chapter_id.clone(), seg.segment_id.clone()));
            }
        }
    }

    let words: Vec<String> = word_locations.keys().cloned().collect();
    let mut reported_pairs = HashSet::new();

    // Vowel normalization key (e.g. Yuu -> Yu, Kyouya -> Kyoya)
    fn normalize_vowels(w: &str) -> String {
        w.to_lowercase()
            .replace("uu", "u")
            .replace("ou", "o")
            .replace("ii", "i")
            .replace("ee", "e")
            .replace("oo", "o")
    }

    for i in 0..words.len() {
        for j in (i + 1)..words.len() {
            let w1 = &words[i];
            let w2 = &words[j];

            let dist = levenshtein(&w1.to_lowercase(), &w2.to_lowercase());
            let same_normalized = normalize_vowels(w1) == normalize_vowels(w2);

            if dist <= 2 || same_normalized {
                let pair_key = if w1 < w2 {
                    format!("{}:{}", w1, w2)
                } else {
                    format!("{}:{}", w2, w1)
                };

                if !reported_pairs.contains(&pair_key) {
                    reported_pairs.insert(pair_key);

                    // Flag occurrences of the variant
                    if let Some(locs) = word_locations.get(w2) {
                        for (chap_id, seg_id) in locs.iter().take(3) {
                            findings.push(QaFinding::new(
                                "Q09",
                                chap_id,
                                seg_id,
                                "warning",
                                &format!("Potential name variant of '{}'", w1),
                                w2,
                            ));
                        }
                    }
                }
            }
        }
    }

    findings
}

/// Q11: Style drift between straight quotes and curly quotes.
pub fn check_style_drift(segments: &[SegmentForQa]) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    let mut straight_count = 0;
    let mut curly_count = 0;

    for seg in segments {
        straight_count += seg
            .tgt_text
            .chars()
            .filter(|&c| c == '"' || c == '\'')
            .count();
        curly_count += seg
            .tgt_text
            .chars()
            .filter(|&c| c == '“' || c == '”' || c == '‘' || c == '’')
            .count();
    }

    // If both styles appear, flag minority style
    if straight_count > 0 && curly_count > 0 {
        let minority_is_straight = straight_count < curly_count;
        let minority_desc = if minority_is_straight {
            "Straight quotes mixed with curly quotes"
        } else {
            "Curly quotes mixed with straight quotes"
        };

        for seg in segments {
            let has_minority = if minority_is_straight {
                seg.tgt_text.contains('"') || seg.tgt_text.contains('\'')
            } else {
                seg.tgt_text.contains('“')
                    || seg.tgt_text.contains('”')
                    || seg.tgt_text.contains('‘')
                    || seg.tgt_text.contains('’')
            };

            if has_minority {
                findings.push(QaFinding::new(
                    "Q11",
                    &seg.chapter_id,
                    &seg.segment_id,
                    "warning",
                    minority_desc,
                    if minority_is_straight { "\"" } else { "“" },
                ));
            }
        }
    }

    findings
}

/// Q24: Corrections that correct nothing.
/// Short paragraphs containing correction markers ("bukan", "maksudku", "salah", "yang benar", "no", "not", "I mean", "wrong")
/// and sharing a content word with the preceding paragraph.
pub fn check_corrections_that_correct_nothing(segments: &[SegmentForQa]) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    let correction_markers = [
        "bukan",
        "maksudku",
        "salah",
        "yang benar",
        "itu ...",
        "no",
        "not",
        "i mean",
        "wrong",
        "that's",
    ];

    for i in 1..segments.len() {
        let prev = &segments[i - 1];
        let curr = &segments[i];

        let curr_lower = curr.tgt_text.to_lowercase();
        let is_short = curr.tgt_text.chars().count() <= 60;

        let has_marker = is_short && correction_markers.iter().any(|&m| curr_lower.contains(m));

        if has_marker {
            // Check if curr shares a content word (> 3 chars) with prev
            let prev_words: HashSet<String> = prev
                .tgt_text
                .to_lowercase()
                .split_whitespace()
                .filter(|w| w.chars().count() >= 4)
                .map(|w| w.to_string())
                .collect();

            let curr_words: HashSet<String> = curr_lower
                .split_whitespace()
                .filter(|w| w.chars().count() >= 4)
                .map(|w| w.to_string())
                .collect();

            let shared: Vec<&String> = curr_words.intersection(&prev_words).collect();
            if !shared.is_empty() {
                findings.push(QaFinding::new(
                    "Q24",
                    &curr.chapter_id,
                    &curr.segment_id,
                    "warning",
                    &format!(
                        "Correction marker with shared word '{}' (potential dead wordplay)",
                        shared[0]
                    ),
                    &curr.tgt_text,
                ));
            }
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_q01_residual_japanese() {
        let seg = SegmentForQa {
            segment_id: "s1".to_string(),
            chapter_id: "c1".to_string(),
            idx: 1,
            src_text: "空".to_string(),
            tgt_text: "Langit 「biru」 sekali……".to_string(),
        };

        let findings = check_residual_source_and_quotes(&seg);
        assert!(findings.iter().any(|f| f.check_code == "Q01a"));
        assert!(findings.iter().any(|f| f.check_code == "Q01d"));
    }

    #[test]
    fn test_q02_macron() {
        let seg = SegmentForQa {
            segment_id: "s1".to_string(),
            chapter_id: "c1".to_string(),
            idx: 1,
            src_text: "優".to_string(),
            tgt_text: "Yū berdiri.".to_string(),
        };

        let findings = check_macrons(&seg);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].check_code, "Q02");
    }

    #[test]
    fn test_q03_markup_bugs() {
        let seg = SegmentForQa {
            segment_id: "s1".to_string(),
            chapter_id: "c1".to_string(),
            idx: 1,
            src_text: "bold".to_string(),
            tgt_text: "**Tebal** dan *miring".to_string(),
        };

        let findings = check_markup_bugs(&seg);
        assert!(findings.iter().any(|f| f.check_code == "Q03a"));
        assert!(findings.iter().any(|f| f.check_code == "Q03b"));
    }

    #[test]
    fn test_q10_repeated_letters() {
        let seg = SegmentForQa {
            segment_id: "s1".to_string(),
            chapter_id: "c1".to_string(),
            idx: 1,
            src_text: "shout".to_string(),
            tgt_text: "Aaaaaakh!".to_string(),
        };

        let findings = check_repeated_letters(&seg);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].check_code, "Q10");
    }

    #[test]
    fn test_q23_duplicate_paragraphs() {
        let long_p = "Ini adalah paragraf yang sangat panjang sekali lebih dari enam puluh karakter untuk pengujian duplikat paragraf di QA.".to_string();
        let segments = vec![
            SegmentForQa {
                segment_id: "s1".to_string(),
                chapter_id: "c1".to_string(),
                idx: 1,
                src_text: "1".to_string(),
                tgt_text: long_p.clone(),
            },
            SegmentForQa {
                segment_id: "s2".to_string(),
                chapter_id: "c2".to_string(),
                idx: 2,
                src_text: "2".to_string(),
                tgt_text: long_p,
            },
        ];

        let findings = check_duplicate_paragraphs(&segments);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].check_code, "Q23");
        assert_eq!(findings[0].chapter_id, "c2");
        assert_eq!(findings[0].segment_id, "s2");
    }
}
