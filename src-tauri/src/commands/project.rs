//! Project commands per FR-PRJ-01..06, Story A1, A2.

use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;
use crate::project::config::ProjectConfig;
use crate::project::discover::discover_chapters;
use crate::project::layout::ProjectLayout;
use crate::project::preflight::{run_preflight_on_folder, PreflightReport};
use crate::project::settings::GlobalSettings;
use crate::raw::encoding::read_raw_file;
use crate::store::repo::{Chapter, ChapterRepo, Run, RunRepo};
use crate::store::{migrate, open_path};

/// Result returned when a project folder is opened.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/ProjectOpenResult.ts")]
pub struct ProjectOpenResult {
    pub path: String,
    pub config: ProjectConfig,
    pub chapters: Vec<Chapter>,
    pub preflight: PreflightReport,
    pub latest_run: Option<Run>,
}

/// An entry in the recent projects list with progress stats per FR-PRJ-06.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/RecentProjectEntry.ts")]
pub struct RecentProjectEntry {
    pub path: String,
    pub name: String,
    pub profile: String,
    pub chapters_done: usize,
    pub chapters_total: usize,
    pub last_opened: String,
}

#[tauri::command]
pub fn open_project(path: String) -> Result<ProjectOpenResult, AppError> {
    let p = Path::new(&path);
    if !p.exists() || !p.is_dir() {
        return Err(AppError::FileNotFound {
            path: format!("Project directory not found: {path}"),
        });
    }

    let layout = ProjectLayout::new(p);
    let config = if layout.config_file.exists() {
        ProjectConfig::load_from_file(&layout.config_file)?
    } else {
        ProjectConfig::for_profile("ja-id")
    };

    layout.ensure_layout(&config.profile)?;

    let mut conn = open_path(&layout.db_file)?;
    migrate(&mut conn)?;

    let chapter_repo = ChapterRepo::new(&conn);
    let existing_chapters = chapter_repo.list_all()?;

    // Same parser and options as the run engine, so titles, image-page chapters, and segment
    // counts shown before a run match what the run will actually do.
    let illustrations = crate::raw::classify::IllustrationSet::from_dir(&layout.illustrations_dir);
    let parse_opts = crate::raw::segment::ParseOptions {
        strip_brackets: config.strip_title_brackets,
        ruby_normalize: config.normalize_ruby,
        ..Default::default()
    };

    if existing_chapters.is_empty() {
        if let Ok(discovered) = discover_chapters(p) {
            for disc in discovered {
                let content = read_raw_file(&disc.path).unwrap_or_default();
                let parsed = crate::raw::segment::parse_chapter(
                    &disc.file_name,
                    &content,
                    &illustrations,
                    &parse_opts,
                );

                let ch = Chapter {
                    id: format!("chap_{}", disc.prefix),
                    ordinal: disc.ordinal,
                    file_name: disc.file_name.clone(),
                    prefix: disc.prefix.clone(),
                    title_src: parsed
                        .title_clean
                        .clone()
                        .or(parsed.title_raw.clone())
                        .unwrap_or_default(),
                    title_tgt: None,
                    src_hash: format!("{:x}", md5_or_simple(&content)),
                    line_count: parsed.line_count,
                    char_count: parsed.char_count,
                    status: "pending".to_string(),
                    frozen: false,
                    parts_planned: 0,
                    updated_at: chrono::Utc::now().to_rfc3339(),
                };
                if chapter_repo.insert(&ch).is_ok() {
                    crate::engine::chapter::persist_chapter_segments(
                        &conn,
                        &ch.id,
                        &parsed.segments,
                    )?;
                }
            }
        }
    }

    // Projects opened before segments were persisted have empty segment tables; fill them in.
    let segment_repo = crate::store::repo::SegmentRepo::new(&conn);
    for ch in chapter_repo.list_all()? {
        if segment_repo.list_by_chapter(&ch.id)?.is_empty() {
            if let Ok(content) = read_raw_file(&layout.root.join(&ch.file_name)) {
                let parsed = crate::raw::segment::parse_chapter(
                    &ch.file_name,
                    &content,
                    &illustrations,
                    &parse_opts,
                );
                crate::engine::chapter::persist_chapter_segments(&conn, &ch.id, &parsed.segments)?;
            }
        }
    }

    let chapters = chapter_repo.list_all()?;
    let latest_run = RunRepo::new(&conn).get_latest()?;

    let global_settings = GlobalSettings::load_from_app_data().unwrap_or_default();
    let preflight = run_preflight_on_folder(p, &global_settings);

    // Update recent projects list
    if let Ok(mut settings) = GlobalSettings::load_from_app_data() {
        settings.recent_projects.retain(|item| item != &path);
        settings.recent_projects.insert(0, path.clone());
        if settings.recent_projects.len() > 10 {
            settings.recent_projects.truncate(10);
        }
        let _ = settings.save_to_app_data();
    }

    Ok(ProjectOpenResult {
        path,
        config,
        chapters,
        preflight,
        latest_run,
    })
}

