//! Title extraction and decorative bracket stripping per FR-RAW-04 and FR-RAW-08.
//!
//! Handles:
//! - Extracting title from first-line markdown heading (`# 【...】`).
//! - Stripping outer decorative brackets (`【】`, `〔〕`, `［］`, `[]`) if they enclose the whole title.
//! - Deriving fallback title from chapter filename when no heading is present.
//! - Detecting image-page chapters (no heading and only image/scene segments) which emit NO title.

use regex::Regex;

/// Strips a single pair of decorative brackets that wrap the entire title.
/// Pairs: 【】, 〔〕, ［］, []
pub fn strip_enclosing_brackets(title: &str) -> String {
    let trimmed = title.trim();
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() < 2 {
        return trimmed.to_string();
    }

    let first = chars[0];
    let last = chars[chars.len() - 1];

    let is_pair = matches!(
        (first, last),
        ('【', '】') | ('〔', '〕') | ('［', '］') | ('[', ']')
    );

    if is_pair {
        let inner: String = chars[1..chars.len() - 1].iter().collect();
        inner.trim().to_string()
    } else {
        trimmed.to_string()
    }
}

/// Derives a clean title from a chapter filename (e.g. `02_第一話_始まり.md` -> `第一話 始まり`).
pub fn title_from_filename(filename: &str, strip_brackets: bool) -> String {
    #[allow(clippy::unwrap_used)]
    let re = Regex::new(r"(?i)^(\d+)[ _\-.]?(.*)\.md$").unwrap();

    let raw = if let Some(caps) = re.captures(filename) {
        let rest = caps.get(2).map_or("", |m| m.as_str());
        rest.replace('_', " ").trim().to_string()
    } else {
        filename
            .trim_end_matches(".md")
            .replace('_', " ")
            .trim()
            .to_string()
    };

    if strip_brackets {
        strip_enclosing_brackets(&raw)
    } else {
        raw
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_enclosing_brackets() {
        assert_eq!(strip_enclosing_brackets("【第一話】"), "第一話");
        assert_eq!(
            strip_enclosing_brackets("【第一話　始まりの朝】"),
            "第一話　始まりの朝"
        );
        assert_eq!(strip_enclosing_brackets("〔プロローグ〕"), "プロローグ");
        assert_eq!(strip_enclosing_brackets("［幕間］"), "幕間");
        assert_eq!(strip_enclosing_brackets("[Chapter 1]"), "Chapter 1");

        // Does not strip when closing bracket is not at the end
        assert_eq!(
            strip_enclosing_brackets("【第一話】始まりの朝"),
            "【第一話】始まりの朝"
        );
        assert_eq!(strip_enclosing_brackets("普通の見出し"), "普通の見出し");
    }

    #[test]
    fn test_title_from_filename() {
        assert_eq!(title_from_filename("02_第一話.md", true), "第一話");
        assert_eq!(
            title_from_filename("03_【第二話】_朝.md", true),
            "【第二話】 朝"
        );
        assert_eq!(title_from_filename("04_【第三話】.md", true), "第三話");
    }
}
