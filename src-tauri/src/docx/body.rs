//! WordprocessingML body builder and text formatting per FR-DOC-01..04, 08, 15.

use regex::Regex;

/// A text run inside a paragraph with optional italic style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRun {
    pub text: String,
    pub is_italic: bool,
}

/// Escapes XML special characters for inclusion in `<w:t>`.
pub fn xml_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Parses inline markdown `*italic*` formatting into text runs per FR-DOC-08.
/// Unpaired `*` (such as `* * *` or isolated `*`) are rendered as literal text.
pub fn parse_italic_runs(text: &str) -> Vec<TextRun> {
    if !text.contains('*') {
        return vec![TextRun {
            text: text.to_string(),
            is_italic: false,
        }];
    }

    #[allow(clippy::unwrap_used)]
    let italic_re = Regex::new(r"\*([^\s*](?:[^*\n]*[^\s*])?)\*").unwrap();

    let mut runs = Vec::new();
    let mut last_idx = 0;

    for caps in italic_re.captures_iter(text) {
        #[allow(clippy::unwrap_used)]
        let full_match = caps.get(0).unwrap();
        #[allow(clippy::unwrap_used)]
        let inner_match = caps.get(1).unwrap();

        let start = full_match.start();
        let end = full_match.end();

        if start > last_idx {
            runs.push(TextRun {
                text: text[last_idx..start].to_string(),
                is_italic: false,
            });
        }

        runs.push(TextRun {
            text: inner_match.as_str().to_string(),
            is_italic: true,
        });

        last_idx = end;
    }

    if last_idx < text.len() {
        runs.push(TextRun {
            text: text[last_idx..].to_string(),
            is_italic: false,
        });
    }

    runs
}

