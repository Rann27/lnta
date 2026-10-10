//! Chunk splitting algorithm per FR-PLN-01..05 and Story B3.

use serde::{Deserialize, Serialize};

use crate::raw::segment::{RawSegment, SegmentKind};

use super::estimate::{
    calculate_required_parts, estimate_chunk_tokens, EstimationWeights, ModelLimits,
};

/// A planned chapter part ready for execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedPart {
    pub part_index: i32,
    pub total_parts: i32,
    pub seg_from: i32,
    pub seg_to: i32,
    pub line_start: i32,
    pub line_end: i32,
    pub segment_count: usize,
    pub estimated_input_tokens: u32,
    pub estimated_output_tokens: u32,
}

/// The result of planning chunks for a chapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChapterPlanResult {
    pub chapter_file: String,
    pub total_lines: usize,
    pub total_segments: usize,
    pub parts: Vec<PlannedPart>,
}

/// Checks if a boundary between `segments[i]` and `segments[i + 1]` is forbidden.
///
/// Forbidden split points per FR-PLN-03:
/// - Adjacent to `image` or `image_missing` segment (neither before nor after).
/// - Between two consecutive `dialogue` segments.
/// - Between a `dialogue` segment and a following short `narration` (<= 40 characters).
/// - Title segment cannot be split from body alone if other segments exist.
pub fn is_forbidden_split(segments: &[RawSegment], i: usize) -> bool {
    if i + 1 >= segments.len() {
        return true;
    }

    let left = &segments[i];
    let right = &segments[i + 1];

    // 1. Image adjacent: neither side can be Image or ImageMissing
    if left.kind == SegmentKind::Image || left.kind == SegmentKind::ImageMissing {
        return true;
    }
    if right.kind == SegmentKind::Image || right.kind == SegmentKind::ImageMissing {
        return true;
    }

    // 2. Dialogue -> Dialogue
    if left.kind == SegmentKind::Dialogue && right.kind == SegmentKind::Dialogue {
        return true;
    }

    // 3. Dialogue -> short Narration (<= 40 characters)
    if left.kind == SegmentKind::Dialogue
        && right.kind == SegmentKind::Narration
        && right.src_text.chars().count() <= 40
    {
        return true;
    }

    // 4. Do not orphan title alone if there are other segments
    if i == 0 && left.kind == SegmentKind::Title && segments.len() > 2 {
        return true;
    }

    false
}

/// Boundary priority class per FR-PLN-03:
/// Priority 1: Scene marker within ±15% of target
/// Priority 2: Narration -> Narration
/// Priority 3: Narration -> Dialogue
/// Priority 4: Other allowed boundaries
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SplitPriority {
    SceneWithin15Percent = 1,
    NarrationToNarration = 2,
    NarrationToDialogue = 3,
    OtherAllowed = 4,
}

/// Evaluates the priority class of an allowed split point at `i`.
pub fn evaluate_split_priority(
    segments: &[RawSegment],
    i: usize,
    target: usize,
    tolerance: usize,
) -> SplitPriority {
    let left = &segments[i];
    let right = &segments[i + 1];

    let dist = (i as isize - target as isize).unsigned_abs();

    // Scene boundary within ±15%
    if (left.kind == SegmentKind::Scene || right.kind == SegmentKind::Scene) && dist <= tolerance {
        return SplitPriority::SceneWithin15Percent;
    }

    if left.kind == SegmentKind::Narration && right.kind == SegmentKind::Narration {
        return SplitPriority::NarrationToNarration;
    }

    if left.kind == SegmentKind::Narration && right.kind == SegmentKind::Dialogue {
        return SplitPriority::NarrationToDialogue;
    }

    SplitPriority::OtherAllowed
}

