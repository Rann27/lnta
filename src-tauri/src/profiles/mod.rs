use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::AppError;

/// Profile data loaded from resource files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfileConfig {
    pub name: String,
    pub source_language: String,
    pub target_language: String,
    pub instruction_candidates: Vec<String>,
    pub chunk_threshold_lines: usize,
    pub gender_keywords: Vec<String>,
    pub scene_marker_set: Vec<String>,
    pub ellipsis_form: String,
    pub ratio_band_min: f64,
    pub ratio_band_max: f64,
    pub title_strip_brackets: bool,
    pub ruby_normalize: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QaConfig {
    #[serde(default)]
    pub genre_terms: Vec<String>,
    #[serde(default)]
    pub honorifics: Vec<String>,
    #[serde(default)]
    pub pronoun_pairs: Vec<(String, String)>,
    #[serde(default)]
    pub metronome_openers: Vec<String>,
    #[serde(default)]
    pub sfx_romaji: Vec<String>,
    #[serde(default)]
    pub sfx_allowed: Vec<String>,
    #[serde(default)]
    pub confusable_pairs: Vec<(String, String)>,
    #[serde(default)]
    pub ambiguous_acronyms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutofixConfig {
    #[serde(default)]
    pub unambiguous_acronyms: Vec<String>,
    #[serde(default)]
    pub spelling_replacements: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProfileData {
    pub id: String,
    pub config: ProfileConfig,
    pub qa_config: QaConfig,
    pub autofix_config: AutofixConfig,
    pub contract_text: &'static str,
    pub contract_version: String,
    pub qa_contract_text: &'static str,
    pub qa_directives: HashMap<String, String>,
}

// Embedded resource contents
const JA_ID_PROFILE_TOML: &str = include_str!("../../resources/profiles/ja-id/profile.toml");
const JA_ID_QA_TOML: &str = include_str!("../../resources/profiles/ja-id/qa.toml");
const JA_ID_AUTOFIX_TOML: &str = include_str!("../../resources/profiles/ja-id/autofix.toml");
const JA_ID_CONTRACT_MD: &str = include_str!("../../resources/profiles/ja-id/contract.md");
const JA_ID_QA_CONTRACT_MD: &str = include_str!("../../resources/profiles/ja-id/qa-contract.md");

const JA_EN_PROFILE_TOML: &str = include_str!("../../resources/profiles/ja-en/profile.toml");
const JA_EN_QA_TOML: &str = include_str!("../../resources/profiles/ja-en/qa.toml");
const JA_EN_AUTOFIX_TOML: &str = include_str!("../../resources/profiles/ja-en/autofix.toml");
const JA_EN_CONTRACT_MD: &str = include_str!("../../resources/profiles/ja-en/contract.md");
const JA_EN_QA_CONTRACT_MD: &str = include_str!("../../resources/profiles/ja-en/qa-contract.md");

/// Parses the contract version from the first heading of a contract file.
pub fn parse_contract_version(text: &str) -> String {
    for line in text.lines() {
        if line.starts_with('#') {
            if let Some(pos) = line.to_lowercase().find("versi") {
                let rest = &line[pos + 5..];
                let num_str: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
                if !num_str.is_empty() {
                    return num_str;
                }
            } else if let Some(pos) = line.to_lowercase().find("version") {
                let rest = &line[pos + 7..];
                let num_str: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
                if !num_str.is_empty() {
                    return num_str;
                }
            }
        }
    }
    "1".to_string()
}

/// Parses QA directives from `### <code>` headings in a QA contract.
pub fn parse_qa_directives(text: &str) -> HashMap<String, String> {
    let mut directives = HashMap::new();
    let mut current_code: Option<String> = None;
    let mut current_content = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("### ") {
            if let Some(code) = current_code.take() {
                directives.insert(code, current_content.join("\n").trim().to_string());
                current_content.clear();
            }
            let code = trimmed.trim_start_matches("### ").trim().to_string();
            current_code = Some(code);
        } else if trimmed.starts_with("## ") {
            if let Some(code) = current_code.take() {
                directives.insert(code, current_content.join("\n").trim().to_string());
                current_content.clear();
            }
        } else if current_code.is_some() {
            current_content.push(line);
        }
    }

    if let Some(code) = current_code {
        directives.insert(code, current_content.join("\n").trim().to_string());
    }

    directives
}

/// Loads profile data for a given profile ID (`ja-id` or `ja-en`).
pub fn load_profile(id: &str) -> Result<ProfileData, AppError> {
    match id {
        "ja-id" => {
            let config: ProfileConfig =
                toml::from_str(JA_ID_PROFILE_TOML).map_err(|e| AppError::Internal {
                    message: format!("Failed to parse ja-id profile.toml: {e}"),
                })?;
            let qa_config: QaConfig =
                toml::from_str(JA_ID_QA_TOML).map_err(|e| AppError::Internal {
                    message: format!("Failed to parse ja-id qa.toml: {e}"),
                })?;
            let autofix_config: AutofixConfig =
                toml::from_str(JA_ID_AUTOFIX_TOML).map_err(|e| AppError::Internal {
                    message: format!("Failed to parse ja-id autofix.toml: {e}"),
                })?;

            let contract_version = parse_contract_version(JA_ID_CONTRACT_MD);
            let qa_directives = parse_qa_directives(JA_ID_QA_CONTRACT_MD);

            Ok(ProfileData {
                id: "ja-id".to_string(),
                config,
                qa_config,
                autofix_config,
                contract_text: JA_ID_CONTRACT_MD,
                contract_version,
                qa_contract_text: JA_ID_QA_CONTRACT_MD,
                qa_directives,
            })
        }
        "ja-en" => {
            let config: ProfileConfig =
                toml::from_str(JA_EN_PROFILE_TOML).map_err(|e| AppError::Internal {
                    message: format!("Failed to parse ja-en profile.toml: {e}"),
                })?;
            let qa_config: QaConfig =
                toml::from_str(JA_EN_QA_TOML).map_err(|e| AppError::Internal {
                    message: format!("Failed to parse ja-en qa.toml: {e}"),
                })?;
            let autofix_config: AutofixConfig =
                toml::from_str(JA_EN_AUTOFIX_TOML).map_err(|e| AppError::Internal {
                    message: format!("Failed to parse ja-en autofix.toml: {e}"),
                })?;

            let contract_version = parse_contract_version(JA_EN_CONTRACT_MD);
            let qa_directives = parse_qa_directives(JA_EN_QA_CONTRACT_MD);

            Ok(ProfileData {
                id: "ja-en".to_string(),
                config,
                qa_config,
                autofix_config,
                contract_text: JA_EN_CONTRACT_MD,
                contract_version,
                qa_contract_text: JA_EN_QA_CONTRACT_MD,
                qa_directives,
            })
        }
        _ => Err(AppError::NotFound {
            entity: "profile".to_string(),
            id: id.to_string(),
        }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::chunks_exact_to_as_chunks)]
mod tests {
    use super::*;

    // Standard FIPS 180-4 SHA-256 implementation for self-check test
    fn sha256(data: &[u8]) -> String {
        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];

        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19,
        ];

        let bit_len = (data.len() as u64) * 8;
        let mut msg = data.to_vec();
        msg.push(0x80);
        while (msg.len() % 64) != 56 {
            msg.push(0x00);
        }
        msg.extend_from_slice(&bit_len.to_be_bytes());

        for chunk in msg.chunks_exact(64) {
            let mut w = [0u32; 64];
            for (i, part) in chunk.chunks_exact(4).enumerate() {
                w[i] = u32::from_be_bytes([part[0], part[1], part[2], part[3]]);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }

            let mut a = h[0];
            let mut b = h[1];
            let mut c = h[2];
            let mut d = h[3];
            let mut e = h[4];
            let mut f = h[5];
            let mut g = h[6];
            let mut h_val = h[7];

            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let temp1 = h_val
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let temp2 = s0.wrapping_add(maj);

                h_val = g;
                g = f;
                f = e;
                e = d.wrapping_add(temp1);
                d = c;
                c = b;
                b = a;
                a = temp1.wrapping_add(temp2);
            }

            h[0] = h[0].wrapping_add(a);
            h[1] = h[1].wrapping_add(b);
            h[2] = h[2].wrapping_add(c);
            h[3] = h[3].wrapping_add(d);
            h[4] = h[4].wrapping_add(e);
            h[5] = h[5].wrapping_add(f);
            h[6] = h[6].wrapping_add(g);
            h[7] = h[7].wrapping_add(h_val);
        }

        h.iter().map(|val| format!("{val:08x}")).collect::<String>()
    }

    #[test]
    fn test_contract_byte_identical_hashes() {
        const HASH_JA_ID_CONTRACT: &str =
            "5fc20aa2206f6813ed194be608e9a72f6e660eea512d29ac792905a1309b73b0";
        const HASH_JA_ID_QA_CONTRACT: &str =
            "7ec5c246efaa38154a6bd09b44ac667eae64d94489204aa6bc18c346e380481d";
        const HASH_JA_EN_CONTRACT: &str =
            "c2e27419287b0ec309f9a42cfae828753d3efadba721e20477fb983c292b7886";
        const HASH_JA_EN_QA_CONTRACT: &str =
            "749094992f9dfe3e598b13cef9193af71e033fb0a48c30facf32d13383a10a40";

        assert_eq!(sha256(JA_ID_CONTRACT_MD.as_bytes()), HASH_JA_ID_CONTRACT);
        assert_eq!(
            sha256(JA_ID_QA_CONTRACT_MD.as_bytes()),
            HASH_JA_ID_QA_CONTRACT
        );
        assert_eq!(sha256(JA_EN_CONTRACT_MD.as_bytes()), HASH_JA_EN_CONTRACT);
        assert_eq!(
            sha256(JA_EN_QA_CONTRACT_MD.as_bytes()),
            HASH_JA_EN_QA_CONTRACT
        );
    }

    #[test]
    fn test_load_profile_ja_id() {
        let p = load_profile("ja-id").unwrap();
        assert_eq!(p.config.name, "ja-id");
        assert_eq!(p.config.chunk_threshold_lines, 1500);
        assert!(p.config.gender_keywords.contains(&"laki-laki".to_string()));
        assert!(p.config.scene_marker_set.contains(&"☆".to_string()));
        assert_eq!(p.config.ellipsis_form, "……");
        assert_eq!(p.contract_version, "1");

        // Verify parsed QA directives
        assert!(p.qa_directives.contains_key("Q01"));
        assert!(p.qa_directives.contains_key("Q04"));
        assert!(p.qa_directives.contains_key("boundary"));
        assert!(p.qa_directives.contains_key("long_run"));
    }

    #[test]
    fn test_load_profile_ja_en() {
        let p = load_profile("ja-en").unwrap();
        assert_eq!(p.config.name, "ja-en");
        assert_eq!(p.config.chunk_threshold_lines, 1500);
        assert!(p.config.gender_keywords.contains(&"female".to_string()));
        assert_eq!(p.config.ellipsis_form, "...");
        assert_eq!(p.contract_version, "1");

        assert!(p.qa_directives.contains_key("Q01"));
        assert!(p.qa_directives.contains_key("seam"));
    }
}
