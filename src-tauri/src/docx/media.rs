//! Docx media handling, image conversion, and relationship management per FR-DOC-07.

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::io::Cursor;

use image::ImageFormat;

use crate::error::AppError;

use super::template::DocxPackage;

/// Decodes WebP bytes and converts them into PNG bytes per FR-DOC-07.
pub fn convert_webp_to_png(webp_bytes: &[u8]) -> Result<Vec<u8>, AppError> {
    let img = image::load_from_memory_with_format(webp_bytes, ImageFormat::WebP)
        .or_else(|_| image::load_from_memory(webp_bytes))
        .map_err(|e| AppError::Internal {
            message: format!("Failed to decode webp image: {e}"),
        })?;

    let mut png_bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
        .map_err(|e| AppError::Internal {
            message: format!("Failed to encode image to PNG: {e}"),
        })?;

    Ok(png_bytes)
}

/// Reads image dimensions `(width, height)` in pixels from raw image bytes.
pub fn get_image_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if let Ok(reader) = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format() {
        if let Ok((w, h)) = reader.into_dimensions() {
            return Some((w, h));
        }
    }
    image::load_from_memory(bytes)
        .ok()
        .map(|img| (img.width(), img.height()))
}

/// Computes a deterministic hex hash for media bytes.
pub fn compute_media_hash(bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    hasher.write(bytes);
    let hash_val = hasher.finish();
    format!("{:012x}", hash_val)
}

/// Registers an image file in the docx package:
/// - Converts WebP to PNG if applicable.
/// - Stores under `word/media/lnta_<hash>.<ext>`.
/// - Adds relationship `rIdLnta<N>` in `word/_rels/document.xml.rels`.
/// - Ensures `[Content_Types].xml` has the image extension registered.
///
/// Returns `(rel_id, media_name, (width, height))`.
pub fn add_image_to_package(
    package: &mut DocxPackage,
    image_bytes: &[u8],
    filename_hint: &str,
    rel_index: usize,
) -> Result<(String, String, (u32, u32)), AppError> {
    let is_webp = filename_hint.to_lowercase().ends_with(".webp")
        || image::guess_format(image_bytes).is_ok_and(|f| f == ImageFormat::WebP);

    let (final_bytes, ext) = if is_webp {
        let png = convert_webp_to_png(image_bytes)?;
        (png, "png")
    } else {
        let ext = if filename_hint.to_lowercase().ends_with(".png") {
            "png"
        } else {
            "jpeg"
        };
        (image_bytes.to_vec(), ext)
    };

    let (width, height) = get_image_dimensions(&final_bytes).unwrap_or((800, 600));

    let hash_str = compute_media_hash(&final_bytes);
    let media_filename = format!("lnta_{hash_str}.{ext}");
    let package_path = format!("word/media/{media_filename}");

    package.files.entry(package_path).or_insert(final_bytes);

    let rel_id = format!("rIdLnta{rel_index}");

    // Update word/_rels/document.xml.rels
    let rels_entry_name = "word/_rels/document.xml.rels";
    let rels_xml = package.get_text_entry(rels_entry_name).unwrap_or_else(|| {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
</Relationships>"#
            .to_string()
    });

    let new_rel_tag = format!(
        r#"<Relationship Id="{rel_id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/{media_filename}"/>"#
    );

    let updated_rels_xml = if let Some(pos) = rels_xml.rfind("</Relationships>") {
        format!("{}{new_rel_tag}\n{}", &rels_xml[..pos], &rels_xml[pos..])
    } else {
        format!("{rels_xml}\n{new_rel_tag}")
    };
    package.set_text_entry(rels_entry_name, &updated_rels_xml);

    // Update [Content_Types].xml
    let ct_entry_name = "[Content_Types].xml";
    if let Some(ct_xml) = package.get_text_entry(ct_entry_name) {
        let mut updated_ct = ct_xml;
        let ext_type = if ext == "png" {
            r#"<Default Extension="png" ContentType="image/png"/>"#
        } else {
            r#"<Default Extension="jpeg" ContentType="image/jpeg"/>"#
        };

        let already_registered = if ext == "png" {
            updated_ct.contains(r#"Extension="png""#) || updated_ct.contains(r#"Extension="PNG""#)
        } else {
            updated_ct.contains(r#"Extension="jpeg""#) || updated_ct.contains(r#"Extension="JPEG""#)
        };

        if !already_registered {
            if let Some(pos) = updated_ct.rfind("</Types>") {
                updated_ct = format!("{}{ext_type}\n{}", &updated_ct[..pos], &updated_ct[pos..]);
                package.set_text_entry(ct_entry_name, &updated_ct);
            }
        }
    }

    Ok((rel_id, media_filename, (width, height)))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::docx::template::create_minimal_template;

    #[test]
    fn test_add_image_and_webp_conversion() {
        let mut pkg = create_minimal_template("Heading1");

        // Create a 10x10 synthetic PNG
        let mut png_bytes = Vec::new();
        let img = image::RgbImage::new(10, 10);
        img.write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
            .unwrap();

        let (rel_id, media_name, (w, h)) =
            add_image_to_package(&mut pkg, &png_bytes, "test.png", 1).unwrap();

        assert_eq!(rel_id, "rIdLnta1");
        assert!(media_name.starts_with("lnta_"));
        assert!(media_name.ends_with(".png"));
        assert_eq!(w, 10);
        assert_eq!(h, 10);

        // Verify media exists in package
        let media_path = format!("word/media/{media_name}");
        assert!(pkg.files.contains_key(&media_path));

        // Verify rels updated
        let rels = pkg.get_text_entry("word/_rels/document.xml.rels").unwrap();
        assert!(rels.contains(&rel_id));
        assert!(rels.contains(&media_name));

        // Verify content types updated
        let ct = pkg.get_text_entry("[Content_Types].xml").unwrap();
        assert!(ct.contains(r#"Extension="png""#));
    }
}
