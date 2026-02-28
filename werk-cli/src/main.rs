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

fn cmd_init(output: &Output, _global: bool) -> Result<(), WerkError> {
    let _ = output.error("not implemented: init command coming soon");
    Err(WerkError::InvalidInput(
        "command not implemented".to_string(),
    ))
}

fn cmd_config(output: &Output, _command: werk::commands::ConfigCommand) -> Result<(), WerkError> {
    let _ = output.error("not implemented: config command coming soon");
    Err(WerkError::InvalidInput(
        "command not implemented".to_string(),
    ))
}

fn cmd_add(
    output: &Output,
    _desired: Option<String>,
    _actual: Option<String>,
    _parent: Option<String>,
) -> Result<(), WerkError> {
    let _ = output.error("not implemented: add command coming soon");
    Err(WerkError::InvalidInput(
        "command not implemented".to_string(),
    ))
}

fn cmd_show(output: &Output, _id: String, _verbose: bool) -> Result<(), WerkError> {
    let _ = output.error("not implemented: show command coming soon");
    Err(WerkError::InvalidInput(
        "command not implemented".to_string(),
    ))
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
