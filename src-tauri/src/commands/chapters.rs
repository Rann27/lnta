//! Chapter and segment query commands per FR-REV-01, Story B1.

use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;
use crate::project::layout::ProjectLayout;
use crate::store::open_path;
use crate::store::repo::{
    Chapter, ChapterRepo, Segment, SegmentRepo, Translation, TranslationRepo,
};

/// Full chapter detail including segments and translations.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/ChapterDetail.ts")]
pub struct ChapterDetail {
    pub chapter: Chapter,
    pub segments: Vec<Segment>,
    pub translations: Vec<Translation>,
}

#[tauri::command]
pub fn list_chapters(path: String) -> Result<Vec<Chapter>, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    if !layout.db_file.exists() {
        return Ok(Vec::new());
    }
    let conn = open_path(&layout.db_file)?;
    ChapterRepo::new(&conn).list_all()
}

#[tauri::command]
pub fn get_chapter(path: String, chapter_id: String) -> Result<ChapterDetail, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    let conn = open_path(&layout.db_file)?;

    let chapter = ChapterRepo::new(&conn)
        .get_by_id(&chapter_id)?
        .ok_or_else(|| AppError::NotFound {
            entity: "chapter".to_string(),
            id: chapter_id.clone(),
        })?;

    let segments = SegmentRepo::new(&conn).list_by_chapter(&chapter_id)?;
    let translations = TranslationRepo::new(&conn).get_current_by_chapter(&chapter_id)?;

    Ok(ChapterDetail {
        chapter,
        segments,
        translations,
    })
}

#[tauri::command]
pub fn list_segments(path: String, chapter_id: String) -> Result<Vec<Segment>, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    if !layout.db_file.exists() {
        return Ok(Vec::new());
    }
    let conn = open_path(&layout.db_file)?;
    SegmentRepo::new(&conn).list_by_chapter(&chapter_id)
}

#[tauri::command]
pub fn list_translations(path: String, chapter_id: String) -> Result<Vec<Translation>, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    if !layout.db_file.exists() {
        return Ok(Vec::new());
    }
    let conn = open_path(&layout.db_file)?;
    TranslationRepo::new(&conn).get_current_by_chapter(&chapter_id)
}

#[tauri::command]
pub fn get_segment_history(
    path: String,
    chapter_id: String,
    seg_from: i32,
    seg_to: i32,
) -> Result<Vec<Translation>, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    if !layout.db_file.exists() {
        return Ok(Vec::new());
    }
    let conn = open_path(&layout.db_file)?;
    TranslationRepo::new(&conn).list_history(&chapter_id, seg_from, seg_to)
}

#[tauri::command]
pub fn edit_segment(
    path: String,
    chapter_id: String,
    seg_from: i32,
    seg_to: i32,
    text: String,
) -> Result<Translation, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    let conn = open_path(&layout.db_file)?;

    let config = if layout.config_file.exists() {
        crate::project::config::ProjectConfig::load_from_file(&layout.config_file)?
    } else {
        crate::project::config::ProjectConfig::for_profile("ja-id")
    };
    let global = crate::project::settings::GlobalSettings::load_from_app_data().unwrap_or_default();
    let effective = crate::project::settings::EffectiveSettings::merge(&global, &config, None);

    edit_segment_core(
        &conn,
        &layout,
        &effective,
        &chapter_id,
        seg_from,
        seg_to,
        &text,
    )
}

pub fn edit_segment_core(
    conn: &rusqlite::Connection,
    layout: &ProjectLayout,
    settings: &crate::project::settings::EffectiveSettings,
    chapter_id: &str,
    seg_from: i32,
    seg_to: i32,
    text: &str,
) -> Result<Translation, AppError> {
    let (fixed_text, _) = crate::validate::fixers::apply_mechanical_fixes(text, &[], &[]);
    if fixed_text.trim().is_empty() {
        return Err(AppError::ValidationFailed {
            reason: "Translation text cannot be empty".to_string(),
        });
    }
    if crate::validate::content::contains_residual_japanese(&fixed_text) {
        return Err(AppError::ValidationFailed {
            reason: "Translation text contains residual Japanese characters".to_string(),
        });
    }

    let trans_repo = TranslationRepo::new(conn);
    let new_tr =
        trans_repo.insert_version(chapter_id, seg_from, seg_to, &fixed_text, "manual", None)?;

    let _ = crate::engine::chapter::render_docx_after_chapter(
        conn,
        layout,
        settings,
        &crate::engine::events::RecordingEventEmitter::default(),
    );

    Ok(new_tr)
}

