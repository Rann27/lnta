//! Engine lifecycle states, chapter statuses, and execution controls per FR-RUN-01..04.
//!
//! Tracks state transitions, persistence in SQLite, and cooperative pause/stop signals.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// High-level run state machine per FR-RUN-01.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Idle,
    Preflight,
    Running,
    Pausing,
    Paused,
    QaRunning,
    Completed,
    Failed,
}

impl RunState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Preflight => "preflight",
            Self::Running => "running",
            Self::Pausing => "pausing",
            Self::Paused => "paused",
            Self::QaRunning => "qa_running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

/// Status of an individual chapter in the volume per FR-RUN-02.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChapterStatus {
    Pending,
    Planned,
    Translating,
    Validating,
    Repairing,
    Done,
    NeedsAttention,
    Blocked,
    Frozen,
    SourceChanged,
}

impl ChapterStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Planned => "planned",
            Self::Translating => "translating",
            Self::Validating => "validating",
            Self::Repairing => "repairing",
            Self::Done => "done",
            Self::NeedsAttention => "needs_attention",
            Self::Blocked => "blocked",
            Self::Frozen => "frozen",
            Self::SourceChanged => "source_changed",
        }
    }
}

const CONTROL_CONTINUE: u8 = 0;
const CONTROL_PAUSE: u8 = 1;
const CONTROL_STOP_NOW: u8 = 2;

/// Cooperative execution control handle for the engine per FR-RUN-07.
#[derive(Debug, Clone)]
pub struct RunControl {
    signal: Arc<AtomicU8>,
}

impl Default for RunControl {
    fn default() -> Self {
        Self::new()
    }
}

impl RunControl {
    pub fn new() -> Self {
        Self {
            signal: Arc::new(AtomicU8::new(CONTROL_CONTINUE)),
        }
    }

    /// Requests cooperative pause after finishing the currently running call.
    pub fn request_pause(&self) {
        self.signal.store(CONTROL_PAUSE, Ordering::SeqCst);
    }

    /// Requests immediate cancellation of the in-flight call.
    pub fn request_stop_now(&self) {
        self.signal.store(CONTROL_STOP_NOW, Ordering::SeqCst);
    }

    /// Resumes normal continuous execution.
    pub fn reset_continue(&self) {
        self.signal.store(CONTROL_CONTINUE, Ordering::SeqCst);
    }

    pub fn should_pause(&self) -> bool {
        self.signal.load(Ordering::SeqCst) == CONTROL_PAUSE
    }

    pub fn should_stop_now(&self) -> bool {
        self.signal.load(Ordering::SeqCst) == CONTROL_STOP_NOW
    }

    pub fn is_interrupted(&self) -> bool {
        self.signal.load(Ordering::SeqCst) != CONTROL_CONTINUE
    }
}
