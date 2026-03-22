//! CLI commands for werk-cli.
//!
//! Each subcommand is defined with clap derive macros and implemented
//! in its own module file.

pub mod add;
pub mod batch;
pub mod config;
pub mod context;
pub mod desire;
pub mod diff;
pub mod health;
pub mod hold;
pub mod horizon;
pub mod init;
pub mod insights;
pub mod list;
pub mod move_cmd;
pub mod note;
pub mod notes;
pub mod position;
pub mod nuke;
pub mod reality;
pub mod recur;
pub mod release;
pub mod reopen;
pub mod resolve;
pub mod rm;
pub mod run;
pub mod show;
pub mod snooze;
pub mod trajectory;
pub mod tree;
pub mod watch;

use clap::Subcommand;
use batch::BatchCommand;

/// CLI subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize a workspace (creates .werk/ directory with sd.db).
    Init {
        /// Use global workspace (~/.werk/) instead of local.
        #[arg(short, long)]
        global: bool,
    },

    /// Get or set configuration values. Run without subcommand for interactive menu.
    Config {
        /// Config subcommand (omit for interactive menu).
        #[command(subcommand)]
        command: Option<ConfigCommand>,
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

        /// When resolution actually happened (e.g., "yesterday", "2026-03-20").
        /// If omitted, actual resolution time = now.
        #[arg(long)]
        actual_at: Option<String>,
    },

    /// Release a tension (abandon desired state).
    Release {
        /// Tension ID or prefix.
        id: String,

        /// Reason for releasing (required).
        #[arg(short, long)]
        reason: String,
    },

    /// Reopen a resolved or released tension (set status back to Active).
    Reopen {
        /// Tension ID or prefix.
        id: String,
    },

    /// Snooze a tension until a future date.
    Snooze {
        /// Tension ID or prefix.
        id: String,

        /// Date to snooze until (+3d, +2w, +1m, or YYYY-MM-DD).
        date: Option<String>,

        /// Clear the snooze (unhide the tension).
        #[arg(long)]
        clear: bool,
    },

    /// Set or clear a recurrence interval on a tension.
    Recur {
        /// Tension ID or prefix.
        id: String,

        /// Recurrence interval (+1d, +1w, +2w, +1m).
        interval: Option<String>,

        /// Clear the recurrence.
        #[arg(long)]
        clear: bool,
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

    /// Remove a tension from the sequence (set to held/unpositioned).
    Hold {
        /// Tension ID or prefix.
        id: String,
    },

    /// Set the position of a tension in the order of operations.
    Position {
        /// Tension ID or prefix.
        id: String,

        /// Position number (1-based, higher = earlier in sequence).
        n: i32,
    },

    /// Attach a narrative annotation to a tension.
    /// Usage: `werk note <text>` for workspace note, or `werk note <id> <text>` for tension note.
    Note {
        /// First argument: either tension ID/prefix (if second arg present) or note text.
        arg1: Option<String>,

        /// Second argument: note text (when first arg is ID).
        arg2: Option<String>,
    },

    /// List notes. Without an ID, shows workspace notes. With an ID, shows notes for that tension.
    Notes {
        /// Optional tension ID to show notes for a specific tension.
        id: Option<String>,
    },

    /// Show system health summary (phase distribution, movement ratios, alerts).
    Health,

    /// Show behavioral pattern insights from mutation history.
    Insights {
        /// Analysis window in days.
        #[arg(long, default_value = "30")]
        days: i64,
    },

    /// Show what changed in a time window.
    Diff {
        /// Show changes since (e.g., "today", "yesterday", "3 days ago", "2026-03-10", "monday")
        #[arg(long, default_value = "today")]
        since: String,
    },

    /// List tensions with filtering and sorting.
    List {
        /// Show all tensions (including resolved/released).
        #[arg(long)]
        all: bool,

        /// Show only urgent tensions.
        #[arg(long)]
        urgent: bool,

        /// Show only neglected tensions.
        #[arg(long)]
        neglected: bool,

        /// Show only stagnant tensions (no movement).
        #[arg(long)]
        stagnant: bool,

        /// Filter by phase (G, A, C, M).
        #[arg(long)]
        phase: Option<String>,

        /// Sort by field (urgency, phase, name, horizon).
        #[arg(long, default_value = "urgency")]
        sort: String,
    },

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

    /// Show structural trajectory projections.
    ///
    /// Without arguments: field-wide structural funnel.
    /// With a tension ID: per-tension trajectory projection.
    /// With --collisions: upcoming urgency collision windows.
    Trajectory {
        /// Tension ID or prefix (omit for field-wide projection).
        id: Option<String>,

        /// Show urgency collision windows.
        #[arg(long)]
        collisions: bool,
    },

    /// Output structural context for agent consumption (JSON only).
    Context {
        /// Tension ID or prefix (omit for bulk modes).
        id: Option<String>,

        /// Output context for all active tensions.
        #[arg(long)]
        all: bool,

        /// Output context for urgent tensions only.
        #[arg(long)]
        urgent: bool,
    },

    /// Launch an agent with structural context.
    ///
    /// Three modes:
    ///   werk run <id> "prompt"       One-shot: send prompt with tension context, get response
    ///   werk run <id> -- <command>   Interactive: launch agent with context piped to stdin
    ///   werk run --system "prompt"   System-wide: all active tensions as context
    ///   werk run <id> --decompose    Decompose: break tension into sub-tensions
    Run {
        /// Tension ID or prefix (optional with --system).
        id: Option<String>,

        /// User prompt for one-shot mode.
        #[arg(value_name = "PROMPT")]
        prompt: Option<String>,

        /// Don't prompt for reality updates from agent suggestions.
        #[arg(long)]
        no_suggest: bool,

        /// Agent command to run (overrides config default, for interactive mode).
        #[arg(last = true)]
        command: Vec<String>,

        /// System-wide context (all active tensions, no specific ID needed).
        #[arg(long)]
        system: bool,

        /// Auto-decompose: ask agent to break tension into sub-tensions.
        #[arg(long)]
        decompose: bool,

        /// Dry run: show what would be applied without applying.
        #[arg(long)]
        dry_run: bool,
    },

    /// Monitor tension dynamics and invoke agent on threshold crossings.
    Watch {
        /// Start as background daemon.
        #[arg(long)]
        daemon: bool,

        /// Stop the background daemon.
        #[arg(long)]
        stop: bool,

        /// Show watch status (daemon, last check, pending insights).
        #[arg(long)]
        status: bool,

        /// List pending insights.
        #[arg(long)]
        pending: bool,

        /// Show recent watch activity.
        #[arg(long)]
        history: bool,
    },

    /// Batch operations (apply/validate mutations from YAML).
    Batch {
        #[command(subcommand)]
        command: BatchCommand,
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
