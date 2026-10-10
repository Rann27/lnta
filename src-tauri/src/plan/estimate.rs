//! Token estimation per FR-PLN-04 and part count calculation per FR-PLN-01..02.

use serde::{Deserialize, Serialize};

use crate::raw::segment::RawSegment;

/// Model window and output limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLimits {
    pub context_window: u32,
    pub max_output_tokens: u32,
}

impl Default for ModelLimits {
    fn default() -> Self {
        Self {
            // Modern defaults (e.g. 128k context, 16k max output like GPT-4o / Claude 3.5 Sonnet / Gemini)
            context_window: 128_000,
            max_output_tokens: 16_384,
        }
    }
}

/// Token estimation parameters per profile.
#[derive(Debug, Clone, Copy)]
pub struct EstimationWeights {
    /// Japanese characters per input token (default ~1.0 chars/token)
    pub chars_per_input_token: f64,
    /// Japanese source characters per translated output token (default ~2.0 chars/token for ID/EN)
    pub chars_per_output_token: f64,
    /// Fixed prompt overhead in tokens (contract + instructions + glossary prefix)
    pub prompt_overhead_tokens: u32,
}

impl Default for EstimationWeights {
    fn default() -> Self {
        Self {
            chars_per_input_token: 1.0,
            chars_per_output_token: 2.0,
            prompt_overhead_tokens: 2000,
        }
    }
}

/// Estimates input and output tokens for a set of segments.
pub fn estimate_chunk_tokens(segments: &[RawSegment], weights: &EstimationWeights) -> (u32, u32) {
    let mut total_chars = 0usize;

    for seg in segments {
        if seg.kind.sent_to_model() {
            total_chars += seg.src_text.chars().count();
        }
    }

    let input_content_tokens = (total_chars as f64 / weights.chars_per_input_token).ceil() as u32;
    let input_tokens = weights.prompt_overhead_tokens + input_content_tokens;

    let output_tokens = (total_chars as f64 / weights.chars_per_output_token).ceil() as u32;

    (input_tokens, output_tokens)
}

/// Calculates the minimum number of parts required to satisfy:
/// 1. line_count <= threshold (FR-PLN-01)
/// 2. estimated_output <= 70% of max_output_tokens (FR-PLN-02)
/// 3. estimated_total_input + max_output_tokens <= context_window (FR-PLN-02)
pub fn calculate_required_parts(
    total_lines: usize,
    total_text_chars: usize,
    chunk_line_threshold: usize,
    limits: &ModelLimits,
    weights: &EstimationWeights,
) -> usize {
    if total_lines == 0 {
        return 1;
    }

    // 1. Line count constraint: ceil(total_lines / chunk_line_threshold)
    let parts_by_lines = match chunk_line_threshold.checked_div(1) {
        Some(t) if t > 0 => total_lines.div_ceil(chunk_line_threshold),
        _ => 1,
    };

    // 2. Output token constraint: estimated_output <= 0.70 * max_output_tokens
    let safe_output_limit = ((limits.max_output_tokens as f64) * 0.70).floor() as u32;
    let safe_output_limit = safe_output_limit.max(1);

    let total_output_tokens =
        (total_text_chars as f64 / weights.chars_per_output_token).ceil() as u32;
    let parts_by_output = total_output_tokens.div_ceil(safe_output_limit);

    // 3. Context window constraint: total_input + max_output <= context_window
    // total_input = overhead + (chunk_chars / chars_per_input_token)
    let available_input_budget = limits
        .context_window
        .saturating_sub(limits.max_output_tokens)
        .saturating_sub(weights.prompt_overhead_tokens)
        .max(1);

    let total_input_content_tokens =
        (total_text_chars as f64 / weights.chars_per_input_token).ceil() as u32;
    let parts_by_input = total_input_content_tokens.div_ceil(available_input_budget);

    parts_by_lines
        .max(parts_by_output as usize)
        .max(parts_by_input as usize)
        .max(1)
}

