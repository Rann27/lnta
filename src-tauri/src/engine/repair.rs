//! Repair loop execution per FR-RUN-02, Story C1, and PRD §5.3.
//!
//! When segment validation fails, repair calls are triggered for the failing segments only
//! (with neighbors as context). Runs up to 2 rounds per model, then tries ordered fallback
//! models. If all fallbacks fail, the chapter is marked `needs_attention` and the run continues.

use crate::error::AppError;
use crate::project::settings::{ModelReference, RoleConfig};
use crate::prompt::assemble::{
    assemble_system_prompt, assemble_user_prompt, ContextPair, UserPromptArgs,
};
use crate::prompt::parse::{parse_model_output, ParsedModelOutput, ParsedSegment};
use crate::provider::types::{Provider, ProviderOutcome, ProviderRequest};
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
    _glossary_content: &str,
    context_pairs: &[ContextPair],
    chapter_file: &str,
    part_index: usize,
    part_count: usize,
) -> Result<RepairLoopOutcome, AppError> {
    let mut total_calls = 0;

    // Sequence of models to try: primary model first, followed by ordered fallbacks
    let mut models_to_try = vec![repair_role_cfg.primary.clone()];
    models_to_try.extend(repair_role_cfg.fallbacks.clone());

    let mut last_validation_errors = Vec::new();
    let val_cfg = ContentValidationConfig::default();

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
                &[],
                part_index as i32,
                &val_cfg,
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

            // Build repair segments (failing segments)
            let mut prompt_repair_segments = Vec::new();
            for raw_seg in raw_segments {
                if failing_ids.contains(&raw_seg.idx) {
                    prompt_repair_segments.push(raw_seg.clone());
                }
            }

            let args = UserPromptArgs {
                chapter_file,
                part_index: part_index as i32,
                total_parts: part_count as i32,
                selected_glossary: &[],
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
            let outcome = match provider.send_request(&req, None).await {
                Ok(o) => o,
                Err(e) => {
                    last_validation_errors.push(format!("Repair call error: {e}"));
                    break; // Move to next model
                }
            };

            match outcome {
                ProviderOutcome::Success(resp) | ProviderOutcome::LengthTruncated(resp) => {
                    if let Ok(parsed) = parse_model_output(&resp.text, &[]) {
                        if parsed.title.is_some() {
                            current_title = parsed.title;
                        }
                        for rep_seg in parsed.segments {
                            if let Some(existing) = current_translations
                                .iter_mut()
                                .find(|s| s.raw_id == rep_seg.raw_id)
                            {
                                *existing = rep_seg;
                            } else {
                                current_translations.push(rep_seg);
                            }
                        }
                        current_translations.sort_by_key(|s| s.id_start);
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
        &[],
        part_index as i32,
        &val_cfg,
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
