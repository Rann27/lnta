//! English (ja-en) profile QA scan checks.
//!
//! Implements pure-core scan checks per PRD Appendix B, start-en.md, and instruction-en.md:
//! - Q04: Redundant attribution (`<Name> said that.` / `<Name> said so.`)
//! - Q05: Metronome openers (`As `, `When `, `With that `) compared to volume average
//! - Q06: Flattened sibling and honorific terms with descriptive narration exception
//! - Q08: Romanized SFX left in (with explicit rule: "fufu" and "ara ara" never flagged)
//! - Q13: Pronoun/gender conflicts against locked glossary entries
//! - Q18: US/UK spelling consistency and commonly confused pairs
//! - Q19b: Inconsistent ellipsis across volume (`...`, `…`, `. . .`)
//! - Q20: Lowercase acronyms (`tv`, `dvd`, `am/pm` after numbers)

use super::common::{QaFinding, SegmentForQa};

/// Character entry for gender conflict checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterGender {
    pub name: String,
    pub is_female: bool,
}

impl CharacterGender {
    pub fn new(name: impl Into<String>, is_female: bool) -> Self {
        Self {
            name: name.into(),
            is_female,
        }
    }
}

/// US/UK spelling pairs per start-en.md (Q18).
pub const US_UK_PAIRS: &[(&str, &str)] = &[
    ("color", "colour"),
    ("realize", "realise"),
    ("traveled", "travelled"),
    ("gray", "grey"),
    ("toward", "towards"),
    ("defense", "defence"),
];

/// Romanized SFX that should have been translated into natural English or prose per Q08.
pub const ROMANIZED_SFX: &[&str] = &[
    "doki doki",
    "doki-doki",
    "gacha",
    "suya suya",
    "suya-suya",
    "munya munya",
    "munya-munya",
    "pekori",
    "fuwa fuwa",
    "fuwa-fuwa",
];

/// Q04: Redundant attribution (e.g. `<Name> said that.` or `said <Name>.`).
pub fn check_redundant_attribution(seg: &SegmentForQa) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    let text = seg.tgt_text.trim();

    if text.ends_with(" said that.")
        || text.ends_with(" said so.")
        || (text.starts_with("said ") && text.ends_with('.'))
        || text == "He said that."
        || text == "She said that."
        || text == "He said so."
        || text == "She said so."
    {
        findings.push(QaFinding::new(
            "Q04",
            &seg.chapter_id,
            &seg.segment_id,
            "warning",
            "Redundant attribution in standalone narration",
            text,
        ));
    }

    findings
}

/// Q05: Metronome openers (paragraphs starting with `As `, `When `, `With that`).
/// Flags chapters whose opener rate is > 1.5 × volume average (with >= 3 openers).
pub fn check_metronome_openers(chapters: &[Vec<SegmentForQa>]) -> Vec<QaFinding> {
    let mut findings = Vec::new();

    let mut total_segments = 0;
    let mut total_openers = 0;

    for chap_segs in chapters {
        for seg in chap_segs {
            total_segments += 1;
            let trimmed = seg.tgt_text.trim();
            if trimmed.starts_with("As ")
                || trimmed.starts_with("When ")
                || trimmed.starts_with("With that ")
                || trimmed.starts_with("With that,")
            {
                total_openers += 1;
            }
        }
    }

    if total_segments == 0 || total_openers == 0 {
        return findings;
    }

    let volume_rate = total_openers as f64 / total_segments as f64;

    for chap_segs in chapters {
        let chap_total = chap_segs.len();
        if chap_total == 0 {
            continue;
        }

        let mut chap_openers = 0;
        let mut opener_segs = Vec::new();

        for seg in chap_segs {
            let trimmed = seg.tgt_text.trim();
            if trimmed.starts_with("As ")
                || trimmed.starts_with("When ")
                || trimmed.starts_with("With that ")
                || trimmed.starts_with("With that,")
            {
                chap_openers += 1;
                opener_segs.push(seg);
            }
        }

        let chap_rate = chap_openers as f64 / chap_total as f64;
        if chap_openers >= 3 && chap_rate > volume_rate * 1.5 {
            for seg in opener_segs {
                findings.push(QaFinding::new(
                    "Q05",
                    &seg.chapter_id,
                    &seg.segment_id,
                    "warning",
                    &format!(
                        "Metronome opener rate ({:.1}%) is >1.5x volume average ({:.1}%)",
                        chap_rate * 100.0,
                        volume_rate * 100.0
                    ),
                    &seg.tgt_text,
                ));
            }
        }
    }

    findings
}

