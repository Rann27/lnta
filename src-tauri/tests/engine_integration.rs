//! Integration tests for the LNTA engine per PRD §10.7, Epic B, and task T26.
//!
//! Tests scripted execution on `testdata/project-min/`:
//! - Normal execution
//! - Missing segment repair
//! - Malformed tags tolerance
//! - Truncation splitting
//! - Refusal + fallback
//! - HTTP 429 retry
//! - Five consecutive failures pausing the run
//! - Crash-resume with no duplicate chapters in docx
//! - Fixture smoke with mock provider (#[ignore])

#![allow(clippy::unwrap_used, clippy::panic)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::Connection;

use lnta_lib::docx::template::DocxPackage;
use lnta_lib::engine::events::RecordingEventEmitter;
use lnta_lib::engine::repair::ProviderResolver;
use lnta_lib::engine::run_engine;
use lnta_lib::engine::state::{RunControl, RunState};
use lnta_lib::error::AppError;
use lnta_lib::project::config::ProjectConfig;
use lnta_lib::project::layout::ProjectLayout;
use lnta_lib::project::settings::{EffectiveSettings, GlobalSettings, ModelReference, Role};
use lnta_lib::provider::mock::{MockProvider, MockStep};
use lnta_lib::provider::retry::is_retryable_status;
use lnta_lib::provider::types::TokenUsage;
use lnta_lib::provider::AnyProvider;
use lnta_lib::store::repo::{CallRepo, ChapterRepo, GlossaryRepo, RunRepo, TranslationRepo};
use lnta_lib::store::{migrate, open_path};

/// Finds the project-min testdata directory whether running from workspace root or src-tauri.
fn get_project_min_dir() -> PathBuf {
    let p1 = Path::new("testdata/project-min");
    if p1.exists() {
        return p1.to_path_buf();
    }
    let p2 = Path::new("../testdata/project-min");
    if p2.exists() {
        return p2.to_path_buf();
    }
    p1.to_path_buf()
}

/// Recursively copies a directory tree to a target path.
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let dest_child = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir_all(&entry.path(), &dest_child)?;
        } else {
            fs::copy(entry.path(), &dest_child)?;
        }
    }
    Ok(())
}