/// Builds XML for a paragraph run `<w:r>`.
pub fn build_run_xml(run: &TextRun) -> String {
    let escaped = xml_escape(&run.text);
    if run.is_italic {
        format!(r#"<w:r><w:rPr><w:i/></w:rPr><w:t xml:space="preserve">{escaped}</w:t></w:r>"#)
    } else {
        format!(r#"<w:r><w:t xml:space="preserve">{escaped}</w:t></w:r>"#)
    }
}

/// Builds a body paragraph `<w:p>` from raw translated text.
pub fn build_body_paragraph_xml(text: &str) -> String {
    let runs = parse_italic_runs(text);
    let mut inner = String::new();
    for run in &runs {
        inner.push_str(&build_run_xml(run));
    }
    format!("<w:p>{inner}</w:p>")
}

/// Builds a chapter heading paragraph `<w:p>`.
/// Emits `<w:pageBreakBefore/>` unless `page_break_before` is false.
pub fn build_heading_paragraph_xml(
    title: &str,
    heading_style_id: &str,
    page_break_before: bool,
) -> String {
    let escaped = xml_escape(title);
    let page_break_tag = if page_break_before {
        "<w:pageBreakBefore/>"
    } else {
        ""
    };

    format!(
        r#"<w:p><w:pPr><w:pStyle w:val="{heading_style_id}"/>{page_break_tag}</w:pPr><w:r><w:t xml:space="preserve">{escaped}</w:t></w:r></w:p>"#
    )
}

/// Checks whether a `<w:p>` block contains any meaningful text or drawing.
pub fn is_empty_paragraph(p_xml: &str) -> bool {
    // If it contains drawings or pictures, it is not empty
    if p_xml.contains("w:drawing") || p_xml.contains("w:pict") {
        return false;
    }

    // Extract all text inside <w:t> tags
    #[allow(clippy::unwrap_used)]
    let t_re = Regex::new(r"<w:t[^>]*>(.*?)</w:t>").unwrap();

    let mut text_content = String::new();
    for caps in t_re.captures_iter(p_xml) {
        if let Some(m) = caps.get(1) {
            text_content.push_str(m.as_str());
        }
    }

    text_content.trim().is_empty()
}

/// Removes empty template paragraphs per FR-DOC-15.
/// Returns the cleaned document XML and whether the document still contains non-empty paragraphs.
pub fn clean_template_document(document_xml: &str) -> (String, bool) {
    #[allow(clippy::unwrap_used)]
    let p_re = Regex::new(r"(?s)<w:p\b[^>]*>.*?</w:p>").unwrap();

    let mut has_non_empty = false;
    let mut cleaned = String::with_capacity(document_xml.len());
    let mut last_idx = 0;

    for mat in p_re.find_iter(document_xml) {
        cleaned.push_str(&document_xml[last_idx..mat.start()]);
        let p_content = mat.as_str();

        if is_empty_paragraph(p_content) {
            // Discard empty template paragraph
        } else {
            cleaned.push_str(p_content);
            has_non_empty = true;
        }

        last_idx = mat.end();
    }

    cleaned.push_str(&document_xml[last_idx..]);
    (cleaned, has_non_empty)
}

/// Represents a chapter to be rendered into the docx body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderChapter {
    pub title: Option<String>,
    pub is_image_page: bool,
    pub paragraphs: Vec<String>,
}

/// Injects rendered chapters into `<w:body>` of `word/document.xml`.
pub fn inject_chapters_into_body(
    document_xml: &str,
    chapters: &[RenderChapter],
    heading_style_id: &str,
) -> String {
    // 1. Clean empty paragraphs from template
    let (cleaned_doc, doc_had_prior_content) = clean_template_document(document_xml);

    // Locate insertion point right before the final `<w:sectPr` or `</w:body>`
    let insert_pos = if let Some(sect_pos) = cleaned_doc.rfind("<w:sectPr") {
        sect_pos
    } else if let Some(body_end) = cleaned_doc.rfind("</w:body>") {
        body_end
    } else {
        cleaned_doc.len()
    };

    let mut chapters_xml = String::new();
    let mut is_first_rendered_chapter = true;

    for chapter in chapters {
        // Image-page chapters (FR-RAW-08) emit NO heading
        if !chapter.is_image_page {
            if let Some(ref title) = chapter.title {
                // Page break before on all chapter headings except the very first chapter of an empty document
                let page_break_before = doc_had_prior_content || !is_first_rendered_chapter;
                let heading_xml =
                    build_heading_paragraph_xml(title, heading_style_id, page_break_before);
                chapters_xml.push_str(&heading_xml);
            }
        }

        // Render body paragraphs
        for para in &chapter.paragraphs {
            let para_xml = build_body_paragraph_xml(para);
            chapters_xml.push_str(&para_xml);
        }

        is_first_rendered_chapter = false;
    }

    let mut result = String::with_capacity(cleaned_doc.len() + chapters_xml.len());
    result.push_str(&cleaned_doc[..insert_pos]);
    result.push_str(&chapters_xml);
    result.push_str(&cleaned_doc[insert_pos..]);

    result
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_italic_runs() {
        let text = "Dia berkata, *apakah itu benar?*, lalu terdiam.";
        let runs = parse_italic_runs(text);

        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].text, "Dia berkata, ");
        assert!(!runs[0].is_italic);
        assert_eq!(runs[1].text, "apakah itu benar?");
        assert!(runs[1].is_italic);
        assert_eq!(runs[2].text, ", lalu terdiam.");
        assert!(!runs[2].is_italic);
    }

    #[test]
    fn test_unpaired_asterisk_remains_literal() {
        let text = "* * *";
        let runs = parse_italic_runs(text);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "* * *");
        assert!(!runs[0].is_italic);
    }

    #[test]
    fn test_empty_template_paragraph_removal() {
        let doc = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:pPr><w:rPr/></w:pPr></w:p><w:sectPr/></w:body></w:document>"#;
        let (cleaned, has_content) = clean_template_document(doc);

        assert!(!has_content);
        assert!(!cleaned.contains("<w:p>"));
    }

    #[test]
    fn test_inject_chapters_with_page_break_rules() {
        let template = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t></w:t></w:r></w:p><w:sectPr/></w:body></w:document>"#;

        let chapters = vec![
            RenderChapter {
                title: Some("Bab 1".to_string()),
                is_image_page: false,
                paragraphs: vec!["Paragraf satu.".to_string()],
            },
            RenderChapter {
                title: Some("Bab 2".to_string()),
                is_image_page: false,
                paragraphs: vec!["Paragraf dua.".to_string()],
            },
            RenderChapter {
                title: None,
                is_image_page: true,
                paragraphs: vec![],
            },
        ];

        let result = inject_chapters_into_body(template, &chapters, "Heading1");

        // First chapter in empty document should NOT have pageBreakBefore
        assert!(result.contains(
            r#"<w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t xml:space="preserve">Bab 1"#
        ));

        // Second chapter SHOULD have pageBreakBefore
        assert!(result.contains(r#"<w:pStyle w:val="Heading1"/><w:pageBreakBefore/></w:pPr><w:r><w:t xml:space="preserve">Bab 2"#));

        // Image-page chapter emits NO heading
        assert!(!result.contains("Bab 3"));
    }
}