/// Helper to check whether a substring match at byte index `start` in `text` is inside quotes.
fn is_inside_quotes(text: &str, match_start: usize) -> bool {
    let mut inside_double = false;
    let mut inside_single = false;
    let chars: Vec<(usize, char)> = text.char_indices().collect();

    for i in 0..chars.len() {
        let (idx, ch) = chars[i];
        if idx >= match_start {
            break;
        }
        if ch == '"' || ch == '“' || ch == '”' {
            inside_double = !inside_double;
        } else if ch == '\'' || ch == '‘' || ch == '’' {
            // It is an apostrophe (not a quotation mark) if surrounded by alphabetic characters
            let is_apostrophe = i > 0
                && chars[i - 1].1.is_alphabetic()
                && i + 1 < chars.len()
                && chars[i + 1].1.is_alphabetic();
            if !is_apostrophe {
                inside_single = !inside_single;
            }
        }
    }

    inside_double || inside_single
}

/// Helper to check whether a sibling term is preceded by a possessive determiner/pronoun
/// indicating descriptive narration (e.g. "his older sister", "Kenta's big brother").
fn is_preceded_by_possessive_or_relation(before: &str) -> bool {
    let words: Vec<&str> = before.split_whitespace().collect();
    if let Some(&last_word) = words.last() {
        let clean = last_word.trim_matches(|c: char| !c.is_alphabetic() && c != '\'' && c != '’');
        let lower = clean.to_lowercase();
        if matches!(
            lower.as_str(),
            "his"
                | "her"
                | "their"
                | "my"
                | "our"
                | "your"
                | "its"
                | "one's"
                | "an"
                | "a"
                | "the"
                | "younger"
        ) || clean.ends_with("'s")
            || clean.ends_with("’s")
            || clean.ends_with("s'")
            || clean.ends_with("s’")
        {
            return true;
        }
    }
    false
}

/// Q06: Flattened sibling terms ("big brother", "big sis", "older sister", "Mr.", "Ms.").
/// Honors the descriptive narration exception when used as a relation noun (e.g. "his older sister").
pub fn check_flattened_sibling_terms(seg: &SegmentForQa) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    let text = &seg.tgt_text;

    // 1. Honorifics Mr. / Ms. are always flattened forms of -san (no descriptive exception)
    if let Some(re_honorific) = crate::qa::re::cached(r"\b(Mr\.|Ms\.)(?:\s+|$)") {
        for m in re_honorific.find_iter(text) {
            let matched = m.as_str().trim();
            findings.push(QaFinding::new(
                "Q06",
                &seg.chapter_id,
                &seg.segment_id,
                "warning",
                &format!(
                    "Flattened honorific '{}' (should retain Japanese suffix)",
                    matched
                ),
                matched,
            ));
        }
    }

    // 2. Sibling terms with descriptive-narration exception
    let sibling_pattern = r"(?i)\b(big brother|big sister|big sis|big bro|older sister|older brother|elder sister|elder brother)\b";
    if let Some(re_sibling) = crate::qa::re::cached(sibling_pattern) {
        for m in re_sibling.find_iter(text) {
            let start = m.start();
            let in_dialogue = is_inside_quotes(text, start);

            if in_dialogue {
                // In dialogue, it is an address/vocative -> always flag
                findings.push(QaFinding::new(
                    "Q06",
                    &seg.chapter_id,
                    &seg.segment_id,
                    "warning",
                    &format!("Flattened sibling address '{}' in dialogue", m.as_str()),
                    m.as_str(),
                ));
            } else {
                // In narration, check if preceded by possessive/determiner
                let prefix = &text[..start];
                if !is_preceded_by_possessive_or_relation(prefix) {
                    // Standalone vocative/address in narration (e.g. "Big brother walked in.")
                    findings.push(QaFinding::new(
                        "Q06",
                        &seg.chapter_id,
                        &seg.segment_id,
                        "warning",
                        &format!("Flattened sibling term '{}' used as address", m.as_str()),
                        m.as_str(),
                    ));
                }
            }
        }
    }

    findings
}

