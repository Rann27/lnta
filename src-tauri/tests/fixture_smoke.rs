//! Smoke test against local Fixture/ volumes per AGENTS.md §4 and task T19.
//!
//! Marked `#[ignore]` and skipped unless `LNTA_FIXTURE_DIR` is set.
//! Copies each `Fixture/Karamawari 1 *` folder to a temporary directory
//! (never modifying or writing into `Fixture/`), and asserts structural counts
//! without embedding any copyrighted text or images.

#![allow(clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use lnta_lib::docx::body::{RenderChapter, RenderItem};
use lnta_lib::docx::images::{parse_content_box_from_doc, scale_image_dimensions};
use lnta_lib::docx::media::add_image_to_package;
use lnta_lib::docx::styles::find_heading1_style_id;
use lnta_lib::docx::template::DocxPackage;
use lnta_lib::docx::write::render_and_write_docx;
use lnta_lib::project::discover::discover_chapters;
use lnta_lib::project::preflight::{run_preflight, PreflightEnvironment};
use lnta_lib::raw::classify::IllustrationSet;
use lnta_lib::raw::encoding::read_raw_file;
use lnta_lib::raw::ruby::normalize_ruby_line;
use lnta_lib::raw::segment::{parse_chapter, ParseOptions, SegmentKind};

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

