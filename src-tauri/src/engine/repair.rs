//! Repair loop execution per FR-RUN-02, Story C1, and PRD §5.3.
//!
//! When segment validation fails, repair calls are triggered for the failing segments only
//! (with neighbors as context). Runs up to 2 rounds per model, then tries ordered fallback
//! models. If all fallbacks fail, the chapter is marked `needs_attention` and the run continues.

use crate::error::AppError;
use crate::glossary::parse::GlossaryEntryItem;
use crate::project::settings::{ModelReference, RoleConfig};
use crate::prompt::assemble::{
    assemble_system_prompt, assemble_user_prompt, ContextPair, UserPromptArgs,
};
use crate::prompt::parse::{parse_model_output, ParsedModelOutput, ParsedSegment};
use crate::provider::types::{
    FinishReason, Provider, ProviderOutcome, ProviderRequest, TokenUsage,
};
use crate::provider::AnyProvider;
use crate::raw::RawSegment;
use crate::validate::content::ContentValidationConfig;
use crate::validate::validate_chunk;

/// Outcome of the repair loop.
#[derive(Debug, Clone, PartialEq)]
pub enum RepairLoopOutcome {
    Success {
        repaired_title: Option<String>,
        repaired_segments: Vec<ParsedSegment>,
        total_repair_calls: usize,
    },
    NeedsAttention {
        repaired_so_far: Vec<ParsedSegment>,
        reasons: Vec<String>,
        total_repair_calls: usize,
    },
}

/// One repair call, returned to the caller so it can be recorded in usage like any other call.
#[derive(Debug, Clone, PartialEq)]
pub struct RepairCall {
    pub model: ModelReference,
    pub usage: TokenUsage,
    pub finish_reason: Option<FinishReason>,
    pub latency_ms: i64,
    pub raw_response: Option<String>,
    pub error: Option<String>,
}

/// Replaces translations overlapping `incoming` so merged ranges such as `12-14` never
/// coexist with a repaired `13`, which would duplicate text in the docx.
fn merge_repaired_segment(current: &mut Vec<ParsedSegment>, incoming: ParsedSegment) {
    current.retain(|s| s.id_end < incoming.id_start || s.id_start > incoming.id_end);
    current.push(incoming);
    current.sort_by_key(|s| s.id_start);
}

/// Resolves a `Provider` implementation for a model reference.
pub trait ProviderResolver: Send + Sync {
    fn resolve(&self, reference: &ModelReference) -> Result<AnyProvider, AppError>;
}

