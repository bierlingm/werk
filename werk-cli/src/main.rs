// werk: Operative instrument for structural dynamics
//
// The practitioner's workspace. Practice, presence, oracle.
// Built on sd-core. Maximally opinionated.
//
// Exit codes:
//   0 - Success
//   1 - User error (bad input, not found, invalid operation)
//   2 - Internal error (unexpected failure)

#![forbid(unsafe_code)]

use clap::Parser;
use werk::commands::Commands;
use werk::error::WerkError;
use werk::output::Output;

/// Operative instrument for structural dynamics.
#[derive(Parser, Debug)]
#[command(name = "werk")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Output in JSON format.
    #[arg(short, long, global = true)]
    json: bool,

    /// Disable colored output.
    #[arg(long, global = true)]
    no_color: bool,

    /// Subcommand to execute.
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    let args = Cli::parse();
    let output = Output::new(args.json, args.no_color);

    // Dispatch to subcommand handlers
    let result = match args.command {
        Commands::Init { global } => cmd_init(&output, global),
        Commands::Config { command } => cmd_config(&output, command),
        Commands::Add {
            desired,
            actual,
            parent,
        } => cmd_add(&output, desired, actual, parent),
        Commands::Show { id, verbose } => cmd_show(&output, id, verbose),
        Commands::Reality { id, value } => cmd_reality(&output, id, value),
        Commands::Desire { id, value } => cmd_desire(&output, id, value),
        Commands::Resolve { id } => cmd_resolve(&output, id),
        Commands::Release { id, reason } => cmd_release(&output, id, reason),
        Commands::Rm { id } => cmd_rm(&output, id),
        Commands::Move { id, parent } => cmd_move(&output, id, parent),
        Commands::Note { text, id } => cmd_note(&output, id, text),
        Commands::Tree {
            open,
            all,
            resolved,
            released,
        } => cmd_tree(&output, open, all, resolved, released),
        Commands::Context { id } => cmd_context(&output, id),
        Commands::Run { id, command } => cmd_run(&output, id, command),
    };

    match result {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            let _ = output.error(&e.to_string());
            std::process::exit(e.exit_code());
        }
    }
}

// Stub implementations for subcommands.
// These will be implemented in future features.

fn cmd_init(output: &Output, global: bool) -> Result<(), WerkError> {
    use serde::Serialize;
    use std::path::PathBuf;

    /// JSON output structure for init command.
    #[derive(Serialize)]
    struct InitResult {
        path: String,
        created: bool,
    }

    let cwd = std::env::current_dir()
        .map_err(|e| WerkError::IoError(format!("failed to get current directory: {}", e)))?;

    // Determine target path
    let target_path: PathBuf = if global {
        dirs::home_dir()
            .ok_or_else(|| WerkError::IoError("cannot determine home directory".to_string()))?
    } else {
        cwd.clone()
    };

    // Check if workspace already exists
    let werk_dir = target_path.join(".werk");
    let db_path = werk_dir.join("sd.db");
    let already_exists = db_path.exists();

    // Initialize the store (this creates .werk/ and sd.db)
    // Store::init is idempotent - it won't overwrite existing data
    let _store = sd_core::Store::init(&target_path)?;

    let result = InitResult {
        path: werk_dir.to_string_lossy().to_string(),
        created: !already_exists,
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        let message = if already_exists {
            format!("Workspace already initialized at {}", werk_dir.display())
        } else {
            format!("Workspace initialized at {}", werk_dir.display())
        };
        output
            .success(&message)
            .map_err(|e| WerkError::IoError(e.to_string()))?;
    }

    Ok(())
}

fn cmd_config(output: &Output, _command: werk::commands::ConfigCommand) -> Result<(), WerkError> {
    let _ = output.error("not implemented: config command coming soon");
    Err(WerkError::InvalidInput(
        "command not implemented".to_string(),
    ))
}

