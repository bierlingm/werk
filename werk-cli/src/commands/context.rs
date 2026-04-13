//! Context command handler.

use crate::serialize::{
    mutation_to_info, node_to_tension_info, tension_to_info,
    MutationInfo, TensionInfo,
};
use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::Utc;
use sd_core::{
    compute_urgency, extract_mutation_pattern, ProjectionThresholds, TensionStatus,
};
use serde::Serialize;

/// Context output structure - always JSON, designed for agent consumption.
#[derive(Serialize)]
pub struct ContextResult {
    pub tension: TensionInfo,
    pub ancestors: Vec<TensionInfo>,
    pub siblings: Vec<TensionInfo>,
    pub children: Vec<TensionInfo>,
    pub mutations: Vec<MutationInfo>,
    pub engagement: serde_json::Value,
}

pub fn cmd_context(
    output: &Output,
    id: Option<String>,
    all: bool,
    urgent: bool,
) -> Result<(), WerkError> {
    if all && urgent {
        return Err(WerkError::InvalidInput(
            "Cannot use both --all and --urgent flags".to_string(),
        ));
    }

    if all || urgent {
        return cmd_context_bulk(output, urgent);
    }

    let id = id.ok_or_else(|| {
        WerkError::InvalidInput(
            "Tension ID is required (or use --all / --urgent for bulk mode)".to_string(),
        )
    })?;

    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;
    let analysis = crate::commands::analysis_thresholds_from(&workspace);
    let proj_thresholds = crate::commands::to_projection_thresholds(&analysis);

    let all_tensions = store
        .list_tensions()
        .map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(all_tensions.clone());

    let tension = resolver.resolve(&id)?;

    let mutations = store
        .get_mutations(&tension.id)
        .map_err(WerkError::StoreError)?;

    let forest = sd_core::Forest::from_tensions(all_tensions.clone())
        .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

    let now = Utc::now();

    let tension_info = tension_to_info(tension, &mutations, now);

    let ancestors: Vec<TensionInfo> = forest
        .ancestors(&tension.id)
        .unwrap_or_default()
        .into_iter()
        .map(|node| node_to_tension_info(node, now))
        .collect();

    let siblings: Vec<TensionInfo> = forest
        .siblings(&tension.id)
        .unwrap_or_default()
        .into_iter()
        .map(|node| node_to_tension_info(node, now))
        .collect();

    let children: Vec<TensionInfo> = forest
        .children(&tension.id)
        .unwrap_or_default()
        .into_iter()
        .map(|node| node_to_tension_info(node, now))
        .collect();

    let engagement_json = build_engagement_json(tension, &mutations, &proj_thresholds, now);

    let mutation_infos: Vec<MutationInfo> = mutations.iter().map(mutation_to_info).collect();

    let result = ContextResult {
        tension: tension_info,
        ancestors,
        siblings,
        children,
        mutations: mutation_infos,
        engagement: engagement_json,
    };

    output
        .print_structured(&result)
        .map_err(WerkError::IoError)?;

    Ok(())
}

fn cmd_context_bulk(output: &Output, urgent_only: bool) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;
    let analysis = crate::commands::analysis_thresholds_from(&workspace);
    let proj_thresholds = crate::commands::to_projection_thresholds(&analysis);

    let all_tensions = store
        .list_tensions()
        .map_err(WerkError::StoreError)?;

    let now = Utc::now();

    let mut targets: Vec<_> = all_tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Active)
        .collect();

    if urgent_only {
        targets.retain(|t| {
            compute_urgency(t, now)
                .map(|u| u.value > 0.75)
                .unwrap_or(false)
        });
    }

    let forest = sd_core::Forest::from_tensions(all_tensions.clone())
        .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

    let mut results: Vec<ContextResult> = Vec::new();

    for tension in &targets {
        let mutations = store
            .get_mutations(&tension.id)
            .map_err(WerkError::StoreError)?;

        let tension_info = tension_to_info(tension, &mutations, now);

        let ancestors: Vec<TensionInfo> = forest
            .ancestors(&tension.id)
            .unwrap_or_default()
            .into_iter()
            .map(|node| node_to_tension_info(node, now))
            .collect();

        let siblings: Vec<TensionInfo> = forest
            .siblings(&tension.id)
            .unwrap_or_default()
            .into_iter()
            .map(|node| node_to_tension_info(node, now))
            .collect();

        let children: Vec<TensionInfo> = forest
            .children(&tension.id)
            .unwrap_or_default()
            .into_iter()
            .map(|node| node_to_tension_info(node, now))
            .collect();

        let engagement_json = build_engagement_json(tension, &mutations, &proj_thresholds, now);

        let mutation_infos: Vec<MutationInfo> = mutations.iter().map(mutation_to_info).collect();

        results.push(ContextResult {
            tension: tension_info,
            ancestors,
            siblings,
            children,
            mutations: mutation_infos,
            engagement: engagement_json,
        });
    }

    output
        .print_structured(&results)
        .map_err(WerkError::IoError)?;

    Ok(())
}

/// Engagement metrics: raw mutation pattern data anchored to user actions.
/// Standard of Measurement: these are facts computed from what the user did —
/// mutation frequency, gap direction, engagement trend. No classification,
/// no thresholds, no instrument-originated standards.
/// Classification (trajectory enum) and projection live in `stats --trajectory`.
fn build_engagement_json(
    tension: &sd_core::Tension,
    mutations: &[sd_core::Mutation],
    thresholds: &ProjectionThresholds,
    now: chrono::DateTime<Utc>,
) -> serde_json::Value {
    let pattern = extract_mutation_pattern(tension, mutations, thresholds.pattern_window_seconds, now);
    let gap = sd_core::gap_magnitude(&tension.desired, &tension.actual);

    serde_json::json!({
        "current_gap": gap,
        "mutation_count": pattern.mutation_count,
        "frequency_per_day": pattern.frequency_per_day,
        "frequency_trend": pattern.frequency_trend,
        "gap_trend": pattern.gap_trend,
        "gap_samples": pattern.gap_samples,
        "mean_interval_seconds": pattern.mean_interval_seconds,
    })
}
