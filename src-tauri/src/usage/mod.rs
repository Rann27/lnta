//! Token accounting, cost calculation, and budget checking per FR-USG-01..03,
//! FR-RUN-10, and Story H2.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::project::settings::EffectiveSettings;
use crate::provider::registry::{ModelPricing, ModelRegistry, ValueSource};
use crate::store::repo::{Call, Chapter};

/// Usage row for calls that cover several chapters (QA batches across the volume).
pub const VOLUME_ROW_ID: &str = "volume";

/// Overall token usage and cost summary for a project or run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/UsageSummary.ts")]
pub struct UsageSummary {
    #[ts(type = "number")]
    pub total_input_tokens: i64,
    #[ts(type = "number")]
    pub total_cached_tokens: i64,
    #[ts(type = "number")]
    pub total_output_tokens: i64,
    #[ts(type = "number")]
    pub total_reasoning_tokens: i64,
    pub total_cost: Option<f64>,
    pub has_unknown_cost: bool,
    #[ts(type = "number")]
    pub total_calls: i64,
}

/// Token usage and cost grouped by chapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/ChapterUsage.ts")]
pub struct ChapterUsage {
    pub chapter_id: String,
    pub chapter_ordinal: i32,
    pub chapter_title: String,
    #[ts(type = "number")]
    pub input_tokens: i64,
    #[ts(type = "number")]
    pub cached_tokens: i64,
    #[ts(type = "number")]
    pub output_tokens: i64,
    #[ts(type = "number")]
    pub reasoning_tokens: i64,
    pub cost: Option<f64>,
    #[ts(type = "number")]
    pub call_count: i64,
}

/// Token usage and cost grouped by pipeline role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/RoleUsage.ts")]
pub struct RoleUsage {
    pub role: String,
    #[ts(type = "number")]
    pub input_tokens: i64,
    #[ts(type = "number")]
    pub cached_tokens: i64,
    #[ts(type = "number")]
    pub output_tokens: i64,
    #[ts(type = "number")]
    pub reasoning_tokens: i64,
    pub cost: Option<f64>,
    #[ts(type = "number")]
    pub call_count: i64,
}

/// Token usage and cost grouped by provider/model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/ModelUsage.ts")]
pub struct ModelUsage {
    pub provider: String,
    pub model: String,
    #[ts(type = "number")]
    pub input_tokens: i64,
    #[ts(type = "number")]
    pub cached_tokens: i64,
    #[ts(type = "number")]
    pub output_tokens: i64,
    #[ts(type = "number")]
    pub reasoning_tokens: i64,
    pub cost: Option<f64>,
    #[ts(type = "number")]
    pub call_count: i64,
    pub pricing_source: Option<String>,
}

/// Complete usage response returned to the frontend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/ProjectUsageResponse.ts")]
pub struct ProjectUsageResponse {
    pub summary: UsageSummary,
    pub by_chapter: Vec<ChapterUsage>,
    pub by_role: Vec<RoleUsage>,
    pub by_model: Vec<ModelUsage>,
}

/// Reason a budget limit was exceeded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BudgetExceeded {
    TokenLimit { limit: u64, current: u64 },
    CostLimit { limit: f64, current: f64 },
}

/// Calculates the cost in USD for a single model call based on its pricing.
///
/// Returns None if pricing is not configured or either input or output pricing is missing.
pub fn calculate_call_cost(
    pricing: Option<&ModelPricing>,
    in_tokens: i64,
    cached_in: i64,
    out_tokens: i64,
) -> Option<f64> {
    let p = pricing?;
    let p_in = p.input_per_million?;
    let p_out = p.output_per_million?;
    let p_cached = p.cached_input_per_million.unwrap_or(p_in * 0.5);

    let uncached = (in_tokens - cached_in).max(0);
    let in_cost = (uncached as f64 * p_in + (cached_in.max(0) as f64) * p_cached) / 1_000_000.0;
    let out_cost = (out_tokens.max(0) as f64 * p_out) / 1_000_000.0;

    Some(in_cost + out_cost)
}

