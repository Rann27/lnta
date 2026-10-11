//! Project preflight verification engine per PRD §5.2 and Story A2.
//!
//! Evaluates the 16 preflight checks:
//! 1. instruction_file (Blocker)
//! 2. instruction_candidates (Blocker)
//! 3. instruction_language_match (Warning)
//! 4. chapter_files_detected (Blocker)
//! 5. chapter_prefixes_unique (Blocker)
//! 6. translator_model_configured (Blocker)
//! 7. docx_template_valid (Blocker)
//! 8. docx_template_contains_text (Warning)
//! 9. glossary_file_exists (Warning)
//! 10. glossary_macrons (Warning)
//! 11. glossary_conflicts (Warning)
//! 12. missing_illustrations (Warning)
//! 13. unsupported_image_formats (Warning)
//! 14. raw_encoding_utf8 (Warning)
//! 15. chapters_exceed_chunk_threshold (Info)
//! 16. model_max_output_tokens (Warning)

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::docx::template::DocxPackage;
use crate::docx::template_content::does_template_contain_text;
use crate::error::AppError;
use crate::glossary::lint::{lint_glossary, GlossaryLintKind};
use crate::glossary::parse::parse_glossary;
use crate::profiles::load_profile;
use crate::project::discover::discover_chapters;
use crate::project::instruction::rank_instruction_candidates;
use crate::raw::classify::{Classifier, IllustrationSet, RawLineKind};
use crate::raw::encoding::read_raw_file;
use ts_rs::TS;

/// Preflight check severity status per PRD §5.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../src/lib/bindings/PreflightStatus.ts")]
pub enum PreflightStatus {
    Pass,
    Info,
    Warning,
    Blocker,
}

/// A suggested action offered to resolve a preflight finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/PreflightAction.ts")]
pub struct PreflightAction {
    pub id: String,
    pub label_code: String,
}

/// An individual preflight check result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/PreflightItem.ts")]
pub struct PreflightItem {
    pub id: String,
    pub status: PreflightStatus,
    pub message_code: String,
    pub params: HashMap<String, String>,
    pub actions: Vec<PreflightAction>,
}

impl PreflightItem {
    pub fn pass(id: &str, message_code: &str) -> Self {
        Self {
            id: id.to_string(),
            status: PreflightStatus::Pass,
            message_code: message_code.to_string(),
            params: HashMap::new(),
            actions: Vec::new(),
        }
    }
}

/// Aggregated preflight report for a project folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/PreflightReport.ts")]
pub struct PreflightReport {
    pub items: Vec<PreflightItem>,
    pub can_run: bool,
}

impl PreflightReport {
    pub fn new(items: Vec<PreflightItem>) -> Self {
        let can_run = !items.iter().any(|it| it.status == PreflightStatus::Blocker);
        Self { items, can_run }
    }

    pub fn blockers(&self) -> Vec<&PreflightItem> {
        self.items
            .iter()
            .filter(|it| it.status == PreflightStatus::Blocker)
            .collect()
    }

    pub fn warnings(&self) -> Vec<&PreflightItem> {
        self.items
            .iter()
            .filter(|it| it.status == PreflightStatus::Warning)
            .collect()
    }

    pub fn infos(&self) -> Vec<&PreflightItem> {
        self.items
            .iter()
            .filter(|it| it.status == PreflightStatus::Info)
            .collect()
    }

    pub fn passes(&self) -> Vec<&PreflightItem> {
        self.items
            .iter()
            .filter(|it| it.status == PreflightStatus::Pass)
            .collect()
    }

    pub fn get_item(&self, id: &str) -> Option<&PreflightItem> {
        self.items.iter().find(|it| it.id == id)
    }
}

/// Environment options provided during preflight evaluation.
#[derive(Debug, Clone)]
pub struct PreflightEnvironment {
    pub translator_model_configured: bool,
    pub api_key_or_cli_available: bool,
    pub max_output_tokens: Option<u32>,
    pub skip_model_check: bool,
    pub chunk_threshold_lines: usize,
    /// Instruction file chosen in `project.toml`; it replaces candidate discovery.
    pub instruction_file: Option<String>,
}

impl Default for PreflightEnvironment {
    fn default() -> Self {
        Self {
            translator_model_configured: true,
            api_key_or_cli_available: true,
            max_output_tokens: Some(4096),
            skip_model_check: false,
            chunk_threshold_lines: 1500,
            instruction_file: None,
        }
    }
}

/// Heuristically detects the instruction language (ja-id vs ja-en) per FR-PRJ-05.
pub fn detect_instruction_language(content: &str) -> Option<&'static str> {
    let lower = content.to_lowercase();
    if lower.contains("jp → id")
        || lower.contains("jp -> id")
        || lower.contains("japanese to indonesian")
    {
        return Some("ja-id");
    }
    if lower.contains("jp → en")
        || lower.contains("jp -> en")
        || lower.contains("japanese to english")
    {
        return Some("ja-en");
    }

    let id_words = [
        "yang", "dan", "di", "dari", "untuk", "dengan", "ini", "itu", "pada", "adalah",
    ];
    let en_words = [
        "the", "and", "of", "to", "in", "that", "is", "for", "with", "as",
    ];

    let mut id_count = 0;
    let mut en_count = 0;

    for word in lower.split_whitespace() {
        let cleaned = word.trim_matches(|c: char| !c.is_alphabetic());
        if id_words.contains(&cleaned) {
            id_count += 1;
        } else if en_words.contains(&cleaned) {
            en_count += 1;
        }
    }

    if id_count > en_count + 5 {
        Some("ja-id")
    } else if en_count > id_count + 5 {
        Some("ja-en")
    } else {
        None
    }
}

