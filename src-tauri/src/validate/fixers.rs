//! Mechanical text fixers per FR-VAL-04, 05, 06, 08 and PRD Appendix B.

use serde::{Deserialize, Serialize};

use crate::glossary::convert_macrons;
use crate::glossary::GlossaryEntryItem;

/// Record of a mechanical fix applied to a segment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanicalFixReport {
    pub kind: String,
    pub before: String,
    pub after: String,
}

/// Applies mechanical fixes to bracket quotes:
/// - 「...」 becomes double quotes `"..."`
/// - nested 『...』 inside quotes becomes single quotes `'...'`
/// - standalone 『...』 becomes double quotes `"..."`
pub fn fix_brackets(text: &str) -> (String, bool) {
    if !text.contains('「') && !text.contains('」') && !text.contains('『') && !text.contains('』')
    {
        return (text.to_string(), false);
    }

    let mut result = String::with_capacity(text.len());
    let mut in_outer_quote = false;

    for c in text.chars() {
        match c {
            '「' => {
                result.push('"');
                in_outer_quote = true;
            }
            '」' => {
                result.push('"');
                in_outer_quote = false;
            }
            '『' => {
                if in_outer_quote {
                    result.push('\'');
                } else {
                    result.push('"');
                    in_outer_quote = true;
                }
            }
            '』' => {
                if in_outer_quote {
                    result.push('\'');
                } else {
                    result.push('"');
                    in_outer_quote = false;
                }
            }
            '"' => {
                in_outer_quote = !in_outer_quote;
                result.push(c);
            }
            _ => {
                result.push(c);
            }
        }
    }

    let changed = result != text;
    (result, changed)
}

/// Applies macron conversion to digraphs per FR-VAL-05 and FR-GLO-06.
/// If a token matches a glossary entry translation variant, the glossary form is used.
pub fn fix_macrons(text: &str, glossary_entries: &[&GlossaryEntryItem]) -> (String, bool) {
    let mut working = text.to_string();
    let mut modified = false;

    // Check if any glossary entry matches a macron token
    for entry in glossary_entries {
        if !entry.translation.is_empty() {
            let (converted_trans, has_macron) = convert_macrons(&entry.translation);
            if has_macron && working.contains(&entry.translation) {
                working = working.replace(&entry.translation, &converted_trans);
                modified = true;
            }
        }
    }

    let (digraph_text, has_macrons) = convert_macrons(&working);
    if has_macrons {
        working = digraph_text;
        modified = true;
    }

    (working, modified)
}

/// Strips markdown bold markup (`**...**`) while preserving the enclosed text per FR-VAL-06.
pub fn fix_bold_markdown(text: &str) -> (String, bool) {
    if !text.contains("**") {
        return (text.to_string(), false);
    }

    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '*' && chars.peek() == Some(&'*') {
            chars.next(); // Consume second asterisk
        } else {
            result.push(c);
        }
    }

    let changed = result != text;
    (result, changed)
}

/// Checks if a paragraph consists strictly of scene markers and whitespace.
fn is_scene_marker_only_paragraph(para: &str, scene_markers: &[char]) -> bool {
    let trimmed = para.trim();
    if trimmed.is_empty() {
        return false;
    }

    // Common ASCII text markers like "***", "* * *", "---"
    let ascii_stripped = trimmed.replace(['*', ' ', '-', '―', '—'], "");
    if ascii_stripped.is_empty() {
        return true;
    }

    // Check if every character is in scene_markers
    trimmed
        .chars()
        .all(|c| c.is_whitespace() || scene_markers.contains(&c))
}

/// Removes paragraphs within a multi-paragraph segment that consist only of scene markers per FR-VAL-08.
pub fn fix_scene_marker_paragraphs(text: &str, scene_markers: &[char]) -> (String, bool) {
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    if paragraphs.len() <= 1 {
        // If single paragraph, check if it's marker-only
        if is_scene_marker_only_paragraph(text, scene_markers) {
            return (String::new(), true);
        }
        return (text.to_string(), false);
    }

    let mut retained = Vec::new();
    let mut modified = false;

    for para in paragraphs {
        if is_scene_marker_only_paragraph(para, scene_markers) {
            modified = true;
        } else {
            retained.push(para.trim());
        }
    }

    (retained.join("\n\n"), modified)
}

