//! Indonesian (ja-id) profile QA scan checks.
//!
//! Implements pure-core scan checks per PRD Appendix B and start.md.

use super::common::{QaFinding, SegmentForQa};
use regex::Regex;

/// Fixed list of non-standard Indonesian spellings per start.md and PRD Appendix B (Q17).
/// Format: (correct_standard, wrong_non_standard)
pub const NON_STANDARD_SPELLINGS: &[(&str, &str)] = &[
    ("napas", "nafas"),
    ("risiko", "resiko"),
    ("praktik", "praktek"),
    ("analisis", "analisa"),
    ("zaman", "jaman"),
    ("silakan", "silahkan"),
    ("hakikat", "hakekat"),
    ("aktivitas", "aktifitas"),
    ("nasihat", "nasehat"),
    ("antre", "antri"),
    ("izin", "ijin"),
    ("karier", "karir"),
    ("apotek", "apotik"),
    ("sekadar", "sekedar"),
    ("imbau", "himbau"),
    ("provinsi", "propinsi"),
    ("ekstrem", "ekstrim"),
    ("jadwal", "jadual"),
    ("kualitas", "kwalitas"),
    ("frekuensi", "frekwensi"),
    ("atlet", "atlit"),
    ("objek", "obyek"),
    ("teknik", "tehnik"),
    ("cendekiawan", "cendikiawan"),
];

/// Indonesian translated fantasy/genre terms that should have remained in English per Q07.
pub const GENRE_TERMINOLOGY: &[(&str, &str)] = &[
    ("penjara bawah tanah", "dungeon"),
    ("keterampilan", "skill"),
    ("tingkat", "level"),
    ("ramuan", "potion"),
    ("jendela sistem", "system window"),
    ("serikat", "guild"),
    ("rombongan", "party"),
];

/// Q04: Redundant attribution (e.g. `<Nama> berkata begitu.` or `kata <Nama>.`).
pub fn check_redundant_attribution(seg: &SegmentForQa) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    let text = seg.tgt_text.trim();

    if text.ends_with("berkata begitu.") || text.starts_with("kata ") && text.ends_with('.') {
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

/// Q06: Flattened sibling and honorific terms (kakak, Kak, adik, abang, Pak, Bu).
pub fn check_flattened_sibling_terms(seg: &SegmentForQa) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    // Words bounded by whitespace or punctuation
    let words: Vec<&str> = seg
        .tgt_text
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .collect();

    for w in words {
        if matches!(w, "kakak" | "Kak" | "adik" | "abang" | "Pak" | "Bu") {
            findings.push(QaFinding::new(
                "Q06",
                &seg.chapter_id,
                &seg.segment_id,
                "warning",
                &format!("Flattened sibling or honorific term '{}'", w),
                w,
            ));
            break;
        }
    }

    findings
}

/// Q07: Translated genre terms that should stay English (penjara bawah tanah, keterampilan, etc.).
pub fn check_genre_terminology(seg: &SegmentForQa) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    let text_lower = seg.tgt_text.to_lowercase();

    for &(id_term, en_replacement) in GENRE_TERMINOLOGY {
        if text_lower.contains(id_term) {
            findings.push(QaFinding::new(
                "Q07",
                &seg.chapter_id,
                &seg.segment_id,
                "error",
                &format!(
                    "Genre terminology translated into Indonesian: '{}' (should be '{}')",
                    id_term, en_replacement
                ),
                id_term,
            ));
        }
    }

    findings
}