/// Checks if a project run has exceeded configured token or cost budget limits.
pub fn check_budget(
    settings: &EffectiveSettings,
    total_tokens: u64,
    total_cost: f64,
) -> Option<BudgetExceeded> {
    if let Some(limit) = settings.budget_token_limit {
        if total_tokens >= limit {
            return Some(BudgetExceeded::TokenLimit {
                limit,
                current: total_tokens,
            });
        }
    }

    if let Some(limit) = settings.budget_cost_limit {
        if total_cost >= limit {
            return Some(BudgetExceeded::CostLimit {
                limit,
                current: total_cost,
            });
        }
    }

    None
}

/// Aggregates usage and costs across calls, grouping by chapter, role, and model.
pub fn compute_project_usage(
    calls: &[Call],
    chapters: &[Chapter],
    registry: &ModelRegistry,
) -> ProjectUsageResponse {
    // 1. Enrich each call with cost if not already present
    let enriched_costs: Vec<Option<f64>> = calls
        .iter()
        .map(|c| {
            if c.cost.is_some() {
                c.cost
            } else {
                let model_entry = registry.get(&c.provider, &c.model);
                calculate_call_cost(
                    model_entry.map(|m| &m.pricing),
                    c.in_tokens,
                    c.cached_in,
                    c.out_tokens,
                )
            }
        })
        .collect();

    // 2. Compute Summary
    let mut total_input_tokens = 0i64;
    let mut total_cached_tokens = 0i64;
    let mut total_output_tokens = 0i64;
    let mut total_reasoning_tokens = 0i64;
    let mut sum_cost = 0.0f64;
    let mut has_any_cost = false;
    let mut has_unknown_cost = false;

    for (c, cost) in calls.iter().zip(enriched_costs.iter()) {
        total_input_tokens += c.in_tokens;
        total_cached_tokens += c.cached_in;
        total_output_tokens += c.out_tokens;
        total_reasoning_tokens += c.reasoning_tokens;

        match cost {
            Some(v) => {
                sum_cost += *v;
                has_any_cost = true;
            }
            None => {
                has_unknown_cost = true;
            }
        }
    }

    let summary = UsageSummary {
        total_input_tokens,
        total_cached_tokens,
        total_output_tokens,
        total_reasoning_tokens,
        total_cost: if has_any_cost { Some(sum_cost) } else { None },
        has_unknown_cost,
        total_calls: calls.len() as i64,
    };

    // 3. Group by Chapter
    let mut chapter_map: std::collections::HashMap<String, ChapterUsage> =
        std::collections::HashMap::new();

    // Seed chapter map with known chapters so all chapters are represented
    for ch in chapters {
        chapter_map.insert(
            ch.id.clone(),
            ChapterUsage {
                chapter_id: ch.id.clone(),
                chapter_ordinal: ch.ordinal,
                chapter_title: if !ch.title_src.is_empty() {
                    ch.title_src.clone()
                } else {
                    ch.file_name.clone()
                },
                input_tokens: 0,
                cached_tokens: 0,
                output_tokens: 0,
                reasoning_tokens: 0,
                cost: None,
                call_count: 0,
            },
        );
    }

    for (c, cost) in calls.iter().zip(enriched_costs.iter()) {
        let chap_id = c.chapter_id.as_deref().unwrap_or(VOLUME_ROW_ID);
        let entry = chapter_map
            .entry(chap_id.to_string())
            .or_insert_with(|| ChapterUsage {
                chapter_id: chap_id.to_string(),
                chapter_ordinal: 9999,
                chapter_title: chap_id.to_string(),
                input_tokens: 0,
                cached_tokens: 0,
                output_tokens: 0,
                reasoning_tokens: 0,
                cost: None,
                call_count: 0,
            });

        entry.input_tokens += c.in_tokens;
        entry.cached_tokens += c.cached_in;
        entry.output_tokens += c.out_tokens;
        entry.reasoning_tokens += c.reasoning_tokens;
        entry.call_count += 1;

        if let Some(v) = cost {
            entry.cost = Some(entry.cost.unwrap_or(0.0) + *v);
        }
    }

    let mut by_chapter: Vec<ChapterUsage> = chapter_map.into_values().collect();
    by_chapter.sort_by_key(|c| c.chapter_ordinal);

    // 4. Group by Role
    let mut role_map: std::collections::HashMap<String, RoleUsage> =
        std::collections::HashMap::new();

    for (c, cost) in calls.iter().zip(enriched_costs.iter()) {
        let entry = role_map.entry(c.role.clone()).or_insert_with(|| RoleUsage {
            role: c.role.clone(),
            input_tokens: 0,
            cached_tokens: 0,
            output_tokens: 0,
            reasoning_tokens: 0,
            cost: None,
            call_count: 0,
        });

        entry.input_tokens += c.in_tokens;
        entry.cached_tokens += c.cached_in;
        entry.output_tokens += c.out_tokens;
        entry.reasoning_tokens += c.reasoning_tokens;
        entry.call_count += 1;

        if let Some(v) = cost {
            entry.cost = Some(entry.cost.unwrap_or(0.0) + *v);
        }
    }

    let mut by_role: Vec<RoleUsage> = role_map.into_values().collect();
    by_role.sort_by_key(|a| std::cmp::Reverse(a.call_count));

    // 5. Group by Model
    let mut model_map: std::collections::HashMap<(String, String), ModelUsage> =
        std::collections::HashMap::new();

    for (c, cost) in calls.iter().zip(enriched_costs.iter()) {
        let key = (c.provider.clone(), c.model.clone());
        let entry = model_map.entry(key).or_insert_with(|| {
            let model_entry = registry.get(&c.provider, &c.model);
            let pricing_source = model_entry.map(|m| match m.value_source {
                ValueSource::DefaultRegistry => "default_registry".to_string(),
                ValueSource::ProviderMetadata => "provider_metadata".to_string(),
                ValueSource::UserSupplied => "user_supplied".to_string(),
            });

            ModelUsage {
                provider: c.provider.clone(),
                model: c.model.clone(),
                input_tokens: 0,
                cached_tokens: 0,
                output_tokens: 0,
                reasoning_tokens: 0,
                cost: None,
                call_count: 0,
                pricing_source,
            }
        });

        entry.input_tokens += c.in_tokens;
        entry.cached_tokens += c.cached_in;
        entry.output_tokens += c.out_tokens;
        entry.reasoning_tokens += c.reasoning_tokens;
        entry.call_count += 1;

        if let Some(v) = cost {
            entry.cost = Some(entry.cost.unwrap_or(0.0) + *v);
        }
    }

    let mut by_model: Vec<ModelUsage> = model_map.into_values().collect();
    by_model.sort_by_key(|a| std::cmp::Reverse(a.call_count));

    ProjectUsageResponse {
        summary,
        by_chapter,
        by_role,
        by_model,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_call_cost_known_pricing() {
        let pricing = ModelPricing {
            input_per_million: Some(3.0),
            cached_input_per_million: Some(1.5),
            output_per_million: Some(15.0),
        };

        // 1,000 in (500 cached), 200 out
        // in: (500 * 3.0 + 500 * 1.5) / 1,000,000 = (1500 + 750) / 1,000,000 = 0.00225
        // out: (200 * 15.0) / 1,000,000 = 0.003
        // total: 0.00525
        let cost = calculate_call_cost(Some(&pricing), 1000, 500, 200).unwrap();
        assert!((cost - 0.00525).abs() < 1e-6);
    }

    #[test]
    fn test_calculate_call_cost_missing_pricing_returns_none() {
        let incomplete = ModelPricing {
            input_per_million: Some(3.0),
            cached_input_per_million: None,
            output_per_million: None,
        };
        assert!(calculate_call_cost(Some(&incomplete), 1000, 0, 500).is_none());
        assert!(calculate_call_cost(None, 1000, 0, 500).is_none());
    }

    #[test]
    fn test_check_budget_limits() {
        let global = crate::project::settings::GlobalSettings::default();
        let proj = crate::project::config::ProjectConfig {
            budget_token_limit: Some(10_000),
            budget_cost_limit: Some(5.0),
            ..Default::default()
        };
        let settings = EffectiveSettings::merge(&global, &proj, None);

        // Under budget
        assert!(check_budget(&settings, 5000, 2.0).is_none());

        // Token budget exceeded
        let token_exceeded = check_budget(&settings, 12_000, 2.0).unwrap();
        assert_eq!(
            token_exceeded,
            BudgetExceeded::TokenLimit {
                limit: 10_000,
                current: 12_000
            }
        );

        // Cost budget exceeded
        let cost_exceeded = check_budget(&settings, 5000, 5.5).unwrap();
        assert_eq!(
            cost_exceeded,
            BudgetExceeded::CostLimit {
                limit: 5.0,
                current: 5.5
            }
        );
    }

    #[test]
    fn test_compute_project_usage_grouping() {
        let registry = ModelRegistry::default();
        let chapters = vec![
            Chapter {
                id: "ch_1".to_string(),
                ordinal: 1,
                file_name: "01.md".to_string(),
                prefix: "1".to_string(),
                title_src: "第1話".to_string(),
                title_tgt: None,
                src_hash: "h".to_string(),
                line_count: 10,
                char_count: 50,
                status: "done".to_string(),
                frozen: false,
                parts_planned: 1,
                updated_at: "now".to_string(),
            },
            Chapter {
                id: "ch_2".to_string(),
                ordinal: 2,
                file_name: "02.md".to_string(),
                prefix: "2".to_string(),
                title_src: "第2話".to_string(),
                title_tgt: None,
                src_hash: "h".to_string(),
                line_count: 15,
                char_count: 80,
                status: "pending".to_string(),
                frozen: false,
                parts_planned: 1,
                updated_at: "now".to_string(),
            },
        ];

        let calls = vec![
            Call {
                id: "c1".to_string(),
                run_id: Some("r1".to_string()),
                role: "translator".to_string(),
                provider: "anthropic".to_string(),
                model: "claude-3-5-sonnet-20241022".to_string(),
                chapter_id: Some("ch_1".to_string()),
                part_index: Some(1),
                kind: "part".to_string(),
                status: "success".to_string(),
                finish_reason: Some("Stop".to_string()),
                in_tokens: 1000,
                cached_in: 0,
                cache_write: 0,
                out_tokens: 500,
                reasoning_tokens: 0,
                cost: None, // should be enriched
                latency_ms: 1200,
                contract_version: None,
                manifest_json: None,
                error: None,
                raw_response: None,
            },
            Call {
                id: "c2".to_string(),
                run_id: Some("r1".to_string()),
                role: "repair".to_string(),
                provider: "anthropic".to_string(),
                model: "claude-3-5-sonnet-20241022".to_string(),
                chapter_id: Some("ch_1".to_string()),
                part_index: Some(1),
                kind: "repair".to_string(),
                status: "success".to_string(),
                finish_reason: Some("Stop".to_string()),
                in_tokens: 400,
                cached_in: 0,
                cache_write: 0,
                out_tokens: 100,
                reasoning_tokens: 0,
                cost: None,
                latency_ms: 600,
                contract_version: None,
                manifest_json: None,
                error: None,
                raw_response: None,
            },
        ];

        let resp = compute_project_usage(&calls, &chapters, &registry);
        assert_eq!(resp.summary.total_calls, 2);
        assert_eq!(resp.summary.total_input_tokens, 1400);
        assert_eq!(resp.summary.total_output_tokens, 600);
        assert!(resp.summary.total_cost.is_some());
        assert!(!resp.summary.has_unknown_cost);

        // Group by chapter: ch_1 has 2 calls, ch_2 has 0 calls
        assert_eq!(resp.by_chapter.len(), 2);
        assert_eq!(resp.by_chapter[0].chapter_id, "ch_1");
        assert_eq!(resp.by_chapter[0].call_count, 2);
        assert_eq!(resp.by_chapter[1].chapter_id, "ch_2");
        assert_eq!(resp.by_chapter[1].call_count, 0);

        // Group by role: translator and repair
        assert_eq!(resp.by_role.len(), 2);

        // Group by model: 1 model entry with 2 calls
        assert_eq!(resp.by_model.len(), 1);
        assert_eq!(resp.by_model[0].call_count, 2);
        assert_eq!(
            resp.by_model[0].pricing_source.as_deref(),
            Some("default_registry")
        );
    }
}
