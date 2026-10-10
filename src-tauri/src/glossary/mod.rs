//! LNTA glossary management, parsing, linting, and synchronization.
//!
//! Implements FR-GLO-01..08, Story D1, and Story D2:
//! - Tolerant glossary.md parser (2 and 3 columns, list prefixes `- ` / `* `)
//! - Non-entry line and order preservation during rewrites
//! - In-place macron to digraph fixes with rolling timestamped backups
//! - Automatic Tier 1 detection via profile gender keywords
//! - Comprehensive linting (macron, missing gender, duplicates, near-variants with Levenshtein <= 2 or long-vowels)

pub mod lint;
pub mod parse;
pub mod write;

pub use lint::{
    is_long_vowel_variant, lint_glossary, normalize_long_vowels, GlossaryLintFinding,
    GlossaryLintKind,
};
pub use parse::{
    convert_macrons, infer_tier, parse_glossary, GlossaryEntryItem, GlossaryFile, GlossaryLine,
};
pub use write::{
    backup_glossary_file, fix_macrons_in_place, serialize_glossary, write_glossary_file,
};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_glossary_roundtrip_and_full_workflow() {
        let content = r#"# Volume Characters

- 佐藤 === Satō === laki-laki; protagonis
* 鈴木 === Suzuki
田中 === Tanaka === teman sekolah

// Komentar pelindung
美咲 === Misaki === perempuan; saudari
"#;

        let keywords = vec!["laki-laki".to_string(), "perempuan".to_string()];
        let mut file = parse_glossary(content, &keywords);

        // 1. Initial entries count
        assert_eq!(file.entries().len(), 4);
        assert_eq!(file.entries()[0].tier, 1); // laki-laki
        assert_eq!(file.entries()[1].tier, 2); // no gender
        assert_eq!(file.entries()[2].tier, 2); // no gender
        assert_eq!(file.entries()[3].tier, 1); // perempuan

        // 2. Lint before fix should detect macron in Satō
        let findings_before = lint_glossary(&file, &keywords);
        assert!(findings_before
            .iter()
            .any(|f| f.kind == GlossaryLintKind::Macron));

        // 3. Fix macrons in place
        let fixed = fix_macrons_in_place(&mut file);
        assert_eq!(fixed, 1);
        assert_eq!(file.entries()[0].translation, "Satou");

        // 4. Lint after fix should have zero macron findings
        let findings_after = lint_glossary(&file, &keywords);
        assert!(!findings_after
            .iter()
            .any(|f| f.kind == GlossaryLintKind::Macron));

        // 5. Serialize should preserve structure and updated digraph
        let serialized = serialize_glossary(&file);
        assert!(serialized.contains("- 佐藤 === Satou === laki-laki; protagonis"));
        assert!(serialized.contains("# Volume Characters"));
        assert!(serialized.contains("// Komentar pelindung"));
    }
}
