//! End-to-end run of the real fixture volumes with the echo mock provider.
//!
//! Ignored by default and skipped unless `LNTA_FIXTURE_DIR` points at the folder that holds
//! `Karamawari 1 ID` and `Karamawari 1 EN`. Works on a temp copy; never writes into the fixture.
//! Asserts structure and counts only, so no fixture text ends up in this file.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lnta_lib::docx::template::DocxPackage;
use lnta_lib::engine::events::RecordingEventEmitter;
use lnta_lib::engine::repair::ProviderResolver;
use lnta_lib::engine::run_engine;
use lnta_lib::engine::state::{RunControl, RunState};
use lnta_lib::error::AppError;
use lnta_lib::project::config::ProjectConfig;
use lnta_lib::project::layout::ProjectLayout;
use lnta_lib::project::settings::{EffectiveSettings, GlobalSettings, ModelReference};
use lnta_lib::provider::mock::MockProvider;
use lnta_lib::provider::AnyProvider;
use lnta_lib::store::repo::ChapterRepo;
use lnta_lib::store::{migrate, open_path};

struct EchoResolver(AnyProvider);

impl ProviderResolver for EchoResolver {
    fn resolve(&self, _reference: &ModelReference) -> Result<AnyProvider, AppError> {
        Ok(self.0.clone())
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let dest_child = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &dest_child)?;
        } else {
            fs::copy(entry.path(), &dest_child)?;
        }
    }
    Ok(())
}

fn fixture_volume(name: &str) -> Option<PathBuf> {
    let base = std::env::var("LNTA_FIXTURE_DIR").ok()?;
    let mut base = PathBuf::from(base);
    if base.is_relative() {
        // cargo runs integration tests from src-tauri/, the env var is usually repo-relative.
        let from_repo = Path::new("..").join(&base);
        if from_repo.exists() {
            base = from_repo;
        }
    }
    let dir = base.join(name);
    if dir.exists() {
        Some(dir)
    } else {
        panic!(
            "LNTA_FIXTURE_DIR is set but {} does not exist; refusing to skip silently",
            dir.display()
        );
    }
}

