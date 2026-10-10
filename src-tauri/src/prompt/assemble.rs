//! Prompt assembly for system and user message blocks per PRD §6.1, §6.2, and FR-PRM-01..05.

use serde::{Deserialize, Serialize};

use crate::glossary::GlossaryEntryItem;
use crate::raw::{RawSegment, SegmentKind};

use super::glossary_select::format_reference_glossary;

/// Computes a deterministic 64-bit FNV-1a hex hash of text bytes.
pub fn compute_fnv1a_hash(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

/// A read-only prior translation pair passed in `<context readonly="true">`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextPair {
    pub id: i32,
    pub source: String,
    pub target: String,
}

/// An issue descriptor passed in `<repair>` for retranslation calls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairIssue {
    pub id: i32,
    pub reason: String,
}

/// System prompt blocks containing the profile contract and the user's instruction file.
///
/// Must remain byte-identical across parts of a run per FR-PRM-01.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemPromptBlocks {
    pub contract: String,
    pub instructions: String,
}

impl SystemPromptBlocks {
    pub fn new(contract: impl Into<String>, instructions: impl Into<String>) -> Self {
        Self {
            contract: contract.into(),
            instructions: instructions.into(),
        }
    }

    /// Assembles the combined system prompt string.
    pub fn assemble(&self) -> String {
        assemble_system_prompt(&self.contract, &self.instructions)
    }
}

/// Assembles system blocks into a single string.
/// Block 1 (Contract) + Block 2 (Instructions) verbatim, separated by two newlines.
pub fn assemble_system_prompt(contract_text: &str, instruction_text: &str) -> String {
    format!("{contract_text}\n\n{instruction_text}")
}

/// Arguments required to assemble a user translation message.
#[derive(Debug, Clone)]
pub struct UserPromptArgs<'a> {
    pub chapter_file: &'a str,
    pub part_index: i32,
    pub total_parts: i32,
    pub selected_glossary: &'a [&'a GlossaryEntryItem],
    pub context_pairs: &'a [ContextPair],
    pub repair_issues: Option<&'a [RepairIssue]>,
    pub chunk_segments: &'a [RawSegment],
}

/// Assembles user message blocks in exact order per contract and PRD §6.2:
/// 1. `<repair>` (if present and non-empty for repair calls)
/// 2. `<reference_glossary>` (always present)
/// 3. `<context readonly="true">` (if context pairs exist)
/// 4. `<chapter file="..." part="i" of="n">` (segment elements)
pub fn assemble_user_prompt(args: &UserPromptArgs<'_>) -> String {
    let mut out = String::new();

    // 1. Repair block first if present
    if let Some(issues) = args.repair_issues {
        if !issues.is_empty() {
            out.push_str("<repair>\n");
            for issue in issues {
                out.push_str(&format!(
                    "<issue id=\"{}\">{}</issue>\n",
                    issue.id, issue.reason
                ));
            }
            out.push_str("</repair>\n");
        }
    }

    // 2. Reference glossary
    out.push_str(&format_reference_glossary(args.selected_glossary));
    out.push('\n');

    // 3. Readonly context if present
    if !args.context_pairs.is_empty() {
        out.push_str("<context readonly=\"true\">\n");
        for pair in args.context_pairs {
            out.push_str(&format!(
                "<pair id=\"{}\"><source>{}</source><target>{}</target></pair>\n",
                pair.id, pair.source, pair.target
            ));
        }
        out.push_str("</context>\n");
    }

    // 4. Chapter block
    out.push_str(&format!(
        "<chapter file=\"{}\" part=\"{}\" of=\"{}\">\n",
        args.chapter_file, args.part_index, args.total_parts
    ));

    for seg in args.chunk_segments {
        match seg.kind {
            SegmentKind::Title => {
                out.push_str(&format!("<t>{}</t>\n", seg.src_text));
            }
            SegmentKind::Dialogue | SegmentKind::Narration => {
                out.push_str(&format!("<s id=\"{}\">{}</s>\n", seg.idx, seg.src_text));
            }
            SegmentKind::Scene => {
                out.push_str("<break/>\n");
            }
            SegmentKind::Image | SegmentKind::ImageMissing => {
                out.push_str("<image/>\n");
            }
        }
    }

    out.push_str("</chapter>\n");
    out
}

/// Prompt manifest stored per call per FR-PRM-05 and PRD §6.1.
///
/// Contains hashes and identifier ranges instead of the entire prompt text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptManifest {
    pub instruction_hash: String,
    pub contract_version: String,
    pub glossary_entry_sources: Vec<String>,
    pub segment_range: (i32, i32),
    pub part_index: i32,
    pub total_parts: i32,
    pub chapter_file: String,
    pub is_repair: bool,
}