fn cmd_add(
    output: &Output,
    desired: Option<String>,
    actual: Option<String>,
    parent: Option<String>,
) -> Result<(), WerkError> {
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for add command.
    #[derive(Serialize)]
    struct AddResult {
        id: String,
        desired: String,
        actual: String,
        status: String,
        parent_id: Option<String>,
    }

    // Require both desired and actual as positional args
    let desired = desired.ok_or_else(|| {
        WerkError::InvalidInput(
            "desired state is required: werk add <desired> <actual>".to_string(),
        )
    })?;
    let actual = actual.ok_or_else(|| {
        WerkError::InvalidInput("actual state is required: werk add <desired> <actual>".to_string())
    })?;

    // Validate non-empty
    if desired.is_empty() {
        return Err(WerkError::InvalidInput(
            "desired state cannot be empty".to_string(),
        ));
    }
    if actual.is_empty() {
        return Err(WerkError::InvalidInput(
            "actual state cannot be empty".to_string(),
        ));
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Resolve parent if provided
    let parent_id = if let Some(parent_prefix) = parent {
        let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
        let resolver = werk::prefix::PrefixResolver::new(tensions);
        let parent_tension = resolver.resolve(&parent_prefix)?;
        Some(parent_tension.id.clone())
    } else {
        None
    };

    // Create the tension
    let tension = store.create_tension_with_parent(&desired, &actual, parent_id.clone())?;

    let result = AddResult {
        id: tension.id.clone(),
        desired: tension.desired.clone(),
        actual: tension.actual.clone(),
        status: tension.status.to_string(),
        parent_id,
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension.id, werk::output::ColorStyle::Id);
        let status_styled = output.styled(
            &tension.status.to_string(),
            werk::output::ColorStyle::Active,
        );
        output
            .success(&format!("Created tension {}", id_styled))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!(
            "  Desired: {}",
            output.styled(&tension.desired, werk::output::ColorStyle::Highlight)
        );
        println!(
            "  Actual:  {}",
            output.styled(&tension.actual, werk::output::ColorStyle::Muted)
        );
        println!("  Status:  {}", status_styled);
        if let Some(pid) = &tension.parent_id {
            println!(
                "  Parent:  {}",
                output.styled(pid, werk::output::ColorStyle::Id)
            );
        }
    }

    Ok(())
}

fn cmd_show(output: &Output, id: String, verbose: bool) -> Result<(), WerkError> {
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for show command.
    #[derive(Serialize)]
    struct ShowResult {
        id: String,
        desired: String,
        actual: String,
        status: String,
        parent_id: Option<String>,
        created_at: String,
        mutations: Vec<MutationInfo>,
    }

    /// Mutation information for display.
    #[derive(Serialize)]
    struct MutationInfo {
        timestamp: String,
        field: String,
        old_value: Option<String>,
        new_value: String,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = werk::prefix::PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Get mutations
    let mutations = store
        .get_mutations(&tension.id)
        .map_err(WerkError::StoreError)?;

    // Build mutation info
    let mutation_infos: Vec<MutationInfo> = mutations
        .iter()
        .map(|m| MutationInfo {
            timestamp: m.timestamp().to_rfc3339(),
            field: m.field().to_owned(),
            old_value: m.old_value().map(|s| s.to_owned()),
            new_value: m.new_value().to_owned(),
        })
        .collect();

    let result = ShowResult {
        id: tension.id.clone(),
        desired: tension.desired.clone(),
        actual: tension.actual.clone(),
        status: tension.status.to_string(),
        parent_id: tension.parent_id.clone(),
        created_at: tension.created_at.to_rfc3339(),
        mutations: mutation_infos,
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension.id, werk::output::ColorStyle::Id);
        let status_style = match tension.status {
            sd_core::TensionStatus::Active => werk::output::ColorStyle::Active,
            sd_core::TensionStatus::Resolved => werk::output::ColorStyle::Resolved,
            sd_core::TensionStatus::Released => werk::output::ColorStyle::Released,
        };
        let status_styled = output.styled(&tension.status.to_string(), status_style);

        println!("Tension {}", id_styled);
        println!(
            "  Desired:    {}",
            output.styled(&tension.desired, werk::output::ColorStyle::Highlight)
        );
        println!(
            "  Actual:     {}",
            output.styled(&tension.actual, werk::output::ColorStyle::Muted)
        );
        println!("  Status:     {}", status_styled);
        println!(
            "  Created:    {}",
            output.styled(
                &tension
                    .created_at
                    .format("%Y-%m-%d %H:%M:%S UTC")
                    .to_string(),
                werk::output::ColorStyle::Muted
            )
        );

        if let Some(pid) = &tension.parent_id {
            println!(
                "  Parent:     {}",
                output.styled(pid, werk::output::ColorStyle::Id)
            );
        }

        // Mutation count
        println!(
            "  Mutations:  {}",
            output.styled(
                &format!("{}", result.mutations.len()),
                werk::output::ColorStyle::Info
            )
        );

        // Show mutations if verbose or if there are any beyond creation
        if verbose || result.mutations.len() > 1 {
            println!("\n  Mutation History:");
            for m in &result.mutations {
                let old = m.old_value.as_deref().unwrap_or("(none)");
                println!(
                    "    {} [{}] {} -> {}",
                    output.styled(
                        &m.timestamp[..19].replace('T', " "),
                        werk::output::ColorStyle::Muted
                    ),
                    output.styled(&m.field, werk::output::ColorStyle::Info),
                    output.styled(old, werk::output::ColorStyle::Muted),
                    output.styled(&m.new_value, werk::output::ColorStyle::Highlight)
                );
            }
        }

        // Note about --verbose for future dynamics display
        if !verbose {
            let _ = verbose; // suppress unused warning
        }
    }

    Ok(())
}