/// Q08: Romanized SFX left in (doki doki, gacha, suya suya, etc.).
/// Rule: "fufu" and "ara ara" are cultural verbal tics and must NEVER be flagged.
pub fn check_romanized_sfx(seg: &SegmentForQa) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    let text_lower = seg.tgt_text.to_lowercase();

    for &sfx in ROMANIZED_SFX {
        let pattern = format!(r"\b{}\b", regex::escape(sfx));
        if let Some(re) = crate::qa::re::cached(&pattern) {
            if let Some(m) = re.find(&text_lower) {
                findings.push(QaFinding::new(
                    "Q08",
                    &seg.chapter_id,
                    &seg.segment_id,
                    "warning",
                    &format!(
                        "Romanized SFX '{}' should be adapted to natural English",
                        sfx
                    ),
                    m.as_str(),
                ));
            }
        }
    }

    findings
}

/// Q13: Pronoun/gender conflicts against locked glossary entries.
pub fn check_gender_conflicts(
    segments: &[SegmentForQa],
    characters: &[CharacterGender],
) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    if characters.is_empty() {
        return findings;
    }

    let male_pronoun_re = crate::qa::re::cached(r"(?i)\b(he|him|his|himself)\b");
    let female_pronoun_re = crate::qa::re::cached(r"(?i)\b(she|her|hers|herself)\b");

    for seg in segments {
        let text = &seg.tgt_text;

        for char_info in characters {
            let name_pattern = format!(r"\b{}\b", regex::escape(&char_info.name));
            let has_name = crate::qa::re::cached(&name_pattern)
                .map(|re| re.is_match(text))
                .unwrap_or(false);

            if !has_name {
                continue;
            }

            // Check if there are opposite gender pronouns in the segment
            let has_opposite_pronoun = if char_info.is_female {
                male_pronoun_re.as_ref().is_some_and(|re| re.is_match(text))
            } else {
                female_pronoun_re
                    .as_ref()
                    .is_some_and(|re| re.is_match(text))
            };

            if has_opposite_pronoun {
                // If another character of the opposite gender also appears in this segment,
                // the pronoun likely refers to that character instead.
                let another_opposite_character = characters.iter().any(|other| {
                    other.name != char_info.name
                        && other.is_female != char_info.is_female
                        && crate::qa::re::cached(&format!(r"\b{}\b", regex::escape(&other.name)))
                            .map(|re| re.is_match(text))
                            .unwrap_or(false)
                });

                if !another_opposite_character {
                    findings.push(QaFinding::new(
                        "Q13",
                        &seg.chapter_id,
                        &seg.segment_id,
                        "warning",
                        &format!(
                            "Pronoun/gender conflict: opposite pronoun near locked character '{}'",
                            char_info.name
                        ),
                        &char_info.name,
                    ));
                }
            }
        }
    }

    findings
}

