//! Tolerant model output parser per PRD §6.4 and FR-VAL-12.

use std::collections::HashSet;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// A parsed translation segment from model output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedSegment {
    pub raw_id: String,
    pub id_start: i32,
    pub id_end: i32,
    pub text: String,
}

impl ParsedSegment {
    /// Returns true if this segment spans multiple source segments (e.g. "4-5").
    pub fn is_range(&self) -> bool {
        self.id_start != self.id_end
    }

    /// Returns the inclusive range of segment IDs covered.
    pub fn ids(&self) -> Vec<i32> {
        (self.id_start..=self.id_end).collect()
    }

    /// Splits segment text into multiple paragraphs separated by blank lines.
    pub fn paragraphs(&self) -> Vec<&str> {
        self.text
            .split("\n\n")
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .collect()
    }
}

/// A parsed glossary proposal from `<glossary>` block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedGlossaryProposal {
    pub source: String,
    pub translation: String,
    pub note: String,
}

/// Structured outcome of parsing model output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedModelOutput {
    pub title: Option<String>,
    pub segments: Vec<ParsedSegment>,
    pub glossary_proposals: Vec<ParsedGlossaryProposal>,
    pub echoed_context: Vec<ParsedSegment>,
    pub duplicate_segments: Vec<ParsedSegment>,
    pub model_comments: Vec<String>,
    pub auto_closed_final_tag: bool,
}

/// Extracts the segment ID (or ID range) from attribute string.
///
/// Handles:
/// - ASCII double/single quotes: `id="1"`, `id='1-2'`
/// - Typographic quotes: `id=“1”`, `id=”2”`, `id=‘3’`, `id=’4’`, `id=«5»`
/// - Casing and whitespace: `ID = " 4 - 5 "`
/// - Unquoted: `id=1`
fn parse_id_attribute(attrs: &str) -> Option<(String, i32, i32)> {
    let id_re = Regex::new(
        r#"(?i)\bid\s*=\s*(?:"([^"]*)"|'([^']*)'|“([^”"]*)”|”([^”"]*)”|‘([^’']*)’|’([^’']*)’|«([^»]*)»|([^\s>]+))"#,
    ).ok()?;

    let caps = id_re.captures(attrs)?;
    let raw_val = caps
        .get(1)
        .or_else(|| caps.get(2))
        .or_else(|| caps.get(3))
        .or_else(|| caps.get(4))
        .or_else(|| caps.get(5))
        .or_else(|| caps.get(6))
        .or_else(|| caps.get(7))
        .or_else(|| caps.get(8))?
        .as_str()
        .trim();

    if raw_val.contains('-') {
        let parts: Vec<&str> = raw_val.split('-').collect();
        if parts.len() == 2 {
            let start = parts[0].trim().parse::<i32>().ok()?;
            let end = parts[1].trim().parse::<i32>().ok()?;
            let (min_id, max_id) = if start <= end {
                (start, end)
            } else {
                (end, start)
            };
            Some((raw_val.to_string(), min_id, max_id))
        } else {
            None
        }
    } else {
        let single_id = raw_val.parse::<i32>().ok()?;
        Some((raw_val.to_string(), single_id, single_id))
    }
}

/// Filters out markdown code fences and returns non-empty model comments.
fn extract_comments(text: &str) -> Option<String> {
    let mut comment_lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            continue;
        }
        if !trimmed.is_empty() {
            comment_lines.push(trimmed);
        }
    }
    if comment_lines.is_empty() {
        None
    } else {
        Some(comment_lines.join("\n"))
    }
}

/// Parses glossary block content into proposals.
fn parse_glossary_block(text: &str) -> Vec<ParsedGlossaryProposal> {
    let mut proposals = Vec::new();
    for line in text.lines() {
        let mut trimmed = line.trim();
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            trimmed = trimmed[2..].trim();
        }
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = trimmed.split("===").collect();
        if parts.len() >= 3 {
            let source = parts[0].trim().to_string();
            let translation = parts[1].trim().to_string();
            let note = parts[2..].join("===").trim().to_string();
            if !source.is_empty() && !translation.is_empty() {
                proposals.push(ParsedGlossaryProposal {
                    source,
                    translation,
                    note,
                });
            }
        } else if parts.len() == 2 {
            let source = parts[0].trim().to_string();
            let translation = parts[1].trim().to_string();
            if !source.is_empty() && !translation.is_empty() {
                proposals.push(ParsedGlossaryProposal {
                    source,
                    translation,
                    note: String::new(),
                });
            }
        }
    }
    proposals
}

