//! Docx template content treatments per FR-DOC-10, FR-DOC-15, and Story A4.

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::body::clean_template_document;

/// Treatment for pre-existing content in `Translations.docx` / template per FR-DOC-10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateTreatment {
    /// Keep pre-existing front matter paragraphs in the template (default).
    #[default]
    KeepFrontMatter,
    /// Keep pre-existing chapters and content verbatim as a frozen prefix.
    FrozenPrefix,
    /// Ignore pre-existing template paragraphs; use template styles and layout only.
    IgnoreContent,
}

/// Checks whether a template's `document.xml` contains meaningful text content (FR-DOC-10 preflight check).
///
/// Empty template paragraphs (FR-DOC-15) are ignored.
pub fn does_template_contain_text(document_xml: &str) -> bool {
    let (_, has_content) = clean_template_document(document_xml);
    has_content
}

/// Applies the chosen `TemplateTreatment` to `word/document.xml`.
///
/// Returns the treated document XML and whether prior non-empty content remains.
pub fn apply_template_treatment(
    document_xml: &str,
    treatment: TemplateTreatment,
) -> (String, bool) {
    match treatment {
        TemplateTreatment::IgnoreContent => {
            // Strip all <w:p> paragraphs from the document body, keeping sectPr and outer wrapper
            #[allow(clippy::unwrap_used)]
            let p_re = Regex::new(r"(?s)<w:p\b[^>]*>.*?</w:p>").unwrap();
            let stripped = p_re.replace_all(document_xml, "").to_string();
            (stripped, false)
        }
        TemplateTreatment::KeepFrontMatter | TemplateTreatment::FrozenPrefix => {
            // Clean empty paragraphs per FR-DOC-15 and retain non-empty content
            clean_template_document(document_xml)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_template_has_no_text() {
        let empty_doc = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:pPr><w:rPr/></w:pPr></w:p><w:sectPr/></w:body></w:document>"#;
        assert!(!does_template_contain_text(empty_doc));

        let (cleaned, has_content) =
            apply_template_treatment(empty_doc, TemplateTreatment::KeepFrontMatter);
        assert!(!has_content);
        assert!(!cleaned.contains("<w:p>"));
    }

    #[test]
    fn test_template_treatments_with_content() {
        let content_doc = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Halaman Judul &amp; Hak Cipta</w:t></w:r></w:p><w:sectPr/></w:body></w:document>"#;
        assert!(does_template_contain_text(content_doc));

        // 1. KeepFrontMatter preserves content
        let (keep_xml, has_keep) =
            apply_template_treatment(content_doc, TemplateTreatment::KeepFrontMatter);
        assert!(has_keep);
        assert!(keep_xml.contains("Halaman Judul"));

        // 2. FrozenPrefix preserves content
        let (frozen_xml, has_frozen) =
            apply_template_treatment(content_doc, TemplateTreatment::FrozenPrefix);
        assert!(has_frozen);
        assert!(frozen_xml.contains("Halaman Judul"));

        // 3. IgnoreContent strips paragraphs
        let (ignored_xml, has_ignored) =
            apply_template_treatment(content_doc, TemplateTreatment::IgnoreContent);
        assert!(!has_ignored);
        assert!(!ignored_xml.contains("Halaman Judul"));
        assert!(!ignored_xml.contains("<w:p"));
        assert!(ignored_xml.contains("<w:sectPr/>"));
    }
}
