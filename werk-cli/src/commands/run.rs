//! Run command handler.

use crate::commands::config::Config;
use crate::dynamics::{
    compute_all_dynamics, mutation_to_info, node_to_tension_info, tension_to_info,
    ContextDynamicsJson, MutationInfo, TensionInfo,
};
use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::Utc;
use sd_core::{Forest, Mutation};
use serde::Serialize;
use std::io::Write;
use std::process::Stdio;

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

pub fn cmd_run(_output: &Output, id: String, command: Vec<String>) -> Result<(), WerkError> {
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

    // Build context result (using ContextDynamicsJson for creative_cycle_phase field name)
    let context = ContextResult {
        tension: tension_info,
        ancestors,
        siblings,
        children,
        dynamics: dynamics_json.into(),
        mutations: mutation_infos,
    };

    // Serialize context to JSON
    let context_json = serde_json::to_string(&context)
        .map_err(|e| WerkError::IoError(format!("failed to serialize context JSON: {}", e)))?;

    // === Determine command to run ===
    let (program, args, command_str_for_mutation): (String, Vec<String>, String) = if !command
        .is_empty()
    {
        // Use -- override directly (already properly split by clap)
        let program = command[0].clone();
        let args: Vec<String> = command[1..].to_vec();
        let command_str = command.join(" ");
        (program, args, command_str)
    } else {
        // Try config default
        let config = Config::load(&workspace)?;
        match config.get("agent.command") {
            Some(cmd) => {
                // Parse config command - split on whitespace
                let cmd_parts: Vec<String> =
                    cmd.split_whitespace().map(|s| s.to_string()).collect();
                if cmd_parts.is_empty() {
                    return Err(WerkError::InvalidInput(
                        "agent command in config is empty".to_string(),
                    ));
                }
                let program = cmd_parts[0].clone();
                let args: Vec<String> = cmd_parts[1..].to_vec();
                (program, args, cmd.clone())
            }
            None => {
                return Err(WerkError::InvalidInput(
                    "no agent command configured. Use -- to specify a command or set agent.command in config".to_string(),
                ));
            }
        }
    };

    // Get workspace path
    let workspace_path = workspace.werk_dir();

    // === Spawn subprocess ===
    let mut child = std::process::Command::new(&program)
        .args(&args)
        .env("WERK_TENSION_ID", &tension.id)
        .env("WERK_CONTEXT", &context_json)
        .env("WERK_WORKSPACE", workspace_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                WerkError::InvalidInput(format!("agent command not found: {}", program))
            } else {
                WerkError::IoError(format!("failed to spawn agent process: {}", e))
            }
        })?;

    // Write context to stdin
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| WerkError::IoError("failed to open stdin for subprocess".to_string()))?;
        stdin
            .write_all(context_json.as_bytes())
            .map_err(|e| WerkError::IoError(format!("failed to write context to stdin: {}", e)))?;
    }

    // Wait for subprocess to complete
    let exit_status = child
        .wait()
        .map_err(|e| WerkError::IoError(format!("failed to wait for agent process: {}", e)))?;

    // Get exit code
    let exit_code = exit_status.code().unwrap_or(1);

    // Record session mutation
    store
        .record_mutation(&Mutation::new(
            tension.id.clone(),
            Utc::now(),
            "agent_session".to_owned(),
            None,
            command_str_for_mutation,
        ))
        .map_err(WerkError::SdError)?;

    // If not successful, exit with the subprocess exit code
    if !exit_status.success() {
        std::process::exit(exit_code);
    }

    Ok(())
}