#[tauri::command]
pub fn restore_segment_version(
    path: String,
    chapter_id: String,
    translation_id: String,
) -> Result<Translation, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    let mut conn = open_path(&layout.db_file)?;

    let config = if layout.config_file.exists() {
        crate::project::config::ProjectConfig::load_from_file(&layout.config_file)?
    } else {
        crate::project::config::ProjectConfig::for_profile("ja-id")
    };
    let global = crate::project::settings::GlobalSettings::load_from_app_data().unwrap_or_default();
    let effective = crate::project::settings::EffectiveSettings::merge(&global, &config, None);

    restore_segment_version_core(&mut conn, &layout, &effective, &chapter_id, &translation_id)
}

pub fn restore_segment_version_core(
    conn: &mut rusqlite::Connection,
    layout: &ProjectLayout,
    settings: &crate::project::settings::EffectiveSettings,
    chapter_id: &str,
    translation_id: &str,
) -> Result<Translation, AppError> {
    let (seg_from, seg_to) = {
        let trans_repo = TranslationRepo::new(conn);
        let target = trans_repo
            .get_by_id(translation_id)?
            .ok_or_else(|| AppError::NotFound {
                entity: "translation".to_string(),
                id: translation_id.to_string(),
            })?;
        (target.seg_from, target.seg_to)
    };

    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE translation SET is_current = 0 WHERE chapter_id = ?1 AND seg_from = ?2 AND seg_to = ?3",
        rusqlite::params![chapter_id, seg_from, seg_to],
    )?;
    tx.execute(
        "UPDATE translation SET is_current = 1 WHERE id = ?1",
        rusqlite::params![translation_id],
    )?;
    tx.commit()?;

    let _ = crate::engine::chapter::render_docx_after_chapter(
        conn,
        layout,
        settings,
        &crate::engine::events::RecordingEventEmitter::default(),
    );

    let trans_repo = TranslationRepo::new(conn);
    trans_repo
        .get_by_id(translation_id)?
        .ok_or_else(|| AppError::NotFound {
            entity: "translation".to_string(),
            id: translation_id.to_string(),
        })
}

#[tauri::command]
pub async fn retranslate_segment(
    path: String,
    chapter_id: String,
    seg_from: i32,
    seg_to: i32,
    model: Option<crate::project::settings::ModelReference>,
) -> Result<Translation, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);

    let config = if layout.config_file.exists() {
        crate::project::config::ProjectConfig::load_from_file(&layout.config_file)?
    } else {
        crate::project::config::ProjectConfig::for_profile("ja-id")
    };
    let global = crate::project::settings::GlobalSettings::load_from_app_data().unwrap_or_default();
    let mut effective = crate::project::settings::EffectiveSettings::merge(&global, &config, None);
    if let Some(m) = model {
        effective.roles.translator.primary = m;
    }

    let resolver = crate::commands::run::DynamicProviderResolver::new(global);
    retranslate_segment_core(
        &layout,
        &effective,
        &resolver,
        &chapter_id,
        seg_from,
        seg_to,
    )
    .await
}