fn cmd_reality(output: &Output, id: String, value: Option<String>) -> Result<(), WerkError> {
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for reality command.
    #[derive(Serialize)]
    struct RealityResult {
        id: String,
        actual: String,
        old_actual: String,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = werk::prefix::PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Get the new value - either from argument or editor
    let new_value = match value {
        Some(v) => v,
        None => {
            // Open editor with current actual
            let edited = werk::edit_content(&tension.actual)?;
            match edited {
                Some(v) => v,
                None => {
                    // Editor returned no change - nothing to do
                    if output.is_json() {
                        let result = RealityResult {
                            id: tension.id.clone(),
                            actual: tension.actual.clone(),
                            old_actual: tension.actual.clone(),
                        };
                        let json = serde_json::to_string_pretty(&result).map_err(|e| {
                            WerkError::IoError(format!("failed to serialize JSON: {}", e))
                        })?;
                        println!("{}", json);
                    } else {
                        output
                            .info("No changes made (editor cancelled or content unchanged)")
                            .map_err(|e| WerkError::IoError(e.to_string()))?;
                    }
                    return Ok(());
                }
            }
        }
    };

    // Validate non-empty
    if new_value.is_empty() {
        return Err(WerkError::InvalidInput(
            "actual state cannot be empty".to_string(),
        ));
    }

    // Record old value for output
    let old_actual = tension.actual.clone();

    // Update via store (this handles status validation and mutation recording)
    store
        .update_actual(&tension.id, &new_value)
        .map_err(WerkError::SdError)?;

    let result = RealityResult {
        id: tension.id.clone(),
        actual: new_value,
        old_actual,
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension.id, werk::output::ColorStyle::Id);
        output
            .success(&format!("Updated actual for tension {}", id_styled))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!(
            "  Old:  {}",
            output.styled(&result.old_actual, werk::output::ColorStyle::Muted)
        );
        println!(
            "  New:  {}",
            output.styled(&result.actual, werk::output::ColorStyle::Highlight)
        );
    }

    Ok(())
}

fn cmd_desire(output: &Output, id: String, value: Option<String>) -> Result<(), WerkError> {
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for desire command.
    #[derive(Serialize)]
    struct DesireResult {
        id: String,
        desired: String,
        old_desired: String,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = werk::prefix::PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Get the new value - either from argument or editor
    let new_value = match value {
        Some(v) => v,
        None => {
            // Open editor with current desired
            let edited = werk::edit_content(&tension.desired)?;
            match edited {
                Some(v) => v,
                None => {
                    // Editor returned no change - nothing to do
                    if output.is_json() {
                        let result = DesireResult {
                            id: tension.id.clone(),
                            desired: tension.desired.clone(),
                            old_desired: tension.desired.clone(),
                        };
                        let json = serde_json::to_string_pretty(&result).map_err(|e| {
                            WerkError::IoError(format!("failed to serialize JSON: {}", e))
                        })?;
                        println!("{}", json);
                    } else {
                        output
                            .info("No changes made (editor cancelled or content unchanged)")
                            .map_err(|e| WerkError::IoError(e.to_string()))?;
                    }
                    return Ok(());
                }
            }
        }
    };

    // Validate non-empty
    if new_value.is_empty() {
        return Err(WerkError::InvalidInput(
            "desired state cannot be empty".to_string(),
        ));
    }

    // Record old value for output
    let old_desired = tension.desired.clone();

    // Update via store (this handles status validation and mutation recording)
    store
        .update_desired(&tension.id, &new_value)
        .map_err(WerkError::SdError)?;

    let result = DesireResult {
        id: tension.id.clone(),
        desired: new_value,
        old_desired,
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension.id, werk::output::ColorStyle::Id);
        output
            .success(&format!("Updated desired for tension {}", id_styled))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!(
            "  Old:  {}",
            output.styled(&result.old_desired, werk::output::ColorStyle::Muted)
        );
        println!(
            "  New:  {}",
            output.styled(&result.desired, werk::output::ColorStyle::Highlight)
        );
    }

    Ok(())
}

