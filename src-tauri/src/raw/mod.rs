//! LNTA raw parsing and segmentation module.
//!
//! Implements FR-RAW-01 through FR-RAW-08:
//! - Multi-encoding raw file reading (UTF-8 with/without BOM, Shift_JIS/CP932)
//! - Line ending normalization (CRLF -> LF)
//! - In-memory ruby normalization (` 呆 あき れた` -> `呆《あき》れた`)
//! - Line classification (headings, scene markers, illustrations, dialogues, narrations)
//! - Title extraction with decorative bracket stripping (`# 【...】` -> clean title)
//! - Image-page chapter detection (no heading and no text segments)
//! - Sequential segmentation with 1-based source line ranges

pub mod classify;
pub mod encoding;
pub mod ruby;
pub mod segment;
pub mod title;

pub use classify::{Classifier, IllustrationSet, RawLineKind, DEFAULT_SCENE_MARKERS};
pub use encoding::{decode_bytes, read_raw_file};
pub use ruby::{is_ideograph, is_kana, normalize_ruby_line, normalize_ruby_text};
pub use segment::{parse_chapter, ParseOptions, ParsedChapter, RawSegment, SegmentKind};
pub use title::{strip_enclosing_brackets, title_from_filename};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_image_page_chapter_detection() {
        let content = r#"Image Placeholder: cover.jpg

![color page](insert_01.png)

insert_02.webp
"#;
        let illustrations =
            IllustrationSet::from_names(["cover.jpg", "insert_01.png", "insert_02.webp"]);
        let options = ParseOptions::default();

        let parsed = parse_chapter("1_color_pages.md", content, &illustrations, &options);

        assert!(parsed.is_image_page);
        assert_eq!(parsed.title_clean, None);
        assert_eq!(parsed.title_raw, None);

        // Does NOT have a title segment (idx 0)
        assert!(parsed.segments.iter().all(|s| s.kind != SegmentKind::Title));
        assert_eq!(parsed.segments.len(), 3);
        assert_eq!(parsed.segments[0].idx, 1);
        assert_eq!(parsed.segments[0].kind, SegmentKind::Image);
        assert_eq!(parsed.segments[0].image_file, Some("cover.jpg".to_string()));
        assert_eq!(parsed.segments[1].idx, 2);
        assert_eq!(parsed.segments[1].kind, SegmentKind::Image);
        assert_eq!(
            parsed.segments[1].image_file,
            Some("insert_01.png".to_string())
        );
        assert_eq!(parsed.segments[2].idx, 3);
        assert_eq!(parsed.segments[2].kind, SegmentKind::Image);
        assert_eq!(
            parsed.segments[2].image_file,
            Some("insert_02.webp".to_string())
        );
    }

    #[test]
    fn test_chapter_with_heading_brackets_and_ruby() {
        let content = r#"# 【第一話　始まりの朝】

朝、目が覚めると部屋は静まり返っていた。

「おはよう、美咲」

健太は 呆 あき れた様子で言った。

☆

Image Placeholder: ill_scene.jpg

「行こうか、 健 けん 太 た 」

彼女は微笑んだ。
"#;
        let illustrations =
            IllustrationSet::from_names(["ill_scene.jpg", "ill_scene copy.webp", "ill_scene.psd"]);
        let options = ParseOptions::default();

        let parsed = parse_chapter("02_第一話.md", content, &illustrations, &options);

        assert!(!parsed.is_image_page);
        assert_eq!(parsed.title_raw, Some("【第一話　始まりの朝】".to_string()));
        assert_eq!(parsed.title_clean, Some("第一話　始まりの朝".to_string()));
        assert_eq!(parsed.ruby_count, 3); // 呆あき (1) + 健けん (1) + 太た (1)

        // Check segments
        // Segment 0: Title
        assert_eq!(parsed.segments[0].idx, 0);
        assert_eq!(parsed.segments[0].kind, SegmentKind::Title);
        assert_eq!(parsed.segments[0].src_text, "第一話　始まりの朝");
        assert_eq!(parsed.segments[0].line_start, 1);
        assert_eq!(parsed.segments[0].line_end, 1);

        // Segment 1: Narration (line 3)
        assert_eq!(parsed.segments[1].idx, 1);
        assert_eq!(parsed.segments[1].kind, SegmentKind::Narration);
        assert_eq!(
            parsed.segments[1].src_text,
            "朝、目が覚めると部屋は静まり返っていた。"
        );
        assert_eq!(parsed.segments[1].line_start, 3);

        // Segment 2: Dialogue (line 5)
        assert_eq!(parsed.segments[2].idx, 2);
        assert_eq!(parsed.segments[2].kind, SegmentKind::Dialogue);
        assert_eq!(parsed.segments[2].src_text, "「おはよう、美咲」");
        assert_eq!(parsed.segments[2].line_start, 5);

        // Segment 3: Narration with normalized ruby (line 7)
        assert_eq!(parsed.segments[3].idx, 3);
        assert_eq!(parsed.segments[3].kind, SegmentKind::Narration);
        assert_eq!(
            parsed.segments[3].src_text,
            "健太は呆《あき》れた様子で言った。"
        );
        assert_eq!(parsed.segments[3].line_start, 7);

        // Segment 4: Scene marker (line 9)
        assert_eq!(parsed.segments[4].idx, 4);
        assert_eq!(parsed.segments[4].kind, SegmentKind::Scene);
        assert_eq!(parsed.segments[4].src_text, "☆");
        assert_eq!(parsed.segments[4].line_start, 9);

        // Segment 5: Image (line 11) - matched ill_scene.jpg, ignored copy and psd
        assert_eq!(parsed.segments[5].idx, 5);
        assert_eq!(parsed.segments[5].kind, SegmentKind::Image);
        assert_eq!(
            parsed.segments[5].image_file,
            Some("ill_scene.jpg".to_string())
        );
        assert_eq!(parsed.segments[5].line_start, 11);

        // Segment 6: Dialogue with normalized ruby (line 13)
        assert_eq!(parsed.segments[6].idx, 6);
        assert_eq!(parsed.segments[6].kind, SegmentKind::Dialogue);
        assert_eq!(
            parsed.segments[6].src_text,
            "「行こうか、健《けん》太《た》」"
        );
        assert_eq!(parsed.segments[6].line_start, 13);

        // Segment 7: Narration (line 15)
        assert_eq!(parsed.segments[7].idx, 7);
        assert_eq!(parsed.segments[7].kind, SegmentKind::Narration);
        assert_eq!(parsed.segments[7].src_text, "彼女は微笑んだ。");
        assert_eq!(parsed.segments[7].line_start, 15);
    }

    #[test]
    fn test_golden_chapter_parsing() {
        let raw_path = std::path::Path::new("../testdata/raw/02_chapter_normal.md");
        let golden_path = std::path::Path::new("../testdata/golden/raw_02_parsed.json");

        let content = std::fs::read_to_string(raw_path).unwrap();
        let golden_json = std::fs::read_to_string(golden_path).unwrap();

        let illustrations =
            IllustrationSet::from_names(["ill_scene.jpg", "ill_scene copy.webp", "ill_scene.psd"]);
        let options = ParseOptions::default();

        let parsed = parse_chapter("02_chapter_normal.md", &content, &illustrations, &options);
        let actual_json = serde_json::to_string_pretty(&parsed).unwrap();

        let expected_val: serde_json::Value = serde_json::from_str(&golden_json).unwrap();
        let actual_val: serde_json::Value = serde_json::from_str(&actual_json).unwrap();

        assert_eq!(actual_val, expected_val);
    }

    #[test]
    fn test_encoding_samples_decode_identically() {
        let text = "健太「行こうか」\n☆\n呆《あき》れた。\n";

        // 1. Plain UTF-8
        let utf8_bytes = text.as_bytes();
        let decoded_utf8 = decode_bytes(utf8_bytes).unwrap();

        // 2. UTF-8 with BOM
        let mut utf8_bom_bytes = vec![0xEF, 0xBB, 0xBF];
        utf8_bom_bytes.extend_from_slice(utf8_bytes);
        let decoded_utf8_bom = decode_bytes(&utf8_bom_bytes).unwrap();

        // 3. Shift_JIS (CP932)
        let (sjis_bytes, _, _) = encoding_rs::SHIFT_JIS.encode(text);
        let decoded_sjis = decode_bytes(&sjis_bytes).unwrap();

        assert_eq!(decoded_utf8, text);
        assert_eq!(decoded_utf8_bom, text);
        assert_eq!(decoded_sjis, text);
    }
}
