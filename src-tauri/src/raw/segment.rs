//! Chapter segmentation and segment models per FR-RAW-01..08 and PRD §4.3.

use serde::{Deserialize, Serialize};

use super::classify::{Classifier, IllustrationSet, RawLineKind, DEFAULT_SCENE_MARKERS};
use super::ruby::normalize_ruby_text;
use super::title::{strip_enclosing_brackets, title_from_filename};

/// Segment kind representing work units per PRD §4.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentKind {
    Title,
    Narration,
    Dialogue,
    Scene,
    Image,
    ImageMissing,
}

impl SegmentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Narration => "narration",
            Self::Dialogue => "dialogue",
            Self::Scene => "scene",
            Self::Image => "image",
            Self::ImageMissing => "image_missing",
        }
    }

    /// Fixed segments are managed strictly by the system and never sent to a translation model.
    pub fn is_fixed(&self) -> bool {
        matches!(self, Self::Scene | Self::Image | Self::ImageMissing)
    }

    /// Sent to translation models.
    pub fn sent_to_model(&self) -> bool {
        matches!(self, Self::Title | Self::Narration | Self::Dialogue)
    }
}

/// A parsed segment with 1-based source line range and sequential chapter index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawSegment {
    pub idx: i32,
    pub kind: SegmentKind,
    pub src_text: String,
    pub line_start: i32,
    pub line_end: i32,
    pub image_file: Option<String>,
}

/// A parsed chapter holding structured metadata, title, and segments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedChapter {
    pub file_name: String,
    pub title_raw: Option<String>,
    pub title_clean: Option<String>,
    pub is_image_page: bool,
    pub ruby_count: usize,
    pub line_count: i32,
    pub char_count: i32,
    pub segments: Vec<RawSegment>,
}

/// Options controlling raw parsing and segmentation.
#[derive(Debug, Clone)]
pub struct ParseOptions {
    pub strip_brackets: bool,
    pub ruby_normalize: bool,
    pub scene_markers: Vec<char>,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            strip_brackets: true,
            ruby_normalize: true,
            scene_markers: DEFAULT_SCENE_MARKERS.to_vec(),
        }
    }
}

/// Parses and segments raw chapter content into structured segments.
pub fn parse_chapter(
    file_name: &str,
    content: &str,
    illustrations: &IllustrationSet,
    options: &ParseOptions,
) -> ParsedChapter {
    // 1. Ruby normalization (in memory only, per FR-RAW-07)
    let (normalized_content, ruby_count) = if options.ruby_normalize {
        normalize_ruby_text(content)
    } else {
        (content.to_string(), 0)
    };

    let raw_lines: Vec<&str> = normalized_content.split('\n').collect();
    let line_count = raw_lines.len() as i32;
    let char_count = normalized_content.chars().count() as i32;

    let classifier = Classifier::new(&options.scene_markers);

    // Classify all lines with 1-based line numbers
    let classified_lines: Vec<(i32, &str, RawLineKind)> = raw_lines
        .iter()
        .enumerate()
        .map(|(idx, line)| {
            let line_num = (idx + 1) as i32;
            let kind = classifier.classify(line, illustrations);
            (line_num, *line, kind)
        })
        .collect();

    // 2. Identify first non-blank line for heading / title
    let first_non_blank = classified_lines
        .iter()
        .find(|(_, _, kind)| *kind != RawLineKind::Blank);

    let mut title_heading_line: Option<i32> = None;
    let mut title_raw: Option<String> = None;
    let mut title_clean: Option<String> = None;
    let is_image_page: bool;

    if let Some(&(line_num, _, RawLineKind::Heading { ref text, .. })) = first_non_blank {
        // Line 1 is a markdown heading
        title_heading_line = Some(line_num);
        title_raw = Some(text.clone());
        title_clean = Some(if options.strip_brackets {
            strip_enclosing_brackets(text)
        } else {
            text.clone()
        });
        is_image_page = false;
    } else {
        // No heading on first non-blank line.
        // Check if there are any dialogue or narration lines in the entire chapter.
        let has_text_segments = classified_lines.iter().any(|(_, _, kind)| {
            matches!(
                kind,
                RawLineKind::Dialogue { .. } | RawLineKind::Narration { .. }
            )
        });

        if !has_text_segments {
            // Image-page chapter (FR-RAW-08): only image/scene lines, no title emitted.
            is_image_page = true;
        } else {
            // Chapter has text but no heading: derive title from filename
            let derived_raw = title_from_filename(file_name, false);
            let derived_clean = title_from_filename(file_name, options.strip_brackets);
            title_raw = Some(derived_raw);
            title_clean = Some(derived_clean);
            is_image_page = false;
        }
    }

    let mut segments = Vec::new();

    // 3. Add title segment if present (idx = 0)
    if let Some(clean_title) = &title_clean {
        let line_num = title_heading_line.unwrap_or(1);
        segments.push(RawSegment {
            idx: 0,
            kind: SegmentKind::Title,
            src_text: clean_title.clone(),
            line_start: line_num,
            line_end: line_num,
            image_file: None,
        });
    }

    // 4. Build body segments (1-indexed idx = 1, 2, 3...)
    let mut next_body_idx = 1;

    for (line_num, raw_line, kind) in classified_lines {
        // Skip consumed title heading line
        if Some(line_num) == title_heading_line {
            continue;
        }

        let trimmed = raw_line.trim();

        match kind {
            RawLineKind::Blank => {
                // Blank lines separate paragraphs and are not separate segments
            }
            RawLineKind::Heading { text, .. } => {
                // Subsequent headings inside chapter are treated as narration segments
                segments.push(RawSegment {
                    idx: next_body_idx,
                    kind: SegmentKind::Narration,
                    src_text: text,
                    line_start: line_num,
                    line_end: line_num,
                    image_file: None,
                });
                next_body_idx += 1;
            }
            RawLineKind::Dialogue { text } => {
                segments.push(RawSegment {
                    idx: next_body_idx,
                    kind: SegmentKind::Dialogue,
                    src_text: text,
                    line_start: line_num,
                    line_end: line_num,
                    image_file: None,
                });
                next_body_idx += 1;
            }
            RawLineKind::Narration { text } => {
                segments.push(RawSegment {
                    idx: next_body_idx,
                    kind: SegmentKind::Narration,
                    src_text: text,
                    line_start: line_num,
                    line_end: line_num,
                    image_file: None,
                });
                next_body_idx += 1;
            }
            RawLineKind::Scene => {
                segments.push(RawSegment {
                    idx: next_body_idx,
                    kind: SegmentKind::Scene,
                    src_text: trimmed.to_string(),
                    line_start: line_num,
                    line_end: line_num,
                    image_file: None,
                });
                next_body_idx += 1;
            }
            RawLineKind::Image { file } => {
                segments.push(RawSegment {
                    idx: next_body_idx,
                    kind: SegmentKind::Image,
                    src_text: trimmed.to_string(),
                    line_start: line_num,
                    line_end: line_num,
                    image_file: Some(file),
                });
                next_body_idx += 1;
            }
            RawLineKind::ImageMissing { placeholder } => {
                segments.push(RawSegment {
                    idx: next_body_idx,
                    kind: SegmentKind::ImageMissing,
                    src_text: placeholder,
                    line_start: line_num,
                    line_end: line_num,
                    image_file: None,
                });
                next_body_idx += 1;
            }
        }
    }

    ParsedChapter {
        file_name: file_name.to_string(),
        title_raw,
        title_clean,
        is_image_page,
        ruby_count,
        line_count,
        char_count,
        segments,
    }
}
