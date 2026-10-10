//! Run control commands and active run manager per FR-RUN-01..10, Story H1.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use tauri::Emitter;
use ts_rs::TS;

use crate::engine::events::{EngineEvent, EventEmitter};
use crate::engine::repair::ProviderResolver;
use crate::engine::run_engine;
use crate::engine::state::RunControl;
use crate::error::AppError;
use crate::project::config::ProjectConfig;
use crate::project::layout::ProjectLayout;
use crate::project::settings::{EffectiveSettings, GlobalSettings, ModelReference};
use crate::provider::anthropic::{AnthropicConfig, AnthropicProvider};
use crate::provider::gemini::{GeminiConfig, GeminiProvider};
use crate::provider::openai_compat::{OpenAiCompatConfig, OpenAiCompatProvider};
use crate::provider::AnyProvider;
use crate::secrets;
use crate::store::repo::{ChapterRepo, Run, RunRepo};
use crate::store::{migrate, open_path};

/// Response containing the current run state and active status.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/RunStateResponse.ts")]
pub struct RunStateResponse {
    pub state: String,
    pub active: bool,
    pub current_chapter_id: Option<String>,
    pub latest_run: Option<Run>,
}

/// Global manager tracking active RunControl instances per project path.
static RUN_MANAGER: OnceLock<RunManager> = OnceLock::new();

pub struct RunManager {
    controls: Mutex<HashMap<String, Arc<RunControl>>>,
}

impl RunManager {
    pub fn global() -> &'static Self {
        RUN_MANAGER.get_or_init(|| Self {
            controls: Mutex::new(HashMap::new()),
        })
    }

    pub fn get(&self, path: &str) -> Option<Arc<RunControl>> {
        self.controls.lock().ok()?.get(path).cloned()
    }

    pub fn insert(&self, path: &str, control: Arc<RunControl>) {
        if let Ok(mut map) = self.controls.lock() {
            map.insert(path.to_string(), control);
        }
    }

    pub fn remove(&self, path: &str) {
        if let Ok(mut map) = self.controls.lock() {
            map.remove(path);
        }
    }
}

/// Dynamic provider resolver using global settings and keyring secrets.
pub struct DynamicProviderResolver {
    providers: Mutex<HashMap<String, AnyProvider>>,
    settings: GlobalSettings,
}

impl DynamicProviderResolver {
    pub fn new(settings: GlobalSettings) -> Self {
        Self {
            providers: Mutex::new(HashMap::new()),
            settings,
        }
    }
}

impl ProviderResolver for DynamicProviderResolver {
    fn resolve(&self, reference: &ModelReference) -> Result<AnyProvider, AppError> {
        let key = format!("{}:{}", reference.provider, reference.model_id);
        if let Ok(map) = self.providers.lock() {
            if let Some(existing) = map.get(&key) {
                return Ok(existing.clone());
            }
        }

        let provider_config = self
            .settings
            .providers
            .iter()
            .find(|p| p.id == reference.provider)
            .cloned();

        let api_key = secrets::get_api_key(&reference.provider)
            .ok()
            .flatten()
            .unwrap_or_default();

        let api_opt = if api_key.is_empty() {
            None
        } else {
            Some(api_key)
        };

        let any_provider = match reference.provider.as_str() {
            "anthropic" => {
                let base_url = provider_config
                    .map(|p| p.base_url)
                    .unwrap_or_else(|| "https://api.anthropic.com".to_string());
                AnyProvider::Anthropic(Arc::new(AnthropicProvider::new(
                    AnthropicConfig {
                        base_url,
                        api_key: api_opt,
                        ..Default::default()
                    },
                    None,
                )))
            }
            "gemini" => {
                let base_url = provider_config
                    .map(|p| p.base_url)
                    .unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string());
                AnyProvider::Gemini(Arc::new(GeminiProvider::new(
                    GeminiConfig {
                        base_url,
                        api_key: api_opt,
                        ..Default::default()
                    },
                    None,
                )))
            }
            "cli_claude" => {
                AnyProvider::CliClaude(Arc::new(crate::provider::CliClaudeProvider::new()))
            }
            "cli_gemini" => {
                AnyProvider::CliGemini(Arc::new(crate::provider::CliGeminiProvider::new()))
            }
            "cli_codex" => {
                AnyProvider::CliCodex(Arc::new(crate::provider::CliCodexProvider::new()))
            }
            "mock" => AnyProvider::Mock(Arc::new(crate::provider::mock::MockProvider::default())),
            _ => {
                // Default to OpenAI-compatible provider
                let base_url = provider_config
                    .map(|p| p.base_url)
                    .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                AnyProvider::OpenAiCompat(Arc::new(OpenAiCompatProvider::new(
                    OpenAiCompatConfig {
                        base_url,
                        api_key: api_opt,
                        ..Default::default()
                    },
                    crate::provider::retry::RetryConfig::default(),
                )))
            }
        };

