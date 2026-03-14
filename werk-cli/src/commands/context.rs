//! Context command handler.

use crate::dynamics::{
    compute_all_dynamics, mutation_to_info, node_to_tension_info, tension_to_info,
    ContextDynamicsJson, MutationInfo, TensionInfo,
};
use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::Utc;
use sd_core::{
    compute_urgency, project_tension, DynamicsEngine, ProjectionThresholds, TensionStatus,
};
use serde::Serialize;

/// Context output structure - always JSON, designed for agent consumption.
#[derive(Serialize)]
pub struct ContextResult {
    pub tension: TensionInfo,
    pub ancestors: Vec<TensionInfo>,
    pub siblings: Vec<TensionInfo>,
    pub children: Vec<TensionInfo>,
    pub dynamics: ContextDynamicsJson,
    pub mutations: Vec<MutationInfo>,
    pub projection: serde_json::Value,
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

    // Bulk modes: --all or --urgent
    if all || urgent {
        return cmd_context_bulk(output, urgent);
    }

    // Single-tension mode: id is required
    let id = id.ok_or_else(|| {
        WerkError::InvalidInput(
            "Tension ID is required (or use --all / --urgent for bulk mode)".to_string(),
        )
    })?;

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Create DynamicsEngine from store (all store access goes through engine.store())
    let mut engine = DynamicsEngine::with_store(store);

    // Get all tensions for prefix resolution
    let all_tensions = engine
        .store()
        .list_tensions()
        .map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(all_tensions.clone());

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Get mutations for this tension
    let mutations = engine
        .store()
        .get_mutations(&tension.id)
        .map_err(WerkError::StoreError)?;

    // Build forest for ancestors, siblings, children
    let forest = sd_core::Forest::from_tensions(all_tensions.clone())
        .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

    // === Compute time reference ===
    let now = Utc::now();

    // === Tension Info (with staleness_ratio) ===
    let tension_info = tension_to_info(tension, &mutations, now);

    // === Ancestors (root-first) ===
    let ancestors: Vec<TensionInfo> = forest
        .ancestors(&tension.id)
        .unwrap_or_default()
        .into_iter()
        .map(|node| node_to_tension_info(node, now))
        .collect();

    // === Siblings (excluding self) ===
    let siblings: Vec<TensionInfo> = forest
        .siblings(&tension.id)
        .unwrap_or_default()
        .into_iter()
        .map(|node| node_to_tension_info(node, now))
        .collect();

    // === Children ===
    let children: Vec<TensionInfo> = forest
        .children(&tension.id)
        .unwrap_or_default()
        .into_iter()
        .map(|node| node_to_tension_info(node, now))
        .collect();

    // === Compute all dynamics via DynamicsEngine (shared module) ===
    let dynamics_json = compute_all_dynamics(&mut engine, &tension.id);

    // === Compute projection ===
    let thresholds = ProjectionThresholds::default();
    let projections = project_tension(tension, &mutations, &thresholds, now);
    let projection_json = build_projection_json(&projections);

    // === Mutations (chronological order - oldest first) ===
    let mutation_infos: Vec<MutationInfo> = mutations.iter().map(mutation_to_info).collect();

    // Build final result (using ContextDynamicsJson for creative_cycle_phase field name)
    let result = ContextResult {
        tension: tension_info,
        ancestors,
        siblings,
        children,
        dynamics: dynamics_json.into(),
        mutations: mutation_infos,
        projection: projection_json,
    };

    // Context always outputs structured format (designed for agent consumption)
    output
        .print_structured(&result)
        .map_err(WerkError::IoError)?;

    Ok(())
}

/// Bulk context: iterate tensions and output a JSON array of context objects.
fn cmd_context_bulk(output: &Output, urgent_only: bool) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;
    let mut engine = DynamicsEngine::with_store(store);

    let all_tensions = engine
        .store()
        .list_tensions()
        .map_err(WerkError::StoreError)?;

    let now = Utc::now();

    // Filter to active tensions
    let mut targets: Vec<_> = all_tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Active)
        .collect();

    // If urgent-only, further filter to urgency > 0.75
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
        let mutations = engine
            .store()
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

        let dynamics_json = compute_all_dynamics(&mut engine, &tension.id);

        let thresholds = ProjectionThresholds::default();
        let projections = project_tension(tension, &mutations, &thresholds, now);
        let projection_json = build_projection_json(&projections);

        let mutation_infos: Vec<MutationInfo> = mutations.iter().map(mutation_to_info).collect();

        results.push(ContextResult {
            tension: tension_info,
            ancestors,
            siblings,
            children,
            dynamics: dynamics_json.into(),
            mutations: mutation_infos,
            projection: projection_json,
        });
    }

    output
        .print_structured(&results)
        .map_err(WerkError::IoError)?;

    Ok(())
}

/// Build a projection JSON value from the projection results.
pub fn build_projection_json(
    projections: &[sd_core::TensionProjection],
) -> serde_json::Value {
    if let Some(proj) = projections.first() {
        serde_json::json!({
            "trajectory": format!("{:?}", proj.trajectory),
            "current_gap": proj.current_gap,
            "projected_gap_1w": projections.get(0).map(|p| p.projected_gap),
            "projected_gap_1m": projections.get(1).map(|p| p.projected_gap),
            "projected_gap_3m": projections.get(2).map(|p| p.projected_gap),
            "will_resolve": proj.will_resolve,
            "time_to_resolution": proj.time_to_resolution,
            "oscillation_risk": proj.oscillation_risk,
            "neglect_risk": proj.neglect_risk,
        })
    } else {
        serde_json::Value::Null
    }
}
