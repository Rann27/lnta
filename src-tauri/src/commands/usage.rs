//! Tauri commands for usage, token accounting, and costs.

use std::path::Path;

use crate::error::AppError;
use crate::project::layout::ProjectLayout;
use crate::provider::registry::ModelRegistry;
use crate::store::open_path;
use crate::store::repo::{CallRepo, ChapterRepo};
use crate::usage::{compute_project_usage, ProjectUsageResponse, UsageSummary};

/// Returns aggregated usage statistics for a project, optionally filtered by run ID.
#[tauri::command]
pub fn get_project_usage(
    path: String,
    run_id: Option<String>,
) -> Result<ProjectUsageResponse, AppError> {
    let layout = ProjectLayout::new(Path::new(&path));
    if !layout.db_file.exists() {
        return Ok(ProjectUsageResponse {
            summary: UsageSummary {
                total_input_tokens: 0,
                total_cached_tokens: 0,
                total_output_tokens: 0,
                total_reasoning_tokens: 0,
                total_cost: None,
                has_unknown_cost: false,
                total_calls: 0,
            },
            by_chapter: Vec::new(),
            by_role: Vec::new(),
            by_model: Vec::new(),
        });
    }

    let conn = open_path(&layout.db_file)?;
    let call_repo = CallRepo::new(&conn);
    let chap_repo = ChapterRepo::new(&conn);

    let calls = if let Some(ref rid) = run_id {
        call_repo.list_by_run(rid)?
    } else {
        call_repo.list_all()?
    };

    let chapters = chap_repo.list_all()?;
    let registry = ModelRegistry::default();

    Ok(compute_project_usage(&calls, &chapters, &registry))
}
