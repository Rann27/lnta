//! Mechanical QA fixers per PRD Appendix B "M" rows and FR-QA-03.
//!
//! Applies pure-core mechanical corrections:
//! - Q01a: 「...」 -> "..."
//! - Q01b: 『...』 inside quotes -> '...'
//! - Q01d: …… -> ...
//! - Q01e: ── or ─ -> — (em dash)
//! - Q01g: LaTeX quotes (`` and '') -> standard double quotes
//! - Q02: Macrons (ā, ī, ū, ē, ō) -> digraphs (aa, ii, uu, ee, ou)
//! - Q03a: **bold** -> stripped to plain text
//! - Q09: Name variants matching exactly one glossary entry
//! - Q11: Quote style drift normalized to volume majority (straight vs curly)
//! - Q17: Indonesian non-standard spellings (EXCLUDING glossary terms and manual edits)
//! - Q19: Space before punctuation, multiple spaces, and double hyphens
//! - Q19b: English inconsistent ellipsis normalized to volume majority
//! - Q20: Lowercase acronyms capitalization (sks, tv, cd, dvd, ktp, npwp, am/pm)

use strsim::levenshtein;

use crate::glossary::convert_macrons;
use crate::qa::ja_id::NON_STANDARD_SPELLINGS;

/// Record of an applied mechanical fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedMechanicalFix {
    pub check_code: String,
    pub before: String,
    pub after: String,
}

/// Options configuring mechanical fix execution.
#[derive(Debug, Clone, Default)]
pub struct MechanicalFixOptions<'a> {
    pub profile: &'a str, // "ja-id" or "ja-en"
    pub is_manual_edit: bool,
    pub glossary_terms: &'a [&'a str],
    pub majority_curly_quotes: Option<bool>,
    pub majority_ellipsis: Option<&'a str>,
}

