//! Ruby normalization per FR-RAW-07.
//!
//! EPUB conversion raw files encode furigana as ASCII-spaced ruby:
//! e.g. ` 呆 あき れた` -> `呆《あき》れた`, ` 健 けん 太 た ` -> `健《けん》太《た》`.
//! This module normalizes these patterns in memory while tracking normalization counts.

/// Checks whether a character is a CJK ideograph or iteration mark.
pub fn is_ideograph(c: char) -> bool {
    matches!(
        c,
        '\u{4E00}'..='\u{9FFF}'   // CJK Unified Ideographs
        | '\u{3400}'..='\u{4DBF}' // CJK Unified Ideographs Extension A
        | '\u{F900}'..='\u{FAFF}' // CJK Compatibility Ideographs
        | '々'                    // Ideographic iteration mark
        | '〆'                    // Ideographic closing mark
        | 'ヶ'                    // Small ke
        | '〇'                    // Ideographic zero
    )
}

/// Checks whether a character is Hiragana, Katakana, or long-vowel mark.
pub fn is_kana(c: char) -> bool {
    matches!(
        c,
        '\u{3040}'..='\u{309F}'   // Hiragana
        | '\u{30A0}'..='\u{30FF}' // Katakana (includes 'ー' at \u{30FC})
    )
}

struct RubyMatch<'a> {
    base: &'a str,
    reading: &'a str,
    end_byte: usize,
}

/// Tries to match a ruby pattern at the beginning of `slice`.
/// Pattern: ' ' + [ideographs]+ + ' ' + [kana]+ + ' '
fn try_match_ruby(slice: &str) -> Option<RubyMatch<'_>> {
    if !slice.starts_with(' ') {
        return None;
    }
    let rest = &slice[1..];

    // 1. Base: 1 or more ideographs
    let mut base_end = 0;
    for (idx, c) in rest.char_indices() {
        if is_ideograph(c) {
            base_end = idx + c.len_utf8();
        } else {
            break;
        }
    }
    if base_end == 0 {
        return None;
    }
    let base = &rest[..base_end];
    let after_base = &rest[base_end..];

    // 2. Middle delimiter space
    if !after_base.starts_with(' ') {
        return None;
    }
    let rest_kana = &after_base[1..];

    // 3. Reading: 1 or more kana characters
    let mut reading_end = 0;
    for (idx, c) in rest_kana.char_indices() {
        if is_kana(c) {
            reading_end = idx + c.len_utf8();
        } else {
            break;
        }
    }
    if reading_end == 0 {
        return None;
    }
    let reading = &rest_kana[..reading_end];
    let after_reading = &rest_kana[reading_end..];

    // 4. Trailing delimiter space
    if !after_reading.starts_with(' ') {
        return None;
    }

    let end_byte = 1 + base_end + 1 + reading_end + 1;
    Some(RubyMatch {
        base,
        reading,
        end_byte,
    })
}

/// Normalizes ASCII-spaced ruby in a single line, returning the normalized line
/// and the number of ruby patterns replaced.
pub fn normalize_ruby_line(line: &str) -> (String, usize) {
    let mut result = String::with_capacity(line.len());
    let mut count = 0;
    let mut pos = 0;
    let bytes = line.as_bytes();

    while pos < line.len() {
        if bytes[pos] == b' ' {
            if let Some(m) = try_match_ruby(&line[pos..]) {
                result.push_str(m.base);
                result.push('《');
                result.push_str(m.reading);
                result.push('》');
                count += 1;

                let trailing_space_pos = pos + m.end_byte - 1;

                // Check if this trailing delimiter serves as the leading delimiter of an adjacent ruby
                if trailing_space_pos < line.len()
                    && try_match_ruby(&line[trailing_space_pos..]).is_some()
                {
                    pos = trailing_space_pos;
                } else {
                    pos = trailing_space_pos + 1;
                }
                continue;
            }
        }

        if let Some(ch) = line[pos..].chars().next() {
            result.push(ch);
            pos += ch.len_utf8();
        } else {
            break;
        }
    }

    (result, count)
}

/// Normalizes ruby across multi-line text, returning the full text and total replacement count.
pub fn normalize_ruby_text(text: &str) -> (String, usize) {
    let mut total_count = 0;
    let mut lines = Vec::new();

    for line in text.split('\n') {
        let (norm, count) = normalize_ruby_line(line);
        total_count += count;
        lines.push(norm);
    }

    (lines.join("\n"), total_count)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_ruby_single_kanji() {
        let (out, count) = normalize_ruby_line(" 呆 あき れた");
        assert_eq!(out, "呆《あき》れた");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_ruby_adjacent_names() {
        let (out, count) = normalize_ruby_line(" 健 けん 太 た ");
        assert_eq!(out, "健《けん》太《た》");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_ruby_spring_horse() {
        let (out, count) = normalize_ruby_line(" 春 はる 馬 ま ぁ");
        assert_eq!(out, "春《はる》馬《ま》ぁ");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_ruby_multiple_kanji_base() {
        let (out, count) = normalize_ruby_line(" 漢字 かな ");
        assert_eq!(out, "漢字《かな》");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_unrelated_text_untouched() {
        let cases = [
            "「おはようございます」",
            "健太は美咲を見た。",
            "Hello world with spaces",
            "   ",
        ];

        for case in cases {
            let (out, count) = normalize_ruby_line(case);
            assert_eq!(out, case);
            assert_eq!(count, 0);
        }
    }

    #[test]
    fn test_multi_line_text_count() {
        let text = " 呆 あき れた\n普通の一行\n 健 けん 太 た は歩いた";
        let (out, count) = normalize_ruby_text(text);
        assert_eq!(
            out,
            "呆《あき》れた\n普通の一行\n健《けん》太《た》は歩いた"
        );
        assert_eq!(count, 3);
    }
}
