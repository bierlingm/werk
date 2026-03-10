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
use sd_core::Forest;
use serde::Serialize;

/// Context output structure - always JSON, designed for agent consumption.
#[derive(Serialize)]
struct ContextResult {
    tension: TensionInfo,
    ancestors: Vec<TensionInfo>,
    siblings: Vec<TensionInfo>,
    children: Vec<TensionInfo>,
    dynamics: ContextDynamicsJson,
    mutations: Vec<MutationInfo>,
}

pub fn cmd_context(output: &Output, id: String) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let all_tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(all_tensions.clone());

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Get mutations for this tension
    let mutations = store
        .get_mutations(&tension.id)
        .map_err(WerkError::StoreError)?;

    // Get all mutations for conflict and orientation detection
    let all_mutations = store.all_mutations().map_err(WerkError::StoreError)?;

    // Build forest for ancestors, siblings, children, and conflict/neglect detection
    let forest = Forest::from_tensions(all_tensions.clone())
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

    // === Compute all dynamics via shared module ===
    let dynamics_json = compute_all_dynamics(
        tension,
        &mutations,
        &forest,
        &all_tensions,
        &all_mutations,
        now,
    );

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
    };

    // Context always outputs structured format (designed for agent consumption)
    output
        .print_structured(&result)
        .map_err(WerkError::IoError)?;

    Ok(())
}