#[tauri::command]
pub fn list_recent_projects() -> Result<Vec<RecentProjectEntry>, AppError> {
    let settings = GlobalSettings::load_from_app_data().unwrap_or_default();
    let mut results = Vec::new();

    for proj_path in settings.recent_projects {
        let p = Path::new(&proj_path);
        if !p.exists() {
            continue;
        }

        let layout = ProjectLayout::new(p);
        // A folder whose .lnta was reset is no longer a project; opening it starts the wizard.
        if !layout.config_file.exists() {
            continue;
        }
        let name = p
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| proj_path.clone());

        let config = ProjectConfig::load_from_file(&layout.config_file).unwrap_or_default();

        let mut chapters_done = 0usize;
        let mut chapters_total = 0usize;
        // Latest run start when there is one, otherwise the last write to the project files.
        let mut last_opened = [&layout.db_file, &layout.config_file]
            .iter()
            .filter_map(|f| std::fs::metadata(f).and_then(|m| m.modified()).ok())
            .max()
            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
            .unwrap_or_default();

        if layout.db_file.exists() {
            if let Ok(conn) = open_path(&layout.db_file) {
                if let Ok(chaps) = ChapterRepo::new(&conn).list_all() {
                    chapters_total = chaps.len();
                    chapters_done = chaps.iter().filter(|c| c.status == "done").count();
                }
                if let Ok(Some(run)) = RunRepo::new(&conn).get_latest() {
                    last_opened = run.started_at;
                }
            }
        }

        results.push(RecentProjectEntry {
            path: proj_path,
            name,
            profile: config.profile,
            chapters_done,
            chapters_total,
            last_opened,
        });
    }

    Ok(results)
}