/// Executes the repair loop for segments that failed initial chunk validation.
#[allow(clippy::too_many_arguments)]
pub async fn execute_repair_loop(
    raw_segments: &[RawSegment],
    mut current_title: Option<String>,
    mut current_translations: Vec<ParsedSegment>,
    repair_role_cfg: &RoleConfig,
    provider_resolver: &dyn ProviderResolver,
    contract_text: &str,
    instruction_text: &str,
    selected_glossary: &[&GlossaryEntryItem],
    val_cfg: &ContentValidationConfig,
    context_pairs: &[ContextPair],
    chapter_file: &str,
    part_index: usize,
    part_count: usize,
    call_log: &mut Vec<RepairCall>,
) -> Result<RepairLoopOutcome, AppError> {
    let mut total_calls = 0;

    // Sequence of models to try: primary model first, followed by ordered fallbacks
    let mut models_to_try = vec![repair_role_cfg.primary.clone()];
    models_to_try.extend(repair_role_cfg.fallbacks.clone());

    let mut last_validation_errors = Vec::new();

    for model_ref in &models_to_try {
        let provider = match provider_resolver.resolve(model_ref) {
            Ok(p) => p,
            Err(e) => {
                last_validation_errors.push(format!(
                    "Provider resolution failed for {}: {e}",
                    model_ref.model_id
                ));
                continue;
            }
        };

        // Up to 2 rounds per model per PRD §5.3
        for _round in 1..=2 {
            let model_output = ParsedModelOutput {
                title: current_title.clone(),
                segments: current_translations.clone(),
                glossary_proposals: Vec::new(),
                echoed_context: Vec::new(),
                duplicate_segments: Vec::new(),
                model_comments: Vec::new(),
                auto_closed_final_tag: false,
            };

            let validation = validate_chunk(
                &model_output,
                raw_segments,
                selected_glossary,
                part_index as i32,
                val_cfg,
            );

            if validation.is_valid {
                return Ok(RepairLoopOutcome::Success {
                    repaired_title: current_title,
                    repaired_segments: current_translations,
                    total_repair_calls: total_calls,
                });
            }

            let failing_ids: Vec<i32> = validation
                .repair_issues
                .iter()
                .map(|issue| issue.id)
                .collect();

            if failing_ids.is_empty() {
                return Ok(RepairLoopOutcome::Success {
                    repaired_title: current_title,
                    repaired_segments: current_translations,
                    total_repair_calls: total_calls,
                });
            }

            last_validation_errors = validation
                .repair_issues
                .iter()
                .map(|it| it.reason.clone())
                .collect();

            // A failing id inside a merged range sends the whole range, so the model can
            // rebuild that paragraph instead of leaving half of it behind.
            let mut repair_ids: Vec<i32> = Vec::new();
            for id in &failing_ids {
                match current_translations
                    .iter()
                    .find(|s| s.id_start <= *id && *id <= s.id_end)
                {
                    Some(range) => repair_ids.extend(range.id_start..=range.id_end),
                    None => repair_ids.push(*id),
                }
            }
            let prompt_repair_segments: Vec<RawSegment> = raw_segments
                .iter()
                .filter(|raw_seg| repair_ids.contains(&raw_seg.idx))
                .cloned()
                .collect();

            let args = UserPromptArgs {
                chapter_file,
                part_index: part_index as i32,
                total_parts: part_count as i32,
                selected_glossary,
                context_pairs,
                repair_issues: Some(&validation.repair_issues),
                chunk_segments: &prompt_repair_segments,
            };

            let req = ProviderRequest {
                system_prompt: assemble_system_prompt(contract_text, instruction_text),
                user_prompt: assemble_user_prompt(&args),
                model: model_ref.model_id.clone(),
                temperature: repair_role_cfg.temperature,
                max_output_tokens: repair_role_cfg.max_output_tokens,
                reasoning_effort: repair_role_cfg.reasoning_effort.clone(),
            };

            total_calls += 1;
            let started = std::time::Instant::now();
            let outcome = match provider.send_request(&req, None).await {
                Ok(o) => o,
                Err(e) => {
                    call_log.push(RepairCall {
                        model: model_ref.clone(),
                        usage: TokenUsage::default(),
                        finish_reason: None,
                        latency_ms: started.elapsed().as_millis() as i64,
                        raw_response: None,
                        error: Some(e.to_string()),
                    });
                    last_validation_errors.push(format!("Repair call error: {e}"));
                    break; // Move to next model
                }
            };
            call_log.push(RepairCall {
                model: model_ref.clone(),
                usage: outcome.usage().cloned().unwrap_or_default(),
                finish_reason: match &outcome {
                    ProviderOutcome::Success(r) | ProviderOutcome::LengthTruncated(r) => {
                        Some(r.finish_reason)
                    }
                    ProviderOutcome::Refused { .. } => Some(FinishReason::Refusal),
                    ProviderOutcome::ContentFiltered { .. } => Some(FinishReason::ContentFilter),
                },
                latency_ms: started.elapsed().as_millis() as i64,
                raw_response: outcome.text().map(str::to_string),
                error: None,
            });

            match outcome {
                ProviderOutcome::Success(resp) | ProviderOutcome::LengthTruncated(resp) => {
                    if let Ok(parsed) = parse_model_output(&resp.text, &[]) {
                        if parsed.title.is_some() {
                            current_title = parsed.title;
                        }
                        for rep_seg in parsed.segments {
                            merge_repaired_segment(&mut current_translations, rep_seg);
                        }
                    }
                }
                ProviderOutcome::Refused { reason }
                | ProviderOutcome::ContentFiltered { reason } => {
                    last_validation_errors.push(format!("Repair model refusal/filter: {reason}"));
                    break; // Move to fallback model
                }
            }
        }
    }

    // Final check
    let model_output = ParsedModelOutput {
        title: current_title.clone(),
        segments: current_translations.clone(),
        glossary_proposals: Vec::new(),
        echoed_context: Vec::new(),
        duplicate_segments: Vec::new(),
        model_comments: Vec::new(),
        auto_closed_final_tag: false,
    };

    let final_val = validate_chunk(
        &model_output,
        raw_segments,
        selected_glossary,
        part_index as i32,
        val_cfg,
    );

    if final_val.is_valid {
        Ok(RepairLoopOutcome::Success {
            repaired_title: current_title,
            repaired_segments: current_translations,
            total_repair_calls: total_calls,
        })
    } else {
        Ok(RepairLoopOutcome::NeedsAttention {
            repaired_so_far: current_translations,
            reasons: final_val
                .repair_issues
                .into_iter()
                .map(|it| it.reason)
                .collect(),
            total_repair_calls: total_calls,
        })
    }
}
