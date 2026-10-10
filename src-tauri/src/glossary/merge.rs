//! Pure core glossary comparison and merging per Story D3 and AGENTS.md §8.
//!
//! Provides deterministic comparison between a target project's glossary
//! and an imported source glossary, identifying:
//! - Added entries (new source keys)
//! - Identical entries (same source, translation, and normalized note)
//! - Conflicting entries (differing translation or note)
//!
//! Also provides pure merging of resolved entries into a target `GlossaryFile`.

use crate::glossary::parse::{infer_tier, GlossaryEntryItem, GlossaryFile, GlossaryLine};

/// Represents a conflict between target glossary and incoming imported glossary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlossaryConflict {
    pub source: String,
    pub current_translation: String,
    pub current_note: String,
    pub incoming_translation: String,
    pub incoming_note: String,
}

/// Result of comparing incoming glossary against target glossary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlossaryComparison {
    pub added: Vec<GlossaryEntryItem>,
    pub identical: Vec<GlossaryEntryItem>,
    pub conflicts: Vec<GlossaryConflict>,
}

/// Compares `incoming` against `target` and partitions incoming entries.
pub fn compare_glossaries(target: &GlossaryFile, incoming: &GlossaryFile) -> GlossaryComparison {
    let mut added = Vec::new();
    let mut identical = Vec::new();
    let mut conflicts = Vec::new();

    for inc in incoming.entries() {
        if let Some(cur) = target.entries().iter().find(|e| e.source == inc.source) {
            let cur_trans = cur.translation.trim();
            let inc_trans = inc.translation.trim();
            let cur_note = cur.note.trim();
            let inc_note = inc.note.trim();

            if cur_trans == inc_trans && cur_note == inc_note {
                identical.push(inc.clone());
            } else {
                conflicts.push(GlossaryConflict {
                    source: inc.source.clone(),
                    current_translation: cur_trans.to_string(),
                    current_note: cur_note.to_string(),
                    incoming_translation: inc_trans.to_string(),
                    incoming_note: inc_note.to_string(),
                });
            }
        } else {
            added.push(inc.clone());
        }
    }

    GlossaryComparison {
        added,
        identical,
        conflicts,
    }
}

/// Merges a list of resolved entries (both newly added and resolved conflicts)
/// into `target`, updating existing lines or appending new ones.
pub fn apply_glossary_merge(
    target: &mut GlossaryFile,
    resolved: &[GlossaryEntryItem],
    gender_keywords: &[String],
) {
    for res in resolved {
        let mut found = false;
        for line in &mut target.lines {
            if let GlossaryLine::Entry(ref mut item) = line {
                if item.source == res.source {
                    item.translation = res.translation.clone();
                    item.note = res.note.clone();
                    item.tier = infer_tier(&res.note, gender_keywords);
                    found = true;
                    break;
                }
            }
        }

        if !found {
            let note = res.note.clone();
            let tier = infer_tier(&note, gender_keywords);
            let new_item = GlossaryEntryItem {
                source: res.source.clone(),
                translation: res.translation.clone(),
                note,
                tier,
                line_index: target.lines.len(),
                has_list_prefix: res.has_list_prefix,
                list_prefix_char: res.list_prefix_char,
            };
            target.lines.push(GlossaryLine::Entry(new_item));
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::glossary::parse::parse_glossary;

    #[test]
    fn test_compare_and_merge_glossaries() {
        let keywords = vec!["laki-laki".to_string(), "perempuan".to_string()];

        let target_raw = r#"
佐藤 === Satou === laki-laki
美咲 === Misaki === perempuan; saudari
鈴木 === Suzuki
"#;

        let incoming_raw = r#"
佐藤 === Satou === laki-laki
美咲 === Misaki === saudara perempuan
田中 === Tanaka === teman sekelas
"#;

        let target_file = parse_glossary(target_raw, &keywords);
        let incoming_file = parse_glossary(incoming_raw, &keywords);

        let comparison = compare_glossaries(&target_file, &incoming_file);

        // Satou is identical
        assert_eq!(comparison.identical.len(), 1);
        assert_eq!(comparison.identical[0].source, "佐藤");

        // Tanaka is newly added
        assert_eq!(comparison.added.len(), 1);
        assert_eq!(comparison.added[0].source, "田中");

        // Misaki has conflicting note
        assert_eq!(comparison.conflicts.len(), 1);
        assert_eq!(comparison.conflicts[0].source, "美咲");
        assert_eq!(comparison.conflicts[0].current_note, "perempuan; saudari");
        assert_eq!(comparison.conflicts[0].incoming_note, "saudara perempuan");

        // Apply merge with Tanaka added and Misaki resolved to incoming
        let mut target_to_merge = target_file.clone();
        let resolved = vec![
            comparison.added[0].clone(),
            GlossaryEntryItem {
                source: "美咲".to_string(),
                translation: "Misaki".to_string(),
                note: "saudara perempuan".to_string(),
                tier: 2,
                line_index: 0,
                has_list_prefix: false,
                list_prefix_char: None,
            },
        ];

        apply_glossary_merge(&mut target_to_merge, &resolved, &keywords);

        assert_eq!(target_to_merge.entries().len(), 4);
        let entries = target_to_merge.entries();
        let misaki = entries.iter().find(|e| e.source == "美咲").unwrap();
        assert_eq!(misaki.note, "saudara perempuan");

        let tanaka = entries.iter().find(|e| e.source == "田中").unwrap();
        assert_eq!(tanaka.translation, "Tanaka");
    }
}
