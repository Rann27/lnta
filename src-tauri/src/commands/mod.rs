//! LNTA commands module.
//!
//! Provides thin Tauri command handlers bridging frontend IPC to pure-core functionality.

pub mod app;
pub mod chapters;
pub mod glossary;
pub mod models;
pub mod preflight;
pub mod project;
pub mod providers;
pub mod qa;
pub mod run;
pub mod settings;
pub mod usage;

pub use app::*;
pub use chapters::*;
pub use glossary::*;
pub use models::*;
pub use preflight::*;
pub use project::*;
pub use providers::*;
pub use qa::*;
pub use run::*;
pub use settings::*;
pub use usage::*;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Typed payload for AppError sent across IPC.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/AppErrorPayload.ts")]
pub struct AppErrorPayload {
    pub code: String,
    pub params: std::collections::HashMap<String, String>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ts_rs::TS;

    #[test]
    fn test_export_all_typescript_bindings() {
        let cfg = ts_rs::Config::default();

        AppInfo::export(&cfg).unwrap();
        project::ProjectOpenResult::export(&cfg).unwrap();
        project::RecentProjectEntry::export(&cfg).unwrap();
        project::PlannedPartPreview::export(&cfg).unwrap();
        project::ChapterPlanPreview::export(&cfg).unwrap();
        project::ProjectPlanPreview::export(&cfg).unwrap();
        crate::project::preflight::PreflightStatus::export(&cfg).unwrap();
        crate::project::preflight::PreflightAction::export(&cfg).unwrap();
        crate::project::preflight::PreflightItem::export(&cfg).unwrap();
        crate::project::preflight::PreflightReport::export(&cfg).unwrap();
        crate::project::config::ProjectConfig::export(&cfg).unwrap();
        crate::project::settings::Role::export(&cfg).unwrap();
        crate::project::settings::ModelReference::export(&cfg).unwrap();
        crate::project::settings::RoleConfig::export(&cfg).unwrap();
        crate::project::settings::RolesConfig::export(&cfg).unwrap();
        crate::project::settings::ProviderConfigEntry::export(&cfg).unwrap();
        crate::project::settings::GlobalSettings::export(&cfg).unwrap();
        crate::project::settings::EffectiveSettings::export(&cfg).unwrap();
        crate::provider::registry::ValueSource::export(&cfg).unwrap();
        crate::provider::registry::ModelPricing::export(&cfg).unwrap();
        crate::provider::registry::ModelEntry::export(&cfg).unwrap();
        providers::ProviderTestResult::export(&cfg).unwrap();
        run::RunStateResponse::export(&cfg).unwrap();
        crate::store::repo::Chapter::export(&cfg).unwrap();
        crate::store::repo::Segment::export(&cfg).unwrap();
        crate::store::repo::Translation::export(&cfg).unwrap();
        crate::store::repo::ChunkPlan::export(&cfg).unwrap();
        crate::store::repo::Run::export(&cfg).unwrap();
        crate::store::repo::Call::export(&cfg).unwrap();
        crate::store::repo::GlossaryEntry::export(&cfg).unwrap();
        chapters::ChapterDetail::export(&cfg).unwrap();
        glossary::GlossaryEntryInput::export(&cfg).unwrap();
        crate::engine::events::EngineEvent::export(&cfg).unwrap();
        qa::QaFindingItem::export(&cfg).unwrap();
        qa::QaCheckGroup::export(&cfg).unwrap();
        qa::QaFindingsSummary::export(&cfg).unwrap();
        qa::QaFixLogItem::export(&cfg).unwrap();
        crate::docx::import::ExtractedDocxHeading::export(&cfg).unwrap();
        crate::docx::import::FrozenChapterMapping::export(&cfg).unwrap();
        crate::usage::UsageSummary::export(&cfg).unwrap();
        crate::usage::ChapterUsage::export(&cfg).unwrap();
        crate::usage::RoleUsage::export(&cfg).unwrap();
        crate::usage::ModelUsage::export(&cfg).unwrap();
        crate::usage::ProjectUsageResponse::export(&cfg).unwrap();
        AppErrorPayload::export(&cfg).unwrap();
    }
}
