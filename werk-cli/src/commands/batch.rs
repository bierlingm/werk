//! Batch command handler.
//!
//! Apply or validate mutations in bulk from a YAML file or stdin.

use crate::agent_response::{Mutation as AgentMutation, StructuredResponse};
use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use chrono::Utc;
use clap::Subcommand;
use sd_core::{DynamicsEngine, Mutation, TensionStatus};

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

    // Discover workspace and open store
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;
    let mut engine = DynamicsEngine::with_store(store);

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
/// Supports two formats:
/// 1. StructuredResponse format (with `mutations` and `response` keys, between `---` markers)
/// 2. Raw YAML array of mutations (bare list without wrapping)
fn parse_mutations(content: &str) -> Result<Vec<AgentMutation>, WerkError> {
    // First try StructuredResponse format (with --- markers)
    if let Some(structured) = StructuredResponse::from_response(content) {
        return Ok(structured.mutations);
    }

    // Wrap in --- markers and add a dummy response, then try again
    let wrapped = format!(
        "---\nmutations:\n{}\nresponse: \"batch import\"\n---",
        content.trim()
    );
    if let Some(structured) = StructuredResponse::from_response(&wrapped) {
        return Ok(structured.mutations);
    }

    // Try parsing as a StructuredResponse directly (no --- markers needed)
    if let Ok(structured) = serde_yaml::from_str::<StructuredResponse>(content) {
        return Ok(structured.mutations);
    }

    // Try parsing as a bare array of mutations
    if let Ok(mutations) = serde_yaml::from_str::<Vec<AgentMutation>>(content) {
        return Ok(mutations);
    }

    Err(WerkError::InvalidInput(
        "could not parse YAML as mutations. Expected either a StructuredResponse \
         (with 'mutations' and 'response' keys) or a bare list of mutation objects."
            .to_string(),
    ))
}

/// Validate a mutation without applying it.
fn validate_mutation(engine: &DynamicsEngine, mutation: &AgentMutation) -> Result<(), WerkError> {
    match mutation {
        AgentMutation::UpdateActual { tension_id, .. }
        | AgentMutation::AddNote { tension_id, .. }
        | AgentMutation::UpdateStatus { tension_id, .. }
        | AgentMutation::UpdateDesired { tension_id, .. } => {
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
        AgentMutation::CreateChild { parent_id, .. } => {
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
    }

    // Validate status values for UpdateStatus
    if let AgentMutation::UpdateStatus { new_status, .. } = mutation {
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
    engine: &mut DynamicsEngine,
    mutation: &AgentMutation,
) -> Result<(), WerkError> {
    match mutation {
        AgentMutation::UpdateActual {
            tension_id,
            new_value,
            ..
        } => {
            engine
                .store()
                .update_actual(tension_id, new_value)
                .map_err(WerkError::SdError)?;
        }
        AgentMutation::CreateChild {
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
        AgentMutation::AddNote {
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
        AgentMutation::UpdateStatus {
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
        AgentMutation::UpdateDesired {
            tension_id,
            new_value,
            ..
        } => {
            engine
                .store()
                .update_desired(tension_id, new_value)
                .map_err(WerkError::SdError)?;
        }
    }

    Ok(())
}