        if let Ok(mut map) = self.providers.lock() {
            map.insert(key, any_provider.clone());
        }

        Ok(any_provider)
    }
}

/// Bridges EngineEvent into Tauri's event bus.
pub struct TauriEventEmitter {
    pub app: tauri::AppHandle,
}

impl EventEmitter for TauriEventEmitter {
    fn emit(&self, event: EngineEvent) {
        let _ = self.app.emit("lnta:event", &event);
    }
}

#[tauri::command]
pub async fn start_run(app: tauri::AppHandle, path: String) -> Result<String, AppError> {
    let p = Path::new(&path);
    if !p.exists() || !p.is_dir() {
        return Err(AppError::FileNotFound {
            path: format!("Directory not found: {path}"),
        });
    }

    let manager = RunManager::global();
    if let Some(existing) = manager.get(&path) {
        if !existing.should_stop_now() {
            existing.reset_continue();
            return Ok("resumed".to_string());
        }
    }

    let layout = ProjectLayout::new(p);
    let config = if layout.config_file.exists() {
        ProjectConfig::load_from_file(&layout.config_file)?
    } else {
        ProjectConfig::for_profile("ja-id")
    };
    layout.ensure_layout(&config.profile)?;

    let global = GlobalSettings::load_from_app_data().unwrap_or_default();
    let effective = EffectiveSettings::merge(&global, &config, None);

    let run_control = Arc::new(RunControl::new());
    manager.insert(&path, run_control.clone());

    let path_clone = path.clone();
    let run_ctrl_clone = run_control.clone();

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(r) => r,
            Err(_) => {
                RunManager::global().remove(&path_clone);
                return;
            }
        };

        rt.block_on(async move {
            if let Ok(mut conn) = open_path(&layout.db_file) {
                if migrate(&mut conn).is_ok() {
                    let emitter = TauriEventEmitter { app };
                    let resolver = DynamicProviderResolver::new(global);
                    let _ = run_engine(
                        &conn,
                        &layout,
                        &effective,
                        &resolver,
                        &emitter,
                        &run_ctrl_clone,
                    )
                    .await;
                }
            }
            RunManager::global().remove(&path_clone);
        });
    });

    Ok("started".to_string())
}

#[tauri::command]
pub fn pause_run(path: String) -> Result<(), AppError> {
    if let Some(ctrl) = RunManager::global().get(&path) {
        ctrl.request_pause();
    }
    Ok(())
}

#[tauri::command]
pub async fn resume_run(app: tauri::AppHandle, path: String) -> Result<(), AppError> {
    if let Some(ctrl) = RunManager::global().get(&path) {
        ctrl.reset_continue();
        Ok(())
    } else {
        let _ = start_run(app, path).await?;
        Ok(())
    }
}

#[tauri::command]
pub fn stop_run(path: String) -> Result<(), AppError> {
    if let Some(ctrl) = RunManager::global().get(&path) {
        ctrl.request_stop_now();
    }
    RunManager::global().remove(&path);
    Ok(())
}

