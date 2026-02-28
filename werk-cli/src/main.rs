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
        Commands::Note { id, text } => cmd_note(&output, id, text),
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

fn cmd_reality(output: &Output, _id: String, _value: Option<String>) -> Result<(), WerkError> {
    let _ = output.error("not implemented: reality command coming soon");
    Err(WerkError::InvalidInput(
        "command not implemented".to_string(),
    ))
}

fn cmd_desire(output: &Output, _id: String, _value: Option<String>) -> Result<(), WerkError> {
    let _ = output.error("not implemented: desire command coming soon");
    Err(WerkError::InvalidInput(
        "command not implemented".to_string(),
    ))
}

fn cmd_resolve(output: &Output, _id: String) -> Result<(), WerkError> {
    let _ = output.error("not implemented: resolve command coming soon");
    Err(WerkError::InvalidInput(
        "command not implemented".to_string(),
    ))
}

fn cmd_release(output: &Output, _id: String, _reason: String) -> Result<(), WerkError> {
    let _ = output.error("not implemented: release command coming soon");
    Err(WerkError::InvalidInput(
        "command not implemented".to_string(),
    ))
}

fn cmd_rm(output: &Output, _id: String) -> Result<(), WerkError> {
    let _ = output.error("not implemented: rm command coming soon");
    Err(WerkError::InvalidInput(
        "command not implemented".to_string(),
    ))
}

fn cmd_move(output: &Output, _id: String, _parent: Option<String>) -> Result<(), WerkError> {
    let _ = output.error("not implemented: move command coming soon");
    Err(WerkError::InvalidInput(
        "command not implemented".to_string(),
    ))
}

fn cmd_note(output: &Output, _id: Option<String>, _text: String) -> Result<(), WerkError> {
    let _ = output.error("not implemented: note command coming soon");
    Err(WerkError::InvalidInput(
        "command not implemented".to_string(),
    ))
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
