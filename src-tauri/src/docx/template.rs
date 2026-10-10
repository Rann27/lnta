//! Docx package reading, in-memory archive extraction, and re-packing via `zip`.

use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::error::AppError;

/// An in-memory representation of an unzipped docx package.
#[derive(Debug, Clone)]
pub struct DocxPackage {
    pub files: HashMap<String, Vec<u8>>,
}

impl DocxPackage {
    /// Reads a `.docx` package from raw bytes into memory.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AppError> {
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor).map_err(|e| AppError::InvalidTemplate {
            reason: format!("Failed to open docx zip package: {e}"),
        })?;

        let mut files = HashMap::new();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| AppError::InvalidTemplate {
                reason: format!("Failed to read entry {i} from docx archive: {e}"),
            })?;

            let name = file.name().to_string();
            let mut content = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut content)
                .map_err(|e| AppError::InvalidTemplate {
                    reason: format!("Failed to read entry {name} data: {e}"),
                })?;

            files.insert(name, content);
        }

        Ok(Self { files })
    }

    /// Retrieves an entry's content as a UTF-8 string.
    pub fn get_text_entry(&self, name: &str) -> Option<String> {
        self.files
            .get(name)
            .and_then(|bytes| String::from_utf8(bytes.clone()).ok())
    }

    /// Sets or replaces an entry's content from a string.
    pub fn set_text_entry(&mut self, name: &str, content: &str) {
        self.files
            .insert(name.to_string(), content.as_bytes().to_vec());
    }

    /// Writes all entries back into a valid `.docx` ZIP archive bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, AppError> {
        let mut buffer = Vec::new();
        let mut writer = ZipWriter::new(Cursor::new(&mut buffer));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for (name, content) in &self.files {
            writer
                .start_file(name, options)
                .map_err(|e| AppError::Internal {
                    message: format!("Failed to write zip entry {name}: {e}"),
                })?;

            writer.write_all(content).map_err(|e| AppError::Internal {
                message: format!("Failed to write zip entry {name} bytes: {e}"),
            })?;
        }

        writer.finish().map_err(|e| AppError::Internal {
            message: format!("Failed to finalize docx zip package: {e}"),
        })?;

        Ok(buffer)
    }
}

/// Generates a minimal default template package per FR-DOC-13.
/// Standard Letter page size (12240x15840 twips), margins 1440/1800/1440/1800 twips,
/// one empty paragraph, and styles with Heading 1 and Normal.
pub fn create_minimal_template(heading_style_id: &str) -> DocxPackage {
    let mut files = HashMap::new();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
</Types>"#;

    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;

    let document = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:pPr><w:rPr/></w:pPr></w:p>
    <w:sectPr>
      <w:pgSz w:w="12240" w:h="15840"/>
      <w:pgMar w:top="1440" w:right="1800" w:bottom="1440" w:left="1800"/>
    </w:sectPr>
  </w:body>
</w:document>"#;

    let styles = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:styleId="{heading_style_id}">
    <w:name w:val="heading 1"/>
    <w:pPr><w:spacing w:before="240" w:after="120"/></w:pPr>
  </w:style>
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
    <w:name w:val="Normal"/>
  </w:style>
</w:styles>"#
    );

    files.insert(
        "[Content_Types].xml".to_string(),
        content_types.as_bytes().to_vec(),
    );
    files.insert("_rels/.rels".to_string(), rels.as_bytes().to_vec());
    files.insert(
        "word/document.xml".to_string(),
        document.as_bytes().to_vec(),
    );
    files.insert("word/styles.xml".to_string(), styles.as_bytes().to_vec());

    DocxPackage { files }
}
