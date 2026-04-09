//! Batch command handler.
//!
//! Apply or validate mutations in bulk from a YAML file or stdin.

use werk_shared::{BatchMutation, Config, HookBridge, HookRunner};
use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use chrono::Utc;
use clap::Subcommand;
use sd_core::{Engine, Mutation, TensionStatus};
use std::sync::Arc;

/// Batch subcommands.
#[derive(Debug, Subcommand)]
pub enum BatchCommand {
    /// Apply mutations from a YAML file or stdin.
    Apply {
        /// Path to YAML file, or "-" for stdin.
        file: String,

        /// Validate only, don't apply.
        #[arg(long)]
        dry_run: bool,
    },
    /// Validate mutations without applying.
    Validate {
        /// Path to YAML file, or "-" for stdin.
        file: String,
    },
}

pub fn cmd_batch(output: &Output, command: &BatchCommand) -> Result<(), WerkError> {
    match command {
        BatchCommand::Apply { file, dry_run } => cmd_batch_apply(output, file, *dry_run),
        BatchCommand::Validate { file } => cmd_batch_apply(output, file, true),
    }
}

fn cmd_batch_apply(output: &Output, file: &str, dry_run: bool) -> Result<(), WerkError> {
    // Read YAML content from file or stdin
    let content = if file == "-" {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)
            .map_err(|e| WerkError::IoError(format!("failed to read from stdin: {}", e)))?;
        buf
    } else {
        std::fs::read_to_string(file)
            .map_err(|e| WerkError::IoError(format!("failed to read '{}': {}", file, e)))?
    };

    if content.trim().is_empty() {
        return Err(WerkError::InvalidInput("input is empty".to_string()));
    }

    // Parse mutations from YAML
    let mutations = parse_mutations(&content)?;

    if mutations.is_empty() {
        if output.is_json() {
            let _ = output.print_structured(&serde_json::json!({
                "applied": 0,
                "failed": 0,
                "dry_run": dry_run,
                "mutations": [],
            }));
        } else {
            println!("No mutations found in input.");
        }
        return Ok(());
    }

    // Discover workspace and open store with hooks
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;
    let mut engine = Engine::with_store(store);

    // Wire the HookBridge to the Engine's EventBus
    let config = Config::load(&workspace).unwrap_or_default();
    let global_config = Config::load_global().ok();
    let runner = Arc::new(HookRunner::from_configs(global_config.as_ref(), &config));
    let _hook_bridge = HookBridge::new(engine.event_bus(), runner);

    // Begin a single gesture for the entire batch — a batch IS one gesture
    if !dry_run {
        let _ = engine.store_mut().begin_gesture(
            Some(&format!("batch apply ({} mutations)", mutations.len())),
        );
    }

    // Validate and optionally apply each mutation
    let mut applied = 0usize;
    let mut failed = 0usize;
    let mut results: Vec<serde_json::Value> = Vec::new();

    for (i, mutation) in mutations.iter().enumerate() {
        let summary = mutation.summary();
        let result = validate_mutation(&engine, mutation);

        match result {
            Ok(()) => {
                if dry_run {
                    if !output.is_json() {
                        println!("  [ok]   {}. {}", i + 1, summary);
                    }
                    results.push(serde_json::json!({
                        "index": i,
                        "status": "valid",
                        "summary": summary,
                    }));
                    applied += 1;
                } else {
                    match apply_single_mutation(&mut engine, mutation) {
                        Ok(()) => {
                            if !output.is_json() {
                                println!("  [done] {}. {}", i + 1, summary);
                            }
                            results.push(serde_json::json!({
                                "index": i,
                                "status": "applied",
                                "summary": summary,
                            }));
                            applied += 1;
                        }
                        Err(e) => {
                            if !output.is_json() {
                                eprintln!("  [FAIL] {}. {} -- {}", i + 1, summary, e);
                            }
                            results.push(serde_json::json!({
                                "index": i,
                                "status": "failed",
                                "summary": summary,
                                "error": e.to_string(),
                            }));
                            failed += 1;
                        }
                    }
                }
            }
            Err(e) => {
                if !output.is_json() {
                    eprintln!("  [FAIL] {}. {} -- {}", i + 1, summary, e);
                }
                results.push(serde_json::json!({
                    "index": i,
                    "status": "failed",
                    "summary": summary,
                    "error": e.to_string(),
                }));
                failed += 1;
            }
        }
    }

    // End the batch gesture
    if !dry_run {
        engine.store_mut().end_gesture();
    }

    // Summary
    if output.is_json() {
        let _ = output.print_structured(&serde_json::json!({
            "applied": applied,
            "failed": failed,
            "dry_run": dry_run,
            "mutations": results,
        }));
    } else {
        let verb = if dry_run { "Validated" } else { "Applied" };
        println!("\n{} {} mutation(s) ({} failed)", verb, applied, failed);
    }

    if failed > 0 {
        Err(WerkError::InvalidInput(format!(
            "{} mutation(s) failed",
            failed
        )))
    } else {
        Ok(())
    }
}

