//! LNTA run engine per FR-RUN-01..08, PRD §5.3, §5.5, and Epic B.
//!
//! Coordinates sequential chapter translation, chunk planning, prompt assembly,
//! validation, repair loop, glossary proposals, docx rendering, and resume.

pub mod chapter;
pub mod events;
pub mod repair;
pub mod state;

pub use chapter::{execute_single_chapter, ChapterExecutionResult};
pub use events::{EngineEvent, EventEmitter, RecordingEventEmitter};
pub use repair::{execute_repair_loop, ProviderResolver, RepairLoopOutcome};
pub use state::{ChapterStatus, RunControl, RunState};

use rusqlite::Connection;

use crate::error::AppError;
use crate::profiles::load_profile;
use crate::project::discover::discover_chapters;
use crate::project::layout::ProjectLayout;
use crate::project::preflight::{run_preflight, PreflightEnvironment};
use crate::project::settings::EffectiveSettings;
use crate::raw::classify::IllustrationSet;
use crate::raw::encoding::read_raw_file;
use crate::raw::segment::{parse_chapter, ParseOptions};
use crate::store::repo::{CallRepo, Chapter, ChapterRepo, Run, RunRepo};

/// Runs the translation pipeline across all chapters of the project.
pub async fn run_engine(
    conn: &Connection,
    layout: &ProjectLayout,
    settings: &EffectiveSettings,
    provider_resolver: &dyn ProviderResolver,
    event_emitter: &dyn EventEmitter,
    run_control: &RunControl,
) -> Result<RunState, AppError> {
    let run_repo = RunRepo::new(conn);
    let chapter_repo = ChapterRepo::new(conn);

    // 1. Run preflight checks
    let preflight_env = PreflightEnvironment::default();
    let preflight = run_preflight(&layout.root, &settings.profile, &preflight_env);
    if !preflight.can_run {
        event_emitter.emit(EngineEvent::RunState {
            state: "failed".to_string(),
            run_id: None,
        });
        return Err(AppError::Internal {
            message: "Preflight blocked run execution".to_string(),
        });
    }

    // 2. Load profile contracts and instruction text
    let profile = load_profile(&settings.profile)?;
    let contract_text = &profile.contract_text;
    let instruction_path = layout.root.join(
        settings
            .instruction_file
            .as_deref()
            .unwrap_or("instruction.md"),
    );
    let instruction_text = std::fs::read_to_string(&instruction_path).unwrap_or_default();

    // 3. Initialize or resume Run in SQLite
    let run_id = format!("run_{}", uuid::Uuid::new_v4().simple());
    let run_record = Run {
        id: run_id.clone(),
        kind: "translation".to_string(),
        status: "running".to_string(),
        started_at: chrono::Utc::now().to_rfc3339(),
        ended_at: None,
        settings_snapshot: serde_json::to_string(settings).unwrap_or_default(),
    };
    run_repo.insert(&run_record)?;

    event_emitter.emit(EngineEvent::RunState {
        state: "running".to_string(),
        run_id: Some(run_id.clone()),
    });

    // 4. Ensure chapters are populated in SQLite in numerical prefix order
    let discovered = discover_chapters(&layout.root)?;
    let illustrations = IllustrationSet::from_dir(&layout.illustrations_dir);
    let options = ParseOptions {
        strip_brackets: settings.strip_title_brackets,
        ruby_normalize: settings.normalize_ruby,
        ..Default::default()
    };
    for (ordinal, dc) in discovered.iter().enumerate() {
        if chapter_repo.get_by_prefix(&dc.prefix)?.is_none() {
            let chapter_path = layout.root.join(&dc.file_name);
            let content = read_raw_file(&chapter_path)?;
            let parsed = parse_chapter(&dc.file_name, &content, &illustrations, &options);

            let ch = Chapter {
                id: format!("chap_{}", dc.prefix),
                ordinal: (ordinal + 1) as i32,
                file_name: dc.file_name.clone(),
                prefix: dc.prefix.clone(),
                title_src: parsed.title_clean.or(parsed.title_raw).unwrap_or_default(),
                title_tgt: None,
                src_hash: "initial_hash".to_string(),
                line_count: parsed.line_count,
                char_count: parsed.char_count,
                status: "pending".to_string(),
                frozen: false,
                parts_planned: 1,
                updated_at: chrono::Utc::now().to_rfc3339(),
            };
            chapter_repo.insert(&ch)?;
        }
    }

    // 5. Process chapters sequentially (FR-RUN-03, FR-RUN-06)
    let all_chapters = chapter_repo.list_all()?;

    let mut consecutive_failures = 0usize;
    for ch in &all_chapters {
        if ch.status == "done" || ch.frozen {
            continue;
        }

        if run_control.should_stop_now() || run_control.should_pause() {
            run_repo.update_status(&run_id, "paused")?;
            event_emitter.emit(EngineEvent::RunState {
                state: "paused".to_string(),
                run_id: Some(run_id),
            });
            return Ok(RunState::Paused);
        }

        // Check budget limits before starting next chapter per FR-RUN-10 and Story H2
        let call_repo = CallRepo::new(conn);
        let calls_so_far = call_repo.list_by_run(&run_id).unwrap_or_default();
        let total_run_tokens: u64 = calls_so_far
            .iter()
            .map(|c| (c.in_tokens + c.out_tokens) as u64)
            .sum();
        let total_run_cost: f64 = calls_so_far.iter().filter_map(|c| c.cost).sum();

        if let Some(reason) = crate::usage::check_budget(settings, total_run_tokens, total_run_cost)
        {
            let msg = match reason {
                crate::usage::BudgetExceeded::TokenLimit { limit, current } => {
                    format!(
                        "Run dijeda: batas anggaran {limit} token tercapai (terpakai: {current})"
                    )
                }
                crate::usage::BudgetExceeded::CostLimit { limit, current } => {
                    format!(
                        "Run dijeda: batas anggaran ${limit:.2} tercapai (terpakai: ${current:.2})"
                    )
                }
            };
            run_repo.update_status(&run_id, "paused")?;
            event_emitter.emit(EngineEvent::LogLine {
                level: "warn".to_string(),
                message: msg,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
            event_emitter.emit(EngineEvent::RunState {
                state: "paused".to_string(),
                run_id: Some(run_id),
            });
            return Ok(RunState::Paused);
        }

        let result = execute_single_chapter(
            conn,
            layout,
            settings,
            contract_text,
            &instruction_text,
            ch,
            provider_resolver,
            event_emitter,
            run_control,
            Some(&run_id),
        )
        .await;

        match result {
            Ok(ChapterExecutionResult::Done { .. }) => {
                consecutive_failures = 0;
            }
            Ok(ChapterExecutionResult::NeedsAttention { .. }) => {
                consecutive_failures += 1;
                if consecutive_failures >= 5 {
                    run_repo.update_status(&run_id, "paused")?;
                    event_emitter.emit(EngineEvent::RunState {
                        state: "paused".to_string(),
                        run_id: Some(run_id),
                    });
                    event_emitter.emit(EngineEvent::LogLine {
                        level: "error".to_string(),
                        message: "Run dijeda: provider gagal 5 kali berturut-turut".to_string(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    });
                    return Ok(RunState::Paused);
                }
            }
            Ok(ChapterExecutionResult::Paused) => {
                run_repo.update_status(&run_id, "paused")?;
                event_emitter.emit(EngineEvent::RunState {
                    state: "paused".to_string(),
                    run_id: Some(run_id),
                });
                return Ok(RunState::Paused);
            }
            Err(e) => {
                consecutive_failures += 1;
                let _ = chapter_repo.update_status(&ch.id, "needs_attention");
                if consecutive_failures >= 5 {
                    run_repo.update_status(&run_id, "paused")?;
                    event_emitter.emit(EngineEvent::RunState {
                        state: "paused".to_string(),
                        run_id: Some(run_id),
                    });
                    event_emitter.emit(EngineEvent::LogLine {
                        level: "error".to_string(),
                        message: format!("Run dijeda: provider gagal 5 kali berturut-turut: {e}"),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    });
                    return Ok(RunState::Paused);
                }
            }
        }
    }

    // 6. Complete Run
    run_repo.update_status(&run_id, "completed")?;
    event_emitter.emit(EngineEvent::RunState {
        state: "completed".to_string(),
        run_id: Some(run_id),
    });

    Ok(RunState::Completed)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Arc;

    use crate::docx::create_minimal_template;
    use crate::project::settings::ModelReference;
    use crate::provider::mock::{MockProvider, MockStep};
    use crate::provider::AnyProvider;
    use crate::store::open_in_memory;
    use crate::store::repo::GlossaryRepo;

    struct SimpleResolver {
        provider: AnyProvider,
    }

    impl ProviderResolver for SimpleResolver {
        fn resolve(&self, _reference: &ModelReference) -> Result<AnyProvider, AppError> {
            Ok(self.provider.clone())
        }
    }

    #[tokio::test]
    async fn test_image_page_chapter_completes_without_model_calls() {
        let conn = open_in_memory().unwrap();
        let temp_dir =
            std::env::temp_dir().join(format!("lnta_test_engine_img_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let layout = ProjectLayout::new(&temp_dir);
        layout.ensure_layout("ja-id").unwrap();

        // Write an image-page chapter (chapter 1 with placeholders only, no heading)
        let ch1_path = layout.root.join("1_images.md");
        fs::write(
            &ch1_path,
            "Image PlaceHolder: cover.jpg\nIllustration Placeholder: p001.jpg\n☆\n",
        )
        .unwrap();

        let chapter = Chapter {
            id: "chap_1".to_string(),
            ordinal: 1,
            file_name: "1_images.md".to_string(),
            prefix: "1".to_string(),
            title_src: "".to_string(),
            title_tgt: None,
            src_hash: "hash".to_string(),
            line_count: 3,
            char_count: 50,
            status: "pending".to_string(),
            frozen: false,
            parts_planned: 0,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        ChapterRepo::new(&conn).insert(&chapter).unwrap();

        let mock_provider = Arc::new(MockProvider::new());
        let resolver = SimpleResolver {
            provider: AnyProvider::Mock(mock_provider.clone()),
        };
        let emitter = RecordingEventEmitter::new();
        let run_control = RunControl::new();

        let mut settings = EffectiveSettings::merge(
            &crate::project::settings::GlobalSettings::default(),
            &crate::project::config::ProjectConfig::for_profile("ja-id"),
            None,
        );
        settings.target_language = "id".to_string();

        let result = execute_single_chapter(
            &conn,
            &layout,
            &settings,
            "Contract",
            "Instruction",
            &chapter,
            &resolver,
            &emitter,
            &run_control,
            None,
        )
        .await
        .unwrap();

        assert_eq!(
            result,
            ChapterExecutionResult::Done {
                parts_count: 0,
                new_glossary_terms: 0
            }
        );

        // Assert 0 calls were made to the model
        assert_eq!(mock_provider.calls().len(), 0);

        // Assert chapter in DB is marked "done"
        let chap_db = ChapterRepo::new(&conn)
            .get_by_id("chap_1")
            .unwrap()
            .unwrap();
        assert_eq!(chap_db.status, "done");

        let events = emitter.events();
        let has_done_status = events.iter().any(|e| match e {
            EngineEvent::ChapterStatus { status, .. } => status == "done",
            _ => false,
        });
        assert!(has_done_status);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_chapter_translation_with_mock_and_glossary_merge() {
        let conn = open_in_memory().unwrap();
        let temp_dir =
            std::env::temp_dir().join(format!("lnta_test_engine_text_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let layout = ProjectLayout::new(&temp_dir);
        layout.ensure_layout("ja-id").unwrap();

        // Write template docx
        let template_docx = create_minimal_template("Heading1").to_bytes().unwrap();
        fs::write(&layout.template_file, template_docx).unwrap();

        // Write instruction file
        fs::write(layout.root.join("instruction.md"), "# Panduan").unwrap();

        // Write chapter with heading and text
        let ch_path = layout.root.join("02_chap.md");
        fs::write(
            &ch_path,
            "# 【第1話　出会い】\n朝、健太は美咲に出会った。\n「おはよう」\n",
        )
        .unwrap();

        let chapter = Chapter {
            id: "chap_2".to_string(),
            ordinal: 2,
            file_name: "02_chap.md".to_string(),
            prefix: "02".to_string(),
            title_src: "第1話　出会い".to_string(),
            title_tgt: None,
            src_hash: "hash".to_string(),
            line_count: 3,
            char_count: 30,
            status: "pending".to_string(),
            frozen: false,
            parts_planned: 1,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        ChapterRepo::new(&conn).insert(&chapter).unwrap();

        let run_record = Run {
            id: "run_123".to_string(),
            kind: "translation".to_string(),
            status: "running".to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            ended_at: None,
            settings_snapshot: "{}".to_string(),
        };
        RunRepo::new(&conn).insert(&run_record).unwrap();

        // Script mock response with translated segments and a glossary proposal for 健太
        let mock_response = r#"<t>Bab 1: Pertemuan</t>
<s id="1">Pagi itu, Kenta bertemu dengan Misaki.</s>
<s id="2">"Selamat pagi."</s>
<glossary>
健太 === Kenta === laki-laki; tokoh utama
</glossary>"#;

        let mock_provider = Arc::new(MockProvider::new());
        mock_provider.push_step(MockStep::simple_success(mock_response));

        let resolver = SimpleResolver {
            provider: AnyProvider::Mock(mock_provider.clone()),
        };
        let emitter = RecordingEventEmitter::new();
        let run_control = RunControl::new();

        let settings = EffectiveSettings::merge(
            &crate::project::settings::GlobalSettings::default(),
            &crate::project::config::ProjectConfig::for_profile("ja-id"),
            None,
        );

        let result = execute_single_chapter(
            &conn,
            &layout,
            &settings,
            "Contract",
            "Instruction",
            &chapter,
            &resolver,
            &emitter,
            &run_control,
            Some("run_123"),
        )
        .await
        .unwrap();

        assert_eq!(
            result,
            ChapterExecutionResult::Done {
                parts_count: 1,
                new_glossary_terms: 1
            }
        );

        // Check model was called exactly once
        assert_eq!(mock_provider.calls().len(), 1);

        // Check chapter status is done and target title is saved
        let chap_db = ChapterRepo::new(&conn)
            .get_by_id("chap_2")
            .unwrap()
            .unwrap();
        assert_eq!(chap_db.status, "done");
        assert_eq!(chap_db.title_tgt.as_deref(), Some("Bab 1: Pertemuan"));

        // Check glossary entry was written to disk and SQLite
        let glo_entries = GlossaryRepo::new(&conn).list_all().unwrap();
        assert_eq!(glo_entries.len(), 1);
        assert_eq!(glo_entries[0].source, "健太");
        assert_eq!(glo_entries[0].translation, "Kenta");

        let disk_glossary = fs::read_to_string(&layout.glossary_file).unwrap();
        assert!(disk_glossary.contains("健太 === Kenta"));

        // Check Translations.docx was generated
        assert!(layout.docx_output.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
