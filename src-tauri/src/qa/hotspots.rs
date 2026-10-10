//! Step C Hotspot selection per PRD Appendix C and FR-QA-07..09.
//!
//! Selects targeted areas for QA Reviewer inspection:
//! - boundary: last N segments of chapter k + first N segments of chapter k+1 (default N=4)
//! - long_run: Q10 repeated letter runs + neighbor segments
//! - names: Tier 1 glossary character names (source + target)
//! - single_character: scenes between scene markers with exactly one Tier 1 character
//! - homophone: volume-wide occurrences of suspected homophone tokens
//! - wordplay: Q24 dead wordplay findings + neighbors
//! - frequency: Q21 top content words / phrases across volume
//! - seam: split chunk seams from chunk_plan (3 segments on each side)
//! - late_volume: sampled segments from the final third of the volume

use std::collections::HashMap;

use crate::qa::common::{QaFinding, SegmentForQa};

/// An aligned pair of source and target text in a hotspot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotspotPair {
    pub chapter_ordinal: i32,
    pub seg_idx: i32,
    pub seg_from: i32,
    pub seg_to: i32,
    pub chapter_id: String,
    pub source: String,
    pub target: String,
}

impl HotspotPair {
    pub fn ref_str(&self) -> String {
        if self.seg_from == self.seg_to {
            format!("{}:{}", self.chapter_ordinal, self.seg_idx)
        } else {
            format!("{}:{}-{}", self.chapter_ordinal, self.seg_from, self.seg_to)
        }
    }
}

/// A selected hotspot ready to be formatted into a `<qa_review>` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hotspot {
    pub kind: String,
    pub directive: String,
    pub pairs: Vec<HotspotPair>,
}

/// Selects boundary hotspots (transition between consecutive chapters).
pub fn select_boundary_hotspots(
    chapters: &[Vec<SegmentForQa>],
    directive: &str,
    n_boundary: usize,
) -> Vec<Hotspot> {
    let mut hotspots = Vec::new();
    if chapters.len() < 2 {
        return hotspots;
    }

    for (idx, pair) in chapters.windows(2).enumerate() {
        let chap_k = &pair[0];
        let chap_k1 = &pair[1];

        let mut pairs = Vec::new();

        // End of chapter k
        let k_start = chap_k.len().saturating_sub(n_boundary);
        for s in &chap_k[k_start..] {
            pairs.push(HotspotPair {
                chapter_ordinal: (idx + 1) as i32,
                seg_idx: s.idx,
                seg_from: s.idx,
                seg_to: s.idx,
                chapter_id: s.chapter_id.clone(),
                source: s.src_text.clone(),
                target: s.tgt_text.clone(),
            });
        }

        // Start of chapter k+1
        let k1_end = n_boundary.min(chap_k1.len());
        for s in &chap_k1[..k1_end] {
            pairs.push(HotspotPair {
                chapter_ordinal: (idx + 2) as i32,
                seg_idx: s.idx,
                seg_from: s.idx,
                seg_to: s.idx,
                chapter_id: s.chapter_id.clone(),
                source: s.src_text.clone(),
                target: s.tgt_text.clone(),
            });
        }

        if !pairs.is_empty() {
            hotspots.push(Hotspot {
                kind: "boundary".to_string(),
                directive: directive.to_string(),
                pairs,
            });
        }
    }

    hotspots
}

/// Selects long repeated-letter runs (Q10 findings) and their neighbors.
pub fn select_long_run_hotspots(
    chapters: &[Vec<SegmentForQa>],
    findings: &[QaFinding],
    directive: &str,
) -> Vec<Hotspot> {
    let mut hotspots = Vec::new();

    let q10_findings: Vec<&QaFinding> = findings.iter().filter(|f| f.check_code == "Q10").collect();

    if q10_findings.is_empty() {
        return hotspots;
    }

    for (chap_idx, chapter) in chapters.iter().enumerate() {
        for (seg_idx, seg) in chapter.iter().enumerate() {
            if q10_findings.iter().any(|f| f.segment_id == seg.segment_id) {
                let mut pairs = Vec::new();
                let start = seg_idx.saturating_sub(1);
                let end = (seg_idx + 2).min(chapter.len());

                for s in &chapter[start..end] {
                    pairs.push(HotspotPair {
                        chapter_ordinal: (chap_idx + 1) as i32,
                        seg_idx: s.idx,
                        seg_from: s.idx,
                        seg_to: s.idx,
                        chapter_id: s.chapter_id.clone(),
                        source: s.src_text.clone(),
                        target: s.tgt_text.clone(),
                    });
                }

                hotspots.push(Hotspot {
                    kind: "long_run".to_string(),
                    directive: directive.to_string(),
                    pairs,
                });
            }
        }
    }

    hotspots
}