#[derive(Debug)]
enum ActiveTag {
    None,
    Title,
    Segment {
        raw_id: String,
        id_start: i32,
        id_end: i32,
    },
    Glossary,
}

/// Tolerantly parses model output per PRD §6.4 and FR-VAL-12.
///
/// Accepts:
/// - Code fences around output
/// - Missing final close tag (auto-closed with warning flag)
/// - Casing and whitespace variations in tags and attributes
/// - Typographic curly quotes in attributes
/// - Echoed context IDs (separated and filtered from segments)
/// - Duplicate IDs (first occurrence kept in segments, duplicates recorded)
/// - Range segments (`<s id="4-5">`)
/// - Multi-paragraph segments
/// - `<glossary>` block proposals
///
/// Fails with `AppError::ParseFailed` if no valid `<s>` or `<t>` tags exist.
pub fn parse_model_output(
    raw_output: &str,
    expected_context_ids: &[i32],
) -> Result<ParsedModelOutput, AppError> {
    let tag_re = Regex::new(
        r#"(?is)<\s*(/)?\s*(s|t|glossary|break|image|chapter|context|pair|repair|issue)\b([^>]*)>"#,
    )
    .map_err(|e| AppError::Internal {
        message: format!("Regex error: {e}"),
    })?;

    let mut title: Option<String> = None;
    let mut segments: Vec<ParsedSegment> = Vec::new();
    let mut glossary_proposals: Vec<ParsedGlossaryProposal> = Vec::new();
    let mut echoed_context: Vec<ParsedSegment> = Vec::new();
    let mut duplicate_segments: Vec<ParsedSegment> = Vec::new();
    let mut model_comments: Vec<String> = Vec::new();
    let mut auto_closed_final_tag = false;

    let mut seen_ids: HashSet<i32> = HashSet::new();
    let mut active = ActiveTag::None;
    let mut last_tag_end: usize = 0;

    for mat in tag_re.find_iter(raw_output) {
        let tag_match = mat.as_str();
        let caps = tag_re.captures(tag_match);
        let (is_closing, tag_name, attrs) = match caps {
            Some(c) => {
                let closing = c.get(1).is_some();
                let name = c
                    .get(2)
                    .map(|m| m.as_str().to_ascii_lowercase())
                    .unwrap_or_default();
                let a = c.get(3).map(|m| m.as_str()).unwrap_or("");
                (closing, name, a)
            }
            None => continue,
        };

        let between = &raw_output[last_tag_end..mat.start()];

        if is_closing {
            match (&active, tag_name.as_str()) {
                (ActiveTag::Title, "t") => {
                    title = Some(between.trim().to_string());
                    active = ActiveTag::None;
                }
                (
                    ActiveTag::Segment {
                        raw_id,
                        id_start,
                        id_end,
                    },
                    "s",
                ) => {
                    let seg = ParsedSegment {
                        raw_id: raw_id.clone(),
                        id_start: *id_start,
                        id_end: *id_end,
                        text: between.trim().to_string(),
                    };
                    process_segment(
                        seg,
                        expected_context_ids,
                        &mut seen_ids,
                        &mut segments,
                        &mut echoed_context,
                        &mut duplicate_segments,
                    );
                    active = ActiveTag::None;
                }
                (ActiveTag::Glossary, "glossary") => {
                    glossary_proposals.extend(parse_glossary_block(between));
                    active = ActiveTag::None;
                }
                _ => {
                    // Mismatched closing tag, ignore
                }
            }
        } else {
            // Opening tag encountered. If a previous tag was open, auto-close it.
            match active {
                ActiveTag::Title => {
                    title = Some(between.trim().to_string());
                    auto_closed_final_tag = true;
                }
                ActiveTag::Segment {
                    raw_id,
                    id_start,
                    id_end,
                } => {
                    let seg = ParsedSegment {
                        raw_id,
                        id_start,
                        id_end,
                        text: between.trim().to_string(),
                    };
                    process_segment(
                        seg,
                        expected_context_ids,
                        &mut seen_ids,
                        &mut segments,
                        &mut echoed_context,
                        &mut duplicate_segments,
                    );
                    auto_closed_final_tag = true;
                }
                ActiveTag::Glossary => {
                    glossary_proposals.extend(parse_glossary_block(between));
                    auto_closed_final_tag = true;
                }
                ActiveTag::None => {
                    if let Some(comment) = extract_comments(between) {
                        model_comments.push(comment);
                    }
                }
            }

            match tag_name.as_str() {
                "t" => {
                    active = ActiveTag::Title;
                }
                "s" => {
                    if let Some((raw_id, id_start, id_end)) = parse_id_attribute(attrs) {
                        active = ActiveTag::Segment {
                            raw_id,
                            id_start,
                            id_end,
                        };
                    } else {
                        active = ActiveTag::None;
                    }
                }
                "glossary" => {
                    active = ActiveTag::Glossary;
                }
                _ => {
                    active = ActiveTag::None;
                }
            }
        }

        last_tag_end = mat.end();
    }

    // Handle EOF content
    let remaining = &raw_output[last_tag_end..];
    match active {
        ActiveTag::Title => {
            title = Some(remaining.trim().to_string());
            auto_closed_final_tag = true;
        }
        ActiveTag::Segment {
            raw_id,
            id_start,
            id_end,
        } => {
            let seg = ParsedSegment {
                raw_id,
                id_start,
                id_end,
                text: remaining.trim().to_string(),
            };
            process_segment(
                seg,
                expected_context_ids,
                &mut seen_ids,
                &mut segments,
                &mut echoed_context,
                &mut duplicate_segments,
            );
            auto_closed_final_tag = true;
        }
        ActiveTag::Glossary => {
            glossary_proposals.extend(parse_glossary_block(remaining));
            auto_closed_final_tag = true;
        }
        ActiveTag::None => {
            if let Some(comment) = extract_comments(remaining) {
                model_comments.push(comment);
            }
        }
    }

    // Check for total parse failure: must have at least one valid <s> or <t>
    if segments.is_empty() && title.is_none() {
        return Err(AppError::ParseFailed {
            reason: "No valid <s> or <t> tags found in model output".to_string(),
        });
    }

    Ok(ParsedModelOutput {
        title,
        segments,
        glossary_proposals,
        echoed_context,
        duplicate_segments,
        model_comments,
        auto_closed_final_tag,
    })
}