/// Q12: Pronoun consistency within dialogue (mix of aku-kau, aku-kamu, saya-Anda).
pub fn check_pronoun_consistency(segments: &[SegmentForQa]) -> Vec<QaFinding> {
    let mut findings = Vec::new();

    let mut has_aku_kau = false;
    let mut has_aku_kamu = false;
    let mut has_saya_anda = false;

    for seg in segments {
        let text_lower = seg.tgt_text.to_lowercase();
        if text_lower.contains("aku") && text_lower.contains("kau") {
            has_aku_kau = true;
        }
        if text_lower.contains("aku") && text_lower.contains("kamu") {
            has_aku_kamu = true;
        }
        if text_lower.contains("saya") && text_lower.contains("anda") {
            has_saya_anda = true;
        }
    }

    let styles_count = (has_aku_kau as u8) + (has_aku_kamu as u8) + (has_saya_anda as u8);
    if styles_count > 1 {
        // Flag segments containing the minority/divergent pronoun pairs
        for seg in segments {
            let text_lower = seg.tgt_text.to_lowercase();
            if (has_aku_kamu && text_lower.contains("kamu"))
                || (has_saya_anda && text_lower.contains("anda"))
            {
                findings.push(QaFinding::new(
                    "Q12",
                    &seg.chapter_id,
                    &seg.segment_id,
                    "warning",
                    "Pronoun drift (mixture of aku-kau vs aku-kamu vs saya-Anda)",
                    &seg.tgt_text,
                ));
            }
        }
    }

    findings
}

/// Q17: Non-standard Indonesian spellings.
pub fn check_non_standard_spelling(seg: &SegmentForQa) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    let text_lower = seg.tgt_text.to_lowercase();

    for &(standard, non_standard) in NON_STANDARD_SPELLINGS {
        let pattern = format!(r"\b{}\b", non_standard);
        if let Ok(re) = Regex::new(&pattern) {
            if re.is_match(&text_lower) {
                findings.push(QaFinding::new(
                    "Q17",
                    &seg.chapter_id,
                    &seg.segment_id,
                    "warning",
                    &format!(
                        "Non-standard spelling '{}' (standard form: '{}')",
                        non_standard, standard
                    ),
                    non_standard,
                ));
            }
        }
    }

    findings
}

/// Q19a: Misplaced hyphen pattern `se-<kata Indonesia>` (e.g. `se-mustahil`).
pub fn check_misplaced_hyphen(seg: &SegmentForQa) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    if let Ok(re) = Regex::new(r"\bse-[a-z]+") {
        for m in re.find_iter(&seg.tgt_text) {
            findings.push(QaFinding::new(
                "Q19a",
                &seg.chapter_id,
                &seg.segment_id,
                "warning",
                "Misplaced hyphen after prefix se- with lowercase word",
                m.as_str(),
            ));
        }
    }
    findings
}

/// Q20: Lowercase acronyms mid-sentence (sks, tv, cd, dvd, sim, ktp, kk, npwp).
pub fn check_lowercase_acronyms(seg: &SegmentForQa) -> Vec<QaFinding> {
    let mut findings = Vec::new();
    if let Ok(re) = Regex::new(r"\b(sks|tv|cd|dvd|sim|ktp|kk|npwp)\b") {
        for m in re.find_iter(&seg.tgt_text) {
            findings.push(QaFinding::new(
                "Q20",
                &seg.chapter_id,
                &seg.segment_id,
                "warning",
                "Lowercase acronym should be capitalized",
                m.as_str(),
            ));
        }
    }
    findings
}