/// Selects Tier 1 character names hotspot.
pub fn select_names_hotspots(
    tier1_names: &[(String, String)], // (source, target)
    directive: &str,
) -> Option<Hotspot> {
    if tier1_names.is_empty() {
        return None;
    }

    let pairs = tier1_names
        .iter()
        .enumerate()
        .map(|(idx, (src, tgt))| HotspotPair {
            chapter_ordinal: 0,
            seg_idx: (idx + 1) as i32,
            seg_from: (idx + 1) as i32,
            seg_to: (idx + 1) as i32,
            chapter_id: "glossary".to_string(),
            source: src.clone(),
            target: tgt.clone(),
        })
        .collect();

    Some(Hotspot {
        kind: "names".to_string(),
        directive: directive.to_string(),
        pairs,
    })
}

/// Selects homophone hotspots for a specific suspected source token.
pub fn select_homophone_hotspots(
    chapters: &[Vec<SegmentForQa>],
    suspect_token: &str,
    directive: &str,
) -> Option<Hotspot> {
    if suspect_token.trim().is_empty() {
        return None;
    }

    let mut pairs = Vec::new();
    for (chap_idx, chapter) in chapters.iter().enumerate() {
        for seg in chapter {
            if seg.src_text.contains(suspect_token) {
                pairs.push(HotspotPair {
                    chapter_ordinal: (chap_idx + 1) as i32,
                    seg_idx: seg.idx,
                    seg_from: seg.idx,
                    seg_to: seg.idx,
                    chapter_id: seg.chapter_id.clone(),
                    source: seg.src_text.clone(),
                    target: seg.tgt_text.clone(),
                });
            }
        }
    }

    if pairs.is_empty() {
        None
    } else {
        Some(Hotspot {
            kind: "homophone".to_string(),
            directive: directive.to_string(),
            pairs,
        })
    }
}

/// Selects dead wordplay hotspots (Q24 findings).
pub fn select_wordplay_hotspots(
    chapters: &[Vec<SegmentForQa>],
    findings: &[QaFinding],
    directive: &str,
) -> Vec<Hotspot> {
    let mut hotspots = Vec::new();
    let q24_findings: Vec<&QaFinding> = findings.iter().filter(|f| f.check_code == "Q24").collect();

    if q24_findings.is_empty() {
        return hotspots;
    }

    for (chap_idx, chapter) in chapters.iter().enumerate() {
        for (seg_idx, seg) in chapter.iter().enumerate() {
            if q24_findings.iter().any(|f| f.segment_id == seg.segment_id) {
                let mut pairs = Vec::new();
                let start = seg_idx.saturating_sub(1);
                let end = (seg_idx + 2).min(chapter.len());

                for s in &chapter[start..end] {
                    pairs.push(HotspotPair {
                        chapter_ordinal: (chap_idx + 1) as i32,
                        seg_idx: s.idx,
                        seg_from: s.idx,
                        seg_to: s.idx,
                        chapter_id: s.chapter_id.clone(),
                        source: s.src_text.clone(),
                        target: s.tgt_text.clone(),
                    });
                }

                hotspots.push(Hotspot {
                    kind: "wordplay".to_string(),
                    directive: directive.to_string(),
                    pairs,
                });
            }
        }
    }

    hotspots
}

/// Selects split seam hotspots (chunk plan part seams).
pub fn select_seam_hotspots(
    chapter_segments: &[SegmentForQa],
    chap_ordinal: i32,
    seam_seg_idx: i32,
    directive: &str,
) -> Option<Hotspot> {
    let mut pairs = Vec::new();

    let seam_pos = chapter_segments
        .iter()
        .position(|s| s.idx == seam_seg_idx)?;
    let start = seam_pos.saturating_sub(3);
    let end = (seam_pos + 4).min(chapter_segments.len());

    for s in &chapter_segments[start..end] {
        pairs.push(HotspotPair {
            chapter_ordinal: chap_ordinal,
            seg_idx: s.idx,
            seg_from: s.idx,
            seg_to: s.idx,
            chapter_id: s.chapter_id.clone(),
            source: s.src_text.clone(),
            target: s.tgt_text.clone(),
        });
    }

    if pairs.is_empty() {
        None
    } else {
        Some(Hotspot {
            kind: "seam".to_string(),
            directive: directive.to_string(),
            pairs,
        })
    }
}