/// Q01a, Q01b: Converts corner brackets 「」 and nested 『』.
pub fn fix_brackets(text: &str) -> (String, bool) {
    if !text.contains('「') && !text.contains('」') && !text.contains('『') && !text.contains('』')
    {
        return (text.to_string(), false);
    }

    let mut result = String::with_capacity(text.len());
    let mut in_quote = false;

    for c in text.chars() {
        match c {
            '「' => {
                result.push('"');
                in_quote = true;
            }
            '」' => {
                result.push('"');
                in_quote = false;
            }
            '『' => {
                if in_quote {
                    result.push('\'');
                } else {
                    result.push('"');
                    in_quote = true;
                }
            }
            '』' => {
                if in_quote {
                    result.push('\'');
                } else {
                    result.push('"');
                    in_quote = false;
                }
            }
            '"' | '“' | '”' => {
                in_quote = !in_quote;
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

/// Q01d: Japanese double ellipsis …… -> ...
pub fn fix_ellipsis(text: &str) -> (String, bool) {
    if text.contains("……") {
        let fixed = text.replace("……", "...");
        (fixed, true)
    } else {
        (text.to_string(), false)
    }
}

/// Q01e: Dramatic bars ── or ─ -> em dash —
pub fn fix_dramatic_bar(text: &str) -> (String, bool) {
    if text.contains('─') {
        let fixed = text.replace("──", "—").replace('─', "—");
        (fixed, true)
    } else {
        (text.to_string(), false)
    }
}

/// Q01g: LaTeX quotes (`` and '') -> double quotes "
pub fn fix_latex_quotes(text: &str) -> (String, bool) {
    if text.contains("``") || text.contains("''") {
        let fixed = text.replace("``", "\"").replace("''", "\"");
        (fixed, true)
    } else {
        (text.to_string(), false)
    }
}

/// Q02: Macrons -> digraphs.
pub fn fix_macrons(text: &str) -> (String, bool) {
    let (fixed, has_macrons) = convert_macrons(text);
    (fixed, has_macrons)
}

/// Q03a: Strips markdown bold **...**
pub fn fix_bold_markdown(text: &str) -> (String, bool) {
    if text.contains("**") {
        let fixed = text.replace("**", "");
        (fixed, true)
    } else {
        (text.to_string(), false)
    }
}

/// Q09: Name variants matching exactly one glossary entry.
pub fn fix_name_variants(text: &str, glossary_terms: &[&str]) -> (String, bool) {
    if glossary_terms.is_empty() {
        return (text.to_string(), false);
    }

    let Some(word_re) = crate::qa::re::cached(r"\b[A-Z][a-z]{2,}\b") else {
        return (text.to_string(), false);
    };

    let mut result = text.to_string();
    let mut modified = false;

    for mat in word_re.find_iter(text) {
        let word = mat.as_str();
        if glossary_terms.contains(&word) {
            continue;
        }

        // Find candidate glossary matches within edit distance 1 or 2
        let mut candidates = Vec::new();
        for &term in glossary_terms {
            if term.is_empty() {
                continue;
            }
            let dist = levenshtein(word, term);
            if dist > 0 && dist <= 2 {
                candidates.push(term);
            }
        }

        // Only fix if exactly one glossary term matches
        if candidates.len() == 1 {
            let target = candidates[0];
            let pattern = format!(r"\b{}\b", word);
            if let Some(target_re) = crate::qa::re::cached(&pattern) {
                result = target_re.replace_all(&result, target).to_string();
                modified = true;
            }
        }
    }

    (result, modified)
}

/// Q11: Quote style drift normalized to volume majority.
pub fn fix_quote_style_drift(text: &str, majority_curly: Option<bool>) -> (String, bool) {
    let Some(curly) = majority_curly else {
        return (text.to_string(), false);
    };

    if curly {
        // Normalize straight quotes to curly quotes
        let mut result = String::with_capacity(text.len());
        let mut opening = true;

        for c in text.chars() {
            match c {
                '"' => {
                    if opening {
                        result.push('“');
                    } else {
                        result.push('”');
                    }
                    opening = !opening;
                }
                '\'' => {
                    result.push('’');
                }
                _ => {
                    result.push(c);
                }
            }
        }

        let changed = result != text;
        (result, changed)
    } else {
        // Normalize curly quotes to straight quotes
        if text.contains('“') || text.contains('”') || text.contains('‘') || text.contains('’')
        {
            let fixed = text.replace(['“', '”'], "\"").replace(['‘', '’'], "'");
            (fixed, true)
        } else {
            (text.to_string(), false)
        }
    }
}

/// Q17: Non-standard Indonesian spellings.
/// Excludes glossary terms and manually edited segments.
pub fn fix_non_standard_spelling(
    text: &str,
    is_manual_edit: bool,
    glossary_terms: &[&str],
) -> (String, bool) {
    if is_manual_edit {
        return (text.to_string(), false);
    }

    let mut result = text.to_string();
    let mut modified = false;

    for &(standard, non_standard) in NON_STANDARD_SPELLINGS {
        // Exclude if the non-standard term appears in glossary
        if glossary_terms
            .iter()
            .any(|&g| g.eq_ignore_ascii_case(non_standard))
        {
            continue;
        }

        let pattern = format!(r"(?i)\b{}\b", non_standard);
        if let Some(re) = crate::qa::re::cached(&pattern) {
            if re.is_match(&result) {
                result = re
                    .replace_all(&result, |caps: &regex::Captures| {
                        let matched = caps.get(0).map_or("", |m| m.as_str());
                        // Preserve Titlecase if matched starts with uppercase
                        if matched.chars().next().is_some_and(|c| c.is_uppercase()) {
                            let mut chars = standard.chars();
                            match chars.next() {
                                Some(first) => {
                                    first.to_uppercase().collect::<String>() + chars.as_str()
                                }
                                None => standard.to_string(),
                            }
                        } else {
                            standard.to_string()
                        }
                    })
                    .to_string();
                modified = true;
            }
        }
    }

    (result, modified)
}

/// Q19: Punctuation spacing (space before punctuation, multiple spaces, `--` to em dash).
pub fn fix_punctuation_spacing(text: &str) -> (String, bool) {
    let mut result = text.to_string();
    let mut modified = false;

    // Double hyphen -- to em dash —
    if result.contains("--") {
        result = result.replace("--", "—");
        modified = true;
    }

    // Space before punctuation: e.g. " ," -> ","
    if let Some(punct_re) = crate::qa::re::cached(r"\s+([,.;:!?])") {
        if punct_re.is_match(&result) {
            result = punct_re.replace_all(&result, "$1").to_string();
            modified = true;
        }
    }

    // Double/multiple spaces -> single space
    if let Some(space_re) = crate::qa::re::cached(r" {2,}") {
        if space_re.is_match(&result) {
            result = space_re.replace_all(&result, " ").to_string();
            modified = true;
        }
    }

    (result, modified)
}

/// Q19b: English inconsistent ellipsis normalized to volume majority.
pub fn fix_inconsistent_ellipsis(text: &str, majority_ellipsis: Option<&str>) -> (String, bool) {
    let Some(target) = majority_ellipsis else {
        return (text.to_string(), false);
    };

    let mut result = text.to_string();
    let mut modified = false;

    if target == "..." {
        if result.contains(". . .") {
            result = result.replace(". . .", "...");
            modified = true;
        }
        if result.contains('…') {
            result = result.replace('…', "...");
            modified = true;
        }
    } else if target == "…" {
        if result.contains(". . .") {
            result = result.replace(". . .", "…");
            modified = true;
        }
        if result.contains("...") {
            result = result.replace("...", "…");
            modified = true;
        }
    }

    (result, modified)
}

/// Q20: Lowercase acronyms.
pub fn fix_lowercase_acronyms(text: &str, profile: &str) -> (String, bool) {
    let mut result = text.to_string();
    let mut modified = false;

    if profile == "ja-id" {
        if let Some(re) = crate::qa::re::cached(r"\b(sks|tv|cd|dvd|ktp|npwp)\b") {
            if re.is_match(&result) {
                result = re
                    .replace_all(&result, |caps: &regex::Captures| {
                        caps.get(0).map_or("", |m| m.as_str()).to_uppercase()
                    })
                    .to_string();
                modified = true;
            }
        }
    } else if profile == "ja-en" {
        // tv, dvd
        if let Some(re_media) = crate::qa::re::cached(r"\b(tv|dvd)\b") {
            if re_media.is_match(&result) {
                result = re_media
                    .replace_all(&result, |caps: &regex::Captures| {
                        caps.get(0).map_or("", |m| m.as_str()).to_uppercase()
                    })
                    .to_string();
                modified = true;
            }
        }
        // am/pm after number e.g. "8 am" -> "8 AM"
        if let Some(re_time) = crate::qa::re::cached(r"\b(\d+)\s*(am|pm)\b") {
            if re_time.is_match(&result) {
                result = re_time
                    .replace_all(&result, |caps: &regex::Captures| {
                        let num = caps.get(1).map_or("", |m| m.as_str());
                        let unit = caps.get(2).map_or("", |m| m.as_str()).to_uppercase();
                        format!("{} {}", num, unit)
                    })
                    .to_string();
                modified = true;
            }
        }
    }

    (result, modified)
}

/// Applies all mechanical fixes per Appendix B "M" rows to a single text segment.
///
/// Returns the updated text and a list of applied fixes.
pub fn apply_mechanical_fixes_to_text(
    text: &str,
    opts: &MechanicalFixOptions,
) -> (String, Vec<AppliedMechanicalFix>) {
    let mut current = text.to_string();
    let mut fixes = Vec::new();

    // 1. Q01a, Q01b: Brackets
    let (after_brackets, changed) = fix_brackets(&current);
    if changed {
        fixes.push(AppliedMechanicalFix {
            check_code: "Q01a".to_string(),
            before: current.clone(),
            after: after_brackets.clone(),
        });
        current = after_brackets;
    }

    // 2. Q01d: Ellipsis
    let (after_ellipsis, changed) = fix_ellipsis(&current);
    if changed {
        fixes.push(AppliedMechanicalFix {
            check_code: "Q01d".to_string(),
            before: current.clone(),
            after: after_ellipsis.clone(),
        });
        current = after_ellipsis;
    }

    // 3. Q01e: Dramatic bar
    let (after_bar, changed) = fix_dramatic_bar(&current);
    if changed {
        fixes.push(AppliedMechanicalFix {
            check_code: "Q01e".to_string(),
            before: current.clone(),
            after: after_bar.clone(),
        });
        current = after_bar;
    }

    // 4. Q01g: LaTeX quotes
    let (after_latex, changed) = fix_latex_quotes(&current);
    if changed {
        fixes.push(AppliedMechanicalFix {
            check_code: "Q01g".to_string(),
            before: current.clone(),
            after: after_latex.clone(),
        });
        current = after_latex;
    }

    // 5. Q02: Macrons
    let (after_macrons, changed) = fix_macrons(&current);
    if changed {
        fixes.push(AppliedMechanicalFix {
            check_code: "Q02".to_string(),
            before: current.clone(),
            after: after_macrons.clone(),
        });
        current = after_macrons;
    }

    // 6. Q03a: Bold markdown
    let (after_bold, changed) = fix_bold_markdown(&current);
    if changed {
        fixes.push(AppliedMechanicalFix {
            check_code: "Q03a".to_string(),
            before: current.clone(),
            after: after_bold.clone(),
        });
        current = after_bold;
    }

    // 7. Q09: Name variants
    let (after_names, changed) = fix_name_variants(&current, opts.glossary_terms);
    if changed {
        fixes.push(AppliedMechanicalFix {
            check_code: "Q09".to_string(),
            before: current.clone(),
            after: after_names.clone(),
        });
        current = after_names;
    }

    // 8. Q11: Quote style drift
    let (after_style, changed) = fix_quote_style_drift(&current, opts.majority_curly_quotes);
    if changed {
        fixes.push(AppliedMechanicalFix {
            check_code: "Q11".to_string(),
            before: current.clone(),
            after: after_style.clone(),
        });
        current = after_style;
    }

    // 9. Q17: Indonesian non-standard spellings
    if opts.profile == "ja-id" {
        let (after_spelling, changed) =
            fix_non_standard_spelling(&current, opts.is_manual_edit, opts.glossary_terms);
        if changed {
            fixes.push(AppliedMechanicalFix {
                check_code: "Q17".to_string(),
                before: current.clone(),
                after: after_spelling.clone(),
            });
            current = after_spelling;
        }
    }

    // 10. Q19: Punctuation spacing and double hyphens
    let (after_punct, changed) = fix_punctuation_spacing(&current);
    if changed {
        fixes.push(AppliedMechanicalFix {
            check_code: "Q19".to_string(),
            before: current.clone(),
            after: after_punct.clone(),
        });
        current = after_punct;
    }

    // 11. Q19b: English inconsistent ellipsis
    if opts.profile == "ja-en" {
        let (after_en_ellipsis, changed) =
            fix_inconsistent_ellipsis(&current, opts.majority_ellipsis);
        if changed {
            fixes.push(AppliedMechanicalFix {
                check_code: "Q19b".to_string(),
                before: current.clone(),
                after: after_en_ellipsis.clone(),
            });
            current = after_en_ellipsis;
        }
    }

    // 12. Q20: Lowercase acronyms
    let (after_acronyms, changed) = fix_lowercase_acronyms(&current, opts.profile);
    if changed {
        fixes.push(AppliedMechanicalFix {
            check_code: "Q20".to_string(),
            before: current.clone(),
            after: after_acronyms.clone(),
        });
        current = after_acronyms;
    }

    (current, fixes)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_brackets_and_ellipsis() {
        let text = "「Halo, 『Kenta』!」 dia berkata……";
        let (fixed, changed) = fix_brackets(text);
        assert!(changed);
        assert_eq!(fixed, "\"Halo, 'Kenta'!\" dia berkata……");

        let (fixed2, changed2) = fix_ellipsis(&fixed);
        assert!(changed2);
        assert_eq!(fixed2, "\"Halo, 'Kenta'!\" dia berkata...");
    }

    #[test]
    fn test_fix_dramatic_bar_and_latex_quotes() {
        let text = "``Tunggu──sebentar!''";
        let (fixed1, _) = fix_latex_quotes(text);
        assert_eq!(fixed1, "\"Tunggu──sebentar!\"");

        let (fixed2, _) = fix_dramatic_bar(&fixed1);
        assert_eq!(fixed2, "\"Tunggu—sebentar!\"");
    }

    #[test]
    fn test_fix_macrons_and_bold() {
        let text = "**Tōkyō** sangat ramai.";
        let (fixed1, _) = fix_bold_markdown(text);
        assert_eq!(fixed1, "Tōkyō sangat ramai.");

        let (fixed2, _) = fix_macrons(&fixed1);
        assert_eq!(fixed2, "Toukyou sangat ramai.");
    }

    #[test]
    fn test_fix_name_variants_against_glossary() {
        let glossary = vec!["Kenta"];
        let text = "Keenta berjalan bersama Kenta.";
        let (fixed, changed) = fix_name_variants(text, &glossary);
        assert!(changed);
        assert_eq!(fixed, "Kenta berjalan bersama Kenta.");
    }

    #[test]
    fn test_fix_non_standard_spelling_with_exclusions() {
        // Standard replacement
        let text = "Ada resiko besar saat antri.";
        let (fixed, changed) = fix_non_standard_spelling(text, false, &[]);
        assert!(changed);
        assert_eq!(fixed, "Ada risiko besar saat antre.");

        // Excluded because of manual edit
        let (manual_fixed, manual_changed) = fix_non_standard_spelling(text, true, &[]);
        assert!(!manual_changed);
        assert_eq!(manual_fixed, text);

        // Excluded because term is in glossary
        let (glossary_fixed, _) = fix_non_standard_spelling(text, false, &["resiko"]);
        assert_eq!(glossary_fixed, "Ada resiko besar saat antre.");
    }

    #[test]
    fn test_fix_punctuation_spacing_and_acronyms() {
        let text_id = "Dia menonton tv di rumah , setelah ujian sks .";
        let (fixed_punct, _) = fix_punctuation_spacing(text_id);
        assert_eq!(fixed_punct, "Dia menonton tv di rumah, setelah ujian sks.");

        let (fixed_acronyms, _) = fix_lowercase_acronyms(&fixed_punct, "ja-id");
        assert_eq!(
            fixed_acronyms,
            "Dia menonton TV di rumah, setelah ujian SKS."
        );

        let text_en = "The alarm rang at 8 am while watching tv.";
        let (fixed_en, _) = fix_lowercase_acronyms(text_en, "ja-en");
        assert_eq!(fixed_en, "The alarm rang at 8 AM while watching TV.");
    }

    #[test]
    fn test_fix_inconsistent_ellipsis_en() {
        let text = "Wait… really? . . . I knew it.";
        let (fixed, changed) = fix_inconsistent_ellipsis(text, Some("..."));
        assert!(changed);
        assert_eq!(fixed, "Wait... really? ... I knew it.");
    }
}
