//! Docx image sizing, aspect ratio preservation, and drawing XML builder per FR-DOC-05..07.

use regex::Regex;

/// Content dimensions in EMUs (English Metric Units).
/// 1 twip = 635 EMUs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentBoxEmu {
    pub width_emu: u64,
    pub height_emu: u64,
}

impl Default for ContentBoxEmu {
    fn default() -> Self {
        // Standard Letter (8.5x11 in) with 1440 twip top/bottom, 1800 twip left/right margins:
        // width = (12240 - 3600) * 635 = 8640 * 635 = 5,486,400 EMUs (~5.76 in)
        // height = (15840 - 2880) * 635 = 12960 * 635 = 8,229,600 EMUs (~8.64 in)
        Self {
            width_emu: 5_486_400,
            height_emu: 8_229_600,
        }
    }
}

/// Parses the last `<w:sectPr>` from `document.xml` to compute the content box in EMUs per FR-DOC-06.
pub fn parse_content_box_from_doc(doc_xml: &str) -> ContentBoxEmu {
    #[allow(clippy::unwrap_used)]
    let pgsz_re = Regex::new(r#"<w:pgSz\b[^>]*w:w="(\d+)"[^>]*w:h="(\d+)""#).unwrap();
    #[allow(clippy::unwrap_used)]
    let pgmar_re = Regex::new(
        r#"<w:pgMar\b[^>]*w:top="(\d+)"[^>]*w:right="(\d+)"[^>]*w:bottom="(\d+)"[^>]*w:left="(\d+)""#,
    )
    .unwrap();

    let mut w_twips = 12240u64;
    let mut h_twips = 15840u64;
    let mut top_twips = 1440u64;
    let mut right_twips = 1800u64;
    let mut bottom_twips = 1440u64;
    let mut left_twips = 1800u64;

    if let Some(caps) = pgsz_re.captures_iter(doc_xml).last() {
        if let (Some(w), Some(h)) = (caps.get(1), caps.get(2)) {
            if let (Ok(wv), Ok(hv)) = (w.as_str().parse(), h.as_str().parse()) {
                w_twips = wv;
                h_twips = hv;
            }
        }
    }

    if let Some(caps) = pgmar_re.captures_iter(doc_xml).last() {
        if let (Some(t), Some(r), Some(b), Some(l)) =
            (caps.get(1), caps.get(2), caps.get(3), caps.get(4))
        {
            if let (Ok(tv), Ok(rv), Ok(bv), Ok(lv)) = (
                t.as_str().parse(),
                r.as_str().parse(),
                b.as_str().parse(),
                l.as_str().parse(),
            ) {
                top_twips = tv;
                right_twips = rv;
                bottom_twips = bv;
                left_twips = lv;
            }
        }
    }

    let width_twips = w_twips.saturating_sub(left_twips + right_twips).max(1440);
    let height_twips = h_twips.saturating_sub(top_twips + bottom_twips).max(1440);

    ContentBoxEmu {
        width_emu: width_twips * 635,
        height_emu: height_twips * 635,
    }
}

/// Scales image dimensions to fit within `content_box` while strictly preserving aspect ratio.
/// scale = min(box_w / img_w, box_h / img_h).
pub fn scale_image_dimensions(img_w: u32, img_h: u32, content_box: &ContentBoxEmu) -> (u64, u64) {
    if img_w == 0 || img_h == 0 {
        return (content_box.width_emu, content_box.height_emu);
    }

    let scale_w = content_box.width_emu as f64 / img_w as f64;
    let scale_h = content_box.height_emu as f64 / img_h as f64;

    let scale = scale_w.min(scale_h);

    let cx = (img_w as f64 * scale).round() as u64;
    let cy = (img_h as f64 * scale).round() as u64;

    (
        cx.min(content_box.width_emu),
        cy.min(content_box.height_emu),
    )
}

/// Generates a drawing paragraph XML `<w:p>` embedding an image on its own isolated page.
pub fn build_image_paragraph_xml(
    rel_id: &str,
    cx: u64,
    cy: u64,
    doc_pr_id: u32,
    page_break_before: bool,
) -> String {
    let page_break_tag = if page_break_before {
        "<w:pageBreakBefore/>"
    } else {
        ""
    };

    format!(
        r#"<w:p><w:pPr><w:jc w:val="center"/>{page_break_tag}</w:pPr><w:r><w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="{cx}" cy="{cy}"/><wp:docPr id="{doc_pr_id}" name="Picture {doc_pr_id}"/><wp:cNvGraphicFramePr><a:graphicFrameLocks xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" noChangeAspect="1"/></wp:cNvGraphicFramePr><a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:nvPicPr><pic:cNvPr id="{doc_pr_id}" name="Picture {doc_pr_id}"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="{rel_id}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>"#
    )
}

/// Generates a centered fallback paragraph XML for a missing image placeholder.
pub fn build_missing_image_paragraph_xml(placeholder: &str, page_break_before: bool) -> String {
    let escaped = super::body::xml_escape(placeholder);
    let page_break_tag = if page_break_before {
        "<w:pageBreakBefore/>"
    } else {
        ""
    };

    format!(
        r#"<w:p><w:pPr><w:jc w:val="center"/>{page_break_tag}</w:pPr><w:r><w:t xml:space="preserve">{escaped}</w:t></w:r></w:p>"#
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_tall_and_wide_images() {
        let content_box = ContentBoxEmu {
            width_emu: 1000,
            height_emu: 2000,
        };

        // 1. Tall image: 500w x 2000h (aspect 1:4)
        // scale_w = 1000/500 = 2.0; scale_h = 2000/2000 = 1.0; scale = 1.0
        let (cx_tall, cy_tall) = scale_image_dimensions(500, 2000, &content_box);
        assert_eq!(cx_tall, 500);
        assert_eq!(cy_tall, 2000); // fits height

        // 2. Wide image: 2000w x 500h (aspect 4:1)
        // scale_w = 1000/2000 = 0.5; scale_h = 2000/500 = 4.0; scale = 0.5
        let (cx_wide, cy_wide) = scale_image_dimensions(2000, 500, &content_box);
        assert_eq!(cx_wide, 1000); // fits width
        assert_eq!(cy_wide, 250);
    }
}
