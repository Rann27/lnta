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

    if existing_chapters.is_empty() {
        if let Ok(discovered) = discover_chapters(p) {
            for disc in discovered {
                let content = read_raw_file(&disc.path).unwrap_or_default();
                let line_count = content.lines().count() as i32;
                let char_count = content.chars().count() as i32;

                let ch = Chapter {
                    id: format!("chap_{:02}", disc.ordinal),
                    ordinal: disc.ordinal,
                    file_name: disc.file_name.clone(),
                    prefix: disc.prefix.clone(),
                    title_src: disc.title_candidate.clone(),
                    title_tgt: None,
                    src_hash: format!("{:x}", md5_or_simple(&content)),
                    line_count,
                    char_count,
                    status: "pending".to_string(),
                    frozen: false,
                    parts_planned: 0,
                    updated_at: chrono::Utc::now().to_rfc3339(),
                };
                let _ = chapter_repo.insert(&ch);
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
        let name = p
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| proj_path.clone());

        let config = if layout.config_file.exists() {
            ProjectConfig::load_from_file(&layout.config_file).unwrap_or_default()
        } else {
            ProjectConfig::default()
        };

        let mut chapters_done = 0usize;
        let mut chapters_total = 0usize;
        let mut last_opened = chrono::Utc::now().to_rfc3339();

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
    layout.reset_project()
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

fn md5_or_simple(text: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}
