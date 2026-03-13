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
use werk::error::ErrorCode;
use werk::output::Output;

/// Operative instrument for structural dynamics.
#[derive(Parser, Debug)]
#[command(name = "werk")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Output in JSON format.
    #[arg(short, long, global = true, conflicts_with = "toon")]
    json: bool,

    /// Output in TOON format (token-efficient, LLM-optimized).
    #[arg(long, global = true, conflicts_with = "json")]
    toon: bool,

    /// Disable colored output.
    #[arg(long, global = true)]
    no_color: bool,

    /// Subcommand to execute.
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    let args = Cli::parse();
    let output = Output::new_with_toon(args.json, args.toon, args.no_color);

    // Dispatch to subcommand handlers
    let result = match args.command {
        Commands::Init { global } => werk::commands::init::cmd_init(&output, global),
        Commands::Config { command } => werk::commands::config::cmd_config(&output, command.as_ref()),
        Commands::Add {
            desired,
            actual,
            parent,
            horizon,
        } => werk::commands::add::cmd_add(&output, desired, actual, parent, horizon),
        Commands::Horizon { id, value } => werk::commands::horizon::cmd_horizon(&output, id, value),
        Commands::Show { id, verbose } => werk::commands::show::cmd_show(&output, id, verbose),
        Commands::Reality { id, value } => werk::commands::reality::cmd_reality(&output, id, value),
        Commands::Desire { id, value } => werk::commands::desire::cmd_desire(&output, id, value),
        Commands::Resolve { id } => werk::commands::resolve::cmd_resolve(&output, id),
        Commands::Release { id, reason } => {
            werk::commands::release::cmd_release(&output, id, reason)
        }
        Commands::Rm { id } => werk::commands::rm::cmd_rm(&output, id),
        Commands::Move { id, parent } => werk::commands::move_cmd::cmd_move(&output, id, parent),
        Commands::Note { arg1, arg2 } => werk::commands::note::cmd_note(&output, arg1, arg2),
        Commands::Notes => werk::commands::notes::cmd_notes(&output),
        Commands::Tree {
            open,
            all,
            resolved,
            released,
        } => werk::commands::tree::cmd_tree(&output, open, all, resolved, released),
        Commands::Context { id } => werk::commands::context::cmd_context(&output, id),
        Commands::Run {
            id,
            prompt,
            no_suggest,
            command,
        } => werk::commands::run::cmd_run(&output, id, prompt, no_suggest, command),
        Commands::Nuke { confirm, global } => {
            werk::commands::nuke::cmd_nuke(&output, confirm, global)
        }
    };

    match result {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            if output.is_structured() {
                // Output structured error when --json or --toon flag is set
                let code = match e.error_code() {
                    ErrorCode::NOT_FOUND => "NOT_FOUND",
                    ErrorCode::INVALID_INPUT => "INVALID_INPUT",
                    ErrorCode::AMBIGUOUS => "AMBIGUOUS",
                    ErrorCode::NO_WORKSPACE => "NO_WORKSPACE",
                    ErrorCode::PERMISSION_DENIED => "PERMISSION_DENIED",
                    ErrorCode::IO_ERROR => "IO_ERROR",
                    ErrorCode::CONFIG_ERROR => "CONFIG_ERROR",
                    ErrorCode::INTERNAL_ERROR => "INTERNAL_ERROR",
                };
                let _ = output.error_json(code, &e.to_string());
            } else {
                let _ = output.error(&e.to_string());
            }
            std::process::exit(e.exit_code());
        }
    }
}
