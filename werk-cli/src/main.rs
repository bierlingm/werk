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

    // Check mutation status before dispatch consumes the command.
    let is_mutation = args.command.is_mutation();

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
        Commands::ComposeUp {
            desired,
            actual,
            children,
        } => werk::commands::compose_up::cmd_compose_up(&output, desired, actual, children),

        Commands::Flush => werk::commands::flush::cmd_flush(&output),
        Commands::Epoch { id, list, show } => werk::commands::epoch::cmd_epoch(&output, id, list, show),
        Commands::Horizon { id, value } => werk::commands::horizon::cmd_horizon(&output, id, value),
        Commands::Show { id } => werk::commands::show::cmd_show(&output, id),
        Commands::Reality { id, value, no_epoch } => werk::commands::reality::cmd_reality(&output, id, value, no_epoch),
        Commands::Desire { id, value, no_epoch } => werk::commands::desire::cmd_desire(&output, id, value, no_epoch),
        Commands::Resolve { id, actual_at } => werk::commands::resolve::cmd_resolve(&output, id, actual_at),
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
        Commands::Hold { id } => werk::commands::hold::cmd_hold(&output, id),
        Commands::Position { id, n } => werk::commands::position::cmd_position(&output, id, n),
        Commands::Note { command } => match command {
            werk::commands::NoteCommand::Add { arg1, arg2 } => {
                werk::commands::note::cmd_note_add(&output, arg1, arg2)
            }
            werk::commands::NoteCommand::Rm { arg1, arg2 } => {
                werk::commands::note::cmd_note_rm(&output, arg1, arg2)
            }
            werk::commands::NoteCommand::List { id } => {
                werk::commands::note::cmd_note_list(&output, id)
            }
        },
        Commands::List {
            all,
            urgent,
            neglected,
            stagnant,
            sort,
        } => werk::commands::list::cmd_list(&output, all, urgent, neglected, stagnant, sort),
        Commands::Tree {
            open,
            all,
            resolved,
            released,
        } => werk::commands::tree::cmd_tree(&output, open, all, resolved, released),
        Commands::Health => werk::commands::health::cmd_health(&output),
        Commands::Insights { days } => werk::commands::insights::cmd_insights(&output, days),
        Commands::Survey { days } => werk::commands::survey::cmd_survey(&output, days),
        Commands::Ground { days } => werk::commands::ground::cmd_ground(&output, days),
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
        Commands::Watch {
            daemon,
            stop,
            status,
            pending,
            history,
        } => werk::commands::watch::cmd_watch(&output, daemon, stop, status, pending, history),
        Commands::Batch { command } => werk::commands::batch::cmd_batch(&output, &command),
        Commands::Nuke { confirm, global } => {
            werk::commands::nuke::cmd_nuke(&output, confirm, global)
        }
    };

    match result {
        Ok(()) => {
            if is_mutation {
                werk::commands::flush::autoflush();
            }
            std::process::exit(0);
        }
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