/// Q05: Metronome openers (paragraphs starting with `Saat ` or `Ketika `).
/// Flags chapters whose opener rate is > 1.5 × the volume average (with >= 3 openers).
pub fn check_metronome_openers(chapters: &[Vec<SegmentForQa>]) -> Vec<QaFinding> {
    let mut findings = Vec::new();

    let mut total_segments = 0;
    let mut total_openers = 0;

    for chap_segs in chapters {
        for seg in chap_segs {
            total_segments += 1;
            let trimmed = seg.tgt_text.trim();
            if trimmed.starts_with("Saat ") || trimmed.starts_with("Ketika ") {
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

        let chap_openers: Vec<&SegmentForQa> = chap_segs
            .iter()
            .filter(|seg| {
                let trimmed = seg.tgt_text.trim();
                trimmed.starts_with("Saat ") || trimmed.starts_with("Ketika ")
            })
            .collect();

        let chap_rate = chap_openers.len() as f64 / chap_total as f64;

        // Rate > 1.5x volume average and at least 3 openers
        if chap_openers.len() >= 3 && chap_rate > (1.5 * volume_rate) {
            for seg in chap_openers {
                findings.push(QaFinding::new(
                    "Q05",
                    &seg.chapter_id,
                    &seg.segment_id,
                    "warning",
                    &format!(
                        "Metronome opener ('Saat'/'Ketika') above volume average ({:.1}% vs {:.1}%)",
                        chap_rate * 100.0,
                        volume_rate * 100.0
                    ),
                    &seg.tgt_text.chars().take(20).collect::<String>(),
                ));
            }
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_q07_genre_terminology() {
        let seg = SegmentForQa {
            segment_id: "s1".to_string(),
            chapter_id: "c1".to_string(),
            idx: 1,
            src_text: "dungeon".to_string(),
            tgt_text: "Mereka memasuki penjara bawah tanah dan meminum ramuan.".to_string(),
        };

        let findings = check_genre_terminology(&seg);
        assert_eq!(findings.len(), 2);
        assert!(findings
            .iter()
            .any(|f| f.matched_text == "penjara bawah tanah"));
        assert!(findings.iter().any(|f| f.matched_text == "ramuan"));
    }

    #[test]
    fn test_q17_non_standard_spelling() {
        let seg = SegmentForQa {
            segment_id: "s1".to_string(),
            chapter_id: "c1".to_string(),
            idx: 1,
            src_text: "nafas".to_string(),
            tgt_text: "Ia menarik nafas dalam-dalam karena resiko yang besar.".to_string(),
        };

        let findings = check_non_standard_spelling(&seg);
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.matched_text == "nafas"));
        assert!(findings.iter().any(|f| f.matched_text == "resiko"));
    }

    #[test]
    fn test_q19a_misplaced_hyphen() {
        let seg = SegmentForQa {
            segment_id: "s1".to_string(),
            chapter_id: "c1".to_string(),
            idx: 1,
            src_text: "se-mustahil".to_string(),
            tgt_text: "Hal itu se-mustahil menembus dinding.".to_string(),
        };

        let findings = check_misplaced_hyphen(&seg);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].matched_text, "se-mustahil");
    }

    #[test]
    fn test_q20_lowercase_acronym() {
        let seg = SegmentForQa {
            segment_id: "s1".to_string(),
            chapter_id: "c1".to_string(),
            idx: 1,
            src_text: "tv".to_string(),
            tgt_text: "Dia sedang menonton tv bersama adiknya.".to_string(),
        };

        let findings = check_lowercase_acronyms(&seg);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].matched_text, "tv");
    }

    #[test]
    fn test_q05_metronome_openers_compared_to_average() {
        // Chapter 1 has 1 opener out of 10
        let mut chap1 = Vec::new();
        chap1.push(SegmentForQa {
            segment_id: "c1_s1".to_string(),
            chapter_id: "c1".to_string(),
            idx: 1,
            src_text: "src".to_string(),
            tgt_text: "Saat matahari terbit, mereka bersiap.".to_string(),
        });
        for i in 2..=10 {
            chap1.push(SegmentForQa {
                segment_id: format!("c1_s{}", i),
                chapter_id: "c1".to_string(),
                idx: i,
                src_text: "src".to_string(),
                tgt_text: "Langit sangat cerah hari ini.".to_string(),
            });
        }

        // Chapter 2 has 4 openers out of 6 (rate ~ 66%, volume average ~ 25%)
        let mut chap2 = Vec::new();
        for i in 1..=4 {
            chap2.push(SegmentForQa {
                segment_id: format!("c2_s{}", i),
                chapter_id: "c2".to_string(),
                idx: i,
                src_text: "src".to_string(),
                tgt_text: "Ketika pintu terbuka, ia melangkah maju.".to_string(),
            });
        }
        for i in 5..=6 {
            chap2.push(SegmentForQa {
                segment_id: format!("c2_s{}", i),
                chapter_id: "c2".to_string(),
                idx: i,
                src_text: "src".to_string(),
                tgt_text: "Pedang itu berkilau tajam.".to_string(),
            });
        }

        let chapters = vec![chap1, chap2];
        let findings = check_metronome_openers(&chapters);

        // Only chapter 2 should be flagged
        assert!(!findings.is_empty());
        assert!(findings.iter().all(|f| f.chapter_id == "c2"));
        assert_eq!(findings.len(), 4);
    }
}