/// Applies all mechanical fixes in sequence and returns the final text and report list.
pub fn apply_mechanical_fixes(
    text: &str,
    glossary_entries: &[&GlossaryEntryItem],
    scene_markers: &[char],
) -> (String, Vec<MechanicalFixReport>) {
    let mut reports = Vec::new();
    let mut current = text.to_string();

    // 1. Bracket quotes (「」 and 『』)
    let (fixed_brackets, brackets_changed) = fix_brackets(&current);
    if brackets_changed {
        reports.push(MechanicalFixReport {
            kind: "brackets".to_string(),
            before: current,
            after: fixed_brackets.clone(),
        });
        current = fixed_brackets;
    }

    // 2. Bold markdown (**...**)
    let (fixed_bold, bold_changed) = fix_bold_markdown(&current);
    if bold_changed {
        reports.push(MechanicalFixReport {
            kind: "bold_markdown".to_string(),
            before: current,
            after: fixed_bold.clone(),
        });
        current = fixed_bold;
    }

    // 3. Macrons (ā, ī, etc.)
    let (fixed_macrons, macrons_changed) = fix_macrons(&current, glossary_entries);
    if macrons_changed {
        reports.push(MechanicalFixReport {
            kind: "macron".to_string(),
            before: current,
            after: fixed_macrons.clone(),
        });
        current = fixed_macrons;
    }

    // 4. Marker-only paragraphs
    let (fixed_markers, markers_changed) = fix_scene_marker_paragraphs(&current, scene_markers);
    if markers_changed {
        reports.push(MechanicalFixReport {
            kind: "scene_marker_paragraph".to_string(),
            before: current,
            after: fixed_markers.clone(),
        });
        current = fixed_markers;
    }

    (current, reports)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_brackets_positive_and_negative() {
        // Positive: 「」 converted to double quotes, nested 『』 converted to single quotes
        let input = "「Dia berkata, 『Ayo pergi』 sekarang.」";
        let (output, changed) = fix_brackets(input);
        assert!(changed);
        assert_eq!(output, "\"Dia berkata, 'Ayo pergi' sekarang.\"");

        // Negative: standard quotes untouched
        let regular = "\"Dia berkata, 'Ayo pergi' sekarang.\"";
        let (regular_out, changed_reg) = fix_brackets(regular);
        assert!(!changed_reg);
        assert_eq!(regular_out, regular);
    }

    #[test]
    fn test_fix_bold_markdown_positive_and_negative() {
        // Positive: **bold** becomes regular text
        let input = "Ini teks **sangat penting** dalam cerita.";
        let (output, changed) = fix_bold_markdown(input);
        assert!(changed);
        assert_eq!(output, "Ini teks sangat penting dalam cerita.");

        // Negative: single asterisk *italic* untouched
        let italic = "Ini teks *miring* yang valid.";
        let (italic_out, changed_italic) = fix_bold_markdown(italic);
        assert!(!changed_italic);
        assert_eq!(italic_out, italic);
    }

    #[test]
    fn test_fix_macrons_positive_and_negative() {
        // Positive: macrons converted to digraphs
        let input = "Satō pergi bersama Yūko ke Tōkyō.";
        let (output, changed) = fix_macrons(input, &[]);
        assert!(changed);
        assert_eq!(output, "Satou pergi bersama Yuuko ke Toukyou.");

        // Negative: already digraphs
        let clean = "Satou pergi bersama Yuuko.";
        let (clean_out, changed_clean) = fix_macrons(clean, &[]);
        assert!(!changed_clean);
        assert_eq!(clean_out, clean);
    }

    #[test]
    fn test_fix_scene_marker_paragraphs_positive_and_negative() {
        let markers = ['☆', '◆', '◇', '★'];

        // Positive: paragraph with only scene markers removed
        let input = "Paragraf pembuka.\n\n☆\n\nParagraf penutup.";
        let (output, changed) = fix_scene_marker_paragraphs(input, &markers);
        assert!(changed);
        assert_eq!(output, "Paragraf pembuka.\n\nParagraf penutup.");

        // Positive: *** marker removed
        let input_stars = "Paragraf satu.\n\n***\n\nParagraf dua.";
        let (out_stars, changed_stars) = fix_scene_marker_paragraphs(input_stars, &markers);
        assert!(changed_stars);
        assert_eq!(out_stars, "Paragraf satu.\n\nParagraf dua.");

        // Negative: normal paragraphs without markers untouched
        let normal = "Paragraf satu.\n\nParagraf dua.";
        let (normal_out, changed_normal) = fix_scene_marker_paragraphs(normal, &markers);
        assert!(!changed_normal);
        assert_eq!(normal_out, normal);
    }
}
