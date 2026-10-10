//! LNTA docx renderer module.
//!
//! Implements FR-DOC-01..10, 13, 15, Story E1, Story E2, and Story A4:
//! - Package manipulation via zip and quick-xml
//! - Heading 1 detection by `w:name` (including localized style IDs like "Judul1")
//! - Removal of empty template paragraphs
//! - Chapter heading pageBreakBefore rules (suppressed for chapter 1 in empty document)
//! - Image-page chapter heading suppression
//! - Inline `*italic*` markdown run conversion
//! - Image drawing isolation with EMU content box aspect scaling (FR-DOC-05..07)
//! - WebP to PNG conversion
//! - Template treatments (KeepFrontMatter, FrozenPrefix, IgnoreContent per FR-DOC-10)
//! - Default template generator (FR-DOC-13)
//! - Atomic writing and external backup
//! - Locked file detection returning `AppError::DocxLocked`

pub mod body;
pub mod default_template;
pub mod images;
pub mod import;
pub mod media;
pub mod styles;
pub mod template;
pub mod template_content;
pub mod write;

pub use body::{
    build_body_paragraph_xml, build_body_paragraph_xml_with_break, build_heading_paragraph_xml,
    clean_template_document, inject_chapters_into_body, parse_italic_runs, xml_escape,
    RenderChapter, RenderItem, TextRun,
};
pub use default_template::generate_default_template;
pub use images::{
    build_image_paragraph_xml, build_missing_image_paragraph_xml, parse_content_box_from_doc,
    scale_image_dimensions, ContentBoxEmu,
};
pub use import::{
    apply_frozen_mappings_to_db, extract_headings_from_document_xml,
    extract_headings_from_docx_bytes, propose_frozen_chapter_mappings, ExtractedDocxHeading,
    FrozenChapterMapping,
};
pub use media::{
    add_image_to_package, compute_media_hash, convert_webp_to_png, get_image_dimensions,
};
pub use styles::find_heading1_style_id;
pub use template::{create_minimal_template, DocxPackage};
pub use template_content::{
    apply_template_treatment, does_template_contain_text, TemplateTreatment,
};
pub use write::{
    backup_docx_file, check_file_locked, render_and_write_docx,
    render_and_write_docx_with_treatment, write_docx_package_atomically,
};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::io::Cursor;
    use std::path::Path;

    use super::*;
    use crate::error::AppError;

    #[test]
    fn test_heading_style_detection_standard_and_localized() {
        let std_pkg = create_minimal_template("Heading1");
        let std_styles = std_pkg.get_text_entry("word/styles.xml").unwrap();
        assert_eq!(find_heading1_style_id(&std_styles), "Heading1");

        let judul_pkg = create_minimal_template("Judul1");
        let judul_styles = judul_pkg.get_text_entry("word/styles.xml").unwrap();
        assert_eq!(find_heading1_style_id(&judul_styles), "Judul1");

        // Write synthetic template files to testdata/templates/
        let templates_dir = Path::new("../testdata/templates");
        std::fs::create_dir_all(templates_dir).unwrap();
        std::fs::write(
            templates_dir.join("template_standard.docx"),
            std_pkg.to_bytes().unwrap(),
        )
        .unwrap();
        std::fs::write(
            templates_dir.join("template_localized_judul1.docx"),
            judul_pkg.to_bytes().unwrap(),
        )
        .unwrap();

        // Verify reading them back from disk
        let disk_std_bytes = std::fs::read(templates_dir.join("template_standard.docx")).unwrap();
        let loaded_std = DocxPackage::from_bytes(&disk_std_bytes).unwrap();
        assert_eq!(
            find_heading1_style_id(&loaded_std.get_text_entry("word/styles.xml").unwrap()),
            "Heading1"
        );

        let disk_judul_bytes =
            std::fs::read(templates_dir.join("template_localized_judul1.docx")).unwrap();
        let loaded_judul = DocxPackage::from_bytes(&disk_judul_bytes).unwrap();
        assert_eq!(
            find_heading1_style_id(&loaded_judul.get_text_entry("word/styles.xml").unwrap()),
            "Judul1"
        );
    }

    #[test]
    fn test_full_render_workflow_and_golden_document() {
        let template_pkg = create_minimal_template("Heading1");
        let template_bytes = template_pkg.to_bytes().unwrap();

        let chapters = vec![
            RenderChapter::from_paragraphs(
                Some("Bab 1: Pagi yang Cerah".to_string()),
                false,
                vec![
                    "Matahari mulai terbit menyinari ruangan.".to_string(),
                    "Dia bergumam, *hari ini akan menyenangkan*, lalu tersenyum.".to_string(),
                ],
            ),
            RenderChapter::from_paragraphs(
                Some("Bab 2: Pertemuan".to_string()),
                false,
                vec!["Langkah kaki terdengar dari kejauhan.".to_string()],
            ),
            RenderChapter::from_paragraphs(None, true, vec![]),
        ];

        let temp_dir =
            std::env::temp_dir().join(format!("lnta_test_docx_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let target_docx = temp_dir.join("Translations.docx");
        let backup_dir = temp_dir.join("backups");

        // 1. Initial render
        let rendered_xml =
            render_and_write_docx(&template_bytes, &chapters, &target_docx, Some(&backup_dir))
                .unwrap();

        // Verify chapter 1 has NO pageBreakBefore, chapter 2 HAS pageBreakBefore, image chapter emits NO heading
        assert!(rendered_xml.contains(r#"<w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t xml:space="preserve">Bab 1: Pagi yang Cerah"#));
        assert!(rendered_xml.contains(r#"<w:pStyle w:val="Heading1"/><w:pageBreakBefore/></w:pPr><w:r><w:t xml:space="preserve">Bab 2: Pertemuan"#));
        assert!(rendered_xml.contains(
            r#"<w:rPr><w:i/></w:rPr><w:t xml:space="preserve">hari ini akan menyenangkan</w:t>"#
        ));

        // Verify golden file matches
        let golden_path = Path::new("../testdata/golden/document.xml");
        if golden_path.exists() {
            let golden_xml = std::fs::read_to_string(golden_path).unwrap();
            assert_eq!(rendered_xml, golden_xml);
        } else {
            std::fs::create_dir_all(golden_path.parent().unwrap()).unwrap();
            std::fs::write(golden_path, &rendered_xml).unwrap();
        }

        // 2. Second render: verify backup created when target exists
        let _ = render_and_write_docx(&template_bytes, &chapters, &target_docx, Some(&backup_dir))
            .unwrap();

        let backups: Vec<_> = std::fs::read_dir(&backup_dir).unwrap().flatten().collect();
        assert_eq!(backups.len(), 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[allow(clippy::permissions_set_readonly_false)]
    fn test_locked_file_returns_docx_locked() {
        let temp_dir =
            std::env::temp_dir().join(format!("lnta_test_locked_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let target_docx = temp_dir.join("Translations.docx");

        std::fs::write(&target_docx, b"dummy content").unwrap();

        let mut permissions = std::fs::metadata(&target_docx).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&target_docx, permissions.clone()).unwrap();

        let res = check_file_locked(&target_docx);
        assert_eq!(res, Err(AppError::DocxLocked));

        permissions.set_readonly(false);
        let _ = std::fs::set_permissions(&target_docx, permissions);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_docx_image_rendering_and_quickxml_verification() {
        // Create synthetic illustration files in testdata/illustrations/
        let illustrations_dir = Path::new("../testdata/illustrations");
        std::fs::create_dir_all(illustrations_dir).unwrap();

        // 1. Tall synthetic image (400w x 800h)
        let tall_img = image::RgbImage::new(400, 800);
        let mut tall_png = Vec::new();
        tall_img
            .write_to(&mut Cursor::new(&mut tall_png), image::ImageFormat::Png)
            .unwrap();
        std::fs::write(illustrations_dir.join("tall.png"), &tall_png).unwrap();

        // 2. Wide synthetic image (800w x 400h)
        let wide_img = image::RgbImage::new(800, 400);
        let mut wide_png = Vec::new();
        wide_img
            .write_to(&mut Cursor::new(&mut wide_png), image::ImageFormat::Png)
            .unwrap();
        std::fs::write(illustrations_dir.join("wide.png"), &wide_png).unwrap();

        // Prepare docx package
        let mut package = create_minimal_template("Heading1");
        let doc_xml = package
            .get_text_entry("word/document.xml")
            .unwrap_or_default();
        let content_box = parse_content_box_from_doc(&doc_xml);

        // Register tall image
        let (rel1, _m1, (w1, h1)) =
            add_image_to_package(&mut package, &tall_png, "tall.png", 1).unwrap();
        let (cx1, cy1) = scale_image_dimensions(w1, h1, &content_box);

        // Register wide image
        let (rel2, _m2, (w2, h2)) =
            add_image_to_package(&mut package, &wide_png, "wide.png", 2).unwrap();
        let (cx2, cy2) = scale_image_dimensions(w2, h2, &content_box);

        // Build chapters:
        // Ch 1: Image-page chapter with tall image (no heading emitted)
        // Ch 2: Heading, Text 1, Wide image, Text 2, Missing image placeholder
        // Ch 3: Heading, Text 3, Tall image at end
        // Ch 4: Heading, Text 4
        let chapters = vec![
            RenderChapter {
                title: None,
                is_image_page: true,
                items: vec![RenderItem::Image {
                    rel_id: rel1.clone(),
                    cx: cx1,
                    cy: cy1,
                    doc_pr_id: 1,
                }],
            },
            RenderChapter {
                title: Some("Bab 2: Petualangan".to_string()),
                is_image_page: false,
                items: vec![
                    RenderItem::Text("Mereka mulai berjalan menyusuri hutan.".to_string()),
                    RenderItem::Image {
                        rel_id: rel2,
                        cx: cx2,
                        cy: cy2,
                        doc_pr_id: 2,
                    },
                    RenderItem::Text("Pemandangan begitu luas membentang.".to_string()),
                    RenderItem::MissingImage("Image PlaceHolder: map.jpg".to_string()),
                ],
            },
            RenderChapter {
                title: Some("Bab 3: Pertarungan".to_string()),
                is_image_page: false,
                items: vec![
                    RenderItem::Text("Pedang terhunus berkilau tajam.".to_string()),
                    RenderItem::Image {
                        rel_id: rel1,
                        cx: cx1,
                        cy: cy1,
                        doc_pr_id: 3,
                    },
                ],
            },
            RenderChapter {
                title: Some("Bab 4: Kemenangan".to_string()),
                is_image_page: false,
                items: vec![RenderItem::Text("Sorak-sorai memenuhi udara.".to_string())],
            },
        ];

        let heading_style = find_heading1_style_id(
            &package
                .get_text_entry("word/styles.xml")
                .unwrap_or_default(),
        );

        let new_doc_xml = inject_chapters_into_body(
            &doc_xml,
            &chapters,
            &heading_style,
            TemplateTreatment::KeepFrontMatter,
        );
        package.set_text_entry("word/document.xml", &new_doc_xml);

        let final_bytes = package.to_bytes().unwrap();

        // Write to temp file
        let temp_docx = std::env::temp_dir().join(format!("verify_{}.docx", uuid::Uuid::new_v4()));
        std::fs::write(&temp_docx, &final_bytes).unwrap();

        // Re-open with zip and quick-xml to assert counts
        let file = std::fs::File::open(&temp_docx).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();

        // Verify media files exist in zip
        let has_media1 = zip.by_name("word/media/lnta_").is_ok()
            || zip
                .file_names()
                .any(|name| name.starts_with("word/media/lnta_"));
        assert!(has_media1, "word/media/ should contain lnta_* files");

        // Verify [Content_Types].xml contains png extension once
        let ct_xml = {
            let mut ct_file = zip.by_name("[Content_Types].xml").unwrap();
            let mut s = String::new();
            std::io::Read::read_to_string(&mut ct_file, &mut s).unwrap();
            s
        };
        assert!(ct_xml.contains(r#"Extension="png""#));

        // Read and parse word/document.xml with quick-xml
        let doc_content = {
            let mut doc_file = zip.by_name("word/document.xml").unwrap();
            let mut s = String::new();
            std::io::Read::read_to_string(&mut doc_file, &mut s).unwrap();
            s
        };

        let mut reader = quick_xml::Reader::from_str(&doc_content);
        reader.config_mut().trim_text(true);

        let mut heading_count = 0;
        let mut paragraph_count = 0;
        let mut drawing_count = 0;
        let mut page_break_before_count = 0;

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Start(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if name == "w:p" {
                        paragraph_count += 1;
                    } else if name == "w:drawing" {
                        drawing_count += 1;
                    }
                }
                Ok(quick_xml::events::Event::Empty(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if name == "w:pageBreakBefore" {
                        page_break_before_count += 1;
                    } else if name == "w:pStyle" {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"w:val"
                                && attr.value.as_ref() == heading_style.as_bytes()
                            {
                                heading_count += 1;
                            }
                        }
                    }
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Err(e) => panic!("XML parse error: {e}"),
                _ => {}
            }
            buf.clear();
        }

        // Assertions:
        // 1. Headings: exactly 3 (Bab 2, Bab 3, Bab 4 - Bab 1 is image-page chapter with no heading)
        assert_eq!(
            heading_count, 3,
            "Expected 3 headings for Bab 2, Bab 3, Bab 4"
        );

        // 2. Drawings: exactly 3 drawings (Ch 1 image, Ch 2 wide image, Ch 3 tall image)
        assert_eq!(drawing_count, 3, "Expected 3 image drawings");

        // 3. Paragraphs:
        // Ch 1: 1 image para
        // Ch 2: 1 heading + 1 text + 1 image + 1 text + 1 missing image placeholder = 5 paras
        // Ch 3: 1 heading + 1 text + 1 image = 3 paras
        // Ch 4: 1 heading + 1 text = 2 paras
        // Total = 1 + 5 + 3 + 2 = 11 paragraphs
        assert_eq!(paragraph_count, 11, "Expected 11 total paragraphs");

        // 4. Page breaks:
        // Ch 1 image: 0 (first element of empty doc)
        // Ch 2: Heading (1), Wide image (1), Text 2 (1), Missing image (1)
        // Ch 3: Heading (1), Tall image (1)
        // Ch 4: Heading (1)
        // Total = 7 page breaks before
        assert_eq!(
            page_break_before_count, 7,
            "Expected 7 pageBreakBefore tags"
        );

        // 5. Missing image placeholder text is present
        assert!(doc_content.contains("Image PlaceHolder: map.jpg"));

        // 6. Clean up temp file
        let _ = std::fs::remove_file(&temp_docx);
    }
}