/// Plans chapter parts dividing segments into contiguous, non-overlapping parts.
pub fn plan_chapter_chunks(
    chapter_file: &str,
    segments: &[RawSegment],
    total_lines: usize,
    chunk_line_threshold: usize,
    limits: &ModelLimits,
    weights: &EstimationWeights,
) -> ChapterPlanResult {
    if segments.is_empty() {
        return ChapterPlanResult {
            chapter_file: chapter_file.to_string(),
            total_lines,
            total_segments: 0,
            parts: Vec::new(),
        };
    }

    let total_text_chars: usize = segments
        .iter()
        .filter(|s| s.kind.sent_to_model())
        .map(|s| s.src_text.chars().count())
        .sum();

    let num_parts = calculate_required_parts(
        total_lines,
        total_text_chars,
        chunk_line_threshold,
        limits,
        weights,
    );

    if num_parts <= 1 || segments.len() <= 1 {
        let (in_tok, out_tok) = estimate_chunk_tokens(segments, weights);
        let part = PlannedPart {
            part_index: 0,
            total_parts: 1,
            seg_from: segments[0].idx,
            seg_to: segments[segments.len() - 1].idx,
            line_start: segments[0].line_start,
            line_end: segments[segments.len() - 1].line_end,
            segment_count: segments.len(),
            estimated_input_tokens: in_tok,
            estimated_output_tokens: out_tok,
        };

        return ChapterPlanResult {
            chapter_file: chapter_file.to_string(),
            total_lines,
            total_segments: segments.len(),
            parts: vec![part],
        };
    }

    // Determine split cuts for num_parts
    let mut cut_indices = Vec::with_capacity(num_parts - 1);
    let mut last_cut = 0usize;

    for part_idx in 0..num_parts - 1 {
        let target = ((part_idx + 1) * segments.len()) / num_parts;
        // 15% tolerance of target or chunk span
        let tolerance = ((segments.len() as f64 / num_parts as f64) * 0.15)
            .round()
            .max(1.0) as usize;

        let min_idx = last_cut;
        let max_idx = segments.len() - (num_parts - part_idx);

        // Find the best split boundary in [min_idx..max_idx]
        let mut best_cut: Option<(SplitPriority, usize, usize)> = None;

        for i in min_idx..=max_idx {
            if is_forbidden_split(segments, i) {
                continue;
            }

            let prio = evaluate_split_priority(segments, i, target, tolerance);
            let dist = (i as isize - target as isize).unsigned_abs();

            let score = (prio, dist, i);
            match best_cut {
                None => best_cut = Some(score),
                Some(current) => {
                    // Compare (priority, distance)
                    if (score.0, score.1) < (current.0, current.1) {
                        best_cut = Some(score);
                    }
                }
            }
        }

        // If no allowed boundary was found, fallback to any boundary not image-adjacent
        let chosen_cut = match best_cut {
            Some((_, _, idx)) => idx,
            None => {
                let mut fallback_cut = target.clamp(min_idx, max_idx);
                // Ensure not adjacent to image if possible
                for i in min_idx..=max_idx {
                    if segments[i].kind != SegmentKind::Image
                        && segments[i + 1].kind != SegmentKind::Image
                    {
                        fallback_cut = i;
                        break;
                    }
                }
                fallback_cut
            }
        };

        cut_indices.push(chosen_cut);
        last_cut = chosen_cut + 1;
    }

    // Build PlannedParts
    let mut parts = Vec::with_capacity(num_parts);
    let mut start_idx = 0usize;

    for (part_idx, &cut_idx) in cut_indices.iter().enumerate() {
        let chunk_slice = &segments[start_idx..=cut_idx];
        let (in_tok, out_tok) = estimate_chunk_tokens(chunk_slice, weights);

        parts.push(PlannedPart {
            part_index: part_idx as i32,
            total_parts: num_parts as i32,
            seg_from: chunk_slice[0].idx,
            seg_to: chunk_slice[chunk_slice.len() - 1].idx,
            line_start: chunk_slice[0].line_start,
            line_end: chunk_slice[chunk_slice.len() - 1].line_end,
            segment_count: chunk_slice.len(),
            estimated_input_tokens: in_tok,
            estimated_output_tokens: out_tok,
        });

        start_idx = cut_idx + 1;
    }

    // Final part
    let chunk_slice = &segments[start_idx..];
    let (in_tok, out_tok) = estimate_chunk_tokens(chunk_slice, weights);

    parts.push(PlannedPart {
        part_index: (num_parts - 1) as i32,
        total_parts: num_parts as i32,
        seg_from: chunk_slice[0].idx,
        seg_to: chunk_slice[chunk_slice.len() - 1].idx,
        line_start: chunk_slice[0].line_start,
        line_end: chunk_slice[chunk_slice.len() - 1].line_end,
        segment_count: chunk_slice.len(),
        estimated_input_tokens: in_tok,
        estimated_output_tokens: out_tok,
    });

    ChapterPlanResult {
        chapter_file: chapter_file.to_string(),
        total_lines,
        total_segments: segments.len(),
        parts,
    }
}
