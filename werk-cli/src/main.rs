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
    #[arg(short, long, global = true)]
    json: bool,

    /// Subcommand to execute.
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    // No args + interactive terminal → launch TUI
    if std::env::args().len() <= 1 && std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        if let Err(e) = werk_tui::run() {
            eprintln!("Error: {e}");
            std::process::exit(2);
        }
        return;
    }

    let args = Cli::parse();
    let output = Output::new(args.json);

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
        Commands::Show { id } => werk::commands::show::cmd_show(&output, id),
        Commands::Reality { id, value } => werk::commands::reality::cmd_reality(&output, id, value),
        Commands::Desire { id, value } => werk::commands::desire::cmd_desire(&output, id, value),
        Commands::Resolve { id } => werk::commands::resolve::cmd_resolve(&output, id),
        Commands::Release { id, reason } => {
            werk::commands::release::cmd_release(&output, id, reason)
        }
        Commands::Reopen { id } => werk::commands::reopen::cmd_reopen(&output, id),
        Commands::Snooze { id, date, clear } => {
            werk::commands::snooze::cmd_snooze(&output, id, date, clear)
        }
        Commands::Recur {
            id,
            interval,
            clear,
        } => werk::commands::recur::cmd_recur(&output, id, interval, clear),
        Commands::Rm { id } => werk::commands::rm::cmd_rm(&output, id),
        Commands::Move { id, parent } => werk::commands::move_cmd::cmd_move(&output, id, parent),
        Commands::Note { arg1, arg2 } => werk::commands::note::cmd_note(&output, arg1, arg2),
        Commands::Notes => werk::commands::notes::cmd_notes(&output),
        Commands::List {
            all,
            urgent,
            neglected,
            stagnant,
            phase,
            sort,
        } => werk::commands::list::cmd_list(&output, all, urgent, neglected, stagnant, phase, sort),
        Commands::Tree {
            open,
            all,
            resolved,
            released,
        } => werk::commands::tree::cmd_tree(&output, open, all, resolved, released),
        Commands::Health => werk::commands::health::cmd_health(&output),
        Commands::Insights { days } => werk::commands::insights::cmd_insights(&output, days),
        Commands::Diff { since } => werk::commands::diff::cmd_diff(&output, since),
        Commands::Trajectory { id, collisions } => {
            werk::commands::trajectory::cmd_trajectory(&output, id, collisions)
        }
        Commands::Context { id, all, urgent } => werk::commands::context::cmd_context(&output, id, all, urgent),
        Commands::Run {
            id,
            prompt,
            no_suggest,
            command,
            system,
            decompose,
            dry_run,
        } => werk::commands::run::cmd_run(&output, id, prompt, no_suggest, command, system, decompose, dry_run),
        Commands::Batch { command } => werk::commands::batch::cmd_batch(&output, &command),
        Commands::Nuke { confirm, global } => {
            werk::commands::nuke::cmd_nuke(&output, confirm, global)
        }
    };

    match result {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            if output.is_json() {
                // Output structured error when --json flag is set
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