/// Selects late volume sample hotspots (final third of volume).
pub fn select_late_volume_hotspots(
    chapters: &[Vec<SegmentForQa>],
    directive: &str,
) -> Option<Hotspot> {
    if chapters.len() < 3 {
        return None;
    }

    let late_start = (chapters.len() * 2) / 3;
    let mut pairs = Vec::new();

    for (idx, chap) in chapters.iter().enumerate().skip(late_start) {
        // Take a small sample (first 4 segments of each late chapter)
        for s in chap.iter().take(4) {
            pairs.push(HotspotPair {
                chapter_ordinal: (idx + 1) as i32,
                seg_idx: s.idx,
                seg_from: s.idx,
                seg_to: s.idx,
                chapter_id: s.chapter_id.clone(),
                source: s.src_text.clone(),
                target: s.tgt_text.clone(),
            });
        }
    }

    if pairs.is_empty() {
        None
    } else {
        Some(Hotspot {
            kind: "late_volume".to_string(),
            directive: directive.to_string(),
            pairs,
        })
    }
}

/// Selects single character scene hotspots (scenes between scene markers with exactly 1 Tier 1 character).
pub fn select_single_character_hotspots(
    chapters: &[Vec<SegmentForQa>],
    tier1_names: &[(String, String)],
    directive: &str,
) -> Vec<Hotspot> {
    let mut hotspots = Vec::new();
    if tier1_names.is_empty() {
        return hotspots;
    }

    for (chap_idx, chapter) in chapters.iter().enumerate() {
        // Break chapter into scenes bounded by scene markers ("☆", etc.) or chapter boundaries
        let mut scene_start = 0;
        let mut i = 0;
        while i <= chapter.len() {
            let is_boundary = i == chapter.len()
                || chapter[i].src_text.trim() == "☆"
                || chapter[i].tgt_text.trim() == "☆";
            if is_boundary {
                if i > scene_start {
                    let scene_segments = &chapter[scene_start..i];
                    // Count distinct Tier 1 characters appearing in this scene
                    let mut matched_chars = Vec::new();
                    for (src, _tgt) in tier1_names {
                        let present = scene_segments.iter().any(|s| s.src_text.contains(src));
                        if present {
                            matched_chars.push(src);
                        }
                    }

                    // Exactly one Tier 1 character in this scene
                    if matched_chars.len() == 1 {
                        let pairs: Vec<HotspotPair> = scene_segments
                            .iter()
                            .take(8) // Limit to first 8 segments of the scene
                            .map(|s| HotspotPair {
                                chapter_ordinal: (chap_idx + 1) as i32,
                                seg_idx: s.idx,
                                seg_from: s.idx,
                                seg_to: s.idx,
                                chapter_id: s.chapter_id.clone(),
                                source: s.src_text.clone(),
                                target: s.tgt_text.clone(),
                            })
                            .collect();

                        if !pairs.is_empty() {
                            hotspots.push(Hotspot {
                                kind: "single_character".to_string(),
                                directive: directive.to_string(),
                                pairs,
                            });
                        }
                    }
                }
                scene_start = i + 1;
            }
            i += 1;
        }
    }

    hotspots
}

/// Selects high frequency phrase hotspots across the volume.
pub fn select_frequency_hotspots(
    chapters: &[Vec<SegmentForQa>],
    top_words: &[String],
    directive: &str,
) -> Vec<Hotspot> {
    let mut hotspots = Vec::new();
    for word in top_words.iter().take(3) {
        let mut pairs = Vec::new();
        for (chap_idx, chapter) in chapters.iter().enumerate() {
            for seg in chapter {
                if seg.tgt_text.to_lowercase().contains(&word.to_lowercase()) {
                    pairs.push(HotspotPair {
                        chapter_ordinal: (chap_idx + 1) as i32,
                        seg_idx: seg.idx,
                        seg_from: seg.idx,
                        seg_to: seg.idx,
                        chapter_id: seg.chapter_id.clone(),
                        source: seg.src_text.clone(),
                        target: seg.tgt_text.clone(),
                    });
                    if pairs.len() >= 6 {
                        break;
                    }
                }
            }
            if pairs.len() >= 6 {
                break;
            }
        }

        if !pairs.is_empty() {
            hotspots.push(Hotspot {
                kind: "frequency".to_string(),
                directive: directive.to_string(),
                pairs,
            });
        }
    }
    hotspots
}