#[tauri::command]
pub fn reset_project(path: String) -> Result<(), AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    layout.reset_project()?;
    if let Ok(mut settings) = GlobalSettings::load_from_app_data() {
        let before = settings.recent_projects.len();
        settings.recent_projects.retain(|item| item != &path);
        if settings.recent_projects.len() != before {
            settings.save_to_app_data()?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn create_project(path: String, profile: String) -> Result<ProjectOpenResult, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    layout.ensure_layout(&profile)?;

    let config = ProjectConfig::for_profile(&profile);
    config.save_to_file(&layout.config_file)?;

    open_project(path)
}

#[tauri::command]
pub fn propose_frozen_chapters(
    project_path: String,
) -> Result<Vec<crate::docx::import::FrozenChapterMapping>, AppError> {
    let p = Path::new(&project_path);
    let layout = ProjectLayout::new(p);

    let docx_path = if layout.root.join("Translations.docx").exists() {
        layout.root.join("Translations.docx")
    } else if layout.template_file.exists() {
        layout.template_file.clone()
    } else {
        return Ok(Vec::new());
    };

    let bytes = std::fs::read(&docx_path).map_err(|e| AppError::FileNotFound {
        path: format!("Failed to read docx for frozen chapter detection: {e}"),
    })?;

    let docx_headings = crate::docx::import::extract_headings_from_docx_bytes(&bytes)?;
    let discovered = discover_chapters(p)?;

    Ok(crate::docx::import::propose_frozen_chapter_mappings(
        &docx_headings,
        &discovered,
    ))
}

#[tauri::command]
pub fn confirm_frozen_chapters(
    project_path: String,
    mappings: Vec<crate::docx::import::FrozenChapterMapping>,
) -> Result<(), AppError> {
    let p = Path::new(&project_path);
    let layout = ProjectLayout::new(p);

    let docx_path = if layout.root.join("Translations.docx").exists() {
        layout.root.join("Translations.docx")
    } else if layout.template_file.exists() {
        layout.template_file.clone()
    } else {
        return Ok(());
    };

    let bytes = std::fs::read(&docx_path).map_err(|e| AppError::FileNotFound {
        path: format!("Failed to read docx: {e}"),
    })?;
    let docx_headings = crate::docx::import::extract_headings_from_docx_bytes(&bytes)?;

    let conn = open_path(&layout.db_file)?;
    crate::docx::import::apply_frozen_mappings_to_db(&conn, &mappings, &docx_headings)?;

    if layout.config_file.exists() {
        let mut cfg = ProjectConfig::load_from_file(&layout.config_file)?;
        cfg.template_treatment = "frozen_prefix".to_string();
        cfg.save_to_file(&layout.config_file)?;
    }

    Ok(())
}

/// Preview of a single planned part within a chapter.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/PlannedPartPreview.ts")]
pub struct PlannedPartPreview {
    pub part_index: i32,
    pub total_parts: i32,
    pub line_start: i32,
    pub line_end: i32,
    #[ts(type = "number")]
    pub segment_count: usize,
    #[ts(type = "number")]
    pub estimated_input_tokens: u32,
    #[ts(type = "number")]
    pub estimated_output_tokens: u32,
}

/// Preview of a chapter's chunk plan and token/cost estimates.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/ChapterPlanPreview.ts")]
pub struct ChapterPlanPreview {
    pub chapter_id: String,
    pub file_name: String,
    pub title: String,
    #[ts(type = "number")]
    pub line_count: usize,
    #[ts(type = "number")]
    pub char_count: usize,
    #[ts(type = "number")]
    pub part_count: usize,
    #[ts(type = "number")]
    pub estimated_input_tokens: u32,
    #[ts(type = "number")]
    pub estimated_output_tokens: u32,
    pub estimated_cost: Option<f64>,
    pub status: String,
    pub parts: Vec<PlannedPartPreview>,
}

/// Full project plan preview and volume-wide token and cost estimates.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/ProjectPlanPreview.ts")]
pub struct ProjectPlanPreview {
    pub model_id: String,
    pub model_name: String,
    pub provider: String,
    #[ts(type = "number")]
    pub total_chapters: usize,
    #[ts(type = "number")]
    pub total_parts: usize,
    #[ts(type = "number")]
    pub total_input_tokens: u32,
    #[ts(type = "number")]
    pub total_output_tokens: u32,
    pub estimated_total_cost: Option<f64>,
    pub has_cost_estimate: bool,
    pub is_learned_ratio: bool,
    #[ts(type = "number")]
    pub completed_chapters_count: usize,
    pub chars_per_input_token: f64,
    pub chars_per_output_token: f64,
    pub chapters: Vec<ChapterPlanPreview>,
}

#[tauri::command]
pub fn get_plan_preview(
    path: String,
    model: Option<crate::project::settings::ModelReference>,
) -> Result<ProjectPlanPreview, AppError> {
    let p = Path::new(&path);
    if !p.exists() || !p.is_dir() {
        return Err(AppError::FileNotFound {
            path: format!("Project directory not found: {path}"),
        });
    }

    let layout = ProjectLayout::new(p);
    let config = if layout.config_file.exists() {
        ProjectConfig::load_from_file(&layout.config_file)?
    } else {
        ProjectConfig::for_profile("ja-id")
    };

    let global = GlobalSettings::load_from_app_data().unwrap_or_default();
    let settings = crate::project::settings::EffectiveSettings::merge(&global, &config, None);

    let conn = open_path(&layout.db_file)?;
    let registry = crate::provider::registry::ModelRegistry::default();

    let target_ref = model.unwrap_or_else(|| settings.roles.translator.primary.clone());
    let model_entry = registry.get(&target_ref.provider, &target_ref.model_id);
    let model_name = model_entry
        .map(|m| m.display_name.clone())
        .unwrap_or_else(|| target_ref.model_id.clone());

    let role_cfg = settings
        .roles
        .for_role(crate::project::settings::Role::Translator);
    let max_output = role_cfg
        .max_output_tokens
        .or_else(|| model_entry.map(|m| m.max_output_tokens))
        .unwrap_or(4096);
    let context_window = model_entry.map(|m| m.context_window).unwrap_or(128_000);

    let limits = crate::plan::ModelLimits {
        max_output_tokens: max_output,
        context_window,
    };

    // Calculate learned weights if completed chapters exist in DB
    let call_repo = crate::store::repo::CallRepo::new(&conn);
    let learned_stats = call_repo.get_learned_ratio_stats()?;
    let (weights, is_learned, completed_chapters_count) = if let Some(stats) = learned_stats {
        if let Some(w) = crate::plan::compute_learned_ratios(
            stats.completed_chapters_count,
            stats.total_completed_chars,
            stats.total_translator_in_tokens,
            stats.total_translator_out_tokens,
            stats.total_translator_calls,
        ) {
            (w, true, stats.completed_chapters_count)
        } else {
            (crate::plan::EstimationWeights::default(), false, 0)
        }
    } else {
        (crate::plan::EstimationWeights::default(), false, 0)
    };

    let chapter_repo = ChapterRepo::new(&conn);
    let db_chapters = chapter_repo.list_all()?;

    let chapters_to_plan: Vec<(String, String, String, String, bool)> = if !db_chapters.is_empty() {
        db_chapters
            .into_iter()
            .map(|ch| (ch.id, ch.file_name, ch.title_src, ch.status, ch.frozen))
            .collect()
    } else {
        let discovered = discover_chapters(&layout.root)?;
        discovered
            .into_iter()
            .map(|ch| {
                (
                    ch.prefix.clone(),
                    ch.file_name,
                    ch.title_candidate,
                    "not_started".to_string(),
                    false,
                )
            })
            .collect()
    };

    let illustrations = crate::raw::IllustrationSet::from_dir(&layout.illustrations_dir);
    let parse_opts = crate::raw::segment::ParseOptions {
        strip_brackets: settings.strip_title_brackets,
        ruby_normalize: settings.normalize_ruby,
        ..Default::default()
    };

    let mut chapter_previews = Vec::new();
    let mut total_in = 0u32;
    let mut total_out = 0u32;
    let mut total_parts = 0usize;
    let mut total_cost: Option<f64> = None;

    let has_pricing = if let Some(m) = model_entry {
        m.pricing.input_per_million.is_some() || m.pricing.output_per_million.is_some()
    } else {
        false
    };
    if has_pricing {
        total_cost = Some(0.0);
    }

    for (ch_id, file_name, title_src, status, frozen) in chapters_to_plan {
        let chapter_path = layout.root.join(&file_name);
        if !chapter_path.exists() {
            continue;
        }

        let content = read_raw_file(&chapter_path)?;
        let parsed =
            crate::raw::segment::parse_chapter(&file_name, &content, &illustrations, &parse_opts);

        if frozen {
            chapter_previews.push(ChapterPlanPreview {
                chapter_id: ch_id,
                file_name,
                title: parsed.title_clean.unwrap_or(title_src),
                line_count: parsed.line_count as usize,
                char_count: parsed.char_count as usize,
                part_count: 0,
                estimated_input_tokens: 0,
                estimated_output_tokens: 0,
                estimated_cost: None,
                status: "frozen".to_string(),
                parts: Vec::new(),
            });
            continue;
        }

        if parsed.is_image_page {
            chapter_previews.push(ChapterPlanPreview {
                chapter_id: ch_id,
                file_name,
                title: title_src,
                line_count: parsed.line_count as usize,
                char_count: parsed.char_count as usize,
                part_count: 0,
                estimated_input_tokens: 0,
                estimated_output_tokens: 0,
                estimated_cost: None,
                status: "done".to_string(),
                parts: Vec::new(),
            });
            continue;
        }

        let plan = crate::plan::plan_chapter_chunks(
            &file_name,
            &parsed.segments,
            parsed.line_count as usize,
            settings.chunk_threshold_lines,
            &limits,
            &weights,
        );

        let mut ch_in = 0u32;
        let mut ch_out = 0u32;
        let mut parts_preview = Vec::new();

        for p in &plan.parts {
            ch_in += p.estimated_input_tokens;
            ch_out += p.estimated_output_tokens;
            parts_preview.push(PlannedPartPreview {
                part_index: p.part_index,
                total_parts: p.total_parts,
                line_start: p.line_start,
                line_end: p.line_end,
                segment_count: p.segment_count,
                estimated_input_tokens: p.estimated_input_tokens,
                estimated_output_tokens: p.estimated_output_tokens,
            });
        }

        let ch_cost = if let Some(m) = model_entry {
            let in_p = m.pricing.input_per_million.unwrap_or(0.0);
            let out_p = m.pricing.output_per_million.unwrap_or(0.0);
            if in_p > 0.0 || out_p > 0.0 {
                let cost =
                    (ch_in as f64 * in_p / 1_000_000.0) + (ch_out as f64 * out_p / 1_000_000.0);
                Some((cost * 10_000.0).round() / 10_000.0)
            } else {
                None
            }
        } else {
            None
        };

        if let (Some(total), Some(ch_c)) = (total_cost.as_mut(), ch_cost) {
            *total += ch_c;
        }

        total_in += ch_in;
        total_out += ch_out;
        total_parts += plan.parts.len();

        chapter_previews.push(ChapterPlanPreview {
            chapter_id: ch_id,
            file_name,
            title: parsed.title_clean.unwrap_or(title_src),
            line_count: parsed.line_count as usize,
            char_count: parsed.char_count as usize,
            part_count: plan.parts.len(),
            estimated_input_tokens: ch_in,
            estimated_output_tokens: ch_out,
            estimated_cost: ch_cost,
            status,
            parts: parts_preview,
        });
    }

    let estimated_total_cost = total_cost.map(|c| (c * 10_000.0).round() / 10_000.0);
    let has_cost_estimate = estimated_total_cost.is_some();

    Ok(ProjectPlanPreview {
        model_id: target_ref.model_id,
        model_name,
        provider: target_ref.provider,
        total_chapters: chapter_previews.len(),
        total_parts,
        total_input_tokens: total_in,
        total_output_tokens: total_out,
        estimated_total_cost,
        has_cost_estimate,
        is_learned_ratio: is_learned,
        completed_chapters_count,
        chars_per_input_token: weights.chars_per_input_token,
        chars_per_output_token: weights.chars_per_output_token,
        chapters: chapter_previews,
    })
}

fn md5_or_simple(text: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Creates a clean default docx template per FR-DOC-13 on explicit user action.
#[tauri::command]
pub fn create_default_template(path: String) -> Result<(), AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);
    let pkg = crate::docx::generate_default_template("Heading1");

    // Write to root Translations.docx
    let backup_dir = if layout.lnta_dir.exists() {
        Some(layout.backups_dir.as_path())
    } else {
        None
    };
    crate::docx::write_docx_package_atomically(&pkg, &layout.docx_output, backup_dir)?;

    // Also sync to .lnta/template.docx if .lnta layout exists
    if layout.lnta_dir.exists() {
        let _ = crate::docx::write_docx_package_atomically(&pkg, &layout.template_file, None);
    }

    Ok(())
}

