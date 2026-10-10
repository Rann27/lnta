//! Phrase and word frequency analysis per Q21 and FR-QA-01.
//!
//! Produces the top 25 bigrams, trigrams, and content words across a volume
//! to detect narrowed diction vs source tics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::common::SegmentForQa;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrequencyItem {
    pub text: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrequencyReport {
    pub top_words: Vec<FrequencyItem>,
    pub top_bigrams: Vec<FrequencyItem>,
    pub top_trigrams: Vec<FrequencyItem>,
}

/// Common stop words to exclude from single content word frequency
const COMMON_STOP_WORDS: &[&str] = &[
    "yang", "dari", "pada", "untuk", "dengan", "adalah", "akan", "bisa", "juga", "atau", "karena",
    "sudah", "telah", "oleh", "dalam", "bagi", "sampai", "tentang", "seperti", "hanya", "saja",
    "lebih", "sangat", "bahwa", "maka", "tetapi", "namun", "ketika", "saat", "tidak", "bukan",
    "the", "and", "that", "this", "with", "from", "have", "were",
];

/// Analyzes phrase frequency across volume segments, outputting top 25 content words,
/// bigrams, and trigrams.
pub fn analyze_phrase_frequency(segments: &[SegmentForQa]) -> FrequencyReport {
    let mut word_counts: HashMap<String, usize> = HashMap::new();
    let mut bigram_counts: HashMap<(String, String), usize> = HashMap::new();
    let mut trigram_counts: HashMap<(String, String, String), usize> = HashMap::new();

    for seg in segments {
        let tokens: Vec<String> = seg
            .tgt_text
            .split(|c: char| !c.is_alphabetic())
            .filter(|w| !w.is_empty())
            .map(|w| w.to_lowercase())
            .collect();

        // Single content words (length >= 4 and not stop words)
        for w in &tokens {
            if w.chars().count() >= 4 && !COMMON_STOP_WORDS.contains(&w.as_str()) {
                *word_counts.entry(w.clone()).or_insert(0) += 1;
            }
        }

        // Bigrams
        for window in tokens.windows(2) {
            let pair = (window[0].clone(), window[1].clone());
            *bigram_counts.entry(pair).or_insert(0) += 1;
        }

        // Trigrams
        for window in tokens.windows(3) {
            let triple = (window[0].clone(), window[1].clone(), window[2].clone());
            *trigram_counts.entry(triple).or_insert(0) += 1;
        }
    }

    // Helper to sort and pick top 25
    fn top_n<K: Ord, F: Fn(&K) -> String>(
        map: HashMap<K, usize>,
        limit: usize,
        formatter: F,
    ) -> Vec<FrequencyItem> {
        let mut list: Vec<(K, usize)> = map.into_iter().collect();
        // Sort descending by count, then ascending by key for determinism
        list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        list.into_iter()
            .take(limit)
            .map(|(k, count)| FrequencyItem {
                text: formatter(&k),
                count,
            })
            .collect()
    }

    let top_words = top_n(word_counts, 25, |w| w.clone());
    let top_bigrams = top_n(bigram_counts, 25, |(w1, w2)| format!("{} {}", w1, w2));
    let top_trigrams = top_n(trigram_counts, 25, |(w1, w2, w3)| {
        format!("{} {} {}", w1, w2, w3)
    });

    FrequencyReport {
        top_words,
        top_bigrams,
        top_trigrams,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency_analysis_top_items() {
        let mut segments = Vec::new();
        for i in 1..=30 {
            segments.push(SegmentForQa {
                segment_id: format!("s{}", i),
                chapter_id: "c1".to_string(),
                idx: i,
                src_text: "src".to_string(),
                tgt_text: "Ksatria pemberani itu mengayunkan pedang pusaka dengan cepat."
                    .to_string(),
            });
        }

        let report = analyze_phrase_frequency(&segments);
        assert!(!report.top_words.is_empty());
        assert!(report
            .top_words
            .iter()
            .any(|w| w.text == "ksatria" && w.count == 30));

        assert!(!report.top_bigrams.is_empty());
        assert!(report
            .top_bigrams
            .iter()
            .any(|b| b.text == "ksatria pemberani" && b.count == 30));

        assert!(!report.top_trigrams.is_empty());
        assert!(report
            .top_trigrams
            .iter()
            .any(|t| t.text == "ksatria pemberani itu" && t.count == 30));

        assert!(report.top_words.len() <= 25);
        assert!(report.top_bigrams.len() <= 25);
        assert!(report.top_trigrams.len() <= 25);
    }
}
