//! LNTA prompt assembly and model output parser module per PRD §6 and FR-PRM-01..05.

pub mod assemble;
pub mod glossary_select;
pub mod parse;

pub use assemble::{
    assemble_system_prompt, assemble_user_prompt, compute_fnv1a_hash, ContextPair, PromptManifest,
    RepairIssue, SystemPromptBlocks, UserPromptArgs,
};
pub use glossary_select::{
    format_reference_glossary, select_from_glossary_file, select_reference_glossary,
};
pub use parse::{parse_model_output, ParsedGlossaryProposal, ParsedModelOutput, ParsedSegment};

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::glossary::GlossaryEntryItem;
    use crate::raw::{RawSegment, SegmentKind};
    use std::fs;
    use std::path::PathBuf;

    fn testdata_path(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("testdata")
            .join(rel)
    }

    fn normalize_crlf(s: &str) -> String {
        s.replace("\r\n", "\n")
    }

    fn make_test_segment(idx: i32, kind: SegmentKind, text: &str) -> RawSegment {
        RawSegment {
            idx,
            kind,
            src_text: text.to_string(),
            line_start: idx,
            line_end: idx,
            image_file: None,
        }
    }

    #[test]
    fn test_golden_prompt_part1() {
        let entries = [
            GlossaryEntryItem {
                source: "佐藤".to_string(),
                translation: "Satou".to_string(),
                note: "laki-laki; protagonis".to_string(),
                tier: 1,
                line_index: 0,
                has_list_prefix: false,
                list_prefix_char: None,
            },
            GlossaryEntryItem {
                source: "美咲".to_string(),
                translation: "Misaki".to_string(),
                note: "perempuan; adik".to_string(),
                tier: 1,
                line_index: 1,
                has_list_prefix: false,
                list_prefix_char: None,
            },
            GlossaryEntryItem {
                source: "王都".to_string(),
                translation: "Ibukota Kerajaan".to_string(),
                note: "lokasi".to_string(),
                tier: 2,
                line_index: 2,
                has_list_prefix: false,
                list_prefix_char: None,
            },
        ];
        let selected: Vec<&GlossaryEntryItem> = entries.iter().collect();

        let segments = vec![
            make_test_segment(0, SegmentKind::Title, "第1話　旅立ちの朝"),
            make_test_segment(
                1,
                SegmentKind::Narration,
                "朝、目が覚めると部屋の中に柔らかな光が差し込んでいた。",
            ),
            make_test_segment(2, SegmentKind::Dialogue, "「おはよう、美咲《みさき》」"),
            make_test_segment(3, SegmentKind::Scene, "☆"),
            make_test_segment(
                4,
                SegmentKind::Narration,
                "佐藤は荷物をまとめ、王都へ向かう準備を整えた。",
            ),
            make_test_segment(5, SegmentKind::Image, "insert_01.jpg"),
            make_test_segment(
                6,
                SegmentKind::Narration,
                "扉を開けると、冷たい風が頬をかすめた。",
            ),
        ];

        let args = UserPromptArgs {
            chapter_file: "02_第1話.md",
            part_index: 1,
            total_parts: 2,
            selected_glossary: &selected,
            context_pairs: &[],
            repair_issues: None,
            chunk_segments: &segments,
        };

        let assembled = assemble_user_prompt(&args);
        let golden_path = testdata_path("golden/prompts/ja_id_part1_prompt.golden");
        let expected = fs::read_to_string(&golden_path).unwrap();

        assert_eq!(normalize_crlf(&assembled), normalize_crlf(&expected));
    }

    #[test]
    fn test_golden_prompt_part2() {
        let entries = [
            GlossaryEntryItem {
                source: "佐藤".to_string(),
                translation: "Satou".to_string(),
                note: "laki-laki; protagonis".to_string(),
                tier: 1,
                line_index: 0,
                has_list_prefix: false,
                list_prefix_char: None,
            },
            GlossaryEntryItem {
                source: "美咲".to_string(),
                translation: "Misaki".to_string(),
                note: "perempuan; adik".to_string(),
                tier: 1,
                line_index: 1,
                has_list_prefix: false,
                list_prefix_char: None,
            },
            GlossaryEntryItem {
                source: "魔導学院".to_string(),
                translation: "Akademi Sihir".to_string(),
                note: "organisasi".to_string(),
                tier: 2,
                line_index: 2,
                has_list_prefix: false,
                list_prefix_char: None,
            },
        ];
        let selected: Vec<&GlossaryEntryItem> = entries.iter().collect();

        let context = vec![
            ContextPair {
                id: 4,
                source: "佐藤は荷物をまとめ、王都へ向かう準備を整えた。".to_string(),
                target: "Satou mengemas barang bawaannya dan bersiap menuju Ibukota Kerajaan."
                    .to_string(),
            },
            ContextPair {
                id: 6,
                source: "扉を開けると、冷たい風が頬をかすめた。".to_string(),
                target: "Saat ia membuka pintu, hembusan angin dingin menyentuh pipinya."
                    .to_string(),
            },
        ];

        let segments = vec![
            make_test_segment(
                7,
                SegmentKind::Narration,
                "街道沿いには多くの旅人が歩いていた。",
            ),
            make_test_segment(
                8,
                SegmentKind::Dialogue,
                "「魔導学院まではあと三日の道のりだな」",
            ),
        ];

        let args = UserPromptArgs {
            chapter_file: "02_第1話.md",
            part_index: 2,
            total_parts: 2,
            selected_glossary: &selected,
            context_pairs: &context,
            repair_issues: None,
            chunk_segments: &segments,
        };

        let assembled = assemble_user_prompt(&args);
        let golden_path = testdata_path("golden/prompts/ja_id_part2_prompt.golden");
        let expected = fs::read_to_string(&golden_path).unwrap();

        assert_eq!(normalize_crlf(&assembled), normalize_crlf(&expected));
    }

    #[test]
    fn test_golden_prompt_repair() {
        let issues = vec![
            RepairIssue {
                id: 17,
                reason: "segmen hilang dari output sebelumnya".to_string(),
            },
            RepairIssue {
                id: 19,
                reason: "tanda petik ganda tidak seimbang".to_string(),
            },
        ];

        let entries = [GlossaryEntryItem {
            source: "佐藤".to_string(),
            translation: "Satou".to_string(),
            note: "laki-laki; protagonis".to_string(),
            tier: 1,
            line_index: 0,
            has_list_prefix: false,
            list_prefix_char: None,
        }];
        let selected: Vec<&GlossaryEntryItem> = entries.iter().collect();

        let context = vec![
            ContextPair {
                id: 16,
                source: "前夜の出来事を思い返していた。".to_string(),
                target: "Ia merenungkan kembali kejadian malam sebelumnya.".to_string(),
            },
            ContextPair {
                id: 18,
                source: "風が止み、沈黙が訪れた。".to_string(),
                target: "Angin mereda, dan keheningan pun menyelimuti.".to_string(),
            },
        ];

        let segments = vec![
            make_test_segment(17, SegmentKind::Narration, "佐藤は静かに息を呑んだ。"),
            make_test_segment(
                19,
                SegmentKind::Dialogue,
                "「本当にこれで良かったのだろうか」",
            ),
        ];

        let args = UserPromptArgs {
            chapter_file: "02_第1話.md",
            part_index: 1,
            total_parts: 1,
            selected_glossary: &selected,
            context_pairs: &context,
            repair_issues: Some(&issues),
            chunk_segments: &segments,
        };

        let assembled = assemble_user_prompt(&args);
        let golden_path = testdata_path("golden/prompts/ja_id_repair_prompt.golden");
        let expected = fs::read_to_string(&golden_path).unwrap();

        assert_eq!(normalize_crlf(&assembled), normalize_crlf(&expected));
    }

    #[test]
    fn test_golden_model_outputs_files() {
        // 1. Normal output
        let normal_raw =
            fs::read_to_string(testdata_path("model-outputs/normal_output.xml")).unwrap();
        let normal_parsed = parse_model_output(&normal_raw, &[]).unwrap();
        assert_eq!(normal_parsed.segments.len(), 4);
        assert_eq!(normal_parsed.glossary_proposals.len(), 1);

        // 2. Code fenced with comments
        let fenced_raw =
            fs::read_to_string(testdata_path("model-outputs/code_fenced_with_comments.xml"))
                .unwrap();
        let fenced_parsed = parse_model_output(&fenced_raw, &[]).unwrap();
        assert_eq!(fenced_parsed.segments.len(), 2);
        assert!(!fenced_parsed.model_comments.is_empty());

        // 3. Malformed tags & unclosed final tag
        let malformed_raw =
            fs::read_to_string(testdata_path("model-outputs/malformed_tags.xml")).unwrap();
        let malformed_parsed = parse_model_output(&malformed_raw, &[]).unwrap();
        assert_eq!(malformed_parsed.segments.len(), 4);
        assert!(malformed_parsed.auto_closed_final_tag);

        // 4. Echoed context and duplicates
        let echo_raw =
            fs::read_to_string(testdata_path("model-outputs/echoed_context_and_dups.xml")).unwrap();
        let echo_parsed = parse_model_output(&echo_raw, &[98, 99]).unwrap();
        assert_eq!(echo_parsed.echoed_context.len(), 2);
        assert_eq!(echo_parsed.segments.len(), 2);
        assert_eq!(echo_parsed.duplicate_segments.len(), 1);

        // 5. Repair output
        let repair_raw =
            fs::read_to_string(testdata_path("model-outputs/repair_output.xml")).unwrap();
        let repair_parsed = parse_model_output(&repair_raw, &[]).unwrap();
        assert_eq!(repair_parsed.segments.len(), 2);
        assert_eq!(repair_parsed.segments[0].id_start, 17);
        assert_eq!(repair_parsed.segments[1].id_start, 19);
    }
}