/// Instruction files in the project root that fit `profile`, best candidate first (PRD §4.2).
#[tauri::command]
pub fn list_instruction_candidates(path: String, profile: String) -> Result<Vec<String>, AppError> {
    match crate::project::instruction::resolve_instruction_candidates(Path::new(&path), &profile) {
        Ok(paths) => Ok(paths
            .iter()
            .filter_map(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .collect()),
        Err(AppError::InstructionMissing) => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

/// Uses a docx chosen by the user as the project template: it becomes `Translations.docx`
/// (the previous file goes to backups) and the render template in `.lnta/`.
#[tauri::command]
pub fn import_template(path: String, source: String) -> Result<(), AppError> {
    let layout = ProjectLayout::new(Path::new(&path));
    let bytes = std::fs::read(&source).map_err(|_| AppError::FileNotFound {
        path: source.clone(),
    })?;
    let pkg = crate::docx::template::DocxPackage::from_bytes(&bytes)?;
    if pkg.get_text_entry("word/document.xml").is_none() {
        return Err(AppError::InvalidTemplate {
            reason: "word/document.xml is missing".to_string(),
        });
    }

    let same_file =
        std::fs::canonicalize(&source).ok() == std::fs::canonicalize(&layout.docx_output).ok();
    if !same_file {
        let backup_dir = layout
            .lnta_dir
            .exists()
            .then_some(layout.backups_dir.as_path());
        crate::docx::write_docx_package_atomically(&pkg, &layout.docx_output, backup_dir)?;
    }
    if layout.lnta_dir.exists() {
        crate::docx::write_docx_package_atomically(&pkg, &layout.template_file, None)?;
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_template() {
        let temp_dir = std::env::temp_dir().join(format!("lnta_test_tpl_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let project_path = temp_dir.to_str().unwrap().to_string();

        create_default_template(project_path).unwrap();

        let target_docx = temp_dir.join("Translations.docx");
        assert!(target_docx.exists());

        let bytes = std::fs::read(&target_docx).unwrap();
        let pkg = crate::docx::template::DocxPackage::from_bytes(&bytes).unwrap();
        assert!(pkg.files.contains_key("word/document.xml"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_import_template_copies_valid_docx_and_rejects_others() {
        let temp_dir = std::env::temp_dir().join(format!("lnta_test_imp_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(temp_dir.join(".lnta")).unwrap();
        let project_path = temp_dir.to_str().unwrap().to_string();

        let source = temp_dir.join("chosen.docx");
        let bytes = crate::docx::generate_default_template("Heading1")
            .to_bytes()
            .unwrap();
        std::fs::write(&source, bytes).unwrap();
        import_template(project_path.clone(), source.to_string_lossy().to_string()).unwrap();
        assert!(temp_dir.join("Translations.docx").exists());
        assert!(temp_dir.join(".lnta").join("template.docx").exists());

        let bogus = temp_dir.join("bogus.docx");
        std::fs::write(&bogus, b"not a zip").unwrap();
        let err = import_template(project_path, bogus.to_string_lossy().to_string()).unwrap_err();
        assert_eq!(err.code(), "errors.invalid_template");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