async fn run_volume(volume: &str, profile: &str) {
    let Some(src) = fixture_volume(volume) else {
        eprintln!("LNTA_FIXTURE_DIR not set; skipping {volume}");
        return;
    };
    let temp = std::env::temp_dir().join(format!("lnta_e2e_{}", uuid::Uuid::new_v4()));
    copy_dir_all(&src, &temp).unwrap();

    let layout = ProjectLayout::new(&temp);
    layout.ensure_layout(profile).unwrap();
    let mut conn = open_path(&layout.db_file).unwrap();
    migrate(&mut conn).unwrap();

    let settings = EffectiveSettings::merge(
        &GlobalSettings::default(),
        &ProjectConfig::for_profile(profile),
        None,
    );
    let mock = Arc::new(MockProvider::new_echo());
    let resolver = EchoResolver(AnyProvider::Mock(mock.clone()));
    let emitter = RecordingEventEmitter::new();
    let control = RunControl::new();

    let state = run_engine(&conn, &layout, &settings, &resolver, &emitter, &control)
        .await
        .unwrap();

    let chapters = ChapterRepo::new(&conn).list_all().unwrap();
    let statuses: Vec<String> = chapters
        .iter()
        .map(|c| format!("{}={}", c.prefix, c.status))
        .collect();
    println!(
        "[{volume}] state={state:?} chapters: {}",
        statuses.join(" ")
    );

    let calls = mock.calls();
    let first = calls.first().expect("at least one model call");
    let instruction_name = if profile == "ja-en" {
        "instruction-en.md"
    } else {
        "instruction.md"
    };
    let instruction = fs::read_to_string(temp.join(instruction_name)).unwrap();
    let ruby_marks = calls
        .iter()
        .map(|c| c.user_prompt.matches('《').count())
        .sum::<usize>();
    println!(
        "[{volume}] calls={} system_len={} instruction_included={} ruby_marks_sent={}",
        calls.len(),
        first.system_prompt.len(),
        first.system_prompt.contains(instruction.trim()),
        ruby_marks
    );

    let docx = fs::read(&layout.docx_output).unwrap();
    let pkg = DocxPackage::from_bytes(&docx).unwrap();
    let xml = pkg.get_text_entry("word/document.xml").unwrap();
    let headings = xml.matches("w:val=\"Heading1\"").count();
    let drawings = xml.matches("<w:drawing>").count();
    let page_breaks = xml.matches("<w:pageBreakBefore").count();
    let jp_brackets = xml.matches('「').count() + xml.matches('」').count();
    let placeholders_left = xml.matches("PlaceHolder").count();
    println!(
        "[{volume}] docx headings={headings} drawings={drawings} pageBreakBefore={page_breaks} jp_brackets={jp_brackets} placeholder_text={placeholders_left}"
    );

    let qa_lines: Vec<String> = emitter
        .events()
        .into_iter()
        .filter_map(|e| match e {
            lnta_lib::engine::events::EngineEvent::LogLine { message, .. }
                if message.starts_with("QA") =>
            {
                Some(message)
            }
            _ => None,
        })
        .collect();
    let finding_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM finding", [], |r| r.get(0))
        .unwrap();
    let segment_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM segment", [], |r| r.get(0))
        .unwrap();
    let qa_runs: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM run WHERE kind = 'qa' AND status = 'completed'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let mut roles = conn
        .prepare("SELECT role, COUNT(*) FROM call GROUP BY role ORDER BY role")
        .unwrap();
    let role_counts: Vec<String> = roles
        .query_map([], |r| {
            Ok(format!(
                "{}={}",
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?
            ))
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    println!(
        "[{volume}] segments={segment_count} findings={finding_count} qa_runs={qa_runs} calls_by_role={role_counts:?} qa_log={qa_lines:?}"
    );

    assert_eq!(state, RunState::Completed, "{volume}: run did not complete");
    assert!(segment_count > 0, "{volume}: segments were not persisted");
    assert_eq!(qa_runs, 1, "{volume}: QA phase did not run");
    assert_eq!(qa_lines.len(), 1, "{volume}: no QA summary line");
    assert_eq!(chapters.len(), 9);
    assert!(
        chapters.iter().all(|c| c.status == "done"),
        "{volume}: not every chapter is done"
    );
    assert!(
        first.system_prompt.contains(instruction.trim()),
        "{volume}: instruction file not in system prompt"
    );
    assert!(ruby_marks > 0, "{volume}: ruby was not normalized");
    assert_eq!(headings, 8, "{volume}: expected 8 chapter headings");
    assert_eq!(drawings, 18, "{volume}: expected 18 images");
    assert_eq!(placeholders_left, 0, "{volume}: placeholder text leaked");
    assert_eq!(
        jp_brackets, 0,
        "{volume}: Japanese brackets reached the docx"
    );

    let _ = fs::remove_dir_all(&temp);
}

#[tokio::test]
#[ignore]
async fn fixture_e2e_ja_id() {
    run_volume("Karamawari 1 ID", "ja-id").await;
}

#[tokio::test]
#[ignore]
async fn fixture_e2e_ja_en() {
    run_volume("Karamawari 1 EN", "ja-en").await;
}

#[test]
#[ignore]
fn probe_validation_reasons() {
    use lnta_lib::prompt::parse::parse_model_output;
    use lnta_lib::raw::classify::IllustrationSet;
    use lnta_lib::raw::segment::{parse_chapter, ParseOptions};
    use lnta_lib::validate::content::ContentValidationConfig;
    use lnta_lib::validate::validate_chunk;
    let Some(dir) = fixture_volume("Karamawari 1 ID") else {
        return;
    };
    let file = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .find(|n| n.starts_with("2_"))
        .unwrap();
    let content = fs::read_to_string(dir.join(&file)).unwrap();
    let ill = IllustrationSet::from_dir(&dir.join("illustrations"));
    let parsed = parse_chapter(
        &file,
        &content,
        &ill,
        &ParseOptions {
            strip_brackets: true,
            ruby_normalize: true,
            ..Default::default()
        },
    );
    let mut out = String::new();
    for s in &parsed.segments {
        match s.kind {
            lnta_lib::raw::SegmentKind::Title => out.push_str("<t>Judul</t>\n"),
            lnta_lib::raw::SegmentKind::Dialogue => {
                out.push_str(&format!("<s id=\"{}\">\"Dialog {}.\"</s>\n", s.idx, s.idx))
            }
            lnta_lib::raw::SegmentKind::Narration => out.push_str(&format!(
                "<s id=\"{}\">Narasi segmen {} yang cukup panjang untuk rasio.</s>\n",
                s.idx, s.idx
            )),
            _ => {}
        }
    }
    let mo = parse_model_output(&out, &[]).unwrap();
    let v = validate_chunk(
        &mo,
        &parsed.segments,
        &[],
        1,
        &ContentValidationConfig::default(),
    );
    println!(
        "segments={} valid={} issues={}",
        parsed.segments.len(),
        v.is_valid,
        v.repair_issues.len()
    );
    for i in v.repair_issues.iter().take(8) {
        println!("  id={} reason={}", i.id, i.reason);
    }
}
