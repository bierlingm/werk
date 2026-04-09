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

/// Operative instrument for structural dynamics practice.
///
/// Holds structural tensions — the gap between desired outcome and current
/// reality — and computes temporal facts from the standards you set. The
/// instrument surfaces signals by exception and does not interpret. It serves
/// operations (closing gaps), not management (coordinating existing structure).
#[derive(Parser, Debug)]
#[command(name = "werk")]
#[command(version, about, after_long_help = "\
Commands by framework:

  Structure (Architecture of Space)
    add, compose, move, rm, show, tree

  Action (Grammar of Action)
    reality, desire, resolve, release, reopen, hold, position, note

  Time (Calculus of Time)
    horizon, snooze, recur, epoch

  Framing (Logic of Framing)
    list, tree, stats, survey

  System
    init, config, flush, batch, nuke, mcp, serve")]
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

    // Rewrite `werk note <arg> ...` to `werk note add <arg> ...`
    // when <arg> is not a recognized subcommand (add, rm, list).
    // This makes `werk note 42 "text"` work as shorthand for `werk note add 42 "text"`.
    let cli_args: Vec<String> = {
        let mut args: Vec<String> = std::env::args().collect();
        if let Some(note_pos) = args.iter().position(|a| a == "note") {
            let next = args.get(note_pos + 1).map(|s| s.as_str());
            match next {
                Some("add") | Some("rm") | Some("list") | Some("--help") | Some("-h") | None => {}
                Some(_) => {
                    args.insert(note_pos + 1, "add".to_string());
                }
            }
        }
        args
    };
    let args = Cli::parse_from(cli_args);
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
        Commands::Compose {
            desired,
            actual,
            children,
        } => werk::commands::compose_up::cmd_compose_up(&output, desired, actual, children),

        Commands::Flush => werk::commands::flush::cmd_flush(&output),
        Commands::Log { id, search, since, compare, session } => werk::commands::log::cmd_log(&output, id, search, since, compare, session),
        Commands::Epoch { id, list, show } => werk::commands::epoch::cmd_epoch(&output, id, list, show),
        Commands::Horizon { id, value } => werk::commands::horizon::cmd_horizon(&output, id, value),
        Commands::Show { id, full } => werk::commands::show::cmd_show(&output, id, full),
        Commands::Reality { id, value, no_epoch } => werk::commands::reality::cmd_reality(&output, id, value, no_epoch),
        Commands::Desire { id, value, no_epoch } => werk::commands::desire::cmd_desire(&output, id, value, no_epoch),
        Commands::Resolve { id, actual_at, dry_run } => werk::commands::resolve::cmd_resolve(&output, id, actual_at, dry_run),
        Commands::Release { id, reason } => {
            werk::commands::release::cmd_release(&output, id, reason)
        }
        Commands::Reopen { id, reason } => werk::commands::reopen::cmd_reopen(&output, id, reason),
        Commands::Undo { gesture_id, last, dry_run } => {
            werk::commands::undo::cmd_undo(&output, gesture_id, last, dry_run)
        }
        Commands::Snooze { id, date, clear } => {
            werk::commands::snooze::cmd_snooze(&output, id, date, clear)
        }
        Commands::Recur {
            id,
            interval,
            clear,
        } => werk::commands::recur::cmd_recur(&output, id, interval, clear),
        Commands::Rm { id, dry_run } => werk::commands::rm::cmd_rm(&output, id, dry_run),
        Commands::Move { id, parent, dry_run } => werk::commands::move_cmd::cmd_move(&output, id, parent, dry_run),
        Commands::Split { id, desires, assign, children_to_parent, children_to, keep, release, hold, dry_run } => {
            werk::commands::split::cmd_split(&output, id, desires, assign, children_to_parent, children_to, keep, release, hold, dry_run)
        }
        Commands::Merge { id1, id2, into, as_desire, desire, assign, children_to_parent, dry_run } => {
            werk::commands::merge::cmd_merge(&output, id1, id2, into, as_desire, desire, assign, children_to_parent, dry_run)
        }
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
            status,
            overdue,
            approaching,
            stale,
            held,
            positioned,
            root,
            parent,
            has_deadline,
            changed,
            signals,
            sort,
            reverse,
            tree,
            long,
            search,
        } => {
            let sig = werk::commands::load_signal_thresholds();
            let params = werk::commands::list::ListParams {
                all,
                status,
                overdue,
                approaching: approaching.map(|opt| opt.unwrap_or(sig.approaching_days)),
                stale: stale.map(|opt| opt.unwrap_or(sig.stale_days)),
                held,
                positioned,
                root,
                parent,
                has_deadline,
                changed,
                signals,
                sort,
                reverse,
                tree,
                long,
                search,
            };
            werk::commands::list::cmd_list(&output, params)
        }
        Commands::Tree {
            id,
            open,
            all,
            resolved,
            released,
            stats,
        } => werk::commands::tree::cmd_tree(&output, id, open, all, resolved, released, stats),
        Commands::Health { repair, yes } => werk::commands::health::cmd_health(&output, repair, yes),
        Commands::Insights { days } => werk::commands::insights::cmd_insights(&output, days),
        Commands::Survey { days } => werk::commands::survey::cmd_survey(&output, days),
        Commands::Ground { days } => werk::commands::ground::cmd_ground(&output, days),
        Commands::Diff { since, verbose } => werk::commands::diff::cmd_diff(&output, since, verbose),
        Commands::Stats {
            temporal,
            attention,
            changes,
            trajectory: traj,
            engagement,
            drift,
            health,
            all,
            days,
            repair,
            yes,
        } => werk::commands::stats::cmd_stats(
            &output, temporal, attention, changes, traj, engagement, drift, health, all, days, repair, yes,
        ),
        Commands::Trajectory { id, collisions } => {
            werk::commands::trajectory::cmd_trajectory(&output, id, collisions)
        }
        Commands::Context { id, all, urgent } => werk::commands::context::cmd_context(&output, id, all, urgent),
        Commands::Batch { command } => werk::commands::batch::cmd_batch(&output, &command),
        Commands::Hooks { command } => {
            use werk::commands::hooks::*;
            use werk::commands::HooksCommand;
            match command {
                HooksCommand::List { verbose } => cmd_hooks_list(&output, verbose),
                HooksCommand::Add { event, command, filter, global } => {
                    cmd_hooks_add(&output, event, command, filter, global)
                }
                HooksCommand::Rm { event, command, global } => {
                    cmd_hooks_rm(&output, event, command, global)
                }
                HooksCommand::Test { event, tension } => cmd_hooks_test(&output, event, tension),
                HooksCommand::Log { tail } => cmd_hooks_log(&output, tail),
                HooksCommand::Install { git, hooks } => cmd_hooks_install(&output, git, hooks),
            }
        }
        Commands::Mcp => {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| werk::error::WerkError::IoError(format!("failed to create runtime: {}", e)));
            match rt {
                Ok(rt) => rt.block_on(async {
                    werk_mcp::run_server().await
                        .map_err(|e| werk::error::WerkError::IoError(e.to_string()))
                }),
                Err(e) => Err(e),
            }
        }
        Commands::Serve { port } => {
            let workspace = werk_shared::Workspace::discover()
                .map_err(|e| werk::error::WerkError::IoError(e.to_string()));
            match workspace {
                Ok(ws) => {
                    let rt = tokio::runtime::Runtime::new()
                        .map_err(|e| werk::error::WerkError::IoError(format!("failed to create runtime: {}", e)));
                    match rt {
                        Ok(rt) => rt.block_on(async {
                            werk_web::serve(ws.root().to_path_buf(), port).await
                                .map_err(|e| werk::error::WerkError::IoError(e.to_string()))
                        }),
                        Err(e) => Err(e),
                    }
                }
                Err(e) => Err(e),
            }
        }
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