impl PromptManifest {
    /// Builds a prompt manifest from prompt inputs.
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        instruction_text: &str,
        contract_version: &str,
        selected_glossary: &[&GlossaryEntryItem],
        chunk_segments: &[RawSegment],
        chapter_file: &str,
        part_index: i32,
        total_parts: i32,
        is_repair: bool,
    ) -> Self {
        let instruction_hash = compute_fnv1a_hash(instruction_text.as_bytes());
        let glossary_entry_sources = selected_glossary.iter().map(|e| e.source.clone()).collect();

        let mut min_id = i32::MAX;
        let mut max_id = i32::MIN;

        for seg in chunk_segments {
            if seg.idx < min_id {
                min_id = seg.idx;
            }
            if seg.idx > max_id {
                max_id = seg.idx;
            }
        }

        let segment_range = if min_id <= max_id {
            (min_id, max_id)
        } else {
            (0, 0)
        };

        Self {
            instruction_hash,
            contract_version: contract_version.to_string(),
            glossary_entry_sources,
            segment_range,
            part_index,
            total_parts,
            chapter_file: chapter_file.to_string(),
            is_repair,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

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
    fn test_system_prompt_byte_identical() {
        let contract = "# KONTRAK LNTA: JP → ID\nAturan kontrak...";
        let instructions = "# Petunjuk Penerjemahan\nPetunjuk khusus...";

        let prompt_part1 = assemble_system_prompt(contract, instructions);
        let prompt_part2 = assemble_system_prompt(contract, instructions);

        assert_eq!(prompt_part1, prompt_part2);
        assert_eq!(
            compute_fnv1a_hash(prompt_part1.as_bytes()),
            compute_fnv1a_hash(prompt_part2.as_bytes())
        );
    }

    #[test]
    fn test_user_prompt_order_normal_call() {
        let entries = [GlossaryEntryItem {
            source: "佐藤".to_string(),
            translation: "Satou".to_string(),
            note: "laki-laki".to_string(),
            tier: 1,
            line_index: 0,
            has_list_prefix: false,
            list_prefix_char: None,
        }];
        let selected_refs: Vec<&GlossaryEntryItem> = entries.iter().collect();

        let context = vec![ContextPair {
            id: 88,
            source: "前の話".to_string(),
            target: "Cerita sebelumnya".to_string(),
        }];

        let segments = vec![
            make_test_segment(0, SegmentKind::Title, "第1話　始まり"),
            make_test_segment(1, SegmentKind::Dialogue, "「おはよう」"),
            make_test_segment(2, SegmentKind::Scene, "☆"),
            make_test_segment(3, SegmentKind::Narration, "朝日が昇る。"),
            make_test_segment(4, SegmentKind::Image, "insert_01.jpg"),
        ];

        let args = UserPromptArgs {
            chapter_file: "01_第1話.md",
            part_index: 1,
            total_parts: 2,
            selected_glossary: &selected_refs,
            context_pairs: &context,
            repair_issues: None,
            chunk_segments: &segments,
        };

        let assembled = assemble_user_prompt(&args);

        // Verify order: <reference_glossary>, then <context>, then <chapter>
        let gloss_pos = assembled.find("<reference_glossary>").unwrap();
        let ctx_pos = assembled.find("<context readonly=\"true\">").unwrap();
        let chap_pos = assembled
            .find("<chapter file=\"01_第1話.md\" part=\"1\" of=\"2\">")
            .unwrap();

        assert!(gloss_pos < ctx_pos);
        assert!(ctx_pos < chap_pos);

        // Verify tags inside chapter
        assert!(assembled.contains("<t>第1話　始まり</t>"));
        assert!(assembled.contains("<s id=\"1\">「おはよう」</s>"));
        assert!(assembled.contains("<break/>"));
        assert!(assembled.contains("<s id=\"3\">朝日が昇る。</s>"));
        assert!(assembled.contains("<image/>"));
    }

    #[test]
    fn test_user_prompt_order_repair_call() {
        let issues = vec![
            RepairIssue {
                id: 17,
                reason: "segmen hilang".to_string(),
            },
            RepairIssue {
                id: 19,
                reason: "petik rusak".to_string(),
            },
        ];

        let segments = vec![make_test_segment(17, SegmentKind::Narration, "走った。")];

        let args = UserPromptArgs {
            chapter_file: "02_第2話.md",
            part_index: 1,
            total_parts: 1,
            selected_glossary: &[],
            context_pairs: &[],
            repair_issues: Some(&issues),
            chunk_segments: &segments,
        };

        let assembled = assemble_user_prompt(&args);

        // In repair calls, <repair> block must appear first
        let repair_pos = assembled.find("<repair>").unwrap();
        let gloss_pos = assembled.find("<reference_glossary>").unwrap();
        let chap_pos = assembled.find("<chapter").unwrap();

        assert!(repair_pos < gloss_pos);
        assert!(gloss_pos < chap_pos);
        assert!(assembled.contains("<issue id=\"17\">segmen hilang</issue>"));
    }

    #[test]
    fn test_prompt_manifest_creation() {
        let segments = vec![
            make_test_segment(10, SegmentKind::Narration, "A"),
            make_test_segment(20, SegmentKind::Narration, "B"),
        ];

        let manifest = PromptManifest::build(
            "Instructions content",
            "1.0",
            &[],
            &segments,
            "01_chapter.md",
            1,
            3,
            false,
        );

        assert_eq!(manifest.segment_range, (10, 20));
        assert_eq!(manifest.part_index, 1);
        assert_eq!(manifest.total_parts, 3);
        assert_eq!(manifest.contract_version, "1.0");
        assert!(!manifest.instruction_hash.is_empty());
    }
}