#[tauri::command]
pub fn get_run_state(path: String) -> Result<RunStateResponse, AppError> {
    let p = Path::new(&path);
    let layout = ProjectLayout::new(p);

    let active_ctrl = RunManager::global().get(&path);
    let is_active = active_ctrl.is_some();

    let mut latest_run = None;
    let mut current_ch = None;
    let mut state = "idle".to_string();

    if layout.db_file.exists() {
        if let Ok(conn) = open_path(&layout.db_file) {
            if let Ok(run) = RunRepo::new(&conn).get_latest() {
                if let Some(ref r) = run {
                    state = r.status.clone();
                }
                latest_run = run;
            }

            if let Ok(chaps) = ChapterRepo::new(&conn).list_all() {
                if let Some(in_progress) = chaps.iter().find(|c| c.status == "in_progress") {
                    current_ch = Some(in_progress.id.clone());
                }
            }
        }
    }

    if let Some(ctrl) = active_ctrl {
        if ctrl.should_pause() {
            state = "paused".to_string();
        } else if !ctrl.should_stop_now() {
            state = "running".to_string();
        }
    }

    Ok(RunStateResponse {
        state,
        active: is_active,
        current_chapter_id: current_ch,
        latest_run,
    })
}

#[tauri::command]
pub async fn retranslate_chapter(
    app: tauri::AppHandle,
    path: String,
    chapter_id: String,
    model: Option<ModelReference>,
) -> Result<String, AppError> {
    let p = Path::new(&path);
    if !p.exists() || !p.is_dir() {
        return Err(AppError::FileNotFound {
            path: format!("Directory not found: {path}"),
        });
    }

    let manager = RunManager::global();
    if let Some(existing) = manager.get(&path) {
        if !existing.should_stop_now() {
            return Err(AppError::Internal {
                message: "A run is already active in this project".to_string(),
            });
        }
    }

    let layout = ProjectLayout::new(p);
    let config = if layout.config_file.exists() {
        ProjectConfig::load_from_file(&layout.config_file)?
    } else {
        ProjectConfig::for_profile("ja-id")
    };
    layout.ensure_layout(&config.profile)?;

    let global = GlobalSettings::load_from_app_data().unwrap_or_default();
    let mut effective = EffectiveSettings::merge(&global, &config, None);
    if let Some(m) = model {
        effective.roles.translator.primary = m;
    }

    // Set chapter status to pending
    {
        let conn = open_path(&layout.db_file)?;
        let chap_repo = ChapterRepo::new(&conn);
        let ch = chap_repo
            .get_by_id(&chapter_id)?
            .ok_or_else(|| AppError::NotFound {
                entity: "chapter".to_string(),
                id: chapter_id.clone(),
            })?;
        if ch.frozen {
            return Err(AppError::Internal {
                message: "Cannot retranslate a frozen chapter".to_string(),
            });
        }
        chap_repo.update_status(&chapter_id, "pending")?;
    }

    let run_control = Arc::new(RunControl::new());
    manager.insert(&path, run_control.clone());

    let path_clone = path.clone();
    let run_ctrl_clone = run_control.clone();
    let chap_id_clone = chapter_id.clone();

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(r) => r,
            Err(_) => {
                RunManager::global().remove(&path_clone);
                return;
            }
        };

        rt.block_on(async move {
            if let Ok(conn) = open_path(&layout.db_file) {
                let emitter = TauriEventEmitter { app };
                let resolver = DynamicProviderResolver::new(global);
                let chap_repo = ChapterRepo::new(&conn);
                if let Ok(Some(ch)) = chap_repo.get_by_id(&chap_id_clone) {
                    if let Ok(profile) = crate::profiles::load_profile(&effective.profile) {
                        let contract_text = &profile.contract_text;
                        let instruction_path = layout.root.join(
                            effective
                                .instruction_file
                                .as_deref()
                                .unwrap_or("instruction.md"),
                        );
                        let instruction_text =
                            std::fs::read_to_string(&instruction_path).unwrap_or_default();

                        let _ = crate::engine::chapter::execute_single_chapter(
                            &conn,
                            &layout,
                            &effective,
                            contract_text,
                            &instruction_text,
                            &ch,
                            &resolver,
                            &emitter,
                            &run_ctrl_clone,
                            None,
                        )
                        .await;
                    }
                }
            }
            RunManager::global().remove(&path_clone);
        });
    });

    Ok("started".to_string())
}