fn cmd_resolve(output: &Output, id: String) -> Result<(), WerkError> {
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for resolve command.
    #[derive(Serialize)]
    struct ResolveResult {
        id: String,
        status: String,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = werk::prefix::PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Record old status for output
    let old_status = tension.status;

    // Check if already resolved
    if old_status != sd_core::TensionStatus::Active {
        return Err(WerkError::InvalidInput(format!(
            "cannot resolve tension with status {} (must be Active)",
            old_status
        )));
    }

    // Update status via store (handles validation and mutation recording)
    store
        .update_status(&tension.id, sd_core::TensionStatus::Resolved)
        .map_err(WerkError::SdError)?;

    let result = ResolveResult {
        id: tension.id.clone(),
        status: "Resolved".to_string(),
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension.id, werk::output::ColorStyle::Id);
        output
            .success(&format!("Resolved tension {}", id_styled))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!(
            "  Status: {} -> {}",
            output.styled(&old_status.to_string(), werk::output::ColorStyle::Muted),
            output.styled("Resolved", werk::output::ColorStyle::Resolved)
        );
    }

    Ok(())
}

fn cmd_release(output: &Output, id: String, reason: String) -> Result<(), WerkError> {
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for release command.
    #[derive(Serialize)]
    struct ReleaseResult {
        id: String,
        status: String,
        reason: String,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = werk::prefix::PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Record old status for output
    let old_status = tension.status;

    // Check if already resolved/released
    if old_status != sd_core::TensionStatus::Active {
        return Err(WerkError::InvalidInput(format!(
            "cannot release tension with status {} (must be Active)",
            old_status
        )));
    }

    // Update status via store (handles validation and mutation recording)
    store
        .update_status(&tension.id, sd_core::TensionStatus::Released)
        .map_err(WerkError::SdError)?;

    // Record the release reason as a mutation
    use chrono::Utc;
    use sd_core::Mutation;
    store
        .record_mutation(&Mutation::new(
            tension.id.clone(),
            Utc::now(),
            "release_reason".to_owned(),
            None,
            reason.clone(),
        ))
        .map_err(WerkError::SdError)?;

    let result = ReleaseResult {
        id: tension.id.clone(),
        status: "Released".to_string(),
        reason: reason.clone(),
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension.id, werk::output::ColorStyle::Id);
        output
            .success(&format!("Released tension {}", id_styled))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!(
            "  Status: {} -> {}",
            output.styled(&old_status.to_string(), werk::output::ColorStyle::Muted),
            output.styled("Released", werk::output::ColorStyle::Released)
        );
        println!(
            "  Reason: {}",
            output.styled(&reason, werk::output::ColorStyle::Muted)
        );
    }

    Ok(())
}

fn cmd_rm(output: &Output, id: String) -> Result<(), WerkError> {
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for rm command.
    #[derive(Serialize)]
    struct RmResult {
        id: String,
        deleted: bool,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = werk::prefix::PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Record the tension ID before deletion
    let tension_id = tension.id.clone();
    let tension_desired = tension.desired.clone();

    // Delete via store (handles reparenting children to grandparent)
    store
        .delete_tension(&tension_id)
        .map_err(WerkError::SdError)?;

    let result = RmResult {
        id: tension_id.clone(),
        deleted: true,
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension_id, werk::output::ColorStyle::Id);
        output
            .success(&format!("Deleted tension {}", id_styled))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!(
            "  Desired: {}",
            output.styled(&tension_desired, werk::output::ColorStyle::Muted)
        );
    }

    Ok(())
}

