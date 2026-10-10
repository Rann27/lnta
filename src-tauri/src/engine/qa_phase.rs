//! QA phase orchestration per PRD §5.4 and FR-QA-01..10: scan, mechanical fixes, AI fixes
//! for semantic findings, hotspot review against the raw (step C), then a rescan.
//!
//! Every step reuses the `qa` building blocks; this module only sequences them, calls the
//! QA roles, records those calls for usage, and reports progress.

use std::collections::{HashMap, HashSet};
use std::fs;

use rusqlite::Connection;

use crate::error::AppError;
use crate::glossary::parse::parse_glossary;
use crate::project::layout::ProjectLayout;
use crate::project::settings::{EffectiveSettings, Role, RoleConfig};
use crate::prompt::assemble::assemble_system_prompt;
use crate::prompt::glossary_select::format_reference_glossary;
use crate::provider::types::{FinishReason, Provider, ProviderOutcome, ProviderRequest};
use crate::qa::common::{QaFinding, SegmentForQa};
use crate::qa::fix_ai::{
    apply_ai_fixes_to_db, build_qa_fix_prompt, parse_qa_fix_response, QaFixItem, QaFixPromptArgs,
};
use crate::qa::fixlog::apply_mechanical_fixes_to_volume;
use crate::qa::hotspots::{select_all_hotspots, select_homophone_hotspots, Hotspot};
use crate::qa::ja_en::CharacterGender;
use crate::qa::review::{
    apply_rename_to_volume, apply_review_fix, build_qa_review_prompt, build_qa_summary_log,
    load_volume_segments_for_qa, parse_qa_review_response, QaLoopResult, QaReviewArgs,
    QaReviewResponseItem,
};
use crate::store::repo::{
    Call, CallRepo, Chapter, Finding, FindingRepo, Run, RunRepo, TranslationRepo,
};

use super::chapter::render_docx_after_chapter;
use super::events::{EngineEvent, EventEmitter};
use super::repair::ProviderResolver;

/// Findings per QA fix call; keeps prompts short enough for any model's output limit.
const FIX_BATCH: usize = 15;
/// Pairs per hotspot review call.
const REVIEW_BATCH: usize = 40;
/// Scan, fix, rescan rounds (FR-QA-10).
const MAX_ROUNDS: usize = 2;

const FEMALE_KEYWORDS: [&str; 6] = ["female", "woman", "girl", "perempuan", "wanita", "cewek"];

struct QaContext<'a> {
    conn: &'a Connection,
    settings: &'a EffectiveSettings,
    resolver: &'a dyn ProviderResolver,
    system_prompt: String,
    glossary_text: String,
    run_id: String,
}

