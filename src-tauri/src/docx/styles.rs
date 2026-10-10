//! WordprocessingML styles parser per FR-DOC-02.
//!
//! Locates the heading 1 style ID by inspecting `<w:name w:val="heading 1"/>`
//! regardless of localized style ID (e.g. "Heading1", "Judul1", "Heading_1").

use quick_xml::events::Event;
use quick_xml::reader::Reader;

/// Finds the paragraph style ID for Heading 1 from `word/styles.xml` content.
///
/// Looks for `<w:style w:type="paragraph" w:styleId="...">` containing
/// `<w:name w:val="heading 1"/>` (case-insensitive match on w:val).
/// Defaults to `"Heading1"` if not found.
pub fn find_heading1_style_id(styles_xml: &str) -> String {
    let mut reader = Reader::from_str(styles_xml);
    reader.config_mut().trim_text(true);

    let mut current_style_id: Option<String> = None;
    let mut current_style_type: Option<String> = None;
    let mut is_in_style = false;

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = e.name();
                let local_name = if let Some(pos) = name.as_ref().iter().position(|&b| b == b':') {
                    &name.as_ref()[pos + 1..]
                } else {
                    name.as_ref()
                };

                if local_name == b"style" {
                    is_in_style = true;
                    current_style_id = None;
                    current_style_type = None;

                    for attr in e.attributes().flatten() {
                        let key = attr.key.as_ref();
                        let key_local = if let Some(pos) = key.iter().position(|&b| b == b':') {
                            &key[pos + 1..]
                        } else {
                            key
                        };

                        if key_local == b"styleId" {
                            current_style_id = String::from_utf8(attr.value.to_vec()).ok();
                        } else if key_local == b"type" {
                            current_style_type = String::from_utf8(attr.value.to_vec()).ok();
                        }
                    }
                } else if is_in_style && local_name == b"name" {
                    for attr in e.attributes().flatten() {
                        let key = attr.key.as_ref();
                        let key_local = if let Some(pos) = key.iter().position(|&b| b == b':') {
                            &key[pos + 1..]
                        } else {
                            key
                        };

                        if key_local == b"val" {
                            if let Ok(val_str) = String::from_utf8(attr.value.to_vec()) {
                                if val_str.trim().eq_ignore_ascii_case("heading 1") {
                                    if let Some(ref style_id) = current_style_id {
                                        // Only match paragraph styles (or unspecified)
                                        if current_style_type
                                            .as_deref()
                                            .is_none_or(|t| t.eq_ignore_ascii_case("paragraph"))
                                        {
                                            return style_id.clone();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                let local_name = if let Some(pos) = name.as_ref().iter().position(|&b| b == b':') {
                    &name.as_ref()[pos + 1..]
                } else {
                    name.as_ref()
                };

                if local_name == b"style" {
                    is_in_style = false;
                    current_style_id = None;
                    current_style_type = None;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    "Heading1".to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_find_standard_heading1() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:style w:type="paragraph" w:styleId="Heading1">
        <w:name w:val="heading 1"/>
        <w:pPr><w:spacing w:before="240" w:after="120"/></w:pPr>
    </w:style>
</w:styles>"#;

        assert_eq!(find_heading1_style_id(xml), "Heading1");
    }

    #[test]
    fn test_find_localized_judul1() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:style w:type="paragraph" w:styleId="Judul1">
        <w:name w:val="Heading 1"/>
        <w:pPr><w:spacing w:before="240" w:after="120"/></w:pPr>
    </w:style>
    <w:style w:type="paragraph" w:styleId="Normal">
        <w:name w:val="Normal"/>
    </w:style>
</w:styles>"#;

        assert_eq!(find_heading1_style_id(xml), "Judul1");
    }

    #[test]
    fn test_fallback_when_not_found() {
        let xml =
            r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"/>"#;
        assert_eq!(find_heading1_style_id(xml), "Heading1");
    }
}