/// Q18: US/UK spelling consistency and commonly confused pairs.
pub fn check_spelling_consistency(segments: &[SegmentForQa]) -> Vec<QaFinding> {
    let mut findings = Vec::new();

    // 1. Check volume-wide US vs UK consistency
    for &(us, uk) in US_UK_PAIRS {
        let us_re = crate::qa::re::cached(&format!(r"(?i)\b{}\b", us));
        let uk_re = crate::qa::re::cached(&format!(r"(?i)\b{}\b", uk));

        let mut us_matches = Vec::new();
        let mut uk_matches = Vec::new();

        for seg in segments {
            if let Some(ref re) = us_re {
                if re.is_match(&seg.tgt_text) {
                    us_matches.push(seg);
                }
            }
            if let Some(ref re) = uk_re {
                if re.is_match(&seg.tgt_text) {
                    uk_matches.push(seg);
                }
            }
        }

        // If both forms appear in the volume, flag the minority form (or both if equal)
        if !us_matches.is_empty() && !uk_matches.is_empty() {
            let flag_us = us_matches.len() <= uk_matches.len();
            let flag_uk = uk_matches.len() <= us_matches.len();

            if flag_us {
                for seg in us_matches {
                    findings.push(QaFinding::new(
                        "Q18",
                        &seg.chapter_id,
                        &seg.segment_id,
                        "warning",
                        &format!(
                            "Mixed US/UK spelling: found minority '{}' alongside UK '{}'",
                            us, uk
                        ),
                        us,
                    ));
                }
            }
            if flag_uk {
                for seg in uk_matches {
                    findings.push(QaFinding::new(
                        "Q18",
                        &seg.chapter_id,
                        &seg.segment_id,
                        "warning",
                        &format!(
                            "Mixed US/UK spelling: found minority '{}' alongside US '{}'",
                            uk, us
                        ),
                        uk,
                    ));
                }
            }
        }
    }

    // 2. Commonly confused pairs per segment
    let confused_patterns: &[(&str, &str)] = &[
        // breath vs breathe
        (
            r"(?i)\b(take|taking|took|held|hold|holding|catch|catching|caught)\s+(a\s+)?(deep\s+)?(his|her|my|their|your|the)?\s*breathe\b",
            "Confused word 'breathe' (should be noun 'breath')",
        ),
        (
            r"(?i)\b(couldn't|could\s+not|hard\s+to|struggled\s+to|struggling\s+to|forgot\s+to|unable\s+to)\s+breath\b",
            "Confused word 'breath' (should be verb 'breathe')",
        ),
        // its vs it's
        (
            r"(?i)\bit's\s+(tail|head|eyes|face|name|color|wings|body|fur|paws)\b",
            "Confused word \"it's\" (should be possessive 'its')",
        ),
        (
            r"(?i)\bits\s+(a|an|the|not|too|time|been|gone)\b",
            "Confused word 'its' (should be contraction \"it's\")",
        ),
        // their vs there
        (
            r"(?i)\bover\s+their\b",
            "Confused word 'their' (should be 'there')",
        ),
    ];

    for seg in segments {
        for &(pattern, message) in confused_patterns {
            if let Some(re) = crate::qa::re::cached(pattern) {
                if let Some(m) = re.find(&seg.tgt_text) {
                    findings.push(QaFinding::new(
                        "Q18",
                        &seg.chapter_id,
                        &seg.segment_id,
                        "warning",
                        message,
                        m.as_str(),
                    ));
                }
            }
        }
    }

    findings
}

/// Q19b: Inconsistent ellipsis across volume (`...`, `…`, `. . .`).
pub fn check_ellipsis_consistency(segments: &[SegmentForQa]) -> Vec<QaFinding> {
    let mut findings = Vec::new();

    let mut dot3_segs = Vec::new();
    let mut unicode_segs = Vec::new();
    let mut spaced_segs = Vec::new();

    for seg in segments {
        let text = &seg.tgt_text;
        if text.contains(". . .") {
            spaced_segs.push(seg);
        } else if text.contains('…') {
            unicode_segs.push(seg);
        } else if text.contains("...") {
            dot3_segs.push(seg);
        }
    }

    let mut counts = [
        (dot3_segs.len(), "...", &dot3_segs),
        (unicode_segs.len(), "…", &unicode_segs),
        (spaced_segs.len(), ". . .", &spaced_segs),
    ];
    counts.sort_by_key(|a| std::cmp::Reverse(a.0));

    let dominant_count = counts[0].0;
    if dominant_count > 0 {
        // Flag non-empty minority styles
        for &(count, style_name, segs) in &counts[1..] {
            if count > 0 {
                for seg in segs {
                    findings.push(QaFinding::new(
                        "Q19b",
                        &seg.chapter_id,
                        &seg.segment_id,
                        "warning",
                        &format!(
                            "Inconsistent ellipsis style '{}' (volume predominantly uses '{}')",
                            style_name, counts[0].1
                        ),
                        style_name,
                    ));
                }
            }
        }
    }

    findings
}