/// Runs the QA phase over every finished chapter. Returns counts for the summary line.
pub async fn run_qa_phase(
    conn: &Connection,
    layout: &ProjectLayout,
    settings: &EffectiveSettings,
    resolver: &dyn ProviderResolver,
    emitter: &dyn EventEmitter,
) -> Result<QaLoopResult, AppError> {
    let profile = crate::profiles::load_profile(&settings.profile)?;
    let instruction = crate::project::instruction::load_instruction_text(
        &layout.root,
        &settings.profile,
        settings.instruction_file.as_deref(),
    )?;

    let run_id = format!("run_{}", uuid::Uuid::new_v4().simple());
    RunRepo::new(conn).insert(&Run {
        id: run_id.clone(),
        kind: "qa".to_string(),
        status: "running".to_string(),
        started_at: chrono::Utc::now().to_rfc3339(),
        ended_at: None,
        settings_snapshot: serde_json::to_string(settings).unwrap_or_default(),
    })?;
    emitter.emit(EngineEvent::RunState {
        state: "qa_running".to_string(),
        run_id: Some(run_id.clone()),
    });

    let glossary_raw = fs::read_to_string(&layout.glossary_file).unwrap_or_default();
    let glossary = parse_glossary(&glossary_raw, &profile.config.gender_keywords);
    let entries = glossary.entries();
    let tier1_names: Vec<(String, String)> = entries
        .iter()
        .filter(|e| e.tier == 1)
        .map(|e| (e.source.clone(), e.translation.clone()))
        .collect();
    let characters: Vec<CharacterGender> = entries
        .iter()
        .filter(|e| e.tier == 1)
        .map(|e| {
            let note = e.note.to_lowercase();
            CharacterGender::new(
                e.translation.clone(),
                FEMALE_KEYWORDS.iter().any(|k| note.contains(k)),
            )
        })
        .collect();
    let glossary_terms: Vec<&str> = entries.iter().map(|e| e.translation.as_str()).collect();

    let ctx = QaContext {
        conn,
        settings,
        resolver,
        system_prompt: assemble_system_prompt(profile.qa_contract_text, &instruction),
        glossary_text: format_reference_glossary(&entries),
        run_id: run_id.clone(),
    };

    let mut mechanical_total = 0usize;
    let mut ai_total = 0usize;
    let mut findings = scan_and_store(&ctx, &settings.profile, &characters)?;
    emit_progress(emitter, "scan", findings.len(), 0);

    for round in 0..MAX_ROUNDS {
        let mut applied_this_round = 0usize;

        if round == 0 {
            let mech = apply_mechanical_fixes_to_volume(
                conn,
                Some(&run_id),
                &settings.profile,
                &glossary_terms,
            )?;
            mechanical_total += mech.applied_count;
            applied_this_round += mech.applied_count;
            emit_progress(emitter, "mechanical", findings.len(), mechanical_total);
            findings = scan_and_store(&ctx, &settings.profile, &characters)?;
        }

        let fixed = run_ai_fixes(&ctx, &findings, &profile.qa_directives).await?;
        ai_total += fixed;
        applied_this_round += fixed;
        emit_progress(
            emitter,
            "ai_fix",
            findings.len(),
            mechanical_total + ai_total,
        );

        if round == 0 {
            let reviewed = run_hotspot_reviews(
                &ctx,
                layout,
                &findings,
                &tier1_names,
                &profile.qa_directives,
            )
            .await?;
            ai_total += reviewed;
            applied_this_round += reviewed;
            emit_progress(
                emitter,
                "review",
                findings.len(),
                mechanical_total + ai_total,
            );
        }

        findings = scan_and_store(&ctx, &settings.profile, &characters)?;
        if applied_this_round == 0 {
            break;
        }
    }

    render_docx_after_chapter(conn, layout, settings, emitter)?;
    RunRepo::new(conn).finish_run(&run_id, "completed")?;

    let summary = build_qa_summary_log(
        mechanical_total,
        ai_total,
        findings.len(),
        settings.target_language != "en",
    );
    emitter.emit(EngineEvent::LogLine {
        level: "info".to_string(),
        message: summary.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
    emit_progress(emitter, "done", findings.len(), mechanical_total + ai_total);

    Ok(QaLoopResult {
        rounds_run: MAX_ROUNDS,
        mechanical_fixes_count: mechanical_total,
        ai_fixes_count: ai_total,
        remaining_findings_count: findings.len(),
        summary_log: summary,
    })
}

fn emit_progress(emitter: &dyn EventEmitter, phase: &str, findings: usize, fixes: usize) {
    emitter.emit(EngineEvent::QaProgress {
        phase: phase.to_string(),
        findings_count: findings,
        fixes_count: fixes,
    });
}

/// Finished, translated chapters only; pending chapters have nothing to check yet.
fn load_finished(conn: &Connection) -> Result<(Vec<Chapter>, Vec<Vec<SegmentForQa>>), AppError> {
    let (chapters, segments) = load_volume_segments_for_qa(conn)?;
    let mut kept_chapters = Vec::new();
    let mut kept_segments = Vec::new();
    for (ch, segs) in chapters.into_iter().zip(segments) {
        if ch.status == "done" || ch.frozen {
            kept_chapters.push(ch);
            kept_segments.push(segs);
        }
    }
    Ok((kept_chapters, kept_segments))
}

/// Scans the volume and replaces the open findings in the database with the new result,
/// so the QA tab always shows what is outstanding right now.
fn scan_and_store(
    ctx: &QaContext<'_>,
    profile: &str,
    characters: &[CharacterGender],
) -> Result<Vec<QaFinding>, AppError> {
    let (_, segments) = load_finished(ctx.conn)?;
    let (findings, _) = if profile == "ja-en" {
        crate::qa::scan_ja_en(&segments, characters)
    } else {
        crate::qa::scan_ja_id(&segments)
    };

    ctx.conn.execute(
        "DELETE FROM finding WHERE status IN ('open', 'unresolved')",
        [],
    )?;
    let rows: Vec<Finding> = findings
        .iter()
        .map(|f| Finding {
            id: f.id.clone(),
            run_id: Some(ctx.run_id.clone()),
            check_code: f.check_code.clone(),
            chapter_id: Some(f.chapter_id.clone()),
            segment_id: Some(f.segment_id.clone()),
            severity: f.severity.clone(),
            detail_json: f.detail_json.clone(),
            status: "open".to_string(),
        })
        .collect();
    FindingRepo::new(ctx.conn).insert_batch(&rows)?;
    Ok(findings)
}

/// Sends semantic findings to the QA Fixer, one batch per check code that has a directive
/// in the profile's QA contract (that set is exactly the AI-handled codes of Appendix B).
async fn run_ai_fixes(
    ctx: &QaContext<'_>,
    findings: &[QaFinding],
    directives: &HashMap<String, String>,
) -> Result<usize, AppError> {
    let (chapters, segments) = load_finished(ctx.conn)?;
    let ordinal_of: HashMap<&str, i32> = chapters
        .iter()
        .map(|c| (c.id.as_str(), c.ordinal))
        .collect();
    let trans_repo = TranslationRepo::new(ctx.conn);
    let mut ranges: HashMap<(String, i32), i32> = HashMap::new();
    for ch in &chapters {
        for t in trans_repo.get_current_by_chapter(&ch.id)? {
            ranges.insert((ch.id.clone(), t.seg_from), t.seg_to);
        }
    }

    let mut by_code: HashMap<&str, Vec<&QaFinding>> = HashMap::new();
    for f in findings {
        if directives.contains_key(&f.check_code) {
            by_code.entry(f.check_code.as_str()).or_default().push(f);
        }
    }

    let role_cfg = ctx.settings.roles.for_role(Role::QaFixer);
    let mut applied = 0usize;
    for (code, code_findings) in by_code {
        let mut seen = HashSet::new();
        let mut items = Vec::new();
        for f in code_findings {
            if !seen.insert(f.segment_id.clone()) {
                continue;
            }
            let Some((ci, si)) = locate(&segments, &f.segment_id) else {
                continue;
            };
            let seg = &segments[ci][si];
            if seg.tgt_text.trim().is_empty() {
                continue;
            }
            let seg_to = ranges
                .get(&(seg.chapter_id.clone(), seg.idx))
                .copied()
                .unwrap_or(seg.idx);
            let ordinal = ordinal_of
                .get(seg.chapter_id.as_str())
                .copied()
                .unwrap_or(0);
            items.push(QaFixItem {
                id: format!("f{}", items.len() + 1),
                ref_str: if seg_to == seg.idx {
                    format!("{ordinal}:{}", seg.idx)
                } else {
                    format!("{ordinal}:{}-{seg_to}", seg.idx)
                },
                chapter_id: seg.chapter_id.clone(),
                seg_from: seg.idx,
                seg_to,
                source: seg.src_text.clone(),
                target: seg.tgt_text.clone(),
                prev_target: neighbor(&segments[ci], si, -1),
                next_target: neighbor(&segments[ci], si, 1),
                finding_id: Some(f.id.clone()),
            });
        }

        let directive = directives.get(code).map(String::as_str).unwrap_or("");
        for batch in items.chunks(FIX_BATCH) {
            let user = build_qa_fix_prompt(&QaFixPromptArgs {
                check_code: code,
                directive,
                glossary_text: &ctx.glossary_text,
                items: batch,
            });
            let Some(answer) = call_role(ctx, role_cfg, "qa_fixer", "qa_fix", &user).await? else {
                continue;
            };
            let responses = parse_qa_fix_response(&answer);
            applied +=
                apply_ai_fixes_to_db(ctx.conn, batch, &responses, &ctx.settings.profile)?.len();
        }
    }
    Ok(applied)
}

/// Step C: sends system-selected hotspots, with aligned source, to the QA Reviewer.
async fn run_hotspot_reviews(
    ctx: &QaContext<'_>,
    layout: &ProjectLayout,
    findings: &[QaFinding],
    tier1_names: &[(String, String)],
    directives: &HashMap<String, String>,
) -> Result<usize, AppError> {
    let (chapters, segments) = load_finished(ctx.conn)?;
    let hotspots = select_all_hotspots(&segments, findings, tier1_names, directives);
    let role_cfg = ctx.settings.roles.for_role(Role::QaReviewer);

    let mut applied = 0usize;
    let mut suspects: Vec<String> = Vec::new();
    for hotspot in &hotspots {
        applied += review_hotspot(ctx, layout, role_cfg, hotspot, &chapters, &mut suspects).await?;
    }

    // Homophone suspicion is systematic: gather every occurrence of each suspected token.
    let homophone_directive = directives
        .get("homophone")
        .map(String::as_str)
        .unwrap_or("");
    suspects.sort();
    suspects.dedup();
    for token in suspects {
        if let Some(h) = select_homophone_hotspots(&segments, &token, homophone_directive) {
            let mut ignore = Vec::new();
            applied += review_hotspot(ctx, layout, role_cfg, &h, &chapters, &mut ignore).await?;
        }
    }
    Ok(applied)
}

async fn review_hotspot(
    ctx: &QaContext<'_>,
    layout: &ProjectLayout,
    role_cfg: &RoleConfig,
    hotspot: &Hotspot,
    chapters: &[Chapter],
    suspects: &mut Vec<String>,
) -> Result<usize, AppError> {
    let mut applied = 0usize;
    for pairs in hotspot.pairs.chunks(REVIEW_BATCH) {
        let user = build_qa_review_prompt(&QaReviewArgs {
            kind: &hotspot.kind,
            directive: &hotspot.directive,
            glossary_text: &ctx.glossary_text,
            pairs,
        });
        let Some(answer) = call_role(ctx, role_cfg, "qa_reviewer", "qa_review", &user).await?
        else {
            continue;
        };
        for item in parse_qa_review_response(&answer) {
            match item {
                QaReviewResponseItem::Fix { ref_str, text } => {
                    if apply_review_fix(ctx.conn, &ref_str, &text, &ctx.settings.profile, chapters)?
                        .is_some()
                    {
                        applied += 1;
                    }
                }
                QaReviewResponseItem::Rename { source, to } => {
                    applied +=
                        apply_rename_to_volume(ctx.conn, Some(&layout.root), &source, &to)?.len();
                }
                QaReviewResponseItem::Suspect { token, .. } => suspects.push(token),
                QaReviewResponseItem::None => {}
            }
        }
    }
    Ok(applied)
}

/// Calls a QA role with its fallbacks, records each call, and returns the first usable answer.
async fn call_role(
    ctx: &QaContext<'_>,
    role_cfg: &RoleConfig,
    role_name: &str,
    kind: &str,
    user_prompt: &str,
) -> Result<Option<String>, AppError> {
    let call_repo = CallRepo::new(ctx.conn);
    let mut models = vec![role_cfg.primary.clone()];
    models.extend(role_cfg.fallbacks.clone());

    for model in &models {
        let Ok(provider) = ctx.resolver.resolve(model) else {
            continue;
        };
        let req = ProviderRequest {
            system_prompt: ctx.system_prompt.clone(),
            user_prompt: user_prompt.to_string(),
            model: model.model_id.clone(),
            temperature: role_cfg.temperature,
            max_output_tokens: role_cfg.max_output_tokens,
            reasoning_effort: role_cfg.reasoning_effort.clone(),
        };
        let started = std::time::Instant::now();
        let result = provider.send_request(&req, None).await;
        let latency_ms = started.elapsed().as_millis() as i64;

        let (status, finish, usage, text, error) = match &result {
            Ok(outcome) => {
                let finish = match outcome {
                    ProviderOutcome::Success(r) | ProviderOutcome::LengthTruncated(r) => {
                        r.finish_reason
                    }
                    ProviderOutcome::Refused { .. } => FinishReason::Refusal,
                    ProviderOutcome::ContentFiltered { .. } => FinishReason::ContentFilter,
                };
                (
                    if outcome.is_success() {
                        "success"
                    } else {
                        "failed"
                    },
                    Some(finish),
                    outcome.usage().cloned().unwrap_or_default(),
                    outcome.text().map(str::to_string),
                    None,
                )
            }
            Err(e) => (
                "failed",
                None,
                Default::default(),
                None,
                Some(e.to_string()),
            ),
        };
        let pricing = crate::provider::registry::ModelRegistry::default()
            .get(&model.provider, &model.model_id)
            .map(|m| m.pricing.clone());
        call_repo.insert(&Call {
            id: format!("call_{}", uuid::Uuid::new_v4().simple()),
            run_id: Some(ctx.run_id.clone()),
            role: role_name.to_string(),
            provider: model.provider.clone(),
            model: model.model_id.clone(),
            chapter_id: None,
            part_index: None,
            kind: kind.to_string(),
            status: status.to_string(),
            finish_reason: finish.map(|f| format!("{f:?}")),
            in_tokens: usage.input_tokens,
            cached_in: usage.cached_input_tokens,
            cache_write: usage.cache_write_tokens,
            out_tokens: usage.output_tokens,
            reasoning_tokens: usage.reasoning_tokens,
            cost: crate::usage::calculate_call_cost(
                pricing.as_ref(),
                usage.input_tokens,
                usage.cached_input_tokens,
                usage.output_tokens,
            ),
            latency_ms,
            contract_version: None,
            manifest_json: None,
            error,
            raw_response: text.clone(),
        })?;

        if let Ok(outcome) = result {
            if outcome.is_success() {
                return Ok(text);
            }
        }
    }
    Ok(None)
}

fn locate(segments: &[Vec<SegmentForQa>], segment_id: &str) -> Option<(usize, usize)> {
    segments.iter().enumerate().find_map(|(ci, chapter)| {
        chapter
            .iter()
            .position(|s| s.segment_id == segment_id)
            .map(|si| (ci, si))
    })
}

fn neighbor(chapter: &[SegmentForQa], index: usize, step: isize) -> Option<String> {
    let mut i = index as isize + step;
    while i >= 0 && (i as usize) < chapter.len() {
        let text = &chapter[i as usize].tgt_text;
        if !text.trim().is_empty() {
            return Some(text.clone());
        }
        i += step;
    }
    None
}