/// Helper running preflight on a project folder using its project.toml and global settings.
/// True for endpoints on this machine, which never need an API key.
fn is_local_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    ["://localhost", "://127.0.0.1", "://[::1]", "://0.0.0.0"]
        .iter()
        .any(|host| lower.contains(host))
}

pub fn run_preflight_on_folder(
    project_root: &Path,
    settings: &crate::project::settings::GlobalSettings,
) -> PreflightReport {
    let layout = crate::project::layout::ProjectLayout::new(project_root);
    let config = if layout.config_file.exists() {
        crate::project::config::ProjectConfig::load_from_file(&layout.config_file).ok()
    } else {
        None
    };
    let profile = config
        .as_ref()
        .map(|c| c.profile.clone())
        .unwrap_or_else(|| "ja-id".to_string());

    let trans_role = settings
        .roles
        .for_role(crate::project::settings::Role::Translator);
    // Only hosted APIs need a key. Local OpenAI-compatible servers (Ollama, LM Studio) and the
    // subscription CLIs authenticate on their own, so demanding a key would block them forever.
    let provider_cfg = settings
        .providers
        .iter()
        .find(|p| p.id == trans_role.primary.provider);
    let provider_kind = provider_cfg
        .map(|p| p.provider_type.as_str())
        .unwrap_or(trans_role.primary.provider.as_str());
    let has_key = match provider_kind {
        "cli_claude" | "cli_codex" | "cli_antigravity" => {
            let bin = crate::provider::cli_common::cli_binary_for(provider_kind);
            crate::provider::cli_common::resolve_cli_path(bin).is_some()
        }
        "mock" => true,
        "openai_compat" if provider_cfg.is_some_and(|p| is_local_url(&p.base_url)) => true,
        _ => crate::secrets::get_api_key(&trans_role.primary.provider)
            .ok()
            .flatten()
            .is_some(),
    };

    let env = PreflightEnvironment {
        translator_model_configured: !trans_role.primary.model_id.is_empty(),
        api_key_or_cli_available: has_key,
        max_output_tokens: trans_role.max_output_tokens.or(Some(4096)),
        skip_model_check: false,
        chunk_threshold_lines: config
            .as_ref()
            .map(|c| c.chunk_threshold_lines)
            .unwrap_or(1500),
        instruction_file: config.and_then(|c| c.instruction_file),
    };

    run_preflight(project_root, &profile, &env)
}

