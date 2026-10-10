//! LNTA core library.

pub mod commands;
pub mod docx;
pub mod engine;
pub mod error;
pub mod glossary;
pub mod plan;
pub mod profiles;
pub mod project;
pub mod prompt;
pub mod provider;
pub mod qa;
pub mod raw;
pub mod secrets;
pub mod store;
pub mod usage;
pub mod validate;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(e) = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_info,
            commands::project::open_project,
            commands::project::list_recent_projects,
            commands::project::reset_project,
            commands::project::create_project,
            commands::preflight::run_preflight,
            commands::settings::get_global_settings,
            commands::settings::save_global_settings,
            commands::settings::get_project_config,
            commands::settings::save_project_config,
            commands::providers::list_providers,
            commands::providers::save_provider,
            commands::providers::delete_provider,
            commands::providers::set_api_key,
            commands::providers::has_api_key,
            commands::providers::test_provider,
            commands::models::list_models,
            commands::models::register_model,
            commands::models::fetch_provider_models,
            commands::run::start_run,
            commands::run::pause_run,
            commands::run::resume_run,
            commands::run::stop_run,
            commands::run::get_run_state,
            commands::chapters::list_chapters,
            commands::chapters::get_chapter,
            commands::chapters::list_segments,
            commands::chapters::list_translations,
            commands::glossary::list_glossary,
            commands::glossary::add_glossary_entry,
            commands::glossary::update_glossary_entry,
            commands::glossary::delete_glossary_entry,
            commands::qa::get_qa_summary,
            commands::qa::get_qa_fix_log,
            commands::qa::undo_qa_fix,
            commands::qa::undo_qa_batch,
            commands::qa::undo_qa_run,
        ])
        .run(tauri::generate_context!())
    {
        eprintln!("error while running tauri application: {e}");
    }
}