/// Q20: Lowercase acronyms mid-sentence (`tv`, `dvd`, `am/pm` after numbers).
pub fn check_lowercase_acronyms(seg: &SegmentForQa) -> Vec<QaFinding> {
    let mut findings = Vec::new();

    // tv, dvd
    if let Some(re_media) = crate::qa::re::cached(r"\b(tv|dvd)\b") {
        for m in re_media.find_iter(&seg.tgt_text) {
            findings.push(QaFinding::new(
                "Q20",
                &seg.chapter_id,
                &seg.segment_id,
                "warning",
                "Lowercase acronym should be capitalized (TV/DVD)",
                m.as_str(),
            ));
        }
    }

    // am/pm after numbers e.g. "8 am", "10 pm"
    if let Some(re_time) = crate::qa::re::cached(r"\b\d+\s*(am|pm)\b") {
        for m in re_time.find_iter(&seg.tgt_text) {
            findings.push(QaFinding::new(
                "Q20",
                &seg.chapter_id,
                &seg.segment_id,
                "warning",
                "Lowercase am/pm after number should be AM/PM",
                m.as_str(),
            ));
        }
    }

    findings
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_q04_redundant_attribution() {
        let pos = SegmentForQa {
            segment_id: "s1".to_string(),
            chapter_id: "c1".to_string(),
            idx: 1,
            src_text: "Kenji said that.".to_string(),
            tgt_text: "Kenji said that.".to_string(),
        };
        let findings = check_redundant_attribution(&pos);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].check_code, "Q04");

        let neg = SegmentForQa {
            segment_id: "s2".to_string(),
            chapter_id: "c1".to_string(),
            idx: 2,
            src_text: "source".to_string(),
            tgt_text: "\"I know what to do,\" Kenji replied with a smirk.".to_string(),
        };
        assert!(check_redundant_attribution(&neg).is_empty());
    }

    #[test]
    fn test_q05_metronome_openers() {
        let chap1 = vec![
            SegmentForQa {
                segment_id: "s1".to_string(),
                chapter_id: "c1".to_string(),
                idx: 1,
                src_text: "s".to_string(),
                tgt_text: "As the sun rose over the horizon.".to_string(),
            },
            SegmentForQa {
                segment_id: "s2".to_string(),
                chapter_id: "c1".to_string(),
                idx: 2,
                src_text: "s".to_string(),
                tgt_text: "When he stepped outside the gate.".to_string(),
            },
            SegmentForQa {
                segment_id: "s3".to_string(),
                chapter_id: "c1".to_string(),
                idx: 3,
                src_text: "s".to_string(),
                tgt_text: "With that, they departed into the woods.".to_string(),
            },
            SegmentForQa {
                segment_id: "s4".to_string(),
                chapter_id: "c1".to_string(),
                idx: 4,
                src_text: "s".to_string(),
                tgt_text: "The path was clear.".to_string(),
            },
        ];

        let chap2 = vec![
            SegmentForQa {
                segment_id: "s5".to_string(),
                chapter_id: "c2".to_string(),
                idx: 5,
                src_text: "s".to_string(),
                tgt_text: "Birds sang in the trees.".to_string(),
            },
            SegmentForQa {
                segment_id: "s6".to_string(),
                chapter_id: "c2".to_string(),
                idx: 6,
                src_text: "s".to_string(),
                tgt_text: "The forest was peaceful.".to_string(),
            },
            SegmentForQa {
                segment_id: "s7".to_string(),
                chapter_id: "c2".to_string(),
                idx: 7,
                src_text: "s".to_string(),
                tgt_text: "They walked quietly.".to_string(),
            },
            SegmentForQa {
                segment_id: "s8".to_string(),
                chapter_id: "c2".to_string(),
                idx: 8,
                src_text: "s".to_string(),
                tgt_text: "Nobody spoke.".to_string(),
            },
        ];

        let chapters = vec![chap1, chap2];
        let findings = check_metronome_openers(&chapters);
        assert!(!findings.is_empty());
        assert_eq!(findings[0].check_code, "Q05");
        assert_eq!(findings[0].chapter_id, "c1");
    }

    #[test]
    fn test_q06_flattened_sibling_terms_and_descriptive_exception() {
        // Dialogue address -> flagged
        let dialogue_seg = SegmentForQa {
            segment_id: "s1".to_string(),
            chapter_id: "c1".to_string(),
            idx: 1,
            src_text: "s".to_string(),
            tgt_text: "\"Good morning, big brother!\" she cheered.".to_string(),
        };
        let findings1 = check_flattened_sibling_terms(&dialogue_seg);
        assert_eq!(findings1.len(), 1);
        assert_eq!(findings1[0].check_code, "Q06");

        // Narration address without possessive -> flagged
        let narration_address = SegmentForQa {
            segment_id: "s2".to_string(),
            chapter_id: "c1".to_string(),
            idx: 2,
            src_text: "s".to_string(),
            tgt_text: "Big brother looked back with a stern expression.".to_string(),
        };
        let findings2 = check_flattened_sibling_terms(&narration_address);
        assert_eq!(findings2.len(), 1);
        assert_eq!(findings2[0].check_code, "Q06");

        // Descriptive narration with possessive -> NOT flagged (exception!)
        let descriptive_seg = SegmentForQa {
            segment_id: "s3".to_string(),
            chapter_id: "c1".to_string(),
            idx: 3,
            src_text: "s".to_string(),
            tgt_text: "His older sister was waiting for him at the station.".to_string(),
        };
        let findings3 = check_flattened_sibling_terms(&descriptive_seg);
        assert!(
            findings3.is_empty(),
            "Descriptive narration must be exempted"
        );

        let genitive_seg = SegmentForQa {
            segment_id: "s4".to_string(),
            chapter_id: "c1".to_string(),
            idx: 4,
            src_text: "s".to_string(),
            tgt_text: "Kenta's older sister waved from across the street.".to_string(),
        };
        let findings4 = check_flattened_sibling_terms(&genitive_seg);
        assert!(findings4.is_empty(), "Genitive possessive must be exempted");

        // Mr. and Ms. honorifics -> always flagged
        let mr_seg = SegmentForQa {
            segment_id: "s5".to_string(),
            chapter_id: "c1".to_string(),
            idx: 5,
            src_text: "s".to_string(),
            tgt_text: "Mr. Tanaka bowed politely.".to_string(),
        };
        let findings5 = check_flattened_sibling_terms(&mr_seg);
        assert_eq!(findings5.len(), 1);
        assert_eq!(findings5[0].check_code, "Q06");
    }

    #[test]
    fn test_q08_romanized_sfx_and_fufu_ara_ara_never_flagged() {
        // Romanized SFX -> flagged
        let sfx_seg = SegmentForQa {
            segment_id: "s1".to_string(),
            chapter_id: "c1".to_string(),
            idx: 1,
            src_text: "s".to_string(),
            tgt_text: "Her heart went doki doki as she took his hand.".to_string(),
        };
        let findings1 = check_romanized_sfx(&sfx_seg);
        assert_eq!(findings1.len(), 1);
        assert_eq!(findings1[0].check_code, "Q08");

        let gacha_seg = SegmentForQa {
            segment_id: "s2".to_string(),
            chapter_id: "c1".to_string(),
            idx: 2,
            src_text: "s".to_string(),
            tgt_text: "The door opened with a loud gacha.".to_string(),
        };
        let findings2 = check_romanized_sfx(&gacha_seg);
        assert_eq!(findings2.len(), 1);

        // "fufu" and "ara ara" must NEVER be flagged
        let fufu_seg = SegmentForQa {
            segment_id: "s3".to_string(),
            chapter_id: "c1".to_string(),
            idx: 3,
            src_text: "s".to_string(),
            tgt_text: "\"Fufu, what a curious boy you are,\" she teased.".to_string(),
        };
        assert!(
            check_romanized_sfx(&fufu_seg).is_empty(),
            "'fufu' must never be flagged"
        );

        let ara_ara_seg = SegmentForQa {
            segment_id: "s4".to_string(),
            chapter_id: "c1".to_string(),
            idx: 4,
            src_text: "s".to_string(),
            tgt_text: "\"Ara ara, isn't that nice?\" she smiled warmly.".to_string(),
        };
        assert!(
            check_romanized_sfx(&ara_ara_seg).is_empty(),
            "'ara ara' must never be flagged"
        );
    }

    #[test]
    fn test_q13_gender_conflict() {
        let chars = vec![
            CharacterGender::new("Misaki", true), // Female
            CharacterGender::new("Kenta", false), // Male
        ];

        // Misaki with male pronoun -> conflict
        let conflict_seg = SegmentForQa {
            segment_id: "s1".to_string(),
            chapter_id: "c1".to_string(),
            idx: 1,
            src_text: "s".to_string(),
            tgt_text: "Misaki shook his head in disbelief.".to_string(),
        };
        let findings1 = check_gender_conflicts(&[conflict_seg], &chars);
        assert_eq!(findings1.len(), 1);
        assert_eq!(findings1[0].check_code, "Q13");

        // Misaki with female pronoun -> ok
        let ok_seg = SegmentForQa {
            segment_id: "s2".to_string(),
            chapter_id: "c1".to_string(),
            idx: 2,
            src_text: "s".to_string(),
            tgt_text: "Misaki shook her head in disbelief.".to_string(),
        };
        assert!(check_gender_conflicts(&[ok_seg], &chars).is_empty());

        // Both characters present in same segment -> pronoun can refer to the other
        let multi_seg = SegmentForQa {
            segment_id: "s3".to_string(),
            chapter_id: "c1".to_string(),
            idx: 3,
            src_text: "s".to_string(),
            tgt_text: "Misaki glanced at Kenta. He smiled back gently.".to_string(),
        };
        assert!(check_gender_conflicts(&[multi_seg], &chars).is_empty());
    }

    #[test]
    fn test_q18_spelling_consistency_and_confused_pairs() {
        // Mixed US and UK spelling in volume
        let segs = vec![
            SegmentForQa {
                segment_id: "s1".to_string(),
                chapter_id: "c1".to_string(),
                idx: 1,
                src_text: "s".to_string(),
                tgt_text: "The vivid color filled the room.".to_string(),
            },
            SegmentForQa {
                segment_id: "s2".to_string(),
                chapter_id: "c1".to_string(),
                idx: 2,
                src_text: "s".to_string(),
                tgt_text: "Her favourite colour was emerald green.".to_string(),
            },
        ];
        let findings = check_spelling_consistency(&segs);
        assert!(findings
            .iter()
            .any(|f| f.check_code == "Q18" && f.matched_text == "color"));

        // Confused breathe vs breath
        let confused_seg = SegmentForQa {
            segment_id: "s3".to_string(),
            chapter_id: "c1".to_string(),
            idx: 3,
            src_text: "s".to_string(),
            tgt_text: "She took a deep breathe before entering.".to_string(),
        };
        let findings_confused = check_spelling_consistency(&[confused_seg]);
        assert_eq!(findings_confused.len(), 1);
        assert_eq!(findings_confused[0].check_code, "Q18");
    }

    #[test]
    fn test_q19b_ellipsis_consistency() {
        let segs = vec![
            SegmentForQa {
                segment_id: "s1".to_string(),
                chapter_id: "c1".to_string(),
                idx: 1,
                src_text: "s".to_string(),
                tgt_text: "Wait... what did you say?".to_string(),
            },
            SegmentForQa {
                segment_id: "s2".to_string(),
                chapter_id: "c1".to_string(),
                idx: 2,
                src_text: "s".to_string(),
                tgt_text: "I told you... not today.".to_string(),
            },
            SegmentForQa {
                segment_id: "s3".to_string(),
                chapter_id: "c1".to_string(),
                idx: 3,
                src_text: "s".to_string(),
                tgt_text: "Hold on… is that true?".to_string(), // minority unicode ellipsis
            },
        ];
        let findings = check_ellipsis_consistency(&segs);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].check_code, "Q19b");
        assert_eq!(findings[0].matched_text, "…");
    }

    #[test]
    fn test_q20_lowercase_acronyms() {
        let tv_seg = SegmentForQa {
            segment_id: "s1".to_string(),
            chapter_id: "c1".to_string(),
            idx: 1,
            src_text: "s".to_string(),
            tgt_text: "He turned on the tv to watch the news.".to_string(),
        };
        let findings1 = check_lowercase_acronyms(&tv_seg);
        assert_eq!(findings1.len(), 1);
        assert_eq!(findings1[0].check_code, "Q20");

        let time_seg = SegmentForQa {
            segment_id: "s2".to_string(),
            chapter_id: "c1".to_string(),
            idx: 2,
            src_text: "s".to_string(),
            tgt_text: "The alarm rang at 8 am every morning.".to_string(),
        };
        let findings2 = check_lowercase_acronyms(&time_seg);
        assert_eq!(findings2.len(), 1);
        assert_eq!(findings2[0].check_code, "Q20");
    }
}
