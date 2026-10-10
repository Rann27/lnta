//! LNTA chunk planner module.
//!
//! Implements FR-PLN-01..05:
//! - Multi-constraint part count calculation (lines, output tokens, context window)
//! - Forbidden split point enforcement (image-adjacent, dialogue-dialogue, dialogue-short narration)
//! - Priority split selection (scene boundaries within ±15% preferred, then narration-narration)
//! - Contiguous, non-overlapping segment coverage

pub mod estimate;
pub mod split;

pub use estimate::{
    calculate_required_parts, estimate_chunk_tokens, EstimationWeights, ModelLimits,
};
pub use split::{
    is_forbidden_split, plan_chapter_chunks, ChapterPlanResult, PlannedPart, SplitPriority,
};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::raw::segment::{RawSegment, SegmentKind};

    fn make_segment(idx: i32, kind: SegmentKind, text: &str, line: i32) -> RawSegment {
        RawSegment {
            idx,
            kind,
            src_text: text.to_string(),
            line_start: line,
            line_end: line,
            image_file: if kind == SegmentKind::Image {
                Some("img.jpg".to_string())
            } else {
                None
            },
        }
    }

    #[test]
    fn test_3200_line_chapter_yields_3_parts_similar_size() {
        // Create 3200 segments (simulating a 3200-line chapter)
        let mut segments = Vec::with_capacity(3200);
        segments.push(make_segment(0, SegmentKind::Title, "第一話", 1));

        for i in 1..=3199i32 {
            let line_num = i + 1;
            let kind = if i % 5 == 0 {
                SegmentKind::Dialogue
            } else {
                SegmentKind::Narration
            };
            segments.push(make_segment(i, kind, "これはテストの文です。", line_num));
        }

        let limits = ModelLimits::default();
        let weights = EstimationWeights::default();

        let plan = plan_chapter_chunks("01_chapter.md", &segments, 3200, 1500, &limits, &weights);

        assert_eq!(plan.parts.len(), 3);
        assert_eq!(plan.parts[0].part_index, 0);
        assert_eq!(plan.parts[1].part_index, 1);
        assert_eq!(plan.parts[2].part_index, 2);

        // Check that parts are of similar size (~1066 segments each, within 15% variation)
        let s0 = plan.parts[0].segment_count;
        let s1 = plan.parts[1].segment_count;
        let s2 = plan.parts[2].segment_count;

        assert!((s0 as isize - 1066).abs() < 100);
        assert!((s1 as isize - 1066).abs() < 100);
        assert!((s2 as isize - 1066).abs() < 100);
        assert_eq!(s0 + s1 + s2, 3200);
    }

    #[test]
    fn test_forbidden_split_points_never_chosen() {
        // Build segments with dangerous split boundaries
        let mut segments = Vec::new();
        segments.push(make_segment(0, SegmentKind::Title, "タイトル", 1));

        // Segment 1..10 narration
        for i in 1..=10 {
            segments.push(make_segment(
                i,
                SegmentKind::Narration,
                "長いナレーションの文章。",
                i + 1,
            ));
        }

        // Segment 11: dialogue
        segments.push(make_segment(11, SegmentKind::Dialogue, "「おはよう」", 12));
        // Segment 12: dialogue (consecutive dialogue - forbidden)
        segments.push(make_segment(
            12,
            SegmentKind::Dialogue,
            "「こんにちは」",
            13,
        ));
        // Segment 13: short narration (<= 40 chars, tag tag - forbidden after dialogue)
        segments.push(make_segment(
            13,
            SegmentKind::Narration,
            "健太は言った。",
            14,
        ));
        // Segment 14: image (image adjacent - forbidden)
        segments.push(make_segment(
            14,
            SegmentKind::Image,
            "Image Placeholder: x.jpg",
            15,
        ));
        // Segment 15: narration
        segments.push(make_segment(
            15,
            SegmentKind::Narration,
            "ナレーション文章。",
            16,
        ));

        // Verify forbidden checks directly
        assert!(is_forbidden_split(&segments, 11)); // Dialogue -> Dialogue
        assert!(is_forbidden_split(&segments, 12)); // Dialogue -> Short Narration (14 chars)
        assert!(is_forbidden_split(&segments, 13)); // Left of Image
        assert!(is_forbidden_split(&segments, 14)); // Right of Image

        // Allowed: between narration 9 and 10
        assert!(!is_forbidden_split(&segments, 9));
    }

    #[test]
    fn test_scene_boundary_within_15_percent_preferred() {
        // 100 segments, target split at 50
        // Tolerance is 15% of 50 = ~8 segments (indices 42..58)
        // Put scene marker at 46 (dist 4)
        // Put regular narration at 49, 50, 51
        let mut segments = Vec::new();
        for i in 0..100i32 {
            let kind = if i == 46 {
                SegmentKind::Scene
            } else {
                SegmentKind::Narration
            };
            segments.push(make_segment(i, kind, "ナレーション文章です。", i + 1));
        }

        let limits = ModelLimits::default();
        let weights = EstimationWeights::default();

        let plan = plan_chapter_chunks(
            "02_scene.md",
            &segments,
            100,
            50, // threshold 50 lines to force 2 parts
            &limits,
            &weights,
        );

        assert_eq!(plan.parts.len(), 2);
        // Part 0 should end at index 46 (the scene marker) despite target being 50!
        assert_eq!(plan.parts[0].seg_to, 46);
        assert_eq!(plan.parts[1].seg_from, 47);
    }

    #[test]
    fn test_property_parts_are_contiguous_and_cover_every_segment() {
        // Run against multiple chapter configurations
        let test_sizes = [2, 5, 20, 100, 500, 1600];

        for &size in &test_sizes {
            let mut segments = Vec::with_capacity(size);
            for i in 0..size {
                let kind = match i % 7 {
                    0 => SegmentKind::Narration,
                    1 => SegmentKind::Dialogue,
                    2 => SegmentKind::Narration,
                    3 => SegmentKind::Scene,
                    4 => SegmentKind::Narration,
                    5 => SegmentKind::Dialogue,
                    _ => SegmentKind::Narration,
                };
                segments.push(make_segment(
                    i as i32,
                    kind,
                    "テストのナレーション文章です。",
                    i as i32 + 1,
                ));
            }

            let limits = ModelLimits {
                context_window: 16000,
                max_output_tokens: 1000,
            };
            let weights = EstimationWeights::default();

            let plan = plan_chapter_chunks("test.md", &segments, size, 200, &limits, &weights);

            assert!(!plan.parts.is_empty());

            // 1. First part starts at first segment
            assert_eq!(plan.parts[0].seg_from, segments[0].idx);

            // 2. Last part ends at last segment
            assert_eq!(
                plan.parts[plan.parts.len() - 1].seg_to,
                segments[segments.len() - 1].idx
            );

            // 3. Contiguous and non-overlapping
            let mut covered_segments = 0;
            for p in 0..plan.parts.len() {
                assert!(plan.parts[p].seg_from <= plan.parts[p].seg_to);
                covered_segments += plan.parts[p].segment_count;

                if p + 1 < plan.parts.len() {
                    assert_eq!(plan.parts[p].seg_to + 1, plan.parts[p + 1].seg_from);
                }
            }

            // 4. Covers every segment
            assert_eq!(covered_segments, segments.len());
        }
    }
}