/// Gathers all Step C hotspots across the volume per Appendix C.
pub fn select_all_hotspots(
    chapters: &[Vec<SegmentForQa>],
    findings: &[QaFinding],
    tier1_names: &[(String, String)],
    directives: &HashMap<String, String>,
) -> Vec<Hotspot> {
    let mut hotspots = Vec::new();

    // 1. Boundary hotspots
    let boundary_directive = directives.get("boundary").map_or("", |s| s.as_str());
    hotspots.extend(select_boundary_hotspots(chapters, boundary_directive, 4));

    // 2. Long run hotspots
    let long_run_directive = directives.get("long_run").map_or("", |s| s.as_str());
    hotspots.extend(select_long_run_hotspots(
        chapters,
        findings,
        long_run_directive,
    ));

    // 3. Names hotspot
    let names_directive = directives.get("names").map_or("", |s| s.as_str());
    if let Some(h) = select_names_hotspots(tier1_names, names_directive) {
        hotspots.push(h);
    }

    // 4. Single character scenes
    let single_char_directive = directives
        .get("single_character")
        .map_or("", |s| s.as_str());
    hotspots.extend(select_single_character_hotspots(
        chapters,
        tier1_names,
        single_char_directive,
    ));

    // 5. Wordplay hotspots
    let wordplay_directive = directives.get("wordplay").map_or("", |s| s.as_str());
    hotspots.extend(select_wordplay_hotspots(
        chapters,
        findings,
        wordplay_directive,
    ));

    // 6. Late volume sample hotspot
    let late_directive = directives.get("late_volume").map_or("", |s| s.as_str());
    if let Some(h) = select_late_volume_hotspots(chapters, late_directive) {
        hotspots.push(h);
    }

    hotspots
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_select_boundary_hotspots() {
        let chap1: Vec<SegmentForQa> = (1..=10)
            .map(|i| SegmentForQa {
                segment_id: format!("c1_s{i}"),
                chapter_id: "c1".to_string(),
                idx: i,
                src_text: format!("src1_{i}"),
                tgt_text: format!("tgt1_{i}"),
            })
            .collect();

        let chap2: Vec<SegmentForQa> = (1..=10)
            .map(|i| SegmentForQa {
                segment_id: format!("c2_s{i}"),
                chapter_id: "c2".to_string(),
                idx: i,
                src_text: format!("src2_{i}"),
                tgt_text: format!("tgt2_{i}"),
            })
            .collect();

        let chapters = vec![chap1, chap2];
        let hotspots = select_boundary_hotspots(&chapters, "Directive boundary", 4);

        assert_eq!(hotspots.len(), 1);
        let h = &hotspots[0];
        assert_eq!(h.kind, "boundary");
        assert_eq!(h.pairs.len(), 8); // 4 from chap1 + 4 from chap2
        assert_eq!(h.pairs[0].ref_str(), "1:7");
        assert_eq!(h.pairs[7].ref_str(), "2:4");
    }

    #[test]
    fn test_select_names_and_homophone() {
        let names = vec![("健太".to_string(), "Kenta".to_string())];
        let h_names = select_names_hotspots(&names, "Names directive").unwrap();
        assert_eq!(h_names.kind, "names");
        assert_eq!(h_names.pairs.len(), 1);

        let chap = vec![SegmentForQa {
            segment_id: "s1".to_string(),
            chapter_id: "c1".to_string(),
            idx: 1,
            src_text: "え？本当に？".to_string(),
            tgt_text: "Lukisan itu nyata?".to_string(),
        }];
        let h_homophone = select_homophone_hotspots(&[chap], "え", "Homophone directive").unwrap();
        assert_eq!(h_homophone.kind, "homophone");
        assert_eq!(h_homophone.pairs.len(), 1);
        assert_eq!(h_homophone.pairs[0].source, "え？本当に？");
    }
}
