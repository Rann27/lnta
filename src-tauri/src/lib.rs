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
        .run(tauri::generate_context!())
    {
        eprintln!("error while running tauri application: {e}");
    }
}
