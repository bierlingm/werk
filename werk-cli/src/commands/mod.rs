//! CLI commands for werk-cli.
//!
//! Each subcommand is defined with clap derive macros and implemented
//! in its own module file.

pub mod add;
pub mod config;
pub mod context;
pub mod desire;
pub mod horizon;
pub mod init;
pub mod move_cmd;
pub mod note;
pub mod notes;
pub mod nuke;
pub mod reality;
pub mod release;
pub mod resolve;
pub mod rm;
pub mod run;
pub mod show;
pub mod tree;

use clap::Subcommand;

/// CLI subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize a workspace (creates .werk/ directory with sd.db).
    Init {
        /// Use global workspace (~/.werk/) instead of local.
        #[arg(short, long)]
        global: bool,
    },

    /// Get or set configuration values.
    Config {
        /// Config subcommand.
        #[command(subcommand)]
        command: ConfigCommand,
    },

    /// Create a new tension.
    Add {
        /// The desired state (what you want).
        desired: Option<String>,

        /// The actual state (current reality).
        actual: Option<String>,

        /// Parent tension ID (creates child tension).
        #[arg(short, long)]
        parent: Option<String>,

        /// Temporal horizon (e.g., "2026", "2026-05", "2026-05-15").
        #[arg(long)]
        horizon: Option<String>,
    },

    /// Set or display the temporal horizon of a tension.
    Horizon {
        /// Tension ID or prefix.
        id: String,

        /// New horizon value (e.g., "2026-05", or "none" to clear).
        /// If omitted, displays current horizon with urgency.
        value: Option<String>,
    },

    /// Display tension details.
    Show {
        /// Tension ID or prefix (4+ characters).
        id: String,

        /// Show all computed dynamics in detail.
        #[arg(short, long)]
        verbose: bool,
    },

    /// Update the actual state of a tension.
    Reality {
        /// Tension ID or prefix.
        id: String,

        /// New actual state (opens $EDITOR if omitted).
        value: Option<String>,
    },

    /// Update the desired state of a tension.
    Desire {
        /// Tension ID or prefix.
        id: String,

        /// New desired state (opens $EDITOR if omitted).
        value: Option<String>,
    },

    /// Mark a tension as resolved.
    Resolve {
        /// Tension ID or prefix.
        id: String,
    },

    /// Release a tension (abandon desired state).
    Release {
        /// Tension ID or prefix.
        id: String,

        /// Reason for releasing (required).
        #[arg(short, long)]
        reason: String,
    },

    /// Delete a tension (reparents children to grandparent).
    Rm {
        /// Tension ID or prefix.
        id: String,
    },

    /// Reparent a tension to a new parent.
    Move {
        /// Tension ID or prefix.
        id: String,

        /// New parent ID (omit to make root).
        #[arg(short, long)]
        parent: Option<String>,
    },

    /// Attach a narrative annotation to a tension.
    /// Usage: `werk note <text>` for workspace note, or `werk note <id> <text>` for tension note.
    Note {
        /// First argument: either tension ID/prefix (if second arg present) or note text.
        arg1: Option<String>,

        /// Second argument: note text (when first arg is ID).
        arg2: Option<String>,
    },

    /// List all workspace-level notes.
    Notes,

    /// Display the tension forest as a tree.
    Tree {
        /// Show only active tensions (default).
        #[arg(short, long)]
        open: bool,

        /// Show all tensions including resolved/released.
        #[arg(short, long)]
        all: bool,

        /// Show only resolved tensions.
        #[arg(long)]
        resolved: bool,

        /// Show only released tensions.
        #[arg(long)]
        released: bool,
    },

    /// Output structural context for agent consumption (JSON only).
    Context {
        /// Tension ID or prefix.
        id: String,
    },

    /// Launch an agent with structural context.
    Run {
        /// Tension ID or prefix.
        id: String,

        /// Agent command to run (overrides config default).
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },

    /// Destroy the current workspace (deletes the .werk/ directory).
    Nuke {
        /// Confirm deletion (required for safety).
        #[arg(short = 'y', long)]
        confirm: bool,

        /// Nuke the global workspace (~/.werk/) instead of local.
        #[arg(short, long)]
        global: bool,
    },
}

/// Config subcommands.
#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Get a configuration value.
    Get {
        /// Configuration key (dot notation, e.g., "agent.command").
        key: String,
    },

    /// Set a configuration value.
    Set {
        /// Configuration key.
        key: String,

        /// Configuration value.
        value: String,
    },
}
