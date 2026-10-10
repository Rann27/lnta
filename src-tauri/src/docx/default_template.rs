//! Default docx template generation per FR-DOC-13.
//!
//! Generates a valid clean docx document with:
//! - Normal and Heading 1 styles
//! - Standard Letter page size (12240x15840 twips)
//! - Margins 1440 twips top/bottom, 1800 twips left/right (matching fixture template)
//! - Single empty paragraph matching Word defaults

use super::template::{create_minimal_template, DocxPackage};

/// Generates a default docx template package for projects lacking `Translations.docx`.
pub fn generate_default_template(heading_style_id: &str) -> DocxPackage {
    create_minimal_template(heading_style_id)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_default_template() {
        let pkg = generate_default_template("Heading1");
        assert!(pkg.files.contains_key("[Content_Types].xml"));
        assert!(pkg.files.contains_key("_rels/.rels"));
        assert!(pkg.files.contains_key("word/document.xml"));
        assert!(pkg.files.contains_key("word/styles.xml"));

        let bytes = pkg.to_bytes().unwrap();
        assert!(!bytes.is_empty());
    }
}
