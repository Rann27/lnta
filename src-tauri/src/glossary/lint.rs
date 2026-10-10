//! Glossary lint rules per Story D2 and FR-GLO-07.
//!
//! Flags:
//! - Macron characters (with suggested digraph fix)
//! - Missing gender notes on Tier 1 entries
//! - Duplicate sources
//! - Near-variant names (edit distance <= 2 or long-vowel-only difference)

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::parse::{convert_macrons, infer_tier, GlossaryFile};

/// Kind of glossary lint issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GlossaryLintKind {
    Macron,
    MissingGender,
    DuplicateSource,
    NearVariantName,
}

/// A lint finding on a glossary entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossaryLintFinding {
    pub line_index: usize,
    pub source: String,
    pub translation: String,
    pub kind: GlossaryLintKind,
    pub message: String,
    pub suggested_fix: Option<String>,
}

/// Normalizes long vowel digraphs to single vowels for variant comparison.
/// e.g. "Satou" -> "sato", "Sato" -> "sato", "Yuuko" -> "yuko", "Yuko" -> "yuko".
pub fn normalize_long_vowels(name: &str) -> String {
    name.to_lowercase()
        .replace("ou", "o")
        .replace("uu", "u")
        .replace("aa", "a")
        .replace("ee", "e")
        .replace("ii", "i")
}

/// Checks if two names differ only by long vowels.
pub fn is_long_vowel_variant(name_a: &str, name_b: &str) -> bool {
    let lower_a = name_a.to_lowercase();
    let lower_b = name_b.to_lowercase();

    if lower_a == lower_b {
        return false;
    }

    normalize_long_vowels(&lower_a) == normalize_long_vowels(&lower_b)
}