fn cmd_move(output: &Output, id: String, parent: Option<String>) -> Result<(), WerkError> {
    use sd_core::Forest;
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for move command.
    #[derive(Serialize)]
    struct MoveResult {
        id: String,
        parent_id: Option<String>,
        old_parent_id: Option<String>,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution and forest building
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = werk::prefix::PrefixResolver::new(tensions.clone());

    // Resolve the tension to move
    let tension = resolver.resolve(&id)?;
    let tension_id = tension.id.clone();
    let old_parent_id = tension.parent_id.clone();

    // Resolve the new parent if provided
    let new_parent_id = if let Some(parent_prefix) = parent {
        // Prevent moving to self
        let parent_tension = resolver.resolve(&parent_prefix)?;
        if parent_tension.id == tension_id {
            return Err(WerkError::InvalidInput(
                "cannot move tension to itself".to_string(),
            ));
        }

        // Check for cycles: new parent cannot be a descendant of the tension being moved
        let forest = Forest::from_tensions(tensions.clone())
            .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

        if let Some(descendants) = forest.descendants(&tension_id) {
            let descendant_ids: std::collections::HashSet<_> =
                descendants.iter().map(|n| n.id()).collect();

            if descendant_ids.contains(parent_tension.id.as_str()) {
                return Err(WerkError::InvalidInput(
                    "cannot move tension under its descendant (would create cycle)".to_string(),
                ));
            }
        }

        Some(parent_tension.id.clone())
    } else {
        None
    };

    // Perform the move via store
    store
        .update_parent(&tension_id, new_parent_id.as_deref())
        .map_err(WerkError::SdError)?;

    let result = MoveResult {
        id: tension_id.clone(),
        parent_id: new_parent_id.clone(),
        old_parent_id,
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension_id, werk::output::ColorStyle::Id);
        match &new_parent_id {
            Some(pid) => {
                output
                    .success(&format!(
                        "Moved tension {} under {}",
                        id_styled,
                        output.styled(pid, werk::output::ColorStyle::Id)
                    ))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
            None => {
                output
                    .success(&format!("Moved tension {} to root", id_styled))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
        }
    }

    Ok(())
}

fn cmd_note(output: &Output, id: Option<String>, text: String) -> Result<(), WerkError> {
    use chrono::Utc;
    use sd_core::Mutation;
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for note command.
    #[derive(Serialize)]
    struct NoteResult {
        id: Option<String>,
        note: String,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    let result = match id {
        Some(id_prefix) => {
            // Note on specific tension
            let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
            let resolver = werk::prefix::PrefixResolver::new(tensions);
            let tension = resolver.resolve(&id_prefix)?;

            // Record note mutation (notes work on any status, no validation needed)
            store
                .record_mutation(&Mutation::new(
                    tension.id.clone(),
                    Utc::now(),
                    "note".to_owned(),
                    None,
                    text.clone(),
                ))
                .map_err(WerkError::SdError)?;

            NoteResult {
                id: Some(tension.id.clone()),
                note: text.clone(),
            }
        }
        None => {
            // General workspace note - store as mutation on a sentinel ID
            // The sentinel is not a real tension but serves as an anchor for workspace-level notes
            const WORKSPACE_NOTE_TENSION_ID: &str = "WORKSPACE_NOTES";

            // Record note mutation on the sentinel
            store
                .record_mutation(&Mutation::new(
                    WORKSPACE_NOTE_TENSION_ID.to_owned(),
                    Utc::now(),
                    "note".to_owned(),
                    None,
                    text.clone(),
                ))
                .map_err(WerkError::SdError)?;

            NoteResult {
                id: None,
                note: text.clone(),
            }
        }
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        match &result.id {
            Some(tid) => {
                output
                    .success(&format!(
                        "Added note to tension {}",
                        output.styled(tid, werk::output::ColorStyle::Id)
                    ))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
            None => {
                output
                    .success("Added workspace note")
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
        }
        println!(
            "  Note: {}",
            output.styled(&text, werk::output::ColorStyle::Muted)
        );
    }

    Ok(())
}

fn cmd_tree(
    output: &Output,
    _open: bool,
    _all: bool,
    _resolved: bool,
    _released: bool,
) -> Result<(), WerkError> {
    let _ = output.error("not implemented: tree command coming soon");
    Err(WerkError::InvalidInput(
        "command not implemented".to_string(),
    ))
}

fn cmd_context(output: &Output, _id: String) -> Result<(), WerkError> {
    let _ = output.error("not implemented: context command coming soon");
    Err(WerkError::InvalidInput(
        "command not implemented".to_string(),
    ))
}

fn cmd_run(output: &Output, _id: String, _command: Vec<String>) -> Result<(), WerkError> {
    let _ = output.error("not implemented: run command coming soon");
    Err(WerkError::InvalidInput(
        "command not implemented".to_string(),
    ))
}