/// Computes learned token estimation weights from completed chapter character counts
/// and actual model calls per FR-PLN-04 and Story B5.
pub fn compute_learned_ratios(
    completed_chapters_count: usize,
    total_completed_chars: usize,
    total_translator_in_tokens: u64,
    total_translator_out_tokens: u64,
    total_translator_calls: usize,
) -> Option<EstimationWeights> {
    if completed_chapters_count == 0
        || total_completed_chars == 0
        || total_translator_out_tokens == 0
        || total_translator_in_tokens == 0
    {
        return None;
    }

    let default_weights = EstimationWeights::default();
    let prompt_overhead =
        default_weights.prompt_overhead_tokens as u64 * total_translator_calls as u64;
    let content_in_tokens = if total_translator_in_tokens > prompt_overhead {
        total_translator_in_tokens - prompt_overhead
    } else {
        total_translator_in_tokens
    };

    // Calculate characters per token with sanity clamping
    let chars_per_in = (total_completed_chars as f64 / content_in_tokens as f64).clamp(0.1, 10.0);
    let chars_per_out =
        (total_completed_chars as f64 / total_translator_out_tokens as f64).clamp(0.2, 10.0);

    Some(EstimationWeights {
        chars_per_input_token: (chars_per_in * 100.0).round() / 100.0,
        chars_per_output_token: (chars_per_out * 100.0).round() / 100.0,
        prompt_overhead_tokens: default_weights.prompt_overhead_tokens,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_3200_line_chapter_yields_3_parts() {
        let limits = ModelLimits::default();
        let weights = EstimationWeights::default();

        // 3200 lines, moderate char count
        let num_parts = calculate_required_parts(3200, 30_000, 1500, &limits, &weights);

        assert_eq!(num_parts, 3);
    }

    #[test]
    fn test_output_constraint_raises_part_count_under_1500_lines() {
        // Only 1000 lines, but max_output_tokens is tiny (e.g. 500)
        let limits = ModelLimits {
            context_window: 8192,
            max_output_tokens: 500,
        };
        let weights = EstimationWeights::default();

        // 1000 lines with 20,000 chars -> ~20,000 output tokens
        // Safe limit is 0.7 * 500 = 350 tokens
        // ceil(20000 / 350) = 58 parts
        let num_parts = calculate_required_parts(1000, 20_000, 1500, &limits, &weights);

        assert!(num_parts > 1);
        assert_eq!(num_parts, 29);
    }

    #[test]
    fn test_context_window_constraint_raises_part_count() {
        // Context window very small
        let limits = ModelLimits {
            context_window: 4000,
            max_output_tokens: 1000,
        };
        let weights = EstimationWeights::default();

        // 500 lines, 10,000 chars
        // available input = 4000 - 1000 - 2000 = 1000 tokens
        // total input content = 10000 / 0.8 = 12500 tokens
        // ceil(12500 / 1000) = 13 parts
        let num_parts = calculate_required_parts(500, 10_000, 1500, &limits, &weights);

        assert!(num_parts > 1);
    }

    #[test]
    fn test_compute_learned_ratios_empty_or_zero() {
        assert!(compute_learned_ratios(0, 1000, 1500, 800, 1).is_none());
        assert!(compute_learned_ratios(1, 0, 1500, 800, 1).is_none());
        assert!(compute_learned_ratios(1, 1000, 0, 800, 1).is_none());
        assert!(compute_learned_ratios(1, 1000, 1500, 0, 1).is_none());
    }

    #[test]
    fn test_compute_learned_ratios_successful() {
        // 2 chapters, 20,000 chars, 2 calls
        // 2 calls overhead = 4,000 tok
        // in_tokens = 24,000 (content in = 20,000)
        // out_tokens = 10,000
        let weights = compute_learned_ratios(2, 20_000, 24_000, 10_000, 2).unwrap();
        assert_eq!(weights.chars_per_input_token, 1.0);
        assert_eq!(weights.chars_per_output_token, 2.0);

        // Another volume with more concise output
        // 1 chapter, 10,000 chars, 1 call
        // 1 call overhead = 2,000 tok
        // in_tokens = 12,000 (content in = 10,000)
        // out_tokens = 4,000 -> chars_per_output = 2.5
        let weights2 = compute_learned_ratios(1, 10_000, 12_000, 4_000, 1).unwrap();
        assert_eq!(weights2.chars_per_output_token, 2.5);
    }
}