/// Runs all glossary lint checks on a `GlossaryFile`.
pub fn lint_glossary(file: &GlossaryFile, gender_keywords: &[String]) -> Vec<GlossaryLintFinding> {
    let mut findings = Vec::new();
    let entries = file.entries();

    let mut source_counts: HashMap<String, Vec<usize>> = HashMap::new();

    // 1. Single entry checks: macrons, missing gender, and index sources
    for entry in &entries {
        source_counts
            .entry(entry.source.clone())
            .or_default()
            .push(entry.line_index);

        // Check 1: Macron
        let (fix_trans, mod_trans) = convert_macrons(&entry.translation);
        let (fix_note, mod_note) = convert_macrons(&entry.note);

        if mod_trans || mod_note {
            let suggested = if fix_note.is_empty() {
                fix_trans.clone()
            } else {
                format!("{fix_trans} === {fix_note}")
            };

            findings.push(GlossaryLintFinding {
                line_index: entry.line_index,
                source: entry.source.clone(),
                translation: entry.translation.clone(),
                kind: GlossaryLintKind::Macron,
                message: "Entri memuat karakter macron (ā, ī, ū, ē, ō)".to_string(),
                suggested_fix: Some(suggested),
            });
        }

        // Check 2: Missing gender on Tier 1
        if entry.tier == 1 && infer_tier(&entry.note, gender_keywords) != 1 {
            findings.push(GlossaryLintFinding {
                line_index: entry.line_index,
                source: entry.source.clone(),
                translation: entry.translation.clone(),
                kind: GlossaryLintKind::MissingGender,
                message: "Catatan gender kosong pada entri Tier 1".to_string(),
                suggested_fix: None,
            });
        }
    }

    // Check 3: Duplicate sources
    for (source, line_indices) in source_counts {
        if line_indices.len() > 1 {
            for &idx in &line_indices {
                let entry = entries.iter().find(|e| e.line_index == idx);
                let translation = entry.map_or("", |e| &e.translation).to_string();

                findings.push(GlossaryLintFinding {
                    line_index: idx,
                    source: source.clone(),
                    translation,
                    kind: GlossaryLintKind::DuplicateSource,
                    message: format!("Sumber '{source}' muncul lebih dari satu kali"),
                    suggested_fix: None,
                });
            }
        }
    }

    // Check 4: Near-variant names between distinct entries
    // Flags pairs where Levenshtein distance <= 2 or long-vowel-only difference
    for i in 0..entries.len() {
        for j in (i + 1)..entries.len() {
            let e1 = entries[i];
            let e2 = entries[j];

            if e1.source == e2.source {
                continue; // already handled by duplicate check
            }

            let t1 = &e1.translation;
            let t2 = &e2.translation;

            let lower_t1 = t1.to_lowercase();
            let lower_t2 = t2.to_lowercase();

            if lower_t1 == lower_t2 {
                continue;
            }

            let long_vowel_diff = is_long_vowel_variant(t1, t2);
            let edit_distance = strsim::levenshtein(&lower_t1, &lower_t2);

            if long_vowel_diff || edit_distance <= 2 {
                let reason = if long_vowel_diff {
                    format!("varian vokal panjang mirip dengan '{t2}'")
                } else {
                    format!("varian nama mirip dengan '{t2}' (jarak edit: {edit_distance})")
                };

                findings.push(GlossaryLintFinding {
                    line_index: e1.line_index,
                    source: e1.source.clone(),
                    translation: t1.clone(),
                    kind: GlossaryLintKind::NearVariantName,
                    message: reason,
                    suggested_fix: None,
                });
            }
        }
    }

    // Sort findings by line_index for deterministic output
    findings.sort_by_key(|f| f.line_index);
    findings
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::glossary::parse::parse_glossary;

    #[test]
    fn test_lint_flags_macron() {
        let content = "佐藤 === Satō === Tōkyō";
        let keywords = vec!["laki-laki".to_string()];
        let g = parse_glossary(content, &keywords);

        let findings = lint_glossary(&g, &keywords);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, GlossaryLintKind::Macron);
        assert_eq!(
            findings[0].suggested_fix,
            Some("Satou === Toukyou".to_string())
        );
    }

    #[test]
    fn test_lint_flags_missing_gender_tier_1() {
        // User marked tier 1 or inferred tier 1 without gender keyword
        let mut g = GlossaryFile::default();
        g.lines.push(crate::glossary::parse::GlossaryLine::Entry(
            crate::glossary::parse::GlossaryEntryItem {
                source: "佐藤".to_string(),
                translation: "Satou".to_string(),
                note: "teman sekelas".to_string(), // no gender keyword!
                tier: 1,                           // Tier 1
                line_index: 0,
                has_list_prefix: false,
                list_prefix_char: None,
            },
        ));

        let keywords = vec!["laki-laki".to_string(), "perempuan".to_string()];
        let findings = lint_glossary(&g, &keywords);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, GlossaryLintKind::MissingGender);
    }

    #[test]
    fn test_lint_flags_duplicates() {
        let content = "佐藤 === Satou\n佐藤 === Sato";
        let keywords = vec![];
        let g = parse_glossary(content, &keywords);

        let findings = lint_glossary(&g, &keywords);
        let dupes: Vec<_> = findings
            .iter()
            .filter(|f| f.kind == GlossaryLintKind::DuplicateSource)
            .collect();
        assert_eq!(dupes.len(), 2);
    }

    #[test]
    fn test_lint_flags_near_variant_names() {
        // Satou vs Sato (long vowel difference)
        // Misaki vs Misaka (edit distance 1)
        let content = "佐藤 === Satou\n砂糖 === Sato\n美咲 === Misaki\n美坂 === Misaka";
        let keywords = vec![];
        let g = parse_glossary(content, &keywords);

        let findings = lint_glossary(&g, &keywords);
        let variants: Vec<_> = findings
            .iter()
            .filter(|f| f.kind == GlossaryLintKind::NearVariantName)
            .collect();
        assert!(!variants.is_empty());
    }
}