/// Sets up an isolated copy of `testdata/project-min/` for integration testing.
fn setup_min_project() -> (PathBuf, ProjectLayout, Connection, EffectiveSettings) {
    let temp_dir = std::env::temp_dir().join(format!("lnta_test_eng_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).unwrap();
    let src = get_project_min_dir();
    copy_dir_all(&src, &temp_dir).unwrap();

    let layout = ProjectLayout::new(&temp_dir);
    layout.ensure_layout("ja-id").unwrap();

    let mut conn = open_path(&layout.db_file).unwrap();
    migrate(&mut conn).unwrap();

    let settings = EffectiveSettings::merge(
        &GlobalSettings::default(),
        &ProjectConfig::for_profile("ja-id"),
        None,
    );

    (temp_dir, layout, conn, settings)
}

/// Multi-model resolver supporting fallback mapping.
struct MultiModelResolver {
    providers: HashMap<String, AnyProvider>,
    default_provider: AnyProvider,
}

impl MultiModelResolver {
    fn new(default_provider: AnyProvider) -> Self {
        Self {
            providers: HashMap::new(),
            default_provider,
        }
    }

    fn register(&mut self, model_id: &str, provider: AnyProvider) {
        self.providers.insert(model_id.to_string(), provider);
    }
}

impl ProviderResolver for MultiModelResolver {
    fn resolve(&self, reference: &ModelReference) -> Result<AnyProvider, AppError> {
        if let Some(p) = self.providers.get(&reference.model_id) {
            Ok(p.clone())
        } else {
            Ok(self.default_provider.clone())
        }
    }
}

#[tokio::test]
async fn test_normal_run() {
    let (temp_dir, layout, conn, settings) = setup_min_project();

    let mock_xml = r#"<t>Bab 1: Perjalanan Dimulai</t>
<s id="1">Kenta berdiri dalam keheningan malam.</s>
<s id="2">"Misaki, apakah persiapanmu sudah selesai?"</s>
<s id="3">"Tentu saja, kapan pun kita bisa berangkat."</s>
<s id="5">Keduanya melangkah maju menyusuri jalan setapak di tengah hutan.</s>
<s id="7">Cahaya mentari pagi mulai menembus celah dedaunan.</s>
<glossary>
健太 === Kenta === laki-laki; protagonis
美咲 === Misaki === perempuan; rekan
</glossary>"#;

    let mock = Arc::new(MockProvider::new());
    mock.push_step(MockStep::simple_success(mock_xml));

    let resolver = MultiModelResolver::new(AnyProvider::Mock(mock.clone()));
    let emitter = RecordingEventEmitter::new();
    let run_control = RunControl::new();

    let state = run_engine(&conn, &layout, &settings, &resolver, &emitter, &run_control)
        .await
        .unwrap();

    assert_eq!(state, RunState::Completed);

    // Verify Chapter 1 is image-page chapter (done with 0 model calls)
    let ch1 = ChapterRepo::new(&conn)
        .get_by_id("chap_01")
        .unwrap()
        .unwrap();
    assert_eq!(ch1.status, "done");

    // Verify Chapter 2 is translated and title target is saved
    let ch2 = ChapterRepo::new(&conn)
        .get_by_id("chap_02")
        .unwrap()
        .unwrap();
    assert_eq!(ch2.status, "done");
    assert_eq!(ch2.title_tgt.as_deref(), Some("Bab 1: Perjalanan Dimulai"));

    // Verify Translations table in SQLite has segments 1, 2, 3, 5, 7
    let trans = TranslationRepo::new(&conn)
        .get_current_by_chapter(&ch2.id)
        .unwrap();
    assert_eq!(trans.len(), 5);
    assert_eq!(trans[0].seg_from, 1);
    assert_eq!(trans[4].seg_to, 7);

    // Verify Translations.docx was generated
    assert!(layout.docx_output.exists());
    let docx_bytes = fs::read(&layout.docx_output).unwrap();
    let pkg = DocxPackage::from_bytes(&docx_bytes).unwrap();
    let doc_xml = pkg.get_text_entry("word/document.xml").unwrap();
    assert!(doc_xml.contains("Bab 1: Perjalanan Dimulai"));

    // Verify glossary entries persisted to disk and SQLite
    let glo_entries = GlossaryRepo::new(&conn).list_all().unwrap();
    assert_eq!(glo_entries.len(), 2);
    let disk_gloss = fs::read_to_string(&layout.glossary_file).unwrap();
    assert!(disk_gloss.contains("健太 === Kenta"));
    assert!(disk_gloss.contains("美咲 === Misaki"));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_missing_segment_repair() {
    let (temp_dir, layout, conn, settings) = setup_min_project();

    // Call 1: Missing segment 5
    let call1_xml = r#"<t>Bab 1: Perjalanan Dimulai</t>
<s id="1">Kenta berdiri dalam keheningan malam.</s>
<s id="2">"Misaki, apakah persiapanmu sudah selesai?"</s>
<s id="3">"Tentu saja, kapan pun kita bisa berangkat."</s>
<s id="7">Cahaya mentari pagi mulai menembus celah dedaunan.</s>"#;

    // Call 2: Repair call supplying segment 5
    let call2_repair_xml =
        r#"<s id="5">Keduanya melangkah maju menyusuri jalan setapak di tengah hutan.</s>"#;

    let mock = Arc::new(MockProvider::new());
    mock.push_step(MockStep::simple_success(call1_xml));
    mock.push_step(MockStep::simple_success(call2_repair_xml));

    let resolver = MultiModelResolver::new(AnyProvider::Mock(mock.clone()));
    let emitter = RecordingEventEmitter::new();
    let run_control = RunControl::new();

    let state = run_engine(&conn, &layout, &settings, &resolver, &emitter, &run_control)
        .await
        .unwrap();

    assert_eq!(state, RunState::Completed);

    let ch2 = ChapterRepo::new(&conn)
        .get_by_id("chap_02")
        .unwrap()
        .unwrap();
    assert_eq!(ch2.status, "done");

    // Verify repaired segment 5 is present in translations
    let trans = TranslationRepo::new(&conn)
        .get_current_by_chapter(&ch2.id)
        .unwrap();
    let has_seg_5 = trans
        .iter()
        .any(|t| t.seg_from == 5 && t.text.contains("hutan"));
    assert!(has_seg_5, "Segment 5 should be present after repair");

    // Exactly 2 calls made (1 translator call, 1 repair call)
    assert_eq!(mock.calls().len(), 2);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_malformed_tags_tolerance() {
    let (temp_dir, layout, conn, settings) = setup_min_project();

    // Model response with unclosed tags and curly quotes
    let malformed_xml = r#"<t>Bab 1: Perjalanan Dimulai
<s id="1">Kenta berdiri dalam keheningan malam.
<s id="2">“Misaki, apakah persiapanmu sudah selesai?”</s>
<s id="3">“Tentu saja, kapan pun kita bisa berangkat.”</s>
<s id="5">Keduanya melangkah maju menyusuri jalan setapak di tengah hutan.</s>
<s id="7">Cahaya mentari pagi mulai menembus celah dedaunan."#;

    let mock = Arc::new(MockProvider::new());
    mock.push_step(MockStep::simple_success(malformed_xml));

    let resolver = MultiModelResolver::new(AnyProvider::Mock(mock.clone()));
    let emitter = RecordingEventEmitter::new();
    let run_control = RunControl::new();

    let state = run_engine(&conn, &layout, &settings, &resolver, &emitter, &run_control)
        .await
        .unwrap();

    assert_eq!(state, RunState::Completed);

    let ch2 = ChapterRepo::new(&conn)
        .get_by_id("chap_02")
        .unwrap()
        .unwrap();
    assert_eq!(ch2.status, "done");

    let trans = TranslationRepo::new(&conn)
        .get_current_by_chapter(&ch2.id)
        .unwrap();
    assert_eq!(trans.len(), 5);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_truncation_splitting() {
    let (temp_dir, layout, conn, settings) = setup_min_project();

    // Subpart A (first half: title, 1, 2, 3)
    let subpart_a = r#"<t>Bab 1: Perjalanan Dimulai</t>
<s id="1">Kenta berdiri dalam keheningan malam.</s>
<s id="2">"Misaki, apakah persiapanmu sudah selesai?"</s>
<s id="3">"Tentu saja, kapan pun kita bisa berangkat."</s>"#;

    // Subpart B (second half: 5, 7)
    let subpart_b = r#"<s id="5">Keduanya melangkah maju menyusuri jalan setapak di tengah hutan.</s>
<s id="7">Cahaya mentari pagi mulai menembus celah dedaunan.</s>"#;

    let mock = Arc::new(MockProvider::new());
    mock.push_step(MockStep::LengthTruncated {
        text: "<t>Bab 1: Perjalanan Dimulai</t>\n<s id=\"1\">Kenta berdiri".to_string(),
        usage: TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            ..Default::default()
        },
    });
    mock.push_step(MockStep::simple_success(subpart_a));
    mock.push_step(MockStep::simple_success(subpart_b));

    let resolver = MultiModelResolver::new(AnyProvider::Mock(mock.clone()));
    let emitter = RecordingEventEmitter::new();
    let run_control = RunControl::new();

    let state = run_engine(&conn, &layout, &settings, &resolver, &emitter, &run_control)
        .await
        .unwrap();

    assert_eq!(state, RunState::Completed);

    let ch2 = ChapterRepo::new(&conn)
        .get_by_id("chap_02")
        .unwrap()
        .unwrap();
    assert_eq!(ch2.status, "done");

    // Verify all segments covered without seam gaps
    let trans = TranslationRepo::new(&conn)
        .get_current_by_chapter(&ch2.id)
        .unwrap();
    assert_eq!(trans.len(), 5);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_refusal_and_fallback() {
    let (temp_dir, layout, conn, mut settings) = setup_min_project();

    // Configure role with primary and fallback model references
    let primary_ref = ModelReference {
        provider: "mock".to_string(),
        model_id: "model-primary".to_string(),
    };
    let fallback_ref = ModelReference {
        provider: "mock".to_string(),
        model_id: "model-fallback".to_string(),
    };

    let mut role_cfg = settings.roles.for_role(Role::Translator).clone();
    role_cfg.primary = primary_ref.clone();
    role_cfg.fallbacks = vec![fallback_ref.clone()];
    settings.roles.translator = role_cfg;

    // Primary mock: refuses
    let primary_mock = Arc::new(MockProvider::new());
    primary_mock.push_step(MockStep::Refused {
        reason: "Content safety policy refusal".to_string(),
    });

    // Fallback mock: succeeds
    let valid_xml = r#"<t>Bab 1: Perjalanan Dimulai</t>
<s id="1">Kenta berdiri dalam keheningan malam.</s>
<s id="2">"Misaki, apakah persiapanmu sudah selesai?"</s>
<s id="3">"Tentu saja, kapan pun kita bisa berangkat."</s>
<s id="5">Keduanya melangkah maju menyusuri jalan setapak di tengah hutan.</s>
<s id="7">Cahaya mentari pagi mulai menembus celah dedaunan.</s>"#;
    let fallback_mock = Arc::new(MockProvider::new());
    fallback_mock.push_step(MockStep::simple_success(valid_xml));

    let mut resolver = MultiModelResolver::new(AnyProvider::Mock(primary_mock.clone()));
    resolver.register("model-primary", AnyProvider::Mock(primary_mock.clone()));
    resolver.register("model-fallback", AnyProvider::Mock(fallback_mock.clone()));

    let emitter = RecordingEventEmitter::new();
    let run_control = RunControl::new();

    let state = run_engine(&conn, &layout, &settings, &resolver, &emitter, &run_control)
        .await
        .unwrap();

    assert_eq!(state, RunState::Completed);

    // In DB calls table: primary call failed with Refusal, fallback call succeeded
    let calls = CallRepo::new(&conn).list_by_chapter("chap_02").unwrap();
    assert!(calls.iter().any(|c| c.model == "model-primary"
        && c.status == "failed"
        && c.finish_reason.as_deref() == Some("Refusal")));
    assert!(calls
        .iter()
        .any(|c| c.model == "model-fallback" && c.status == "success"));

    let ch2 = ChapterRepo::new(&conn)
        .get_by_id("chap_02")
        .unwrap()
        .unwrap();
    assert_eq!(ch2.status, "done");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_http_429_retry() {
    // Assert 429 is recognized as retryable status per FR-PRV-07
    assert!(is_retryable_status(429));
    assert!(is_retryable_status(500));
    assert!(is_retryable_status(503));
    assert!(!is_retryable_status(400));
    assert!(!is_retryable_status(401));

    let (temp_dir, layout, conn, settings) = setup_min_project();

    let valid_xml = r#"<t>Bab 1: Perjalanan Dimulai</t>
<s id="1">Kenta berdiri dalam keheningan malam.</s>
<s id="2">"Misaki, apakah persiapanmu sudah selesai?"</s>
<s id="3">"Tentu saja, kapan pun kita bisa berangkat."</s>
<s id="5">Keduanya melangkah maju menyusuri jalan setapak di tengah hutan.</s>
<s id="7">Cahaya mentari pagi mulai menembus celah dedaunan.</s>"#;

    let mock = Arc::new(MockProvider::new());
    // Simulate initial rate-limit refusal, followed by success
    mock.push_step(MockStep::Refused {
        reason: "Rate limited".to_string(),
    });
    mock.push_step(MockStep::simple_success(valid_xml));

    // Provide fallback model reference so retry/fallback mechanism catches it
    let mut role_cfg = settings.roles.for_role(Role::Translator).clone();
    role_cfg.fallbacks = vec![role_cfg.primary.clone()];
    let mut settings_with_fallback = settings.clone();
    settings_with_fallback.roles.translator = role_cfg;

    let resolver = MultiModelResolver::new(AnyProvider::Mock(mock.clone()));
    let emitter = RecordingEventEmitter::new();
    let run_control = RunControl::new();

    let state = run_engine(
        &conn,
        &layout,
        &settings_with_fallback,
        &resolver,
        &emitter,
        &run_control,
    )
    .await
    .unwrap();

    assert_eq!(state, RunState::Completed);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_five_consecutive_failures_pause_run() {
    let (temp_dir, layout, conn, settings) = setup_min_project();

    // Populate extra chapters to test 5 consecutive chapter failures
    for i in 3..=7 {
        let ch_file = format!("0{}_ch{}.md", i, i);
        fs::write(
            layout.root.join(&ch_file),
            format!("# 【第{}話】\n本文です。\n", i),
        )
        .unwrap();
    }

    // Provider that always fails with network error
    let mock = Arc::new(MockProvider::new());
    for _ in 0..10 {
        mock.push_step(MockStep::Error(AppError::ProviderFailed {
            reason: "500 Server Error".to_string(),
        }));
    }

    let resolver = MultiModelResolver::new(AnyProvider::Mock(mock.clone()));
    let emitter = RecordingEventEmitter::new();
    let run_control = RunControl::new();

    let state = run_engine(&conn, &layout, &settings, &resolver, &emitter, &run_control)
        .await
        .unwrap();

    // Run paused per FR-PRV-08
    assert_eq!(state, RunState::Paused);

    let latest_run = RunRepo::new(&conn).get_latest().unwrap().unwrap();
    assert_eq!(latest_run.status, "paused");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_crash_resume_no_duplicate_chapters() {
    let (temp_dir, layout, conn, settings) = setup_min_project();

    let valid_xml = r#"<t>Bab 1: Perjalanan Dimulai</t>
<s id="1">Kenta berdiri dalam keheningan malam.</s>
<s id="2">"Misaki, apakah persiapanmu sudah selesai?"</s>
<s id="3">"Tentu saja, kapan pun kita bisa berangkat."</s>
<s id="5">Keduanya melangkah maju menyusuri jalan setapak di tengah hutan.</s>
<s id="7">Cahaya mentari pagi mulai menembus celah dedaunan.</s>"#;

    let mock = Arc::new(MockProvider::new());
    mock.push_step(MockStep::simple_success(valid_xml));

    let resolver = MultiModelResolver::new(AnyProvider::Mock(mock.clone()));
    let emitter = RecordingEventEmitter::new();
    let run_control = RunControl::new();

    // 1. Initial run completes successfully
    let state = run_engine(&conn, &layout, &settings, &resolver, &emitter, &run_control)
        .await
        .unwrap();
    assert_eq!(state, RunState::Completed);

    // Verify docx exists
    assert!(layout.docx_output.exists());
    let docx1 = fs::read(&layout.docx_output).unwrap();
    let pkg1 = DocxPackage::from_bytes(&docx1).unwrap();
    let xml1 = pkg1.get_text_entry("word/document.xml").unwrap();

    // Count occurrences of Chapter 2 heading in docx
    let count1 = xml1.matches("Bab 1: Perjalanan Dimulai").count();
    assert_eq!(count1, 1, "Heading should appear exactly once");

    // 2. Simulate reopen / resume: open a fresh DB connection and re-run
    drop(conn);
    let mut conn2 = open_path(&layout.db_file).unwrap();
    migrate(&mut conn2).unwrap();

    let emitter2 = RecordingEventEmitter::new();
    let run_control2 = RunControl::new();

    let state2 = run_engine(
        &conn2,
        &layout,
        &settings,
        &resolver,
        &emitter2,
        &run_control2,
    )
    .await
    .unwrap();
    assert_eq!(state2, RunState::Completed);

    // Verify Translations.docx still has NO duplicate chapters
    let docx2 = fs::read(&layout.docx_output).unwrap();
    let pkg2 = DocxPackage::from_bytes(&docx2).unwrap();
    let xml2 = pkg2.get_text_entry("word/document.xml").unwrap();
    let count2 = xml2.matches("Bab 1: Perjalanan Dimulai").count();
    assert_eq!(
        count2, 1,
        "Resumed docx must not duplicate chapter headings"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
#[ignore]
async fn test_fixture_smoke_mock_provider() {
    let fixture_env = std::env::var("LNTA_FIXTURE_DIR").ok();
    if fixture_env.is_none() {
        eprintln!("LNTA_FIXTURE_DIR not set; skipping fixture smoke test");
        return;
    }
    let fixture_path = Path::new(&fixture_env.unwrap()).join("Karamawari 1 ID");
    if !fixture_path.exists() {
        eprintln!(
            "Fixture directory {} not found; skipping",
            fixture_path.display()
        );
        return;
    }

    // 1. Copy to temp directory (never mutate Fixture/)
    let temp_dir = std::env::temp_dir().join(format!("lnta_test_fix_eng_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).unwrap();
    copy_dir_all(&fixture_path, &temp_dir).unwrap();

    let layout = ProjectLayout::new(&temp_dir);
    layout.ensure_layout("ja-id").unwrap();

    let mut conn = open_path(&layout.db_file).unwrap();
    migrate(&mut conn).unwrap();

    let settings = EffectiveSettings::merge(
        &GlobalSettings::default(),
        &ProjectConfig::for_profile("ja-id"),
        None,
    );

    // Provide dynamic mock echoing placeholder text per segment
    // With large number of pre-queued steps
    let mock = Arc::new(MockProvider::new());
    for i in 1..=100 {
        let resp = format!(
            "<t>Bab Terjemahan {}</t>\n<s id=\"1\">Teks terjemahan 1</s>\n<s id=\"2\">Teks terjemahan 2</s>",
            i
        );
        mock.push_step(MockStep::simple_success(&resp));
    }

    let resolver = MultiModelResolver::new(AnyProvider::Mock(mock.clone()));
    let emitter = RecordingEventEmitter::new();
    let run_control = RunControl::new();

    let state = run_engine(&conn, &layout, &settings, &resolver, &emitter, &run_control)
        .await
        .unwrap();

    assert_eq!(state, RunState::Completed);

    // Assert all 9 chapters reach done
    let chapters = ChapterRepo::new(&conn).list_all().unwrap();
    assert_eq!(chapters.len(), 9);
    for ch in chapters {
        assert_eq!(ch.status, "done", "Chapter {} should be done", ch.file_name);
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_budget_pause_with_mock_provider() {
    let (temp_dir, layout, conn, mut settings) = setup_min_project();
    // Set a very low budget limit so that a single call exceeds it
    settings.budget_token_limit = Some(15);

    let mock_xml = r#"<t>Bab 1: Perjalanan Dimulai</t>
<s id="1">Kenta berdiri dalam keheningan malam.</s>
<s id="2">"Misaki, apakah persiapanmu sudah selesai?"</s>
<s id="3">"Tentu saja, kapan pun kita bisa berangkat."</s>
<s id="5">Keduanya melangkah maju menyusuri jalan setapak di tengah hutan.</s>
<s id="7">Cahaya mentari pagi mulai menembus celah dedaunan.</s>"#;

    let mock = Arc::new(MockProvider::new());
    // Create a step reporting 20 input + 10 output = 30 tokens (> 15 limit)
    let step = MockStep::Success {
        text: mock_xml.to_string(),
        finish: lnta_lib::provider::types::FinishReason::Stop,
        usage: TokenUsage {
            input_tokens: 20,
            output_tokens: 10,
            cached_input_tokens: 0,
            cache_write_tokens: 0,
            reasoning_tokens: 0,
        },
    };
    mock.push_step(step);

    let resolver = MultiModelResolver::new(AnyProvider::Mock(mock.clone()));
    let emitter = RecordingEventEmitter::new();
    let run_control = RunControl::new();

    let state = run_engine(&conn, &layout, &settings, &resolver, &emitter, &run_control)
        .await
        .unwrap();

    // Verify engine paused on budget exceeded
    assert_eq!(state, RunState::Paused);

    // Verify warning log event was emitted
    let events = emitter.events();
    let has_budget_log = events.iter().any(|e| match e {
        lnta_lib::engine::events::EngineEvent::LogLine { message, .. } => {
            message.contains("batas anggaran")
        }
        _ => false,
    });
    assert!(has_budget_log, "Expected budget warning log event");

    let _ = fs::remove_dir_all(&temp_dir);
}
