//! Glossary parsing per FR-GLO-01, FR-GLO-05, and FR-GLO-06.

use serde::{Deserialize, Serialize};

/// A single glossary entry parsed from glossary.md.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossaryEntryItem {
    pub source: String,
    pub translation: String,
    pub note: String,
    pub tier: i32,
    pub line_index: usize,
    pub has_list_prefix: bool,
    pub list_prefix_char: Option<char>,
}

/// A line in glossary.md (either a structured entry or preserved non-entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlossaryLine {
    Entry(GlossaryEntryItem),
    NonEntry(String),
}

/// In-memory representation of glossary.md preserving line ordering and comments.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GlossaryFile {
    pub lines: Vec<GlossaryLine>,
}

impl GlossaryFile {
    /// Returns only the active glossary entries.
    pub fn entries(&self) -> Vec<&GlossaryEntryItem> {
        self.lines
            .iter()
            .filter_map(|line| match line {
                GlossaryLine::Entry(entry) => Some(entry),
                GlossaryLine::NonEntry(_) => None,
            })
            .collect()
    }

    /// Returns mutable references to the active glossary entries.
    pub fn entries_mut(&mut self) -> Vec<&mut GlossaryEntryItem> {
        self.lines
            .iter_mut()
            .filter_map(|line| match line {
                GlossaryLine::Entry(entry) => Some(entry),
                GlossaryLine::NonEntry(_) => None,
            })
            .collect()
    }
}

/// Converts Japanese Hepburn macrons to digraphs per FR-GLO-06 and FR-VAL-05.
/// ā->aa, ī->ii, ū->uu, ē->ee, ō->ou (and their uppercase equivalents).
pub fn convert_macrons(text: &str) -> (String, bool) {
    let mut result = String::with_capacity(text.len());
    let mut modified = false;

    for c in text.chars() {
        match c {
            'ā' => {
                result.push_str("aa");
                modified = true;
            }
            'Ā' => {
                result.push_str("Aa");
                modified = true;
            }
            'ī' => {
                result.push_str("ii");
                modified = true;
            }
            'Ī' => {
                result.push_str("Ii");
                modified = true;
            }
            'ū' => {
                result.push_str("uu");
                modified = true;
            }
            'Ū' => {
                result.push_str("Uu");
                modified = true;
            }
            'ē' => {
                result.push_str("ee");
                modified = true;
            }
            'Ē' => {
                result.push_str("Ee");
                modified = true;
            }
            'ō' => {
                result.push_str("ou");
                modified = true;
            }
            'Ō' => {
                result.push_str("Ou");
                modified = true;
            }
            other => result.push(other),
        }
    }

    (result, modified)
}

/// Infers the entry tier (1 or 2) from the note field using profile gender keywords.
pub fn infer_tier(note: &str, gender_keywords: &[String]) -> i32 {
    let lower_note = note.to_lowercase();
    for kw in gender_keywords {
        let kw_lower = kw.to_lowercase();
        // Check word boundary or substring match
        if lower_note.contains(&kw_lower) {
            return 1;
        }
    }
    2
}

/// Parses the contents of `glossary.md` into a `GlossaryFile`.
///
/// Tolerant of:
/// - 3 fields: `Source === Translation === Note`
/// - 2 fields: `Source === Translation`
/// - Markdown list prefixes: `- ` or `* `
/// - Preserves empty lines, comments, headings in original positions
pub fn parse_glossary(content: &str, gender_keywords: &[String]) -> GlossaryFile {
    if content.trim().is_empty() {
        return GlossaryFile { lines: Vec::new() };
    }

    let mut lines = Vec::new();

    for (line_index, raw_line) in content.split('\n').enumerate() {
        let trimmed = raw_line.trim();

        if trimmed.is_empty() {
            lines.push(GlossaryLine::NonEntry(raw_line.to_string()));
            continue;
        }

        // Check if the line has the `===` separator
        if !trimmed.contains("===") {
            lines.push(GlossaryLine::NonEntry(raw_line.to_string()));
            continue;
        }

        // Check for list prefixes `- ` or `* `
        let mut entry_line = trimmed;
        let mut has_list_prefix = false;
        let mut list_prefix_char = None;

        if let Some(rest) = entry_line.strip_prefix("- ") {
            has_list_prefix = true;
            list_prefix_char = Some('-');
            entry_line = rest.trim();
        } else if let Some(rest) = entry_line.strip_prefix("* ") {
            has_list_prefix = true;
            list_prefix_char = Some('*');
            entry_line = rest.trim();
        }

        let parts: Vec<&str> = entry_line.split("===").map(|p| p.trim()).collect();

        if parts.len() >= 2 {
            let source = parts[0].to_string();
            let translation = parts[1].to_string();
            let note = if parts.len() >= 3 {
                parts[2..].join(" === ")
            } else {
                String::new()
            };

            let tier = infer_tier(&note, gender_keywords);

            lines.push(GlossaryLine::Entry(GlossaryEntryItem {
                source,
                translation,
                note,
                tier,
                line_index,
                has_list_prefix,
                list_prefix_char,
            }));
        } else {
            lines.push(GlossaryLine::NonEntry(raw_line.to_string()));
        }
    }

    GlossaryFile { lines }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_or_whitespace_file_yields_zero_entries() {
        let keywords = vec!["laki-laki".to_string(), "perempuan".to_string()];

        let g1 = parse_glossary("", &keywords);
        assert_eq!(g1.entries().len(), 0);

        let g2 = parse_glossary("   \n\n\t  \n", &keywords);
        assert_eq!(g2.entries().len(), 0);
    }

    #[test]
    fn test_parse_2_and_3_field_lines_and_list_prefixes() {
        let content = r#"# Characters

- 佐藤 === Satou === laki-laki; teman sekelas
* 鈴木 === Suzuki
田中 === Tanaka === guru olahraga

Catatan tambahan.
"#;
        let keywords = vec!["laki-laki".to_string(), "perempuan".to_string()];
        let g = parse_glossary(content, &keywords);

        let entries = g.entries();
        assert_eq!(entries.len(), 3);

        // Entry 1: 3 fields, list prefix '-'
        assert_eq!(entries[0].source, "佐藤");
        assert_eq!(entries[0].translation, "Satou");
        assert_eq!(entries[0].note, "laki-laki; teman sekelas");
        assert_eq!(entries[0].tier, 1);
        assert!(entries[0].has_list_prefix);
        assert_eq!(entries[0].list_prefix_char, Some('-'));

        // Entry 2: 2 fields, list prefix '*'
        assert_eq!(entries[1].source, "鈴木");
        assert_eq!(entries[1].translation, "Suzuki");
        assert_eq!(entries[1].note, "");
        assert_eq!(entries[1].tier, 2);
        assert!(entries[1].has_list_prefix);
        assert_eq!(entries[1].list_prefix_char, Some('*'));

        // Entry 3: 3 fields, no prefix
        assert_eq!(entries[2].source, "田中");
        assert_eq!(entries[2].translation, "Tanaka");
        assert_eq!(entries[2].note, "guru olahraga");
        assert_eq!(entries[2].tier, 2);
        assert!(!entries[2].has_list_prefix);
    }

    #[test]
    fn test_convert_macrons() {
        let (res, modif) = convert_macrons("Tōkyō Satō Yūko");
        assert_eq!(res, "Toukyou Satou Yuuko");
        assert!(modif);

        let (res2, modif2) = convert_macrons("Tanaka Taro");
        assert_eq!(res2, "Tanaka Taro");
        assert!(!modif2);
    }
}
