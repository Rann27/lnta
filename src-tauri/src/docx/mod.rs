//! LNTA docx renderer module.
//!
//! Implements FR-DOC-01..04, 08, 09, 11, 12, 15, Story E1, and Story E2:
//! - Package manipulation via zip and quick-xml
//! - Heading 1 detection by `w:name` (including localized style IDs like "Judul1")
//! - Removal of empty template paragraphs
//! - Chapter heading pageBreakBefore rules (suppressed for chapter 1 in empty document)
//! - Image-page chapter heading suppression
//! - Inline `*italic*` markdown run conversion
//! - Atomic writing and external backup
//! - Locked file detection returning `AppError::DocxLocked`

pub mod body;
pub mod styles;
pub mod template;
pub mod write;

pub use body::{
    build_body_paragraph_xml, build_heading_paragraph_xml, clean_template_document,
    inject_chapters_into_body, parse_italic_runs, xml_escape, RenderChapter, TextRun,
};
pub use styles::find_heading1_style_id;
pub use template::{create_minimal_template, DocxPackage};
pub use write::{backup_docx_file, check_file_locked, render_and_write_docx};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
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
        let templates_dir = std::path::Path::new("../testdata/templates");
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
            RenderChapter {
                title: Some("Bab 1: Pagi yang Cerah".to_string()),
                is_image_page: false,
                paragraphs: vec![
                    "Matahari mulai terbit menyinari ruangan.".to_string(),
                    "Dia bergumam, *hari ini akan menyenangkan*, lalu tersenyum.".to_string(),
                ],
            },
            RenderChapter {
                title: Some("Bab 2: Pertemuan".to_string()),
                is_image_page: false,
                paragraphs: vec!["Langkah kaki terdengar dari kejauhan.".to_string()],
            },
            RenderChapter {
                title: None,
                is_image_page: true,
                paragraphs: vec![],
            },
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
        let golden_path = std::path::Path::new("../testdata/golden/document.xml");
        if golden_path.exists() {
            let golden_xml = std::fs::read_to_string(golden_path).unwrap();
            assert_eq!(rendered_xml, golden_xml);
        } else {
            // Write golden document.xml if not yet created
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

        // Create the file
        std::fs::write(&target_docx, b"dummy content").unwrap();

        // Lock file with exclusive access using fs2 or read-only/lock simulation
        // In Windows, opening a file with exclusive lock prevents other writers
        // We can test check_file_locked directly by making file read-only on Windows:
        let mut permissions = std::fs::metadata(&target_docx).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&target_docx, permissions.clone()).unwrap();

        let res = check_file_locked(&target_docx);
        assert_eq!(res, Err(AppError::DocxLocked));

        // Restore permissions before cleanup
        permissions.set_readonly(false);
        let _ = std::fs::set_permissions(&target_docx, permissions);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