/// Parse mutations from YAML content.
///
/// Expects a YAML list of mutation objects, each with an `action` tag.
fn parse_mutations(content: &str) -> Result<Vec<BatchMutation>, WerkError> {
    serde_yaml::from_str::<Vec<BatchMutation>>(content).map_err(|e| {
        WerkError::InvalidInput(format!(
            "could not parse YAML as mutations: {}. Expected a YAML list of mutation objects.",
            e
        ))
    })
}

/// Validate a mutation without applying it.
fn validate_mutation(engine: &Engine, mutation: &BatchMutation) -> Result<(), WerkError> {
    match mutation {
        BatchMutation::UpdateActual { tension_id, .. }
        | BatchMutation::AddNote { tension_id, .. }
        | BatchMutation::UpdateStatus { tension_id, .. }
        | BatchMutation::UpdateDesired { tension_id, .. } => {
            // Check that the tension exists
            let tensions = engine
                .store()
                .list_tensions()
                .map_err(WerkError::StoreError)?;
            if !tensions.iter().any(|t| &t.id == tension_id) {
                return Err(WerkError::TensionNotFound(format!(
                    "tension '{}' not found",
                    tension_id
                )));
            }
        }
        BatchMutation::CreateChild { parent_id, .. } => {
            let tensions = engine
                .store()
                .list_tensions()
                .map_err(WerkError::StoreError)?;
            if !tensions.iter().any(|t| &t.id == parent_id) {
                return Err(WerkError::TensionNotFound(format!(
                    "parent tension '{}' not found",
                    parent_id
                )));
            }
        }
        BatchMutation::SetHorizon { tension_id, .. }
        | BatchMutation::MoveTension { tension_id, .. } => {
            let tensions = engine
                .store()
                .list_tensions()
                .map_err(WerkError::StoreError)?;
            if !tensions.iter().any(|t| t.id == *tension_id) {
                return Err(WerkError::TensionNotFound(format!(
                    "tension '{}' not found",
                    tension_id
                )));
            }
        }
        BatchMutation::CreateParent { child_id, .. } => {
            let tensions = engine
                .store()
                .list_tensions()
                .map_err(WerkError::StoreError)?;
            if !tensions.iter().any(|t| t.id == *child_id) {
                return Err(WerkError::TensionNotFound(format!(
                    "child tension '{}' not found",
                    child_id
                )));
            }
        }
    }

    // Validate status values for UpdateStatus
    if let BatchMutation::UpdateStatus { new_status, .. } = mutation {
        match new_status.to_lowercase().as_str() {
            "resolved" | "released" | "active" => {}
            other => {
                return Err(WerkError::InvalidInput(format!(
                    "unknown status: '{}' (expected Active, Resolved, or Released)",
                    other
                )));
            }
        }
    }

    Ok(())
}

/// Apply a single mutation to the store.
fn apply_single_mutation(
    engine: &mut Engine,
    mutation: &BatchMutation,
) -> Result<(), WerkError> {
    match mutation {
        BatchMutation::UpdateActual {
            tension_id,
            new_value,
            ..
        } => {
            engine
                .store()
                .update_actual(tension_id, new_value)
                .map_err(WerkError::SdError)?;
        }
        BatchMutation::CreateChild {
            parent_id,
            desired,
            actual,
            ..
        } => {
            engine
                .store()
                .create_tension_with_parent(desired, actual, Some(parent_id.clone()))
                .map_err(WerkError::SdError)?;
        }
        BatchMutation::AddNote {
            tension_id, text, ..
        } => {
            engine
                .store()
                .record_mutation(&Mutation::new(
                    tension_id.clone(),
                    Utc::now(),
                    "note".to_owned(),
                    None,
                    text.clone(),
                ))
                .map_err(WerkError::SdError)?;
        }
        BatchMutation::UpdateStatus {
            tension_id,
            new_status,
            ..
        } => {
            let status = match new_status.to_lowercase().as_str() {
                "resolved" => TensionStatus::Resolved,
                "released" => TensionStatus::Released,
                "active" => TensionStatus::Active,
                other => {
                    return Err(WerkError::InvalidInput(format!(
                        "unknown status: '{}' (expected Active, Resolved, or Released)",
                        other
                    )));
                }
            };
            engine
                .store()
                .update_status(tension_id, status)
                .map_err(WerkError::SdError)?;
        }
        BatchMutation::UpdateDesired {
            tension_id,
            new_value,
            ..
        } => {
            engine
                .store()
                .update_desired(tension_id, new_value)
                .map_err(WerkError::SdError)?;
        }
        BatchMutation::SetHorizon { tension_id, horizon, .. } => {
            if let Ok(h) = sd_core::Horizon::parse(horizon) {
                engine.update_horizon(tension_id, Some(h))
                    .map_err(WerkError::SdError)?;
            }
        }
        BatchMutation::MoveTension { tension_id, new_parent_id, .. } => {
            engine.update_parent(tension_id, new_parent_id.as_deref())
                .map_err(WerkError::SdError)?;
        }
        BatchMutation::CreateParent { child_id, desired, actual, .. } => {
            let current_parent = engine.store().get_tension(child_id)
                .ok().flatten().and_then(|t| t.parent_id.clone());
            let parent = engine.create_tension_with_parent(desired, actual, current_parent)
                .map_err(WerkError::SdError)?;
            engine.update_parent(child_id, Some(&parent.id))
                .map_err(WerkError::SdError)?;
        }
    }

    Ok(())
}