/// Runs all 16 preflight checks on `project_root` against `profile`.
pub fn run_preflight<P: AsRef<Path>>(
    project_root: P,
    profile: &str,
    env: &PreflightEnvironment,
) -> PreflightReport {
    let root = project_root.as_ref();
    let mut items = Vec::new();

    // 1 & 2. Instruction file detection and candidates. A file chosen in project.toml wins,
    // so a saved choice never shows up again as an unresolved tie.
    let configured_instruction = env
        .instruction_file
        .as_deref()
        .map(str::trim)
        .filter(|f| !f.is_empty())
        .map(|f| root.join(f));
    let instruction_candidates_result = match &configured_instruction {
        Some(path) => Ok(vec![(0, path.clone())]),
        None => rank_instruction_candidates(root, profile),
    };
    let mut resolved_instruction_content: Option<String> = None;

    match instruction_candidates_result {
        Ok(candidates) => {
            if candidates.is_empty() {
                items.push(PreflightItem {
                    id: "instruction_file".to_string(),
                    status: PreflightStatus::Blocker,
                    message_code: "preflight.instruction_missing".to_string(),
                    params: HashMap::new(),
                    // FR-PRJ-09: No "copy bundled instruction" action exists
                    actions: vec![PreflightAction {
                        id: "choose_instruction_file".to_string(),
                        label_code: "actions.choose_instruction_file".to_string(),
                    }],
                });
                items.push(PreflightItem::pass(
                    "instruction_candidates",
                    "preflight.instruction_candidates_ok",
                ));
            } else {
                let first = &candidates[0].1;
                match std::fs::read_to_string(first) {
                    Ok(content) if content.trim().is_empty() => {
                        items.push(PreflightItem {
                            id: "instruction_file".to_string(),
                            status: PreflightStatus::Blocker,
                            message_code: "preflight.instruction_empty".to_string(),
                            params: HashMap::new(),
                            actions: vec![PreflightAction {
                                id: "choose_instruction_file".to_string(),
                                label_code: "actions.choose_instruction_file".to_string(),
                            }],
                        });
                        items.push(PreflightItem::pass(
                            "instruction_candidates",
                            "preflight.instruction_candidates_ok",
                        ));
                    }
                    Ok(content) => {
                        resolved_instruction_content = Some(content);
                        items.push(PreflightItem::pass(
                            "instruction_file",
                            "preflight.instruction_file_ok",
                        ));

                        let tied = candidates.len() > 1 && candidates[0].0 == candidates[1].0;
                        if tied {
                            let best = candidates[0].0;
                            let candidate_names: Vec<String> = candidates
                                .iter()
                                .filter(|(rank, _)| *rank == best)
                                .filter_map(|(_, p)| p.file_name())
                                .map(|f| f.to_string_lossy().to_string())
                                .collect();

                            let mut params = HashMap::new();
                            params.insert("candidates".to_string(), candidate_names.join(", "));

                            items.push(PreflightItem {
                                id: "instruction_candidates".to_string(),
                                status: PreflightStatus::Blocker,
                                message_code: "preflight.instruction_multiple_candidates"
                                    .to_string(),
                                params,
                                actions: vec![PreflightAction {
                                    id: "select_instruction_candidate".to_string(),
                                    label_code: "actions.select_instruction_candidate".to_string(),
                                }],
                            });
                        } else {
                            items.push(PreflightItem::pass(
                                "instruction_candidates",
                                "preflight.instruction_candidates_ok",
                            ));
                        }
                    }
                    Err(_) => {
                        items.push(PreflightItem {
                            id: "instruction_file".to_string(),
                            status: PreflightStatus::Blocker,
                            message_code: "preflight.instruction_unreadable".to_string(),
                            params: HashMap::new(),
                            actions: vec![PreflightAction {
                                id: "choose_instruction_file".to_string(),
                                label_code: "actions.choose_instruction_file".to_string(),
                            }],
                        });
                        items.push(PreflightItem::pass(
                            "instruction_candidates",
                            "preflight.instruction_candidates_ok",
                        ));
                    }
                }
            }
        }
        Err(_) => {
            items.push(PreflightItem {
                id: "instruction_file".to_string(),
                status: PreflightStatus::Blocker,
                message_code: "preflight.instruction_missing".to_string(),
                params: HashMap::new(),
                actions: vec![PreflightAction {
                    id: "choose_instruction_file".to_string(),
                    label_code: "actions.choose_instruction_file".to_string(),
                }],
            });
            items.push(PreflightItem::pass(
                "instruction_candidates",
                "preflight.instruction_candidates_ok",
            ));
        }
    }

    // 3. Instruction language match heuristic
    if let Some(content) = resolved_instruction_content {
        if let Some(detected) = detect_instruction_language(&content) {
            if detected != profile {
                let mut params = HashMap::new();
                params.insert("detected_profile".to_string(), detected.to_string());
                params.insert("selected_profile".to_string(), profile.to_string());

                items.push(PreflightItem {
                    id: "instruction_language_match".to_string(),
                    status: PreflightStatus::Warning,
                    message_code: "preflight.instruction_language_mismatch".to_string(),
                    params,
                    actions: vec![
                        PreflightAction {
                            id: "switch_profile".to_string(),
                            label_code: "actions.switch_profile".to_string(),
                        },
                        PreflightAction {
                            id: "dismiss".to_string(),
                            label_code: "actions.dismiss".to_string(),
                        },
                    ],
                });
            } else {
                items.push(PreflightItem::pass(
                    "instruction_language_match",
                    "preflight.instruction_language_match_ok",
                ));
            }
        } else {
            items.push(PreflightItem::pass(
                "instruction_language_match",
                "preflight.instruction_language_match_ok",
            ));
        }
    } else {
        items.push(PreflightItem::pass(
            "instruction_language_match",
            "preflight.instruction_language_match_ok",
        ));
    }

    // 4 & 5. Chapters discovery and duplicate prefix
    let chapters_result = discover_chapters(root);
    let mut discovered_chapters = Vec::new();

    match chapters_result {
        Ok(chapters) => {
            if chapters.is_empty() {
                items.push(PreflightItem {
                    id: "chapter_files_detected".to_string(),
                    status: PreflightStatus::Blocker,
                    message_code: "preflight.no_chapters".to_string(),
                    params: HashMap::new(),
                    actions: vec![PreflightAction {
                        id: "open_other_folder".to_string(),
                        label_code: "actions.open_other_folder".to_string(),
                    }],
                });
                items.push(PreflightItem::pass(
                    "chapter_prefixes_unique",
                    "preflight.chapter_prefixes_unique_ok",
                ));
            } else {
                items.push(PreflightItem::pass(
                    "chapter_files_detected",
                    "preflight.chapter_files_detected_ok",
                ));
                items.push(PreflightItem::pass(
                    "chapter_prefixes_unique",
                    "preflight.chapter_prefixes_unique_ok",
                ));
                discovered_chapters = chapters;
            }
        }
        Err(AppError::DuplicatePrefix { prefix }) => {
            items.push(PreflightItem::pass(
                "chapter_files_detected",
                "preflight.chapter_files_detected_ok",
            ));
            let mut params = HashMap::new();
            params.insert("prefix".to_string(), prefix.clone());
            params.insert("conflicting_files".to_string(), prefix);

            items.push(PreflightItem {
                id: "chapter_prefixes_unique".to_string(),
                status: PreflightStatus::Blocker,
                message_code: "preflight.duplicate_prefix".to_string(),
                params,
                actions: vec![PreflightAction {
                    id: "show_conflicting_files".to_string(),
                    label_code: "actions.show_conflicting_files".to_string(),
                }],
            });
        }
        Err(_) => {
            items.push(PreflightItem {
                id: "chapter_files_detected".to_string(),
                status: PreflightStatus::Blocker,
                message_code: "preflight.no_chapters".to_string(),
                params: HashMap::new(),
                actions: vec![PreflightAction {
                    id: "open_other_folder".to_string(),
                    label_code: "actions.open_other_folder".to_string(),
                }],
            });
            items.push(PreflightItem::pass(
                "chapter_prefixes_unique",
                "preflight.chapter_prefixes_unique_ok",
            ));
        }
    }

    // 6. Translator model configured
    if env.skip_model_check {
        items.push(PreflightItem::pass(
            "translator_model_configured",
            "preflight.translator_model_ok",
        ));
    } else if !env.translator_model_configured || !env.api_key_or_cli_available {
        items.push(PreflightItem {
            id: "translator_model_configured".to_string(),
            status: PreflightStatus::Blocker,
            message_code: "preflight.translator_model_missing".to_string(),
            params: HashMap::new(),
            actions: vec![PreflightAction {
                id: "open_provider_settings".to_string(),
                label_code: "actions.open_provider_settings".to_string(),
            }],
        });
    } else {
        items.push(PreflightItem::pass(
            "translator_model_configured",
            "preflight.translator_model_ok",
        ));
    }

    // 7 & 8. Template docx valid and text content check
    let template_path = root.join("Translations.docx");
    let mut template_has_text = false;

    if !template_path.exists() {
        items.push(PreflightItem {
            id: "docx_template_valid".to_string(),
            status: PreflightStatus::Blocker,
            message_code: "preflight.template_missing".to_string(),
            params: HashMap::new(),
            actions: vec![
                PreflightAction {
                    id: "choose_template_file".to_string(),
                    label_code: "actions.choose_template_file".to_string(),
                },
                PreflightAction {
                    id: "create_default_template".to_string(),
                    label_code: "actions.create_default_template".to_string(),
                },
            ],
        });
        items.push(PreflightItem::pass(
            "docx_template_contains_text",
            "preflight.template_no_text",
        ));
    } else {
        match std::fs::read(&template_path) {
            Ok(bytes) => match DocxPackage::from_bytes(&bytes) {
                Ok(pkg) => {
                    let doc_xml = pkg.get_text_entry("word/document.xml").unwrap_or_default();
                    let styles_xml = pkg.get_text_entry("word/styles.xml").unwrap_or_default();

                    if doc_xml.is_empty() || styles_xml.is_empty() {
                        items.push(PreflightItem {
                            id: "docx_template_valid".to_string(),
                            status: PreflightStatus::Blocker,
                            message_code: "preflight.template_invalid".to_string(),
                            params: HashMap::new(),
                            actions: vec![
                                PreflightAction {
                                    id: "choose_template_file".to_string(),
                                    label_code: "actions.choose_template_file".to_string(),
                                },
                                PreflightAction {
                                    id: "create_default_template".to_string(),
                                    label_code: "actions.create_default_template".to_string(),
                                },
                            ],
                        });
                        items.push(PreflightItem::pass(
                            "docx_template_contains_text",
                            "preflight.template_no_text",
                        ));
                    } else {
                        items.push(PreflightItem::pass(
                            "docx_template_valid",
                            "preflight.template_valid_ok",
                        ));

                        if does_template_contain_text(&doc_xml) {
                            template_has_text = true;
                        }
                    }
                }
                Err(_) => {
                    items.push(PreflightItem {
                        id: "docx_template_valid".to_string(),
                        status: PreflightStatus::Blocker,
                        message_code: "preflight.template_invalid".to_string(),
                        params: HashMap::new(),
                        actions: vec![
                            PreflightAction {
                                id: "choose_template_file".to_string(),
                                label_code: "actions.choose_template_file".to_string(),
                            },
                            PreflightAction {
                                id: "create_default_template".to_string(),
                                label_code: "actions.create_default_template".to_string(),
                            },
                        ],
                    });
                    items.push(PreflightItem::pass(
                        "docx_template_contains_text",
                        "preflight.template_no_text",
                    ));
                }
            },
            Err(_) => {
                items.push(PreflightItem {
                    id: "docx_template_valid".to_string(),
                    status: PreflightStatus::Blocker,
                    message_code: "preflight.template_unreadable".to_string(),
                    params: HashMap::new(),
                    actions: vec![
                        PreflightAction {
                            id: "choose_template_file".to_string(),
                            label_code: "actions.choose_template_file".to_string(),
                        },
                        PreflightAction {
                            id: "create_default_template".to_string(),
                            label_code: "actions.create_default_template".to_string(),
                        },
                    ],
                });
                items.push(PreflightItem::pass(
                    "docx_template_contains_text",
                    "preflight.template_no_text",
                ));
            }
        }

        if template_has_text {
            items.push(PreflightItem {
                id: "docx_template_contains_text".to_string(),
                status: PreflightStatus::Warning,
                message_code: "preflight.template_has_text".to_string(),
                params: HashMap::new(),
                actions: vec![PreflightAction {
                    id: "choose_template_treatment".to_string(),
                    label_code: "actions.choose_template_treatment".to_string(),
                }],
            });
        } else {
            items.push(PreflightItem::pass(
                "docx_template_contains_text",
                "preflight.template_no_text",
            ));
        }
    }

    // 9, 10, 11. Glossary checks
    let glossary_path = root.join("glossary.md");
    if !glossary_path.exists() {
        items.push(PreflightItem {
            id: "glossary_file_exists".to_string(),
            status: PreflightStatus::Warning,
            message_code: "preflight.glossary_missing".to_string(),
            params: HashMap::new(),
            actions: Vec::new(),
        });
        items.push(PreflightItem::pass(
            "glossary_macrons",
            "preflight.glossary_macrons_ok",
        ));
        items.push(PreflightItem::pass(
            "glossary_conflicts",
            "preflight.glossary_conflicts_ok",
        ));
    } else {
        items.push(PreflightItem::pass(
            "glossary_file_exists",
            "preflight.glossary_exists_ok",
        ));

        let glossary_content = std::fs::read_to_string(&glossary_path).unwrap_or_default();
        let gender_keywords = load_profile(profile)
            .map(|p| p.config.gender_keywords)
            .unwrap_or_else(|_| {
                vec![
                    "laki-laki".to_string(),
                    "perempuan".to_string(),
                    "pria".to_string(),
                    "wanita".to_string(),
                ]
            });
        let glossary_file = parse_glossary(&glossary_content, &gender_keywords);
        let findings = lint_glossary(&glossary_file, &gender_keywords);

        let has_macron = findings.iter().any(|f| f.kind == GlossaryLintKind::Macron);
        let has_conflict = findings.iter().any(|f| {
            f.kind == GlossaryLintKind::DuplicateSource
                || f.kind == GlossaryLintKind::NearVariantName
        });

        if has_macron {
            items.push(PreflightItem {
                id: "glossary_macrons".to_string(),
                status: PreflightStatus::Warning,
                message_code: "preflight.glossary_has_macrons".to_string(),
                params: HashMap::new(),
                actions: vec![PreflightAction {
                    id: "fix_macrons".to_string(),
                    label_code: "actions.fix_macrons".to_string(),
                }],
            });
        } else {
            items.push(PreflightItem::pass(
                "glossary_macrons",
                "preflight.glossary_macrons_ok",
            ));
        }

        if has_conflict {
            items.push(PreflightItem {
                id: "glossary_conflicts".to_string(),
                status: PreflightStatus::Warning,
                message_code: "preflight.glossary_has_conflicts".to_string(),
                params: HashMap::new(),
                actions: vec![PreflightAction {
                    id: "open_glossary_tab".to_string(),
                    label_code: "actions.open_glossary_tab".to_string(),
                }],
            });
        } else {
            items.push(PreflightItem::pass(
                "glossary_conflicts",
                "preflight.glossary_conflicts_ok",
            ));
        }
    }

    // 12, 13, 14, 15. Raw chapter scans: missing illustrations, unsupported webp, non-UTF8, chunk threshold
    let illustrations_dir = root.join("illustrations");
    let illustration_set = IllustrationSet::from_dir(&illustrations_dir);
    let classifier = Classifier::default();

    let mut missing_images = Vec::new();
    let mut has_unsupported_webp = false;
    let mut non_utf8_files = Vec::new();
    let mut exceeding_threshold_files = Vec::new();

    for chapter in &discovered_chapters {
        // Check raw encoding UTF-8
        if let Ok(raw_bytes) = std::fs::read(&chapter.path) {
            let without_bom = if raw_bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                &raw_bytes[3..]
            } else {
                &raw_bytes[..]
            };
            if std::str::from_utf8(without_bom).is_err() {
                non_utf8_files.push(chapter.file_name.clone());
            }
        }

        // Check line count against chunk threshold
        if let Ok(content) = read_raw_file(&chapter.path) {
            let line_count = content.lines().count();
            if line_count > env.chunk_threshold_lines {
                exceeding_threshold_files.push((chapter.file_name.clone(), line_count));
            }

            // Scan lines for images
            for line in content.lines() {
                let kind = classifier.classify(line, &illustration_set);
                match kind {
                    RawLineKind::ImageMissing { placeholder: _ } => {
                        missing_images.push(line.trim().to_string());
                    }
                    RawLineKind::Image { file } if file.to_lowercase().ends_with(".webp") => {
                        has_unsupported_webp = true;
                    }
                    _ => {}
                }
            }
        }
    }

    // 12. Missing illustrations
    if !missing_images.is_empty() {
        let mut params = HashMap::new();
        params.insert(
            "missing_count".to_string(),
            missing_images.len().to_string(),
        );

        items.push(PreflightItem {
            id: "missing_illustrations".to_string(),
            status: PreflightStatus::Warning,
            message_code: "preflight.missing_illustrations".to_string(),
            params,
            actions: vec![PreflightAction {
                id: "view_missing_images".to_string(),
                label_code: "actions.view_missing_images".to_string(),
            }],
        });
    } else {
        items.push(PreflightItem::pass(
            "missing_illustrations",
            "preflight.missing_illustrations_ok",
        ));
    }

    // 13. Unsupported image formats
    if has_unsupported_webp {
        items.push(PreflightItem {
            id: "unsupported_image_formats".to_string(),
            status: PreflightStatus::Warning,
            message_code: "preflight.unsupported_image_formats".to_string(),
            params: HashMap::new(),
            actions: Vec::new(),
        });
    } else {
        items.push(PreflightItem::pass(
            "unsupported_image_formats",
            "preflight.unsupported_image_formats_ok",
        ));
    }

    // 14. Raw encoding UTF-8
    if !non_utf8_files.is_empty() {
        let mut params = HashMap::new();
        params.insert("files".to_string(), non_utf8_files.join(", "));

        items.push(PreflightItem {
            id: "raw_encoding_utf8".to_string(),
            status: PreflightStatus::Warning,
            message_code: "preflight.raw_encoding_non_utf8".to_string(),
            params,
            actions: Vec::new(),
        });
    } else {
        items.push(PreflightItem::pass(
            "raw_encoding_utf8",
            "preflight.raw_encoding_utf8_ok",
        ));
    }

    // 15. Chapter exceeds chunk threshold
    if !exceeding_threshold_files.is_empty() {
        let first = &exceeding_threshold_files[0];
        let mut params = HashMap::new();
        params.insert("file".to_string(), first.0.clone());
        params.insert("line_count".to_string(), first.1.to_string());
        params.insert(
            "threshold".to_string(),
            env.chunk_threshold_lines.to_string(),
        );

        items.push(PreflightItem {
            id: "chapters_exceed_chunk_threshold".to_string(),
            status: PreflightStatus::Info,
            message_code: "preflight.chapter_exceeds_threshold".to_string(),
            params,
            actions: vec![PreflightAction {
                id: "view_split_plan".to_string(),
                label_code: "actions.view_split_plan".to_string(),
            }],
        });
    } else {
        items.push(PreflightItem::pass(
            "chapters_exceed_chunk_threshold",
            "preflight.chapters_exceed_chunk_threshold_ok",
        ));
    }

    // 16. Model max output tokens
    if env.skip_model_check {
        items.push(PreflightItem::pass(
            "model_max_output_tokens",
            "preflight.model_max_output_tokens_ok",
        ));
    } else if env.max_output_tokens.is_none() {
        items.push(PreflightItem {
            id: "model_max_output_tokens".to_string(),
            status: PreflightStatus::Warning,
            message_code: "preflight.model_max_output_unknown".to_string(),
            params: HashMap::new(),
            actions: vec![PreflightAction {
                id: "open_model_settings".to_string(),
                label_code: "actions.open_model_settings".to_string(),
            }],
        });
    } else {
        items.push(PreflightItem::pass(
            "model_max_output_tokens",
            "preflight.model_max_output_tokens_ok",
        ));
    }

    PreflightReport::new(items)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::docx::template::create_minimal_template;
    use std::fs;

    fn create_temp_project(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("lnta_pf_{}_{}", name, uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_instruction_missing_blocks() {
        let dir = create_temp_project("inst_missing");
        let env = PreflightEnvironment::default();

        let report = run_preflight(&dir, "ja-id", &env);
        let item = report.get_item("instruction_file").unwrap();
        assert_eq!(item.status, PreflightStatus::Blocker);
        assert_eq!(item.message_code, "preflight.instruction_missing");
        assert_eq!(item.actions[0].id, "choose_instruction_file");
        // Ensure no bundled instruction copy action exists (FR-PRJ-09 removed)
        assert!(!item.actions.iter().any(|a| a.id.contains("bundled")));
        assert!(!report.can_run);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_instruction_multiple_candidates_blocks() {
        let dir = create_temp_project("inst_mult");
        fs::write(dir.join("instruction-1.md"), "content").unwrap();
        fs::write(dir.join("instruction-2.md"), "content").unwrap();
        let env = PreflightEnvironment::default();

        let report = run_preflight(&dir, "ja-id", &env);
        let item = report.get_item("instruction_candidates").unwrap();
        assert_eq!(item.status, PreflightStatus::Blocker);
        assert_eq!(
            item.message_code,
            "preflight.instruction_multiple_candidates"
        );
        assert_eq!(item.actions[0].id, "select_instruction_candidate");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_ranked_instruction_candidates_do_not_block() {
        let dir = create_temp_project("inst_ranked");
        fs::write(dir.join("instruction-en.md"), "content").unwrap();
        fs::write(dir.join("instruction.md"), "content").unwrap();
        let env = PreflightEnvironment::default();

        let report = run_preflight(&dir, "ja-en", &env);
        let item = report.get_item("instruction_candidates").unwrap();
        assert_eq!(item.status, PreflightStatus::Pass);
        assert_eq!(
            report.get_item("instruction_file").unwrap().status,
            PreflightStatus::Pass
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_configured_instruction_settles_a_tie() {
        let dir = create_temp_project("inst_configured");
        fs::write(dir.join("instruction-1.md"), "content").unwrap();
        fs::write(dir.join("instruction-2.md"), "content").unwrap();
        let env = PreflightEnvironment {
            instruction_file: Some("instruction-2.md".to_string()),
            ..Default::default()
        };

        let report = run_preflight(&dir, "ja-id", &env);
        assert_eq!(
            report.get_item("instruction_candidates").unwrap().status,
            PreflightStatus::Pass
        );
        assert_eq!(
            report.get_item("instruction_file").unwrap().status,
            PreflightStatus::Pass
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_instruction_language_mismatch_warning() {
        let dir = create_temp_project("inst_lang");
        // ja-en instruction in ja-id profile
        fs::write(
            dir.join("instruction.md"),
            "# Japanese to English LN instructions\nThe following guidelines apply for translating novels that is written in Japanese.",
        )
        .unwrap();
        let env = PreflightEnvironment::default();

        let report = run_preflight(&dir, "ja-id", &env);
        let item = report.get_item("instruction_language_match").unwrap();
        assert_eq!(item.status, PreflightStatus::Warning);
        assert_eq!(item.message_code, "preflight.instruction_language_mismatch");
        assert_eq!(item.actions[0].id, "switch_profile");
        assert_eq!(item.actions[1].id, "dismiss");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_no_chapters_blocks() {
        let dir = create_temp_project("no_ch");
        fs::write(dir.join("instruction.md"), "valid instruction").unwrap();
        let env = PreflightEnvironment::default();

        let report = run_preflight(&dir, "ja-id", &env);
        let item = report.get_item("chapter_files_detected").unwrap();
        assert_eq!(item.status, PreflightStatus::Blocker);
        assert_eq!(item.message_code, "preflight.no_chapters");
        assert_eq!(item.actions[0].id, "open_other_folder");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_duplicate_prefix_blocks() {
        let dir = create_temp_project("dup_pref");
        fs::write(dir.join("instruction.md"), "valid instruction").unwrap();
        fs::write(dir.join("01_ch1.md"), "text").unwrap();
        fs::write(dir.join("1_ch1_alt.md"), "text").unwrap();
        let env = PreflightEnvironment::default();

        let report = run_preflight(&dir, "ja-id", &env);
        let item = report.get_item("chapter_prefixes_unique").unwrap();
        assert_eq!(item.status, PreflightStatus::Blocker);
        assert_eq!(item.message_code, "preflight.duplicate_prefix");
        assert!(item
            .params
            .get("conflicting_files")
            .unwrap()
            .contains("01_ch1.md"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_translator_model_missing_blocks() {
        let dir = create_temp_project("no_model");
        let env = PreflightEnvironment {
            translator_model_configured: false,
            api_key_or_cli_available: false,
            ..Default::default()
        };

        let report = run_preflight(&dir, "ja-id", &env);
        let item = report.get_item("translator_model_configured").unwrap();
        assert_eq!(item.status, PreflightStatus::Blocker);
        assert_eq!(item.message_code, "preflight.translator_model_missing");
        assert_eq!(item.actions[0].id, "open_provider_settings");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_docx_template_missing_and_invalid_blocks() {
        let dir = create_temp_project("tmpl_test");
        let env = PreflightEnvironment::default();

        // 1. Missing template
        let report = run_preflight(&dir, "ja-id", &env);
        let item = report.get_item("docx_template_valid").unwrap();
        assert_eq!(item.status, PreflightStatus::Blocker);
        assert_eq!(item.message_code, "preflight.template_missing");
        assert_eq!(item.actions[0].id, "choose_template_file");
        assert_eq!(item.actions[1].id, "create_default_template");

        // 2. Corrupt / non-zip template
        fs::write(dir.join("Translations.docx"), b"not a zip file").unwrap();
        let report2 = run_preflight(&dir, "ja-id", &env);
        let item2 = report2.get_item("docx_template_valid").unwrap();
        assert_eq!(item2.status, PreflightStatus::Blocker);
        assert_eq!(item2.message_code, "preflight.template_invalid");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_docx_template_contains_text_warning() {
        let dir = create_temp_project("tmpl_content");
        let env = PreflightEnvironment::default();

        let mut pkg = create_minimal_template("Heading1");
        let doc_with_text = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Copyright 2026</w:t></w:r></w:p><w:sectPr/></w:body></w:document>"#;
        pkg.set_text_entry("word/document.xml", doc_with_text);
        fs::write(dir.join("Translations.docx"), pkg.to_bytes().unwrap()).unwrap();

        let report = run_preflight(&dir, "ja-id", &env);
        let item = report.get_item("docx_template_contains_text").unwrap();
        assert_eq!(item.status, PreflightStatus::Warning);
        assert_eq!(item.message_code, "preflight.template_has_text");
        assert_eq!(item.actions[0].id, "choose_template_treatment");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_glossary_missing_warning() {
        let dir = create_temp_project("glos_miss");
        let env = PreflightEnvironment::default();

        let report = run_preflight(&dir, "ja-id", &env);
        let item = report.get_item("glossary_file_exists").unwrap();
        assert_eq!(item.status, PreflightStatus::Warning);
        assert_eq!(item.message_code, "preflight.glossary_missing");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_glossary_macrons_and_conflicts_warnings() {
        let dir = create_temp_project("glos_warn");
        fs::write(
            dir.join("glossary.md"),
            "佐藤 === Satō === laki-laki\n田中 === Tanaka === laki-laki\n田中 === Tanakao === laki-laki\n",
        )
        .unwrap();
        let env = PreflightEnvironment::default();

        let report = run_preflight(&dir, "ja-id", &env);
        let item_mac = report.get_item("glossary_macrons").unwrap();
        assert_eq!(item_mac.status, PreflightStatus::Warning);
        assert_eq!(item_mac.message_code, "preflight.glossary_has_macrons");
        assert_eq!(item_mac.actions[0].id, "fix_macrons");

        let item_conf = report.get_item("glossary_conflicts").unwrap();
        assert_eq!(item_conf.status, PreflightStatus::Warning);
        assert_eq!(item_conf.message_code, "preflight.glossary_has_conflicts");
        assert_eq!(item_conf.actions[0].id, "open_glossary_tab");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_missing_and_webp_illustrations() {
        let dir = create_temp_project("ill_test");
        fs::create_dir_all(dir.join("illustrations")).unwrap();
        fs::write(dir.join("illustrations").join("color01.webp"), b"fake webp").unwrap();

        fs::write(
            dir.join("01_ch.md"),
            "Text line\nImage PlaceHolder: color01.webp\nImage PlaceHolder: missing.jpg\n",
        )
        .unwrap();
        let env = PreflightEnvironment::default();

        let report = run_preflight(&dir, "ja-id", &env);
        let item_miss = report.get_item("missing_illustrations").unwrap();
        assert_eq!(item_miss.status, PreflightStatus::Warning);
        assert_eq!(item_miss.message_code, "preflight.missing_illustrations");
        assert_eq!(item_miss.params.get("missing_count").unwrap(), "1");

        let item_webp = report.get_item("unsupported_image_formats").unwrap();
        assert_eq!(item_webp.status, PreflightStatus::Warning);
        assert_eq!(
            item_webp.message_code,
            "preflight.unsupported_image_formats"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_raw_encoding_non_utf8_warning() {
        let dir = create_temp_project("sjis_test");
        let text = "健太と美咲\n第一話\n";
        let (sjis_bytes, _, _) = encoding_rs::SHIFT_JIS.encode(text);
        fs::write(dir.join("01_sjis.md"), &sjis_bytes).unwrap();
        let env = PreflightEnvironment::default();

        let report = run_preflight(&dir, "ja-id", &env);
        let item = report.get_item("raw_encoding_utf8").unwrap();
        assert_eq!(item.status, PreflightStatus::Warning);
        assert_eq!(item.message_code, "preflight.raw_encoding_non_utf8");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_chapter_exceeds_chunk_threshold_info() {
        let dir = create_temp_project("thresh_test");
        let mut content = String::new();
        for i in 1..=100 {
            content.push_str(&format!("Line {}\n", i));
        }
        fs::write(dir.join("01_long.md"), content).unwrap();

        // Threshold set to 50 lines
        let env = PreflightEnvironment {
            chunk_threshold_lines: 50,
            ..Default::default()
        };

        let report = run_preflight(&dir, "ja-id", &env);
        let item = report.get_item("chapters_exceed_chunk_threshold").unwrap();
        assert_eq!(item.status, PreflightStatus::Info);
        assert_eq!(item.message_code, "preflight.chapter_exceeds_threshold");
        assert_eq!(item.actions[0].id, "view_split_plan");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_model_max_output_tokens_unknown_warning() {
        let dir = create_temp_project("max_tok");
        let env = PreflightEnvironment {
            max_output_tokens: None,
            ..Default::default()
        };

        let report = run_preflight(&dir, "ja-id", &env);
        let item = report.get_item("model_max_output_tokens").unwrap();
        assert_eq!(item.status, PreflightStatus::Warning);
        assert_eq!(item.message_code, "preflight.model_max_output_unknown");
        assert_eq!(item.actions[0].id, "open_model_settings");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_perfect_project_all_pass_and_can_run() {
        let dir = create_temp_project("perfect");
        fs::write(
            dir.join("instruction.md"),
            "# JP → ID Pedoman Penerjemahan\nPetunjuk ini digunakan untuk menerjemahkan novel dari bahasa Jepang ke bahasa Indonesia yang baik.",
        )
        .unwrap();
        fs::write(dir.join("01_bab.md"), "Bab 1: Cerita dimulai.\n").unwrap();
        fs::write(dir.join("glossary.md"), "健太 === Kenta === laki-laki\n").unwrap();

        let pkg = create_minimal_template("Heading1");
        fs::write(dir.join("Translations.docx"), pkg.to_bytes().unwrap()).unwrap();

        let env = PreflightEnvironment::default();
        let report = run_preflight(&dir, "ja-id", &env);

        assert!(report.can_run);
        assert_eq!(report.blockers().len(), 0);
        assert_eq!(report.warnings().len(), 0);
        assert_eq!(report.passes().len(), 16);

        let _ = fs::remove_dir_all(&dir);
    }
}