pub async fn retranslate_segment_core(
    layout: &ProjectLayout,
    settings: &crate::project::settings::EffectiveSettings,
    resolver: &dyn crate::engine::repair::ProviderResolver,
    chapter_id: &str,
    seg_from: i32,
    seg_to: i32,
) -> Result<Translation, AppError> {
    let (file_name, current_translations) = {
        let conn = open_path(&layout.db_file)?;
        let chapter_repo = ChapterRepo::new(&conn);
        let trans_repo = TranslationRepo::new(&conn);

        let chap = chapter_repo
            .get_by_id(chapter_id)?
            .ok_or_else(|| AppError::NotFound {
                entity: "chapter".to_string(),
                id: chapter_id.to_string(),
            })?;
        let trs = trans_repo.get_current_by_chapter(chapter_id)?;
        (chap.file_name, trs)
    };

    let raw_path = layout.root.join(&file_name);
    let content = crate::raw::encoding::read_raw_file(&raw_path)?;
    let illustrations = crate::raw::classify::IllustrationSet::from_dir(&layout.illustrations_dir);
    let parse_options = crate::raw::segment::ParseOptions {
        strip_brackets: settings.strip_title_brackets,
        ruby_normalize: settings.normalize_ruby,
        ..Default::default()
    };
    let parsed =
        crate::raw::segment::parse_chapter(&file_name, &content, &illustrations, &parse_options);

    let target_segments: Vec<crate::raw::RawSegment> = parsed
        .segments
        .iter()
        .filter(|s| s.idx >= seg_from && s.idx <= seg_to)
        .cloned()
        .collect();

    if target_segments.is_empty() {
        return Err(AppError::NotFound {
            entity: "segment".to_string(),
            id: format!("{seg_from}-{seg_to}"),
        });
    }

    let mut context_pairs = Vec::new();
    for prev_idx in (seg_from.saturating_sub(2))..seg_from {
        if let Some(raw_s) = parsed.segments.iter().find(|s| s.idx == prev_idx) {
            if let Some(t) = current_translations
                .iter()
                .find(|t| t.seg_from <= prev_idx && t.seg_to >= prev_idx)
            {
                context_pairs.push(crate::prompt::assemble::ContextPair {
                    id: prev_idx,
                    source: raw_s.src_text.clone(),
                    target: t.text.clone(),
                });
            }
        }
    }

    let profile = crate::profiles::load_profile(&settings.profile)?;
    let contract_text = &profile.contract_text;
    let instruction_text = crate::project::instruction::load_instruction_text(
        &layout.root,
        &settings.profile,
        settings.instruction_file.as_deref(),
    )?;

    let args = crate::prompt::assemble::UserPromptArgs {
        chapter_file: &file_name,
        part_index: 0,
        total_parts: 1,
        selected_glossary: &[],
        context_pairs: &context_pairs,
        repair_issues: None,
        chunk_segments: &target_segments,
    };

    let system_prompt =
        crate::prompt::assemble::assemble_system_prompt(contract_text, &instruction_text);
    let user_prompt = crate::prompt::assemble::assemble_user_prompt(&args);

    use crate::provider::types::Provider;
    let provider = resolver.resolve(&settings.roles.translator.primary)?;
    let req = crate::provider::types::ProviderRequest {
        system_prompt,
        user_prompt,
        model: settings.roles.translator.primary.model_id.clone(),
        temperature: settings.roles.translator.temperature,
        max_output_tokens: settings.roles.translator.max_output_tokens,
        reasoning_effort: settings.roles.translator.reasoning_effort.clone(),
    };

    let outcome = provider.send_request(&req, None).await?;
    let resp_text = match outcome {
        crate::provider::types::ProviderOutcome::Success(resp)
        | crate::provider::types::ProviderOutcome::LengthTruncated(resp) => resp.text,
        crate::provider::types::ProviderOutcome::Refused { reason }
        | crate::provider::types::ProviderOutcome::ContentFiltered { reason } => {
            return Err(AppError::ProviderFailed { reason });
        }
    };

    let parsed_out = crate::prompt::parse::parse_model_output(&resp_text, &[])?;
    let translated_text = if !parsed_out.segments.is_empty() {
        parsed_out
            .segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    } else if let Some(ref t) = parsed_out.title {
        t.clone()
    } else {
        return Err(AppError::ParseFailed {
            reason: "Model output contained no segment text".to_string(),
        });
    };

    let (fixed_text, _) =
        crate::validate::fixers::apply_mechanical_fixes(&translated_text, &[], &[]);
    if fixed_text.trim().is_empty() {
        return Err(AppError::ValidationFailed {
            reason: "Retranslated text is empty".to_string(),
        });
    }

    let conn = open_path(&layout.db_file)?;
    let trans_repo = TranslationRepo::new(&conn);
    let new_tr =
        trans_repo.insert_version(chapter_id, seg_from, seg_to, &fixed_text, "ai", None)?;

    let _ = crate::engine::chapter::render_docx_after_chapter(
        &conn,
        layout,
        settings,
        &crate::engine::events::RecordingEventEmitter::default(),
    );

    Ok(new_tr)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::provider::mock::{MockProvider, MockStep};
    use crate::store::open_in_memory;
    use std::sync::Arc;

    struct TestResolver {
        provider: Arc<MockProvider>,
    }

    impl crate::engine::repair::ProviderResolver for TestResolver {
        fn resolve(
            &self,
            _reference: &crate::project::settings::ModelReference,
        ) -> Result<crate::provider::AnyProvider, AppError> {
            Ok(crate::provider::AnyProvider::Mock(self.provider.clone()))
        }
    }

    #[test]
    fn test_edit_segment_and_restore_history() {
        let mut conn = open_in_memory().unwrap();
        crate::store::migrate(&mut conn).unwrap();

        let chap_repo = ChapterRepo::new(&conn);
        let ch = Chapter {
            id: "ch_test".to_string(),
            ordinal: 1,
            file_name: "01_test.md".to_string(),
            prefix: "1".to_string(),
            title_src: "第1話".to_string(),
            title_tgt: None,
            src_hash: "hash".to_string(),
            line_count: 10,
            char_count: 50,
            status: "pending".to_string(),
            frozen: false,
            parts_planned: 1,
            updated_at: "now".to_string(),
        };
        chap_repo.insert(&ch).unwrap();

        let v1 = TranslationRepo::new(&conn)
            .insert_version("ch_test", 1, 1, "Versi asli satu.", "model", None)
            .unwrap();

        let layout = ProjectLayout::new(Path::new("dummy_nonexistent_dir"));
        let settings = crate::project::settings::EffectiveSettings::merge(
            &crate::project::settings::GlobalSettings::default(),
            &crate::project::config::ProjectConfig::for_profile("ja-id"),
            None,
        );

        // 1. Edit segment manually
        let v2 = edit_segment_core(
            &conn,
            &layout,
            &settings,
            "ch_test",
            1,
            1,
            "Versi diedit manual.",
        )
        .unwrap();

        assert_eq!(v2.version, 2);
        assert_eq!(v2.origin, "manual");
        assert_eq!(v2.text, "Versi diedit manual.");

        let history = TranslationRepo::new(&conn)
            .list_history("ch_test", 1, 1)
            .unwrap();
        assert_eq!(history.len(), 2);
        assert!(!history[0].is_current);
        assert!(history[1].is_current);

        // 2. Edit validation check: empty rejected
        let err_empty = edit_segment_core(&conn, &layout, &settings, "ch_test", 1, 1, "   ");
        assert!(err_empty.is_err());

        // 3. Edit validation check: residual Japanese rejected
        let err_jp = edit_segment_core(
            &conn,
            &layout,
            &settings,
            "ch_test",
            1,
            1,
            "Ada kanji 漢字 di sini.",
        );
        assert!(err_jp.is_err());

        // 4. Restore v1
        let restored =
            restore_segment_version_core(&mut conn, &layout, &settings, "ch_test", &v1.id).unwrap();

        assert_eq!(restored.id, v1.id);
        assert!(restored.is_current);

        let current_list = TranslationRepo::new(&conn)
            .get_current_by_chapter("ch_test")
            .unwrap();
        assert_eq!(current_list.len(), 1);
        assert_eq!(current_list[0].id, v1.id);
        assert_eq!(current_list[0].text, "Versi asli satu.");
    }

    #[tokio::test]
    async fn test_retranslate_segment_with_mock() {
        let temp_dir = std::env::temp_dir().join(format!("lnta_test_{}", uuid::Uuid::new_v4()));
        let layout = ProjectLayout::new(&temp_dir);
        let _ = std::fs::create_dir_all(layout.db_file.parent().unwrap());

        let mut conn = open_path(&layout.db_file).unwrap();
        crate::store::migrate(&mut conn).unwrap();

        let chap_repo = ChapterRepo::new(&conn);
        let ch = Chapter {
            id: "ch_test".to_string(),
            ordinal: 1,
            file_name: "01_test.md".to_string(),
            prefix: "1".to_string(),
            title_src: "第1話".to_string(),
            title_tgt: None,
            src_hash: "hash".to_string(),
            line_count: 10,
            char_count: 50,
            status: "pending".to_string(),
            frozen: false,
            parts_planned: 1,
            updated_at: "now".to_string(),
        };
        chap_repo.insert(&ch).unwrap();
        drop(conn);

        // Create temporary chapter raw file
        let raw_file = temp_dir.join("01_test.md");
        std::fs::write(&raw_file, "健太は走った。").unwrap();
        // Retranslation refuses to run without the owner's rules, as every model call does.
        std::fs::write(temp_dir.join("instruction.md"), "# Aturan uji").unwrap();

        let settings = crate::project::settings::EffectiveSettings::merge(
            &crate::project::settings::GlobalSettings::default(),
            &crate::project::config::ProjectConfig::for_profile("ja-id"),
            None,
        );

        let mock_provider = Arc::new(MockProvider::default());
        mock_provider.push_step(MockStep::simple_success(
            "<s id=\"1\">Kenta berlari cepat.</s>",
        ));

        let resolver = TestResolver {
            provider: mock_provider,
        };

        let new_tr = retranslate_segment_core(&layout, &settings, &resolver, "ch_test", 1, 1)
            .await
            .unwrap();

        assert_eq!(new_tr.text, "Kenta berlari cepat.");
        assert_eq!(new_tr.origin, "ai");
        assert_eq!(new_tr.version, 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
