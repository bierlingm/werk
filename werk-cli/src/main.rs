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
        Commands::Note { arg1, arg2 } => cmd_note(&output, arg1, arg2),
        Commands::Notes => cmd_notes(&output),
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

fn cmd_config(output: &Output, command: werk::commands::ConfigCommand) -> Result<(), WerkError> {
    use serde::Serialize;
    use werk::commands::config::Config;
    use werk::workspace::Workspace;

    /// JSON output structure for config set.
    #[derive(Serialize)]
    struct ConfigSetResult {
        key: String,
        value: String,
        path: String,
    }

    /// JSON output structure for config get.
    #[derive(Serialize)]
    struct ConfigGetResult {
        key: String,
        value: String,
    }

    match command {
        werk::commands::ConfigCommand::Set { key, value } => {
            // Validate key is not empty
            if key.is_empty() {
                return Err(WerkError::InvalidInput(
                    "config key cannot be empty".to_string(),
                ));
            }

            // Try to find a local workspace first, fall back to global
            let workspace_result = Workspace::discover();
            let mut config = match workspace_result {
                Ok(ws) => Config::load(&ws)?,
                Err(_) => {
                    // No local workspace - use global config
                    Config::load_global()?
                }
            };

            // Set the value
            config.set(&key, value.clone());

            // Save
            config.save()?;

            // Output
            let path = config
                .path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            if output.is_json() {
                let result = ConfigSetResult { key, value, path };
                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
                println!("{}", json);
            } else {
                output
                    .success(&format!(
                        "Set {} = {}",
                        key,
                        output.styled(&value, werk::output::ColorStyle::Highlight)
                    ))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }

            Ok(())
        }
        werk::commands::ConfigCommand::Get { key } => {
            // Validate key is not empty
            if key.is_empty() {
                return Err(WerkError::InvalidInput(
                    "config key cannot be empty".to_string(),
                ));
            }

            // Try to find a local workspace first, fall back to global
            let workspace_result = Workspace::discover();
            let config = match workspace_result {
                Ok(ws) => Config::load(&ws)?,
                Err(_) => {
                    // No local workspace - use global config
                    Config::load_global()?
                }
            };

            // Get the value
            match config.get(&key) {
                Some(value) => {
                    if output.is_json() {
                        let result = ConfigGetResult {
                            key,
                            value: value.clone(),
                        };
                        let json = serde_json::to_string_pretty(&result).map_err(|e| {
                            WerkError::IoError(format!("failed to serialize JSON: {}", e))
                        })?;
                        println!("{}", json);
                    } else {
                        println!(
                            "{} = {}",
                            output.styled(&key, werk::output::ColorStyle::Info),
                            output.styled(value, werk::output::ColorStyle::Highlight)
                        );
                    }
                    Ok(())
                }
                None => Err(WerkError::ConfigError(format!(
                    "config key '{}' not found",
                    key
                ))),
            }
        }
    }
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

fn cmd_note(output: &Output, arg1: Option<String>, arg2: Option<String>) -> Result<(), WerkError> {
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

    // Parse arguments: determine ID and text
    let (id, text) = match (arg1, arg2) {
        (None, None) => {
            return Err(WerkError::InvalidInput(
                "note text is required: werk note <text> or werk note <id> <text>".to_string(),
            ));
        }
        (Some(text), None) => {
            // Single argument: treat as workspace note
            (None, text)
        }
        (Some(id), Some(text)) => {
            // Two arguments: first is ID, second is text
            (Some(id), text)
        }
        (None, Some(_)) => {
            // This shouldn't happen with clap, but handle it
            unreachable!("arg2 without arg1")
        }
    };

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

fn cmd_notes(output: &Output) -> Result<(), WerkError> {
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for notes command.
    #[derive(Serialize)]
    struct NotesResult {
        notes: Vec<NoteInfo>,
    }

    /// Note information for display.
    #[derive(Serialize)]
    struct NoteInfo {
        timestamp: String,
        text: String,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get workspace-level notes (mutations on the WORKSPACE_NOTES sentinel)
    const WORKSPACE_NOTE_TENSION_ID: &str = "WORKSPACE_NOTES";
    let mutations = store
        .get_mutations(WORKSPACE_NOTE_TENSION_ID)
        .map_err(WerkError::StoreError)?;

    // Filter for note mutations only
    let notes: Vec<NoteInfo> = mutations
        .into_iter()
        .filter(|m| m.field() == "note")
        .map(|m| NoteInfo {
            timestamp: m.timestamp().to_rfc3339(),
            text: m.new_value().to_owned(),
        })
        .collect();

    if output.is_json() {
        let result = NotesResult { notes };
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        if notes.is_empty() {
            output
                .info("No workspace notes")
                .map_err(|e| WerkError::IoError(e.to_string()))?;
        } else {
            output
                .success(&format!("Workspace notes ({})", notes.len()))
                .map_err(|e| WerkError::IoError(e.to_string()))?;
            for (i, note) in notes.iter().enumerate() {
                println!(
                    "\n{}. {}",
                    i + 1,
                    output.styled(&note.text, werk::output::ColorStyle::Highlight)
                );
                println!(
                    "   {}",
                    output.styled(
                        &note.timestamp[..19].replace('T', " "),
                        werk::output::ColorStyle::Muted
                    )
                );
            }
        }
    }

    Ok(())
}

fn cmd_tree(
    output: &Output,
    _open: bool,
    all: bool,
    resolved: bool,
    released: bool,
) -> Result<(), WerkError> {
    use chrono::Utc;
    use sd_core::{
        classify_creative_cycle_phase, detect_structural_conflict,
        predict_structural_tendency, ConflictThresholds, Forest, LifecycleThresholds,
        TensionStatus,
    };
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for a tension in tree.
    #[derive(Serialize)]
    struct TensionJson {
        id: String,
        desired: String,
        actual: String,
        status: String,
        parent_id: Option<String>,
        created_at: String,
        phase: String,
        movement: String,
        has_conflict: bool,
    }

    /// JSON output structure for tree.
    #[derive(Serialize)]
    struct TreeJson {
        tensions: Vec<TensionJson>,
        summary: TreeSummary,
    }

    #[derive(Serialize)]
    struct TreeSummary {
        total: usize,
        active: usize,
        resolved: usize,
        released: usize,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let all_mutations = store.all_mutations().map_err(WerkError::StoreError)?;

    // Build forest
    let forest = Forest::from_tensions(tensions.clone())
        .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

    // Determine filter
    let filter = if all {
        Filter::All
    } else if resolved {
        Filter::Resolved
    } else if released {
        Filter::Released
    } else {
        // Default: --open (active only)
        Filter::Active
    };

    // Filter tensions
    let filtered_tensions: Vec<_> = tensions
        .iter()
        .filter(|t| match filter {
            Filter::All => true,
            Filter::Active => t.status == TensionStatus::Active,
            Filter::Resolved => t.status == TensionStatus::Resolved,
            Filter::Released => t.status == TensionStatus::Released,
        })
        .collect();

    // Handle empty forest
    if filtered_tensions.is_empty() {
        if output.is_json() {
            let result = TreeJson {
                tensions: vec![],
                summary: TreeSummary {
                    total: 0,
                    active: 0,
                    resolved: 0,
                    released: 0,
                },
            };
            let json = serde_json::to_string_pretty(&result)
                .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
            println!("{}", json);
        } else {
            output
                .info("No tensions found")
                .map_err(|e| WerkError::IoError(e.to_string()))?;
        }
        return Ok(());
    }

    // Compute dynamics for each tension
    let now = Utc::now();
    let thresholds = LifecycleThresholds::default();
    let conflict_thresholds = ConflictThresholds::default();

    // Get resolved tensions for momentum phase detection
    let resolved_tensions: Vec<_> = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Resolved)
        .cloned()
        .collect();

    // Build a map of tension ID to computed dynamics
    let mut dynamics_map: std::collections::HashMap<String, (String, String, bool)> =
        std::collections::HashMap::new();

    for tension in &filtered_tensions {
        // Get mutations for this tension
        let mutations: Vec<_> = all_mutations
            .iter()
            .filter(|m| m.tension_id() == tension.id)
            .cloned()
            .collect();

        // Classify phase
        let phase_result = classify_creative_cycle_phase(
            tension,
            &mutations,
            &resolved_tensions,
            &thresholds,
            now,
        );
        let phase_badge = match phase_result.phase {
            sd_core::CreativeCyclePhase::Germination => "[G]",
            sd_core::CreativeCyclePhase::Assimilation => "[A]",
            sd_core::CreativeCyclePhase::Completion => "[C]",
            sd_core::CreativeCyclePhase::Momentum => "[M]",
        };

        // Detect conflict with siblings
        let has_conflict = detect_structural_conflict(
            &forest,
            &tension.id,
            &all_mutations,
            &conflict_thresholds,
            now,
        )
        .is_some();

        // Predict movement tendency
        let tendency = predict_structural_tendency(tension, has_conflict);
        let movement_signal = match tendency.tendency {
            sd_core::StructuralTendency::Advancing => "→",
            sd_core::StructuralTendency::Oscillating => "↔",
            sd_core::StructuralTendency::Stagnant => "○",
        };

        dynamics_map.insert(
            tension.id.clone(),
            (
                phase_badge.to_string(),
                movement_signal.to_string(),
                has_conflict,
            ),
        );
    }

    // If JSON output, build JSON structure
    if output.is_json() {
        let json_tensions: Vec<TensionJson> = filtered_tensions
            .iter()
            .map(|t| {
                let (phase, movement, has_conflict) = dynamics_map.get(&t.id).cloned().unwrap_or((
                    "[G]".to_string(),
                    "○".to_string(),
                    false,
                ));
                TensionJson {
                    id: t.id.clone(),
                    desired: t.desired.clone(),
                    actual: t.actual.clone(),
                    status: t.status.to_string(),
                    parent_id: t.parent_id.clone(),
                    created_at: t.created_at.to_rfc3339(),
                    phase: phase.replace("[", "").replace("]", ""),
                    movement: movement.to_string(),
                    has_conflict,
                }
            })
            .collect();

        // Count by status
        let active_count = tensions
            .iter()
            .filter(|t| t.status == TensionStatus::Active)
            .count();
        let resolved_count = tensions
            .iter()
            .filter(|t| t.status == TensionStatus::Resolved)
            .count();
        let released_count = tensions
            .iter()
            .filter(|t| t.status == TensionStatus::Released)
            .count();

        let result = TreeJson {
            tensions: json_tensions,
            summary: TreeSummary {
                total: tensions.len(),
                active: active_count,
                resolved: resolved_count,
                released: released_count,
            },
        };

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
        return Ok(());
    }

    // Human-readable tree output
    // Build filtered forest for display
    let filtered_ids: std::collections::HashSet<_> =
        filtered_tensions.iter().map(|t| t.id.as_str()).collect();

    // Traverse and render the forest
    fn render_tree(
        forest: &Forest,
        root_ids: &[String],
        filtered_ids: &std::collections::HashSet<&str>,
        dynamics_map: &std::collections::HashMap<String, (String, String, bool)>,
        output: &Output,
        prefix: &str,
        lines: &mut Vec<String>,
    ) {
        let roots: Vec<_> = root_ids
            .iter()
            .filter(|id| filtered_ids.contains(id.as_str()))
            .filter_map(|id| forest.find(id))
            .collect();

        for (i, node) in roots.iter().enumerate() {
            let is_last = i == roots.len() - 1;

            // Get dynamics
            let (phase, movement, has_conflict) = dynamics_map.get(node.id()).cloned().unwrap_or((
                "[G]".to_string(),
                "○".to_string(),
                false,
            ));

            // Status style
            let status_style = match node.tension.status {
                TensionStatus::Active => werk::output::ColorStyle::Active,
                TensionStatus::Resolved => werk::output::ColorStyle::Resolved,
                TensionStatus::Released => werk::output::ColorStyle::Released,
            };

            // Build the line
            let connector = if is_last { "└── " } else { "├── " };

            // Conflict marker
            let conflict_marker = if has_conflict { "!" } else { " " };

            // Format: prefix + connector + [badge] status conflict movement desired
            let id_short = &node.id()[..8.min(node.id().len())];
            let line = format!(
                "{}{}{}{} {} {}{} {}",
                prefix,
                connector,
                output.styled(&phase, werk::output::ColorStyle::Info),
                output.styled(&node.tension.status.to_string(), status_style),
                output.styled(id_short, werk::output::ColorStyle::Id),
                conflict_marker,
                movement,
                output.styled(
                    &truncate(&node.tension.desired, 50),
                    werk::output::ColorStyle::Highlight
                )
            );
            lines.push(line);

            // Recurse for children (only those that pass the filter)
            let children: Vec<_> = node
                .children
                .iter()
                .filter(|id| filtered_ids.contains(id.as_str()))
                .filter_map(|id| forest.find(id))
                .collect();

            if !children.is_empty() {
                let new_prefix = if is_last {
                    format!("{}    ", prefix)
                } else {
                    format!("{}│   ", prefix)
                };
                render_tree(
                    forest,
                    &node.children,
                    filtered_ids,
                    dynamics_map,
                    output,
                    &new_prefix,
                    lines,
                );
            }
        }
    }

    let mut lines = Vec::new();
    render_tree(
        &forest,
        forest.root_ids(),
        &filtered_ids,
        &dynamics_map,
        output,
        "",
        &mut lines,
    );

    // Print tree
    for line in &lines {
        println!("{}", line);
    }

    // Print summary footer
    let active_count = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Active)
        .count();
    let resolved_count = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Resolved)
        .count();
    let released_count = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Released)
        .count();

    println!();
    println!(
        "Total: {}  Active: {}  Resolved: {}  Released: {}",
        output.styled(
            &format!("{}", tensions.len()),
            werk::output::ColorStyle::Highlight
        ),
        output.styled(
            &format!("{}", active_count),
            werk::output::ColorStyle::Active
        ),
        output.styled(
            &format!("{}", resolved_count),
            werk::output::ColorStyle::Resolved
        ),
        output.styled(
            &format!("{}", released_count),
            werk::output::ColorStyle::Released
        )
    );

    Ok(())
}

/// Filter for tree display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Active,
    Resolved,
    Released,
}

/// Truncate a string to max length, adding ellipsis if needed.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
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