fn process_segment(
    seg: ParsedSegment,
    expected_context_ids: &[i32],
    seen_ids: &mut HashSet<i32>,
    segments: &mut Vec<ParsedSegment>,
    echoed_context: &mut Vec<ParsedSegment>,
    duplicate_segments: &mut Vec<ParsedSegment>,
) {
    let ids = seg.ids();

    // 1. Check if echoed context
    let is_echoed = ids.iter().any(|id| expected_context_ids.contains(id));
    if is_echoed {
        echoed_context.push(seg);
        return;
    }

    // 2. Check if duplicate ID
    let is_duplicate = ids.iter().any(|id| seen_ids.contains(id));
    if is_duplicate {
        duplicate_segments.push(seg);
        return;
    }

    // 3. Fresh segment
    for id in ids {
        seen_ids.insert(id);
    }
    segments.push(seg);
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_normal_output() {
        let raw = r#"<t>Bab 1: Pagi Baru</t>
<s id="1">Pagi itu, aku terbangun.</s>
<s id="2">"Selamat pagi, Yuuko-san."</s>
<s id="4-5">Langkah kaki terdengar mendekat dari lorong. Suasana begitu hening.</s>
<s id="6">Paragraf pertama.

Paragraf kedua dari segmen yang sama.</s>
<glossary>
優子 === Yuuko === perempuan; dipanggil Yuuko-san
</glossary>"#;

        let parsed = parse_model_output(raw, &[]).unwrap();
        assert_eq!(parsed.title.as_deref(), Some("Bab 1: Pagi Baru"));
        assert_eq!(parsed.segments.len(), 4);
        assert_eq!(parsed.segments[0].id_start, 1);
        assert_eq!(parsed.segments[0].text, "Pagi itu, aku terbangun.");

        // Range segment
        assert_eq!(parsed.segments[2].raw_id, "4-5");
        assert_eq!(parsed.segments[2].id_start, 4);
        assert_eq!(parsed.segments[2].id_end, 5);
        assert!(parsed.segments[2].is_range());

        // Multi-paragraph segment
        assert_eq!(parsed.segments[3].id_start, 6);
        let paras = parsed.segments[3].paragraphs();
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0], "Paragraf pertama.");
        assert_eq!(paras[1], "Paragraf kedua dari segmen yang sama.");

        // Glossary block
        assert_eq!(parsed.glossary_proposals.len(), 1);
        assert_eq!(parsed.glossary_proposals[0].source, "優子");
        assert_eq!(parsed.glossary_proposals[0].translation, "Yuuko");
        assert!(!parsed.auto_closed_final_tag);
    }

    #[test]
    fn test_parse_code_fences_and_comments() {
        let raw = r#"```xml
Berikut adalah hasil terjemahan:
<t>Episode 1</t>
<s id="1">Teks satu</s>
Catatan: kalimat berikutnya digabung untuk kelancaran.
<s id="2-3">Teks dua dan tiga</s>
<glossary>
健太 === Kenta === laki-laki
</glossary>
Semoga membantu!
```"#;

        let parsed = parse_model_output(raw, &[]).unwrap();
        assert_eq!(parsed.title.as_deref(), Some("Episode 1"));
        assert_eq!(parsed.segments.len(), 2);
        assert_eq!(parsed.model_comments.len(), 3);
        assert!(parsed.model_comments[0].contains("Berikut adalah hasil terjemahan:"));
        assert!(parsed.model_comments[1].contains("Catatan:"));
        assert!(parsed.model_comments[2].contains("Semoga membantu!"));
    }

    #[test]
    fn test_parse_curly_quotes_and_attribute_spacing() {
        let raw = r#"<S ID=“1”>Teks petik ganda lengkung</S>
<s id = '2' >Teks petik tunggal lurus</s>
<s id=‘3’>Teks petik tunggal lengkung</s>
<s id = “4-5” >Teks range lengkung</s>
<s id=«6»>Teks guillemet</s>"#;

        let parsed = parse_model_output(raw, &[]).unwrap();
        assert_eq!(parsed.segments.len(), 5);
        assert_eq!(parsed.segments[0].id_start, 1);
        assert_eq!(parsed.segments[1].id_start, 2);
        assert_eq!(parsed.segments[2].id_start, 3);
        assert_eq!(parsed.segments[3].id_start, 4);
        assert_eq!(parsed.segments[3].id_end, 5);
        assert_eq!(parsed.segments[4].id_start, 6);
    }

    #[test]
    fn test_parse_missing_final_close_tag() {
        let raw = r#"<s id="1">Teks lengkap</s>
<s id="2">Teks terputus di akhir"#;

        let parsed = parse_model_output(raw, &[]).unwrap();
        assert_eq!(parsed.segments.len(), 2);
        assert_eq!(parsed.segments[1].text, "Teks terputus di akhir");
        assert!(parsed.auto_closed_final_tag);
    }

    #[test]
    fn test_parse_echoed_context_and_duplicates() {
        let raw = r#"<s id="88">Teks konteks yang diulang</s>
<s id="1">Teks asli pertama</s>
<s id="1">Teks duplikat</s>
<s id="2">Teks kedua</s>"#;

        let parsed = parse_model_output(raw, &[88]).unwrap();
        // Segment 88 must be in echoed_context
        assert_eq!(parsed.echoed_context.len(), 1);
        assert_eq!(parsed.echoed_context[0].id_start, 88);

        // Segment 1 first occurrence is in segments, duplicate is in duplicate_segments
        assert_eq!(parsed.segments.len(), 2);
        assert_eq!(parsed.segments[0].id_start, 1);
        assert_eq!(parsed.segments[1].id_start, 2);

        assert_eq!(parsed.duplicate_segments.len(), 1);
        assert_eq!(parsed.duplicate_segments[0].id_start, 1);
        assert_eq!(parsed.duplicate_segments[0].text, "Teks duplikat");
    }

    #[test]
    fn test_total_parse_failure() {
        let raw = "Maaf, saya tidak dapat menerjemahkan teks ini karena alasan kebijakan.";
        let err = parse_model_output(raw, &[]).unwrap_err();
        assert!(matches!(err, AppError::ParseFailed { .. }));
    }
}
