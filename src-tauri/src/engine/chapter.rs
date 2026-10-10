//! Single chapter translation pipeline execution per FR-RUN-02..04, FR-RAW-08, FR-PLN-06, and FR-GLO-03..04.
//!
//! Handles image-page chapters (bypassing model calls), chunk planning, prompt assembly,
//! streaming tokens, truncation splitting, validation, repair loop, glossary proposals,
//! and docx rendering.

use std::fs;

use rusqlite::Connection;

use crate::docx::body::{inject_chapters_into_body, RenderChapter, RenderItem};
use crate::docx::images::{parse_content_box_from_doc, scale_image_dimensions};
use crate::docx::media::add_image_to_package;
use crate::docx::styles::find_heading1_style_id;
use crate::docx::template::DocxPackage;
use crate::docx::template_content::TemplateTreatment;
use crate::docx::write::{backup_docx_file, check_file_locked};
use crate::error::AppError;
use crate::glossary::parse::{parse_glossary, GlossaryEntryItem, GlossaryLine};
use crate::glossary::write::write_glossary_file;
use crate::plan::split::is_forbidden_split;
use crate::plan::{plan_chapter_chunks, EstimationWeights, ModelLimits};
use crate::project::layout::ProjectLayout;
use crate::project::settings::{EffectiveSettings, Role};
use crate::prompt::assemble::{
    assemble_system_prompt, assemble_user_prompt, ContextPair, UserPromptArgs,
};
use crate::prompt::parse::parse_model_output;
use crate::prompt::{ParsedModelOutput, ParsedSegment};
use crate::provider::types::{
    FinishReason, Provider, ProviderOutcome, ProviderRequest, TokenUsage,
};
use crate::raw::classify::IllustrationSet;
use crate::raw::encoding::read_raw_file;
use crate::raw::segment::{parse_chapter, ParseOptions, RawSegment, SegmentKind};
use crate::store::repo::{
    Call, CallRepo, Chapter, ChapterRepo, ChunkPlan, ChunkPlanRepo, GlossaryEntry, GlossaryRepo,
    TranslationRepo,
};
use crate::validate::content::ContentValidationConfig;
use crate::validate::validate_chunk;

use super::events::{EngineEvent, EventEmitter};
use super::repair::{execute_repair_loop, ProviderResolver};
use super::state::RunControl;

/// Result of executing a chapter translation.
#[derive(Debug, Clone, PartialEq)]
pub enum ChapterExecutionResult {
    Done {
        parts_count: usize,
        new_glossary_terms: usize,
    },
    NeedsAttention {
        reason: String,
    },
    Paused,
}

/// Finds an allowed split point near the target index per FR-PLN-06.
fn find_split_point(segments: &[RawSegment], target: usize) -> Option<usize> {
    if segments.len() <= 2 {
        return None;
    }
    for offset in 0..segments.len() / 2 {
        if target >= offset && target - offset > 0 {
            let idx = target - offset;
            if !is_forbidden_split(segments, idx) {
                return Some(idx);
            }
        }
        if target + offset < segments.len() - 1 {
            let idx = target + offset;
            if !is_forbidden_split(segments, idx) {
                return Some(idx);
            }
        }
    }
    for i in 1..segments.len() - 1 {
        if segments[i].kind != SegmentKind::Image && segments[i + 1].kind != SegmentKind::Image {
            return Some(i);
        }
    }
    Some(target.clamp(1, segments.len() - 1))
}