fn run_smoke_for_volume(fixture_vol_dir: &Path, profile: &str) {
    if !fixture_vol_dir.exists() {
        eprintln!(
            "Fixture volume dir {} does not exist, skipping profile {}",
            fixture_vol_dir.display(),
            profile
        );
        return;
    }

    // 1. Copy to temp dir (NEVER write into Fixture/)
    let temp_dir = std::env::temp_dir().join(format!(
        "lnta_fixture_smoke_{}_{}",
        profile,
        uuid::Uuid::new_v4()
    ));
    copy_dir_recursive(fixture_vol_dir, &temp_dir).unwrap();

    // 2. Discover chapters: assert 9 chapters in order
    let chapters = discover_chapters(&temp_dir).unwrap();
    assert_eq!(chapters.len(), 9, "Expected exactly 9 chapters in volume");

    for (idx, ch) in chapters.iter().enumerate() {
        assert_eq!(
            ch.prefix_num,
            (idx + 1) as u32,
            "Chapters must be numerically ordered from 1 to 9"
        );
    }

    // 3. Parse chapters and inspect structural counts
    let illustrations_dir = temp_dir.join("illustrations");
    let illustrations = IllustrationSet::from_dir(&illustrations_dir);
    let parse_opts = ParseOptions::default();

    let mut total_image_segments = 0;
    let mut total_scene_segments = 0;
    let mut ruby_count_ch2_to_ch9 = 0;
    let mut parsed_chapters = Vec::new();

    for (idx, ch) in chapters.iter().enumerate() {
        let raw_content = read_raw_file(&ch.path).unwrap();

        // Ruby normalization count for chapters 2 to 9
        if idx > 0 {
            for line in raw_content.lines() {
                let (_norm, count) = normalize_ruby_line(line);
                if count > 0 {
                    ruby_count_ch2_to_ch9 += count;
                }
            }
        }

        let parsed = parse_chapter(&ch.file_name, &raw_content, &illustrations, &parse_opts);

        if idx == 0 {
            // Chapter 1 is an image-page chapter
            assert!(
                parsed.is_image_page,
                "Chapter 1 must be detected as an image-page chapter"
            );
            assert_eq!(
                parsed.title_raw, None,
                "Chapter 1 must have no title heading"
            );
            assert_eq!(parsed.title_clean, None);
        }

        for seg in &parsed.segments {
            match seg.kind {
                SegmentKind::Image => {
                    total_image_segments += 1;
                    if let Some(ref file) = seg.image_file {
                        assert!(
                            file.to_lowercase().ends_with(".jpg")
                                || file.to_lowercase().ends_with(".jpeg"),
                            "All image placeholders must resolve to .jpg files, found: {file}"
                        );
                    }
                }
                SegmentKind::Scene => {
                    total_scene_segments += 1;
                }
                _ => {}
            }
        }

        parsed_chapters.push(parsed);
    }

    // Assert structural counts per AGENTS.md §4
    assert_eq!(
        total_image_segments, 18,
        "Expected exactly 18 image segments in volume"
    );
    assert_eq!(
        total_scene_segments, 16,
        "Expected exactly 16 scene segments (☆) in volume"
    );
    assert!(
        ruby_count_ch2_to_ch9 > 0,
        "Expected ruby normalization count > 0 for chapters 2 to 9, found {}",
        ruby_count_ch2_to_ch9
    );

    // 4. Preflight check with --skip-model-check
    let env = PreflightEnvironment {
        skip_model_check: true,
        ..Default::default()
    };
    let report = run_preflight(&temp_dir, profile, &env);

    assert_eq!(
        report.blockers().len(),
        0,
        "Expected 0 preflight blockers with --skip-model-check, got: {:?}",
        report.blockers()
    );
    assert!(report.can_run, "Preflight report must allow run");

    // 5. Render with placeholder translations
    let template_path = temp_dir.join("Translations.docx");
    let template_bytes = fs::read(&template_path).unwrap();
    let mut package = DocxPackage::from_bytes(&template_bytes).unwrap();
    let doc_xml = package.get_text_entry("word/document.xml").unwrap();
    let content_box = parse_content_box_from_doc(&doc_xml);

    let mut rel_counter = 0;
    let mut doc_pr_counter = 0;
    let mut render_chapters = Vec::new();

    for (idx, ch) in parsed_chapters.iter().enumerate() {
        let title = if ch.is_image_page {
            None
        } else {
            Some(format!("Chapter {}", idx + 1))
        };

        let mut items = Vec::new();
        for seg in &ch.segments {
            match seg.kind {
                SegmentKind::Image => {
                    if let Some(ref img_name) = seg.image_file {
                        let img_path = illustrations_dir.join(img_name);
                        if let Ok(img_bytes) = fs::read(&img_path) {
                            rel_counter += 1;
                            doc_pr_counter += 1;
                            let (rel_id, _m_name, (w, h)) = add_image_to_package(
                                &mut package,
                                &img_bytes,
                                img_name,
                                rel_counter,
                            )
                            .unwrap();
                            let (cx, cy) = scale_image_dimensions(w, h, &content_box);
                            items.push(RenderItem::Image {
                                rel_id,
                                cx,
                                cy,
                                doc_pr_id: doc_pr_counter,
                            });
                        }
                    }
                }
                SegmentKind::Dialogue | SegmentKind::Narration => {
                    items.push(RenderItem::Text("Placeholder text line.".to_string()));
                }
                _ => {}
            }
        }

        render_chapters.push(RenderChapter {
            title,
            is_image_page: ch.is_image_page,
            items,
        });
    }

    let target_docx = temp_dir.join("Rendered_Translations.docx");
    let initial_pkg_bytes = package.to_bytes().unwrap();
    let _ =
        render_and_write_docx(&initial_pkg_bytes, &render_chapters, &target_docx, None).unwrap();

    // 6. Assert rendered docx contains exactly 8 headings and 18 images
    let file = fs::File::open(&target_docx).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();

    let doc_content = {
        let mut doc_file = zip.by_name("word/document.xml").unwrap();
        let mut s = String::new();
        std::io::Read::read_to_string(&mut doc_file, &mut s).unwrap();
        s
    };

    let heading_style = find_heading1_style_id(
        &package
            .get_text_entry("word/styles.xml")
            .unwrap_or_default(),
    );

    let mut reader = quick_xml::Reader::from_str(&doc_content);
    reader.config_mut().trim_text(true);

    let mut heading_count = 0;
    let mut drawing_count = 0;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "w:drawing" {
                    drawing_count += 1;
                }
            }
            Ok(quick_xml::events::Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "w:pStyle" {
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

    assert_eq!(
        heading_count, 8,
        "Expected exactly 8 headings in output docx (Chapter 1 has no heading)"
    );
    assert_eq!(
        drawing_count, 18,
        "Expected exactly 18 image drawings in output docx"
    );

    // Clean up temp dir
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
#[ignore]
fn test_fixture_smoke() {
    let fixture_dir = match std::env::var("LNTA_FIXTURE_DIR") {
        Ok(v) if !v.trim().is_empty() => PathBuf::from(v),
        _ => {
            eprintln!("LNTA_FIXTURE_DIR not set; skipping fixture smoke test.");
            return;
        }
    };

    // Test both Karamawari 1 ID and Karamawari 1 EN volumes
    let id_vol = fixture_dir.join("Karamawari 1 ID");
    let en_vol = fixture_dir.join("Karamawari 1 EN");

    run_smoke_for_volume(&id_vol, "ja-id");
    run_smoke_for_volume(&en_vol, "ja-en");
}
