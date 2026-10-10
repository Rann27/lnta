//! Engine events sent to UI and log subscribers per FR-RUN-05.
//!
//! Emits typed events for run state transitions, chapter progress, streaming tokens,
//! log lines in start.md format, docx render status, and glossary changes.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Typed engine events per FR-RUN-05.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum EngineEvent {
    #[serde(rename = "run:state")]
    RunState {
        state: String,
        run_id: Option<String>,
    },

    #[serde(rename = "chapter:status")]
    ChapterStatus {
        chapter_id: String,
        status: String,
        part_current: Option<usize>,
        part_total: Option<usize>,
    },

    #[serde(rename = "call:progress")]
    CallProgress {
        call_id: String,
        delta: String,
        chapter_id: Option<String>,
        part_index: Option<usize>,
    },

    #[serde(rename = "log:line")]
    LogLine {
        level: String,
        message: String,
        timestamp: String,
    },

    #[serde(rename = "docx:render")]
    DocxRender { status: String, path: String },

    #[serde(rename = "glossary:changed")]
    GlossaryChanged {
        added_count: usize,
        conflicts_count: usize,
    },

    #[serde(rename = "qa:progress")]
    QaProgress {
        phase: String,
        findings_count: usize,
        fixes_count: usize,
    },
}

/// Abstract emitter trait for sending engine events.
pub trait EventEmitter: Send + Sync {
    fn emit(&self, event: EngineEvent);
}

/// In-memory event collector for tests and headless CLI runs.
#[derive(Debug, Default, Clone)]
pub struct RecordingEventEmitter {
    events: Arc<Mutex<Vec<EngineEvent>>>,
}

impl RecordingEventEmitter {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn events(&self) -> Vec<EngineEvent> {
        let guard = self.events.lock().unwrap_or_else(|p| p.into_inner());
        guard.clone()
    }

    pub fn clear(&self) {
        let mut guard = self.events.lock().unwrap_or_else(|p| p.into_inner());
        guard.clear();
    }
}

impl EventEmitter for RecordingEventEmitter {
    fn emit(&self, event: EngineEvent) {
        let mut guard = self.events.lock().unwrap_or_else(|p| p.into_inner());
        guard.push(event);
    }
}
