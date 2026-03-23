//! CLI commands for werk-cli.
//!
//! Each subcommand is defined with clap derive macros and implemented
//! in its own module file.

pub mod add;
pub mod batch;
pub mod compose_up;
pub mod config;
pub mod context;
pub mod desire;
pub mod diff;
pub mod epoch;
pub mod flush;
pub mod ground;
pub mod health;
pub mod hold;
pub mod horizon;
pub mod init;
pub mod insights;
pub mod list;
pub mod move_cmd;
pub mod note;
pub mod position;
pub mod nuke;
pub mod reality;
pub mod recur;
pub mod release;
pub mod reopen;
pub mod resolve;
pub mod rm;
pub mod show;
pub mod snooze;
pub mod survey;
pub mod trajectory;
pub mod tree;

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
        /// The desired outcome (what you want).
        desired: Option<String>,

        /// The current reality (where things stand).
        actual: Option<String>,

        /// Parent tension ID (creates child tension).
        #[arg(short, long)]
        parent: Option<String>,

        /// Temporal horizon (e.g., "2026", "2026-05", "2026-05-15").
        #[arg(long)]
        horizon: Option<String>,
    },

    /// Compose up: create a parent for existing tensions.
    ///
    /// The inverse of decomposing — reveals implicit coherence by
    /// composing existing structure upward. All specified children
    /// must share the same current parent (or all be roots).
    ///
    /// Usage: werk compose "desired outcome" "current reality" <id1> [id2 ...]
    #[command(name = "compose")]
    Compose {
        /// Desired outcome for the new parent tension.
        desired: String,

        /// Current reality for the new parent tension.
        actual: String,

        /// IDs of existing tensions to become children of the new parent.
        #[arg(required = true, num_args = 1..)]
        children: Vec<String>,
    },

    /// Flush tension state to a git-trackable JSON file at workspace root.
    ///
    /// Writes werk-state.json containing all tensions as raw structural data.
    /// Deterministic output: same state produces identical file content.
    Flush,

    /// Mark an epoch boundary for a tension.
    ///
    /// Snapshots the current desire, reality, and children state.
    /// A user-initiated narrative beat when desire or reality shifts
    /// significantly enough to warrant a new delta.
    Epoch {
        /// Tension ID or prefix.
        id: String,

        /// List existing epochs instead of creating a new one.
        #[arg(short, long)]
        list: bool,

        /// Show what happened during epoch N (mutations on tension + descendants).
        #[arg(short, long)]
        show: Option<usize>,
    },

    /// Set or display the deadline of a tension.
    #[command(alias = "deadline")]
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

    /// Update the current reality of a tension.
    ///
    /// Reality updates are epoch boundaries — they snapshot the ending delta
    /// before applying the new reality. Use --no-epoch for minor corrections
    /// that don't warrant a new delta.
    #[command(alias = "actual")]
    Reality {
        /// Tension ID or prefix.
        id: String,

        /// New reality (opens $EDITOR if omitted).
        value: Option<String>,

        /// Skip epoch creation (for minor corrections).
        #[arg(long)]
        no_epoch: bool,
    },

    /// Update the desired state of a tension.
    ///
    /// Desire updates are epoch boundaries — they snapshot the ending delta
    /// before applying the new desire. Use --no-epoch for minor corrections
    /// that don't warrant a new delta.
    Desire {
        /// Tension ID or prefix.
        id: String,

        /// New desired state (opens $EDITOR if omitted).
        value: Option<String>,

        /// Skip epoch creation (for minor corrections).
        #[arg(long)]
        no_epoch: bool,
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

    /// Testimony operations: add, retract, or list notes.
    ///
    /// Notes are first-class operative gestures — observational testimony
    /// that accumulates within the current epoch.
    Note {
        #[command(subcommand)]
        command: NoteCommand,
    },

    /// Show system health summary (structural statistics, temporal alerts).
    Health,

    /// Show behavioral pattern insights from mutation history.
    Insights {
        /// Analysis window in days.
        #[arg(long, default_value = "30")]
        days: i64,
    },

    /// The Napoleonic field survey — all tensions organized by temporal urgency.
    ///
    /// Navigate time, see structure. Shows overdue steps, upcoming deadlines,
    /// held steps, and recently resolved across the entire field.
    Survey {
        /// Temporal frame in days (default: 14).
        #[arg(long, default_value = "14")]
        days: i64,
    },

    /// Ground mode — the debrief and study surface.
    ///
    /// Shows field statistics, epoch history, and recent gestures.
    /// Where you study your own patterns when you're not flying.
    Ground {
        /// Lookback window in days (default: 7).
        #[arg(long, default_value = "7")]
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

        /// Show only stagnant tensions (overdue with no recent activity).
        #[arg(long)]
        stagnant: bool,

        /// Sort by field (urgency, name, deadline).
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

    /// Output structural context (JSON). Useful for MCP, scripts, or agent consumption.
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

    /// Batch operations (apply/validate mutations from YAML).
    Batch {
        #[command(subcommand)]
        command: BatchCommand,
    },

    /// Start the MCP server (stdio transport).
    ///
    /// Exposes all operative gestures as MCP tools discoverable by any
    /// protocol-capable harness. The third interface surface alongside TUI and CLI.
    Mcp,

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

impl Commands {
    /// Whether this command mutates tension state (and should trigger autoflush).
    pub fn is_mutation(&self) -> bool {
        matches!(
            self,
            Commands::Add { .. }
                | Commands::Compose { .. }
                | Commands::Reality { .. }
                | Commands::Desire { .. }
                | Commands::Resolve { .. }
                | Commands::Release { .. }
                | Commands::Reopen { .. }
                | Commands::Rm { .. }
                | Commands::Move { .. }
                | Commands::Hold { .. }
                | Commands::Position { .. }
                | Commands::Note { .. }
                | Commands::Horizon { .. }
                | Commands::Epoch { .. }
                | Commands::Snooze { .. }
                | Commands::Recur { .. }
                | Commands::Batch { .. }
        )
    }
}

/// Note subcommands (noun-verb pattern).
#[derive(Debug, Subcommand)]
pub enum NoteCommand {
    /// Add a note (observational testimony).
    /// Usage: `werk note add <text>` for workspace, `werk note add <id> <text>` for tension.
    Add {
        /// First argument: either tension ID/prefix (if second arg present) or note text.
        arg1: Option<String>,

        /// Second argument: note text (when first arg is ID).
        arg2: Option<String>,
    },

    /// Retract a note (retraction testimony).
    /// Usage: `werk note rm <n>` for workspace, `werk note rm <id> <n>` for tension.
    Rm {
        /// First argument: tension ID/prefix (if second arg present) or note number.
        arg1: String,

        /// Second argument: note number (when first arg is ID).
        arg2: Option<String>,
    },

    /// List active notes.
    /// Usage: `werk note list` for workspace, `werk note list <id>` for tension.
    List {
        /// Optional tension ID to show notes for a specific tension.
        id: Option<String>,
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
