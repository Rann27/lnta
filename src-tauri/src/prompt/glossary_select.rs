//! Reference glossary selection and formatting per FR-PRM-03 and PRD §6.1.

use crate::glossary::{GlossaryEntryItem, GlossaryFile};
use crate::raw::RawSegment;

/// Formats the selected entries into a `<reference_glossary>` XML block.
///
/// Output format per PRD §6.2 and contract:
/// ```xml
/// <reference_glossary>
/// {Source} === {Translation} === {Note}
/// </reference_glossary>
/// ```
/// If note is empty, `{Source} === {Translation}` is emitted.
pub fn format_reference_glossary(entries: &[&GlossaryEntryItem]) -> String {
    let mut out = String::from("<reference_glossary>\n");
    for entry in entries {
        if entry.note.is_empty() {
            out.push_str(&format!("{} === {}\n", entry.source, entry.translation));
        } else {
            out.push_str(&format!(
                "{} === {} === {}\n",
                entry.source, entry.translation, entry.note
            ));
        }
    }
    out.push_str("</reference_glossary>");
    out
}

/// Selects glossary entries relevant to the given chunk segments and context per FR-PRM-03.
///
/// Rules:
/// - If `send_entire_glossary` is true, all active entries are returned.
/// - Otherwise:
///   - All Tier 1 entries are included unconditionally.
///   - Tier 2 entries (and other non-tier-1 entries) are included only if their `{Source}`
///     appears as an exact substring in the chunk segments or the context sources.
/// - Order of entries from the glossary is strictly preserved.
pub fn select_reference_glossary<'a>(
    entries: impl IntoIterator<Item = &'a GlossaryEntryItem>,
    chunk_segments: &[RawSegment],
    context_sources: &[&str],
    send_entire_glossary: bool,
) -> Vec<&'a GlossaryEntryItem> {
    let entries_vec: Vec<&'a GlossaryEntryItem> = entries.into_iter().collect();

    if send_entire_glossary {
        return entries_vec;
    }

    // Build haystack of all source text in chunk and context for fast substring checking
    let mut total_capacity = 0;
    for seg in chunk_segments {
        total_capacity += seg.src_text.len() + 1;
    }
    for ctx in context_sources {
        total_capacity += ctx.len() + 1;
    }

    let mut haystack = String::with_capacity(total_capacity);
    for seg in chunk_segments {
        haystack.push_str(&seg.src_text);
        haystack.push('\n');
    }
    for ctx in context_sources {
        haystack.push_str(ctx);
        haystack.push('\n');
    }

    entries_vec
        .into_iter()
        .filter(|entry| {
            if entry.tier == 1 {
                true
            } else {
                haystack.contains(&entry.source)
            }
        })
        .collect()
}

/// Helper selecting entries directly from a parsed `GlossaryFile`.
pub fn select_from_glossary_file<'a>(
    glossary_file: &'a GlossaryFile,
    chunk_segments: &[RawSegment],
    context_sources: &[&str],
    send_entire_glossary: bool,
) -> Vec<&'a GlossaryEntryItem> {
    let active_entries = glossary_file.entries();
    select_reference_glossary(
        active_entries,
        chunk_segments,
        context_sources,
        send_entire_glossary,
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::raw::SegmentKind;

    fn make_entry(source: &str, translation: &str, note: &str, tier: i32) -> GlossaryEntryItem {
        GlossaryEntryItem {
            source: source.to_string(),
            translation: translation.to_string(),
            note: note.to_string(),
            tier,
            line_index: 0,
            has_list_prefix: false,
            list_prefix_char: None,
        }
    }

    fn make_segment(idx: i32, kind: SegmentKind, src: &str) -> RawSegment {
        RawSegment {
            idx,
            kind,
            src_text: src.to_string(),
            line_start: idx,
            line_end: idx,
            image_file: None,
        }
    }

    #[test]
    fn test_select_tier1_always_and_tier2_when_present() {
        let entries = vec![
            make_entry("佐藤", "Satou", "laki-laki; protagonis", 1),
            make_entry("美咲", "Misaki", "perempuan; adik", 1),
            make_entry("王都", "Ibukota Kerajaan", "lokasi", 2),
            make_entry("聖剣", "Pedang Suci", "item", 2),
            make_entry("魔導学院", "Akademi Sihir", "organisasi", 2),
        ];

        let chunk = vec![
            make_segment(1, SegmentKind::Dialogue, "「王都へ向かうぞ」"),
            make_segment(2, SegmentKind::Narration, "佐藤は立ち上がった。"),
        ];

        // Notice: "魔導学院" is only in context
        let context = vec!["魔導学院での試験が終わった後……"];

        let selected = select_reference_glossary(
            &entries, &chunk, &context, false, // send_entire_glossary = false
        );

        let selected_sources: Vec<&str> = selected.iter().map(|e| e.source.as_str()).collect();

        // Tier 1 must be present even though 美咲 is not in chunk or context
        assert!(selected_sources.contains(&"佐藤"));
        assert!(selected_sources.contains(&"美咲"));

        // Tier 2 present in chunk
        assert!(selected_sources.contains(&"王都"));

        // Tier 2 present in context
        assert!(selected_sources.contains(&"魔導学院"));

        // Tier 2 not present anywhere must be excluded
        assert!(!selected_sources.contains(&"聖剣"));

        // Preserves order from glossary
        assert_eq!(selected_sources, vec!["佐藤", "美咲", "王都", "魔導学院"]);
    }

    #[test]
    fn test_send_entire_glossary_override() {
        let entries = vec![
            make_entry("佐藤", "Satou", "laki-laki", 1),
            make_entry("聖剣", "Pedang Suci", "item", 2),
        ];

        let chunk = vec![make_segment(1, SegmentKind::Narration, "静かな朝だった。")];

        let selected = select_reference_glossary(&entries, &chunk, &[], true);
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn test_format_reference_glossary() {
        let entries = [
            make_entry("佐藤", "Satou", "laki-laki; protagonis", 1),
            make_entry("鈴木", "Suzuki", "", 2),
        ];
        let refs: Vec<&GlossaryEntryItem> = entries.iter().collect();
        let formatted = format_reference_glossary(&refs);

        assert_eq!(
            formatted,
            "<reference_glossary>\n佐藤 === Satou === laki-laki; protagonis\n鈴木 === Suzuki\n</reference_glossary>"
        );
    }
}