/// Executes translation of a single chapter.
#[allow(clippy::too_many_arguments)]
pub async fn execute_single_chapter(
    conn: &Connection,
    layout: &ProjectLayout,
    settings: &EffectiveSettings,
    contract_text: &str,
    instruction_text: &str,
    chapter: &Chapter,
    provider_resolver: &dyn ProviderResolver,
    event_emitter: &dyn EventEmitter,
    run_control: &RunControl,
    run_id: Option<&str>,
) -> Result<ChapterExecutionResult, AppError> {
    let chapter_repo = ChapterRepo::new(conn);
    let chunk_repo = ChunkPlanRepo::new(conn);
    let trans_repo = TranslationRepo::new(conn);

    let chapter_path = layout.root.join(&chapter.file_name);
    let content = read_raw_file(&chapter_path)?;
    let illustrations = IllustrationSet::from_dir(&layout.illustrations_dir);
    let options = ParseOptions {
        strip_brackets: settings.strip_title_brackets,
        ruby_normalize: settings.normalize_ruby,
        ..Default::default()
    };
    let parsed_raw = parse_chapter(&chapter.file_name, &content, &illustrations, &options);

    // 1. Check for image-page chapter (FR-RAW-08): no heading and no text segments
    if parsed_raw.is_image_page {
        chapter_repo.update_status(&chapter.id, "done")?;

        event_emitter.emit(EngineEvent::ChapterStatus {
            chapter_id: chapter.id.clone(),
            status: "done".to_string(),
            part_current: None,
            part_total: None,
        });

        let msg = if settings.target_language == "en" {
            format!("Done: {} (image pages)", chapter.file_name)
        } else {
            format!("Selesai: {} (halaman gambar)", chapter.file_name)
        };

        event_emitter.emit(EngineEvent::LogLine {
            level: "info".to_string(),
            message: msg,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        // Re-render Translations.docx so image pages are included in output
        render_docx_after_chapter(conn, layout, settings, event_emitter)?;

        return Ok(ChapterExecutionResult::Done {
            parts_count: 0,
            new_glossary_terms: 0,
        });
    }

    // 2. Plan chapter chunks per FR-PLN-01..05
    chapter_repo.update_status(&chapter.id, "planned")?;
    let translator_role_cfg = settings.roles.for_role(Role::Translator);
    let max_output = translator_role_cfg.max_output_tokens.unwrap_or(4096);
    let limits = ModelLimits {
        max_output_tokens: max_output,
        context_window: 128_000,
    };
    let weights = EstimationWeights::default();
    let plan = plan_chapter_chunks(
        &chapter.file_name,
        &parsed_raw.segments,
        parsed_raw.line_count as usize,
        settings.chunk_threshold_lines,
        &limits,
        &weights,
    );

    let part_count = plan.parts.len();
    let mut chunk_plans = Vec::with_capacity(part_count);
    for (i, p) in plan.parts.iter().enumerate() {
        let cp = ChunkPlan {
            id: format!("{}_part_{}", chapter.id, i + 1),
            chapter_id: chapter.id.clone(),
            part_index: (i + 1) as i32,
            seg_from: p.seg_from,
            seg_to: p.seg_to,
            line_from: p.line_start,
            line_to: p.line_end,
            est_in: p.estimated_input_tokens as i32,
            est_out: p.estimated_output_tokens as i32,
            status: "planned".to_string(),
        };
        chunk_plans.push(cp);
    }
    chunk_repo.insert_batch(&chunk_plans)?;

    let mut chapter_new_glossary_count = 0;
    let mut accumulated_chapter_translations: Vec<ParsedSegment> = Vec::new();
    let mut last_part_translations: Vec<ParsedSegment> = Vec::new();
    let mut accumulated_title: Option<String> = None;

    // 3. Process each planned part
    for (part_idx_0, p) in plan.parts.iter().enumerate() {
        let part_index = part_idx_0 + 1;

        if run_control.should_stop_now() {
            return Err(AppError::Internal {
                message: "Run stopped by operator".to_string(),
            });
        }
        if run_control.should_pause() {
            return Ok(ChapterExecutionResult::Paused);
        }

        chapter_repo.update_status(
            &chapter.id,
            &format!("translating(part {}/{})", part_index, part_count),
        )?;

        event_emitter.emit(EngineEvent::ChapterStatus {
            chapter_id: chapter.id.clone(),
            status: "translating".to_string(),
            part_current: Some(part_index),
            part_total: Some(part_count),
        });

        // Slice raw segments belonging to this part
        let part_raw_segments: Vec<RawSegment> = parsed_raw
            .segments
            .iter()
            .filter(|s| s.idx >= p.seg_from && s.idx <= p.seg_to)
            .cloned()
            .collect();

        // Reload glossary content from disk (FR-GLO-09: file changes win)
        let glossary_disk_content = if layout.glossary_file.exists() {
            fs::read_to_string(&layout.glossary_file).unwrap_or_default()
        } else {
            String::new()
        };

        // Context from previous part (K segments)
        let mut context_pairs = Vec::new();
        let k = settings.context_k_segments;
        if !last_part_translations.is_empty() {
            let start_idx = last_part_translations.len().saturating_sub(k);
            for t in &last_part_translations[start_idx..] {
                context_pairs.push(ContextPair {
                    id: t.id_start,
                    source: String::new(),
                    target: t.text.clone(),
                });
            }
        }

        // Translate the part (with truncation-split support up to depth 3)
        let (translated_part_title, translated_part_segments, calls_made, new_glossary) =
            translate_part_with_truncation_depth(
                &part_raw_segments,
                chapter,
                part_index,
                part_count,
                contract_text,
                instruction_text,
                &glossary_disk_content,
                &context_pairs,
                settings,
                layout,
                conn,
                provider_resolver,
                event_emitter,
                run_control,
                run_id,
                0,
            )
            .await?;

        if let Some(ref t) = translated_part_title {
            accumulated_title = Some(t.clone());
        }

        chapter_new_glossary_count += new_glossary;

        // Validate translated part segments
        chapter_repo.update_status(&chapter.id, "validating")?;
        let model_output = ParsedModelOutput {
            title: translated_part_title.clone(),
            segments: translated_part_segments.clone(),
            glossary_proposals: Vec::new(),
            echoed_context: Vec::new(),
            duplicate_segments: Vec::new(),
            model_comments: Vec::new(),
            auto_closed_final_tag: false,
        };

        let val_cfg = ContentValidationConfig::default();
        let validation = validate_chunk(
            &model_output,
            &part_raw_segments,
            &[],
            part_index as i32,
            &val_cfg,
        );

        let (final_part_title, final_part_segments) = if !validation.is_valid {
            chapter_repo.update_status(&chapter.id, "repairing")?;
            let repair_role_cfg = settings.roles.for_role(Role::Repair);
            let repair_outcome = execute_repair_loop(
                &part_raw_segments,
                translated_part_title,
                translated_part_segments,
                repair_role_cfg,
                provider_resolver,
                contract_text,
                instruction_text,
                &glossary_disk_content,
                &context_pairs,
                &chapter.file_name,
                part_index,
                part_count,
            )
            .await?;

            match repair_outcome {
                super::repair::RepairLoopOutcome::Success {
                    repaired_title,
                    repaired_segments,
                    ..
                } => (repaired_title, repaired_segments),
                super::repair::RepairLoopOutcome::NeedsAttention { reasons, .. } => {
                    chapter_repo.update_status(&chapter.id, "needs_attention")?;
                    event_emitter.emit(EngineEvent::ChapterStatus {
                        chapter_id: chapter.id.clone(),
                        status: "needs_attention".to_string(),
                        part_current: Some(part_index),
                        part_total: Some(part_count),
                    });
                    return Ok(ChapterExecutionResult::NeedsAttention {
                        reason: reasons.join(", "),
                    });
                }
            }
        } else {
            (translated_part_title, translated_part_segments)
        };

        if let Some(ref t) = final_part_title {
            accumulated_title = Some(t.clone());
        }

        // Persist translations in SQLite
        for seg in &final_part_segments {
            trans_repo.insert_version(
                &chapter.id,
                seg.id_start,
                seg.id_end,
                &seg.text,
                "model",
                calls_made.last().map(|c| c.id.as_str()),
            )?;
        }

        last_part_translations = final_part_segments.clone();
        accumulated_chapter_translations.extend(final_part_segments);
    }

    // 4. Update chapter to done in SQLite
    let translated_title = accumulated_title.or_else(|| {
        parsed_raw
            .title_clean
            .clone()
            .or_else(|| parsed_raw.title_raw.clone())
    });

    if let Some(ref t) = translated_title {
        chapter_repo.update_title_tgt(&chapter.id, t)?;
    }
    chapter_repo.update_status(&chapter.id, "done")?;

    event_emitter.emit(EngineEvent::ChapterStatus {
        chapter_id: chapter.id.clone(),
        status: "done".to_string(),
        part_current: Some(part_count),
        part_total: Some(part_count),
    });

    // 5. Re-render Translations.docx
    render_docx_after_chapter(conn, layout, settings, event_emitter)?;

    // 6. Log line in start.md / start-en.md format
    let is_en = settings.target_language == "en";
    let log_msg = if part_count > 1 {
        if is_en {
            format!(
                "Done: {} ({} parts) → +{} glossary terms",
                chapter.file_name, part_count, chapter_new_glossary_count
            )
        } else {
            format!(
                "Selesai: {} ({} parts) → +{} entri glosarium",
                chapter.file_name, part_count, chapter_new_glossary_count
            )
        }
    } else if is_en {
        format!(
            "Done: {} → +{} glossary terms",
            chapter.file_name, chapter_new_glossary_count
        )
    } else {
        format!(
            "Selesai: {} → +{} entri glosarium",
            chapter.file_name, chapter_new_glossary_count
        )
    };

    event_emitter.emit(EngineEvent::LogLine {
        level: "info".to_string(),
        message: log_msg,
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    Ok(ChapterExecutionResult::Done {
        parts_count: part_count,
        new_glossary_terms: chapter_new_glossary_count,
    })
}

/// Recursively translates a part with truncation splitting up to depth 3 per FR-PLN-06.
#[allow(clippy::too_many_arguments)]
async fn translate_part_with_truncation_depth(
    raw_segments: &[RawSegment],
    chapter: &Chapter,
    part_index: usize,
    part_count: usize,
    contract_text: &str,
    instruction_text: &str,
    glossary_content: &str,
    context_pairs: &[ContextPair],
    settings: &EffectiveSettings,
    layout: &ProjectLayout,
    conn: &Connection,
    provider_resolver: &dyn ProviderResolver,
    event_emitter: &dyn EventEmitter,
    _run_control: &RunControl,
    run_id: Option<&str>,
    depth: usize,
) -> Result<(Option<String>, Vec<ParsedSegment>, Vec<Call>, usize), AppError> {
    let call_repo = CallRepo::new(conn);
    let glossary_repo = GlossaryRepo::new(conn);
    let role_cfg = settings.roles.for_role(Role::Translator);

    let keywords: Vec<String> = vec!["laki-laki".to_string(), "perempuan".to_string()];
    let parsed_gloss = parse_glossary(glossary_content, &keywords);
    let selected_entries = parsed_gloss.entries();

    let args = UserPromptArgs {
        chapter_file: &chapter.file_name,
        part_index: part_index as i32,
        total_parts: part_count as i32,
        selected_glossary: &selected_entries,
        context_pairs,
        repair_issues: None,
        chunk_segments: raw_segments,
    };

    let system_prompt = assemble_system_prompt(contract_text, instruction_text);
    let user_prompt = assemble_user_prompt(&args);

    let mut models_to_try = vec![role_cfg.primary.clone()];
    models_to_try.extend(role_cfg.fallbacks.clone());

    let mut successful_outcome = None;
    let mut call_records = Vec::new();

    for model_ref in &models_to_try {
        let provider = match provider_resolver.resolve(model_ref) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let req = ProviderRequest {
            system_prompt: system_prompt.clone(),
            user_prompt: user_prompt.clone(),
            model: model_ref.model_id.clone(),
            temperature: role_cfg.temperature,
            max_output_tokens: role_cfg.max_output_tokens,
            reasoning_effort: role_cfg.reasoning_effort.clone(),
        };

        let start_time = std::time::Instant::now();
        let call_id = format!("call_{}", uuid::Uuid::new_v4().simple());

        let mut cb = |token: &str| {
            event_emitter.emit(EngineEvent::CallProgress {
                call_id: call_id.clone(),
                delta: token.to_string(),
                chapter_id: Some(chapter.id.clone()),
                part_index: Some(part_index),
            });
        };

        let outcome_res = provider.send_request(&req, Some(&mut cb)).await;
        let latency_ms = start_time.elapsed().as_millis() as i64;

        let outcome = match outcome_res {
            Ok(o) => o,
            Err(e) => {
                let call_rec = Call {
                    id: call_id,
                    run_id: run_id.map(|s| s.to_string()),
                    role: "translator".to_string(),
                    provider: model_ref.provider.clone(),
                    model: model_ref.model_id.clone(),
                    chapter_id: Some(chapter.id.clone()),
                    part_index: Some(part_index as i32),
                    kind: "part".to_string(),
                    status: "failed".to_string(),
                    finish_reason: None,
                    in_tokens: 0,
                    cached_in: 0,
                    cache_write: 0,
                    out_tokens: 0,
                    reasoning_tokens: 0,
                    cost: None,
                    latency_ms,
                    contract_version: Some("1.0".to_string()),
                    manifest_json: None,
                    error: Some(e.to_string()),
                    raw_response: None,
                };
                call_repo.insert(&call_rec)?;
                call_records.push(call_rec);
                continue;
            }
        };

        let finish_reason = match &outcome {
            ProviderOutcome::Success(r) => r.finish_reason,
            ProviderOutcome::LengthTruncated(r) => r.finish_reason,
            ProviderOutcome::Refused { .. } => FinishReason::Refusal,
            ProviderOutcome::ContentFiltered { .. } => FinishReason::ContentFilter,
        };

        let usage = outcome.usage().cloned().unwrap_or(TokenUsage::default());
        let raw_text = outcome.text().unwrap_or("").to_string();

        let call_record = Call {
            id: call_id,
            run_id: run_id.map(|s| s.to_string()),
            role: "translator".to_string(),
            provider: model_ref.provider.clone(),
            model: model_ref.model_id.clone(),
            chapter_id: Some(chapter.id.clone()),
            part_index: Some(part_index as i32),
            kind: "part".to_string(),
            status: if outcome.is_success() {
                "success".to_string()
            } else {
                "failed".to_string()
            },
            finish_reason: Some(format!("{:?}", finish_reason)),
            in_tokens: usage.input_tokens,
            cached_in: usage.cached_input_tokens,
            cache_write: usage.cache_write_tokens,
            out_tokens: usage.output_tokens,
            reasoning_tokens: usage.reasoning_tokens,
            cost: None,
            latency_ms,
            contract_version: Some("1.0".to_string()),
            manifest_json: None,
            error: None,
            raw_response: Some(raw_text.clone()),
        };
        call_repo.insert(&call_record)?;
        call_records.push(call_record);

        if matches!(
            finish_reason,
            FinishReason::Refusal | FinishReason::ContentFilter
        ) {
            continue;
        }

        successful_outcome = Some((outcome, finish_reason, raw_text));
        break;
    }

    let (_outcome, finish_reason, raw_text) = match successful_outcome {
        Some(o) => o,
        None => {
            return Err(AppError::Internal {
                message: "All models failed or refused translation request".to_string(),
            });
        }
    };

    // FR-PLN-06: Check if truncated and depth < 3
    if matches!(finish_reason, FinishReason::Length) && depth < 3 && raw_segments.len() > 2 {
        let half_target = raw_segments.len() / 2;
        if let Some(cut_idx) = find_split_point(raw_segments, half_target) {
            let (left_raw, right_raw) = raw_segments.split_at(cut_idx);
            let (left_title, left_parsed, mut left_calls, left_gloss) =
                Box::pin(translate_part_with_truncation_depth(
                    left_raw,
                    chapter,
                    part_index,
                    part_count,
                    contract_text,
                    instruction_text,
                    glossary_content,
                    context_pairs,
                    settings,
                    layout,
                    conn,
                    provider_resolver,
                    event_emitter,
                    _run_control,
                    run_id,
                    depth + 1,
                ))
                .await?;

            let (right_title, right_parsed, right_calls, right_gloss) =
                Box::pin(translate_part_with_truncation_depth(
                    right_raw,
                    chapter,
                    part_index,
                    part_count,
                    contract_text,
                    instruction_text,
                    glossary_content,
                    context_pairs,
                    settings,
                    layout,
                    conn,
                    provider_resolver,
                    event_emitter,
                    _run_control,
                    run_id,
                    depth + 1,
                ))
                .await?;

            left_calls.extend(right_calls);
            let mut combined = left_parsed;
            combined.extend(right_parsed);
            return Ok((
                left_title.or(right_title),
                combined,
                left_calls,
                left_gloss + right_gloss,
            ));
        }
    }

    let parsed = parse_model_output(&raw_text, &[])?;
    let fixed_segments = parsed.segments;

    // Process glossary proposals (FR-GLO-03, 04)
    let mut new_glossary_count = 0;
    for proposal in parsed.glossary_proposals {
        let appears_in_raw = raw_segments
            .iter()
            .any(|s| s.src_text.contains(&proposal.source));
        if !appears_in_raw {
            continue;
        }

        if let Some(existing) = glossary_repo.get_by_source(&proposal.source)? {
            if existing.translation != proposal.translation {
                let conflict_entry = GlossaryEntry {
                    id: format!("glo_{}", uuid::Uuid::new_v4().simple()),
                    source: proposal.source.clone(),
                    translation: proposal.translation.clone(),
                    note: Some(proposal.note.clone()),
                    tier: 2,
                    origin: "model".to_string(),
                    first_chapter_id: Some(chapter.id.clone()),
                    occurrences: 1,
                    status: "conflict".to_string(),
                    file_line: None,
                };
                glossary_repo.insert(&conflict_entry)?;
            }
        } else {
            let existing_disk = if layout.glossary_file.exists() {
                fs::read_to_string(&layout.glossary_file).unwrap_or_default()
            } else {
                String::new()
            };

            let mut parsed_gloss = parse_glossary(&existing_disk, &keywords);
            parsed_gloss
                .lines
                .push(GlossaryLine::Entry(GlossaryEntryItem {
                    source: proposal.source.clone(),
                    translation: proposal.translation.clone(),
                    note: proposal.note.clone(),
                    tier: 2,
                    line_index: parsed_gloss.lines.len(),
                    has_list_prefix: false,
                    list_prefix_char: None,
                }));

            let _ = write_glossary_file(
                &layout.glossary_file,
                &parsed_gloss,
                Some(&layout.backups_dir),
            );

            let new_entry = GlossaryEntry {
                id: format!("glo_{}", uuid::Uuid::new_v4().simple()),
                source: proposal.source.clone(),
                translation: proposal.translation.clone(),
                note: Some(proposal.note),
                tier: 2,
                origin: "model".to_string(),
                first_chapter_id: Some(chapter.id.clone()),
                occurrences: 1,
                status: "active".to_string(),
                file_line: None,
            };
            glossary_repo.insert(&new_entry)?;
            new_glossary_count += 1;
        }
    }

    if new_glossary_count > 0 {
        event_emitter.emit(EngineEvent::GlossaryChanged {
            added_count: new_glossary_count,
            conflicts_count: 0,
        });
    }

    Ok((
        parsed.title,
        fixed_segments,
        call_records,
        new_glossary_count,
    ))
}

/// Re-renders Translations.docx with all chapters currently marked as `done` or `frozen`.
fn render_docx_after_chapter(
    conn: &Connection,
    layout: &ProjectLayout,
    settings: &EffectiveSettings,
    event_emitter: &dyn EventEmitter,
) -> Result<(), AppError> {
    let chapter_repo = ChapterRepo::new(conn);
    let trans_repo = TranslationRepo::new(conn);

    let all_chapters = chapter_repo.list_all()?;

    let template_bytes = if layout.template_file.exists() {
        fs::read(&layout.template_file).unwrap_or_default()
    } else {
        Vec::new()
    };

    if template_bytes.is_empty() {
        return Ok(());
    }

    let mut package = DocxPackage::from_bytes(&template_bytes)?;
    let doc_xml = package
        .get_text_entry("word/document.xml")
        .unwrap_or_default();
    let content_box = parse_content_box_from_doc(&doc_xml);

    let mut render_chapters = Vec::new();
    let mut doc_pr_id_counter = 0u32;

    for ch in all_chapters {
        if ch.status != "done" && !ch.frozen {
            continue;
        }

        let chapter_path = layout.root.join(&ch.file_name);
        let content = read_raw_file(&chapter_path)?;
        let illustrations = IllustrationSet::from_dir(&layout.illustrations_dir);
        let options = ParseOptions {
            strip_brackets: settings.strip_title_brackets,
            ruby_normalize: settings.normalize_ruby,
            ..Default::default()
        };
        let parsed_raw = parse_chapter(&ch.file_name, &content, &illustrations, &options);

        if parsed_raw.is_image_page {
            let mut items = Vec::new();
            for seg in &parsed_raw.segments {
                match seg.kind {
                    SegmentKind::Image => {
                        if let Some(ref img_file) = seg.image_file {
                            let img_path = layout.illustrations_dir.join(img_file);
                            if let Ok(bytes) = fs::read(&img_path) {
                                doc_pr_id_counter += 1;
                                if let Ok((rel_id, _media, (w, h))) = add_image_to_package(
                                    &mut package,
                                    &bytes,
                                    img_file,
                                    doc_pr_id_counter as usize,
                                ) {
                                    let (cx, cy) = scale_image_dimensions(w, h, &content_box);
                                    items.push(RenderItem::Image {
                                        rel_id,
                                        cx,
                                        cy,
                                        doc_pr_id: doc_pr_id_counter,
                                    });
                                } else {
                                    items.push(RenderItem::MissingImage(img_file.to_string()));
                                }
                            } else {
                                items.push(RenderItem::MissingImage(img_file.to_string()));
                            }
                        }
                    }
                    SegmentKind::ImageMissing => {
                        items.push(RenderItem::MissingImage(seg.src_text.clone()));
                    }
                    _ => {}
                }
            }

            render_chapters.push(RenderChapter {
                title: None,
                is_image_page: true,
                items,
            });
            continue;
        }

        let translations = trans_repo.get_current_by_chapter(&ch.id)?;
        let mut items = Vec::new();

        for seg in &parsed_raw.segments {
            match seg.kind {
                SegmentKind::Scene => {
                    items.push(RenderItem::Text(seg.src_text.clone()));
                }
                SegmentKind::Image => {
                    if let Some(ref img_file) = seg.image_file {
                        let img_path = layout.illustrations_dir.join(img_file);
                        if let Ok(bytes) = fs::read(&img_path) {
                            doc_pr_id_counter += 1;
                            if let Ok((rel_id, _media, (w, h))) = add_image_to_package(
                                &mut package,
                                &bytes,
                                img_file,
                                doc_pr_id_counter as usize,
                            ) {
                                let (cx, cy) = scale_image_dimensions(w, h, &content_box);
                                items.push(RenderItem::Image {
                                    rel_id,
                                    cx,
                                    cy,
                                    doc_pr_id: doc_pr_id_counter,
                                });
                            } else {
                                items.push(RenderItem::MissingImage(img_file.to_string()));
                            }
                        } else {
                            items.push(RenderItem::MissingImage(img_file.to_string()));
                        }
                    }
                }
                SegmentKind::ImageMissing => {
                    items.push(RenderItem::MissingImage(seg.src_text.clone()));
                }
                SegmentKind::Narration | SegmentKind::Dialogue => {
                    if let Some(tr) = translations
                        .iter()
                        .find(|t| t.seg_from <= seg.idx && t.seg_to >= seg.idx)
                    {
                        items.push(RenderItem::Text(tr.text.clone()));
                    } else {
                        items.push(RenderItem::Text(seg.src_text.clone()));
                    }
                }
                SegmentKind::Title => {}
            }
        }

        render_chapters.push(RenderChapter {
            title: ch.title_tgt.clone().or_else(|| {
                parsed_raw
                    .title_clean
                    .clone()
                    .or_else(|| parsed_raw.title_raw.clone())
            }),
            is_image_page: false,
            items,
        });
    }

    let treatment = match settings.template_treatment.as_str() {
        "keep_front_matter" => TemplateTreatment::KeepFrontMatter,
        "frozen_prefix" => TemplateTreatment::FrozenPrefix,
        _ => TemplateTreatment::IgnoreContent,
    };

    check_file_locked(&layout.docx_output)?;

    let styles_xml = package
        .get_text_entry("word/styles.xml")
        .unwrap_or_default();
    let heading_style = find_heading1_style_id(&styles_xml);

    let new_doc_xml =
        inject_chapters_into_body(&doc_xml, &render_chapters, &heading_style, treatment);
    package.set_text_entry("word/document.xml", &new_doc_xml);

    let output_bytes = package.to_bytes()?;
    backup_docx_file(&layout.docx_output, &layout.backups_dir)?;

    let temp_docx = layout
        .docx_output
        .with_extension(format!("tmp_{}", uuid::Uuid::new_v4().simple()));

    fs::write(&temp_docx, &output_bytes).map_err(|e| AppError::Internal {
        message: format!("Failed to write temp docx: {e}"),
    })?;

    if let Err(e) = fs::rename(&temp_docx, &layout.docx_output) {
        let _ = fs::remove_file(&temp_docx);
        return Err(AppError::Internal {
            message: format!("Failed to atomically rename docx: {e}"),
        });
    }

    event_emitter.emit(EngineEvent::DocxRender {
        status: "rendered".to_string(),
        path: layout.docx_output.display().to_string(),
    });

    Ok(())
}
