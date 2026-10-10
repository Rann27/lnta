//! Line classification per FR-RAW-03 and PRD §4.3.
//!
//! Classifies raw lines into:
//! - Blank
//! - Heading (markdown `#`)
//! - Image (placeholder resolved to existing illustration file)
//! - ImageMissing (placeholder with nonexistent file or empty name)
//! - Scene (scene marker lines)
//! - Dialogue (lines starting with 「, 『, or （)
//! - Narration (all other non-empty lines)

use std::collections::HashMap;
use std::path::Path;

use regex::Regex;

/// Index of available illustration files.
///
/// Ignores typeset copies (`* copy.webp`) and source assets (`*.psd`)
/// per FR-RAW-03 and AGENTS.md §4.
#[derive(Debug, Clone, Default)]
pub struct IllustrationSet {
    // Map of lowercase filename -> exact canonical filename
    files: HashMap<String, String>,
}

impl IllustrationSet {
    /// Builds an illustration set by scanning a directory.
    pub fn from_dir(dir: &Path) -> Self {
        let mut files = HashMap::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_file() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if Self::should_ignore(&name) {
                            continue;
                        }
                        files.insert(name.to_lowercase(), name);
                    }
                }
            }
        }
        Self { files }
    }

    /// Builds an illustration set from an iterator of file names (useful for tests).
    pub fn from_names<I: IntoIterator<Item = S>, S: AsRef<str>>(names: I) -> Self {
        let mut files = HashMap::new();
        for n in names {
            let name = n.as_ref().to_string();
            if Self::should_ignore(&name) {
                continue;
            }
            files.insert(name.to_lowercase(), name);
        }
        Self { files }
    }

    /// Ignores typeset copies (* copy.webp) and Photoshop files (*.psd).
    fn should_ignore(name: &str) -> bool {
        let lower = name.to_lowercase();
        lower.ends_with(".psd") || lower.ends_with(" copy.webp")
    }

    /// Looks up a file by exact name (case-insensitive for Windows filesystem compatibility).
    pub fn find(&self, name: &str) -> Option<&String> {
        let trimmed = name.trim();
        self.files.get(&trimmed.to_lowercase())
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// Classified kind of a raw line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawLineKind {
    Blank,
    Heading { level: usize, text: String },
    Image { file: String },
    ImageMissing { placeholder: String },
    Scene,
    Dialogue { text: String },
    Narration { text: String },
}

/// Default scene marker character set from PRD FR-RAW-05:
/// ◆ ◇ ★ ☆ ◈ ■ □
pub const DEFAULT_SCENE_MARKERS: &[char] = &['◆', '◇', '★', '☆', '◈', '■', '□'];

/// Line classifier using regex and scene marker rules.
pub struct Classifier {
    placeholder_re: Regex,
    markdown_img_re: Regex,
    bare_img_re: Regex,
    scene_markers: Vec<char>,
}

impl Default for Classifier {
    fn default() -> Self {
        Self::new(DEFAULT_SCENE_MARKERS)
    }
}

impl Classifier {
    pub fn new(scene_markers: &[char]) -> Self {
        // Regex patterns:
        // 1. Placeholder prefix: "Image PlaceHolder:", "Image Placeholder :", "Illustration Placeholder:"
        // Unwraps in new() are safe because patterns are statically verified regexes.
        #[allow(clippy::unwrap_used)]
        let placeholder_re =
            Regex::new(r"(?i)^\s*(?:image|illustration)\s+placeholder\s*:\s*(.*)$").unwrap();

        #[allow(clippy::unwrap_used)]
        let markdown_img_re = Regex::new(r"(?i)^\s*!\[([^\]]*)\]\(([^)]+)\)\s*$").unwrap();

        #[allow(clippy::unwrap_used)]
        let bare_img_re =
            Regex::new(r"(?i)^\s*([a-zA-Z0-9_\-.\s]+\.(?:jpg|jpeg|png|webp))\s*$").unwrap();

        Self {
            placeholder_re,
            markdown_img_re,
            bare_img_re,
            scene_markers: scene_markers.to_vec(),
        }
    }

    /// Classifies a single raw line.
    pub fn classify(&self, line: &str, illustrations: &IllustrationSet) -> RawLineKind {
        let trimmed = line.trim();

        // 1. Blank
        if trimmed.is_empty() {
            return RawLineKind::Blank;
        }

        // 2. Heading: starts with '#'
        if trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|&c| c == '#').count();
            let text = trimmed[level..].trim().to_string();
            return RawLineKind::Heading { level, text };
        }

        // 3. Image placeholder variants per FR-RAW-03:
        // Variant A: "Image Placeholder: <filename>"
        if let Some(caps) = self.placeholder_re.captures(trimmed) {
            let file_candidate = caps.get(1).map_or("", |m| m.as_str()).trim();
            if file_candidate.is_empty() {
                return RawLineKind::ImageMissing {
                    placeholder: trimmed.to_string(),
                };
            }
            return self.resolve_image(file_candidate, illustrations);
        }

        // Variant B: Markdown image syntax "![alt](filename)"
        if let Some(caps) = self.markdown_img_re.captures(trimmed) {
            let file_candidate = caps.get(2).map_or("", |m| m.as_str()).trim();
            // Extract filename component if candidate contains path delimiters
            let file_name = Path::new(file_candidate)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| file_candidate.to_string());
            return self.resolve_image(&file_name, illustrations);
        }

        // Variant C: Bare image filename alone on a line (e.g. "01.jpg", "cover.png")
        if let Some(caps) = self.bare_img_re.captures(trimmed) {
            let file_candidate = caps.get(1).map_or("", |m| m.as_str()).trim();
            return self.resolve_image(file_candidate, illustrations);
        }

        // 4. Scene marker line: entirely composed of scene marker characters or configured strings
        if self.is_scene_marker(trimmed) {
            return RawLineKind::Scene;
        }

        // 5. Dialogue: line starts with 「, 『, or （ (and ascii '(')
        if self.is_dialogue(trimmed) {
            return RawLineKind::Dialogue {
                text: trimmed.to_string(),
            };
        }

        // 6. Narration: any other non-empty line
        RawLineKind::Narration {
            text: trimmed.to_string(),
        }
    }

    /// Resolves an extracted image filename against the illustrations set.
    fn resolve_image(&self, candidate: &str, illustrations: &IllustrationSet) -> RawLineKind {
        if let Some(exact_name) = illustrations.find(candidate) {
            RawLineKind::Image {
                file: exact_name.clone(),
            }
        } else {
            RawLineKind::ImageMissing {
                placeholder: candidate.to_string(),
            }
        }
    }

    /// Checks if a trimmed line is a scene marker line.
    fn is_scene_marker(&self, trimmed: &str) -> bool {
        // Special multi-char markers
        if matches!(trimmed, "***" | "* * *" | "＊＊＊") {
            return true;
        }

        // Check if every non-whitespace character is in the configured scene marker set
        let mut has_marker = false;
        for c in trimmed.chars() {
            if c.is_whitespace() {
                continue;
            }
            if self.scene_markers.contains(&c) {
                has_marker = true;
            } else {
                return false;
            }
        }

        has_marker
    }

    /// Checks if a trimmed line starts with a dialogue opening character.
    fn is_dialogue(&self, trimmed: &str) -> bool {
        matches!(
            trimmed.chars().next(),
            Some('「') | Some('『') | Some('（') | Some('(')
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_illustrations_set_ignores_typeset_and_psd() {
        let set = IllustrationSet::from_names([
            "01.jpg",
            "01 copy.webp",
            "02.PSD",
            "02.png",
            "cover copy.webp",
        ]);

        assert_eq!(set.len(), 2);
        assert_eq!(set.find("01.jpg"), Some(&"01.jpg".to_string()));
        assert_eq!(set.find("01.JPG"), Some(&"01.jpg".to_string())); // case-insensitive
        assert_eq!(set.find("02.png"), Some(&"02.png".to_string()));
        assert_eq!(set.find("01 copy.webp"), None);
        assert_eq!(set.find("02.psd"), None);
    }

    #[test]
    fn test_classify_all_image_placeholder_variants() {
        let set = IllustrationSet::from_names(["x.jpg", "cover.png", "insert.webp"]);
        let clf = Classifier::default();

        // Variant 1: Image PlaceHolder
        assert_eq!(
            clf.classify("Image PlaceHolder: x.jpg", &set),
            RawLineKind::Image {
                file: "x.jpg".to_string()
            }
        );

        // Variant 1 with space before colon and case variation
        assert_eq!(
            clf.classify("Image Placeholder : x.jpg", &set),
            RawLineKind::Image {
                file: "x.jpg".to_string()
            }
        );

        // Variant 1 with Illustration Placeholder
        assert_eq!(
            clf.classify("Illustration Placeholder: cover.png", &set),
            RawLineKind::Image {
                file: "cover.png".to_string()
            }
        );

        // Variant 2: markdown syntax
        assert_eq!(
            clf.classify("![Color page](insert.webp)", &set),
            RawLineKind::Image {
                file: "insert.webp".to_string()
            }
        );

        // Variant 3: bare filename alone
        assert_eq!(
            clf.classify("x.jpg", &set),
            RawLineKind::Image {
                file: "x.jpg".to_string()
            }
        );

        // Missing file
        assert_eq!(
            clf.classify("Image Placeholder: not_found.jpg", &set),
            RawLineKind::ImageMissing {
                placeholder: "not_found.jpg".to_string()
            }
        );

        // Ignored file variants become missing
        assert_eq!(
            clf.classify("Image Placeholder: x copy.webp", &set),
            RawLineKind::ImageMissing {
                placeholder: "x copy.webp".to_string()
            }
        );
    }

    #[test]
    fn test_classify_scene_markers() {
        let clf = Classifier::default();
        let set = IllustrationSet::default();

        assert_eq!(clf.classify("☆", &set), RawLineKind::Scene);
        assert_eq!(clf.classify("◆", &set), RawLineKind::Scene);
        assert_eq!(clf.classify("◇", &set), RawLineKind::Scene);
        assert_eq!(clf.classify("***", &set), RawLineKind::Scene);
        assert_eq!(clf.classify("* * *", &set), RawLineKind::Scene);
        assert_eq!(clf.classify("＊＊＊", &set), RawLineKind::Scene);
        assert_eq!(clf.classify("◆ ◇ ★", &set), RawLineKind::Scene);

        // Marker followed by text is NOT a scene marker line
        assert!(matches!(
            clf.classify("☆ その時、何かが動いた。", &set),
            RawLineKind::Narration { .. }
        ));
    }

    #[test]
    fn test_classify_dialogue_and_narration() {
        let clf = Classifier::default();
        let set = IllustrationSet::default();

        assert!(matches!(
            clf.classify("「おはよう、健太」", &set),
            RawLineKind::Dialogue { .. }
        ));
        assert!(matches!(
            clf.classify("『魔導の書』", &set),
            RawLineKind::Dialogue { .. }
        ));
        assert!(matches!(
            clf.classify("（どうしてこうなった……）", &set),
            RawLineKind::Dialogue { .. }
        ));
        assert!(matches!(
            clf.classify("朝の光が差し込んでいた。", &set),
            RawLineKind::Narration { .. }
        ));
    }
}
