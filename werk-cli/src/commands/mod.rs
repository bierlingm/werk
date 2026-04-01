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
pub mod log;
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
pub mod stats;
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
    #[command(after_help = "\
Examples:
  werk add \"Novel drafted\" \"42,000 words written\"
  werk add -p 10 \"Complete chapter 3\" \"Outline done\"
  werk add --horizon 2026-06 \"Ship v2.0\" \"Architecture designed\"")]
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
    #[command(name = "compose", after_help = "\
Examples:
  werk compose \"Product launch\" \"Components ready\" 42 43 44
  werk compose \"Q2 goals\" \"Planning complete\" 10 13 15")]
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
    #[command(after_help = "\
Examples:
  werk epoch 42                      Create epoch boundary
  werk epoch 42 --list               List all epochs
  werk epoch 42 --show 2             Show mutations during epoch 2")]
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

    /// Query the logbase — the searchable substrate of all prior epochs.
    ///
    /// Shows epoch history, provenance, and structural timeline.
    /// Accepts addresses: #42, #42~e3, #42@2026-03, g:ULID.
    #[command(after_help = "\
Examples:
  werk log 42                        Epoch history for tension #42
  werk log                           Cross-tension timeline (last 7 days)
  werk log 42 --search \"API\"         Search epoch snapshots
  werk log 42 --since 2026-03        Epochs since March 2026
  werk log 42 --since 7d             Epochs in the last 7 days
  werk log 42 --compare              Ghost geometry (desire-reality evolution)
  werk log \"#42~e3\"                  Show epoch 3 detail
  werk log \"g:01JQXYZ\"               Show gesture mutations")]
    Log {
        /// Tension ID, short code, or address (omit for cross-tension timeline).
        id: Option<String>,

        /// Text search across epoch snapshots.
        #[arg(short, long)]
        search: Option<String>,

        /// Show epochs since (YYYY-MM-DD, YYYY-MM, today, yesterday, Nd, Nw).
        #[arg(long)]
        since: Option<String>,

        /// Show desire-reality evolution (ghost geometry).
        #[arg(long)]
        compare: bool,

        /// Group by session.
        #[arg(long)]
        session: bool,
    },

    /// Set or display the deadline of a tension.
    #[command(alias = "deadline", after_help = "\
Examples:
  werk horizon 42                    Show current horizon and urgency
  werk horizon 42 2026-06            Set deadline to June 2026
  werk horizon 42 none               Clear the deadline")]
    Horizon {
        /// Tension ID or prefix.
        id: String,

        /// New horizon value (e.g., "2026-05", or "none" to clear).
        /// If omitted, displays current horizon with urgency.
        value: Option<String>,
    },

    /// Display tension details.
    #[command(after_help = "\
Examples:
  werk show 42                       Show by short code
  werk show --json 42                Structured output for scripts")]
    Show {
        /// Tension ID or prefix (4+ characters).
        id: String,

        /// Expanded output: include ancestors, siblings, and engagement metrics.
        #[arg(long)]
        full: bool,
    },

    /// Update the current reality of a tension.
    ///
    /// Reality updates are epoch boundaries — they snapshot the ending delta
    /// before applying the new reality. Use --no-epoch for minor corrections
    /// that don't warrant a new delta.
    ///
    /// Opens $EDITOR if value is omitted and a TTY is available.
    /// Fails with an error if no value and no TTY (non-interactive use).
    #[command(alias = "actual", after_help = "\
Examples:
  werk reality 42 \"Draft complete, 60k words\"
  werk reality 42 --no-epoch \"Fix typo in status\"
  werk reality 42                    Opens $EDITOR with current value")]
    Reality {
        /// Tension ID or prefix.
        id: String,

        /// New reality. Required in non-interactive contexts (no TTY).
        /// Opens $EDITOR if omitted and a terminal is available.
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
    ///
    /// Opens $EDITOR if value is omitted and a TTY is available.
    /// Fails with an error if no value and no TTY (non-interactive use).
    #[command(after_help = "\
Examples:
  werk desire 42 \"Novel published and reviewed\"
  werk desire 42 --no-epoch \"Fix typo in desired state\"
  werk desire 42                     Opens $EDITOR with current value")]
    Desire {
        /// Tension ID or prefix.
        id: String,

        /// New desired state. Required in non-interactive contexts (no TTY).
        /// Opens $EDITOR if omitted and a terminal is available.
        value: Option<String>,

        /// Skip epoch creation (for minor corrections).
        #[arg(long)]
        no_epoch: bool,
    },

    /// Mark a tension as resolved.
    #[command(after_help = "\
Examples:
  werk resolve 42                    Resolve now
  werk resolve 42 --actual-at yesterday
  werk resolve 42 --actual-at 2026-03-20
  werk resolve 42 --dry-run          Preview without resolving")]
    Resolve {
        /// Tension ID or prefix.
        id: String,

        /// When resolution actually happened (e.g., "yesterday", "2026-03-20").
        /// If omitted, actual resolution time = now.
        #[arg(long)]
        actual_at: Option<String>,

        /// Preview what would happen without making changes.
        #[arg(long)]
        dry_run: bool,
    },

    /// Release a tension (abandon desired state).
    #[command(after_help = "\
Examples:
  werk release 42 --reason \"No longer relevant after pivot\"
  werk release 42 -r \"Superseded by tension #55\"")]
    Release {
        /// Tension ID or prefix.
        id: String,

        /// Reason for releasing (required).
        #[arg(short, long)]
        reason: String,
    },

    /// Reopen a resolved or released tension (set status back to Active).
    #[command(after_help = "\
Examples:
  werk reopen 42
  werk reopen 42 --reason \"Regression found in production\"")]
    Reopen {
        /// Tension ID or prefix.
        id: String,

        /// Reason for reopening.
        #[arg(short, long)]
        reason: Option<String>,
    },

    /// Snooze a tension until a future date.
    #[command(after_help = "\
Examples:
  werk snooze 42 +3d                 Snooze for 3 days
  werk snooze 42 +2w                 Snooze for 2 weeks
  werk snooze 42 2026-04-15          Snooze until specific date
  werk snooze 42 --clear             Remove snooze")]
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
    #[command(after_help = "\
Examples:
  werk recur 42 +1w                  Recur weekly
  werk recur 42 +1d                  Recur daily
  werk recur 42 --clear              Stop recurring")]
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
    #[command(after_help = "\
Examples:
  werk rm 42                         Delete tension, reparent children
  werk rm 10                         Delete by short code
  werk rm 42 --dry-run               Preview deletion without acting")]
    Rm {
        /// Tension ID or prefix.
        id: String,

        /// Preview what would happen without making changes.
        #[arg(long)]
        dry_run: bool,
    },

    /// Reparent a tension to a new parent.
    #[command(after_help = "\
Examples:
  werk move 42 --parent 10           Move under tension #10
  werk move 42                       Move to root (no parent)
  werk move 42 --parent 10 --dry-run Preview without moving")]
    Move {
        /// Tension ID or prefix.
        id: String,

        /// New parent ID (omit to make root).
        #[arg(short, long)]
        parent: Option<String>,

        /// Preview what would happen without making changes.
        #[arg(long)]
        dry_run: bool,
    },

    /// Remove a tension from the sequence (set to held/unpositioned).
    #[command(after_help = "\
Examples:
  werk hold 42                       Remove from sequence")]
    Hold {
        /// Tension ID or prefix.
        id: String,
    },

    /// Set the position of a tension in the order of operations.
    #[command(after_help = "\
Examples:
  werk position 42 1                 Set as highest priority
  werk position 42 3                 Set as third in sequence")]
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
    ///
    /// Bare args default to add: `werk note 42 "text"` = `werk note add 42 "text"`.
    #[command(after_help = "\
Examples:
  werk note 42 \"Found edge case in validation\"
  werk note \"Workspace-level observation\"
  werk note add 42 \"Found edge case in validation\"
  werk note rm 42 1                  Retract note #1 from tension 42
  werk note list 42                  List notes for tension 42")]
    Note {
        #[command(subcommand)]
        command: NoteCommand,
    },

    /// System health (superseded by stats --health).
    #[command(hide = true, after_help = "\
Examples:
  werk health                        Show health summary
  werk health --repair               Find and optionally purge no-op mutations
  werk health --repair --yes         Purge without confirmation (agent-safe)")]
    Health {
        /// Repair: purge no-op mutations where old_value equals new_value.
        #[arg(long)]
        repair: bool,

        /// Skip confirmation prompt (for non-interactive / agent use).
        #[arg(long)]
        yes: bool,
    },

    /// Behavioral pattern insights (superseded by stats --attention --engagement).
    #[command(hide = true)]
    Insights {
        /// Analysis window in days.
        #[arg(long, default_value = "30")]
        days: i64,
    },

    /// The Napoleonic field survey — all tensions organized by temporal urgency.
    ///
    /// Equivalent to: list --approaching <days> --overdue --sort urgency
    /// Kept as a named perspective for vocabulary continuity.
    Survey {
        /// Temporal frame in days (default: 14).
        #[arg(long, default_value = "14")]
        days: i64,
    },

    /// Field overview (superseded by stats --all).
    ///
    /// Equivalent to: stats --all --days <days>
    #[command(hide = true)]
    Ground {
        /// Lookback window in days (default: 7).
        #[arg(long, default_value = "7")]
        days: i64,
    },

    /// Show what changed (superseded by list --changed).
    #[command(hide = true, after_help = "\
Examples:
  werk diff                          Changes since today (default)
  werk diff --since yesterday        Changes since yesterday
  werk diff --since \"3 days ago\"     Changes in last 3 days
  werk diff --since monday           Changes since Monday")]
    Diff {
        /// Show changes since (e.g., "today", "yesterday", "3 days ago", "2026-03-10", "monday")
        #[arg(long, default_value = "today")]
        since: String,
    },

    /// List tensions with filtering and sorting.
    ///
    /// The general-purpose query engine. Use flags to filter, sort, and
    /// format. Absorbs what survey and diff used to do separately.
    #[command(after_help = "\
Examples:
  werk list                          Active tensions sorted by urgency
  werk list --all                    Include resolved and released
  werk list --overdue                Only overdue tensions
  werk list --approaching 14         Due within 14 days (or overdue)
  werk list --stale 14               No activity in 14 days
  werk list --held                   Unpositioned tensions
  werk list --root                   Root tensions only
  werk list --parent 2               Children of tension #2
  werk list --changed today          What changed today (replaces diff)
  werk list --changed \"3 days ago\"   Changes in last 3 days
  werk list --sort deadline          Sort by deadline
  werk list --tree                   Show results as hierarchy
  werk list --long                   Expanded detail per tension
  werk list --search \"revenue\"       Search by content (ranked by relevance)")]
    List {
        /// Include resolved and released tensions.
        #[arg(long)]
        all: bool,

        /// Filter by status (active, resolved, released).
        #[arg(long)]
        status: Option<String>,

        /// Only overdue tensions (deadline passed, still active).
        #[arg(long)]
        overdue: bool,

        /// Deadline within N days, or overdue (default: 14).
        #[arg(long)]
        approaching: Option<Option<i64>>,

        /// No mutations in N days (default: 14).
        #[arg(long)]
        stale: Option<Option<i64>>,

        /// Only held (unpositioned) tensions.
        #[arg(long)]
        held: bool,

        /// Only positioned tensions.
        #[arg(long)]
        positioned: bool,

        /// Only root tensions (no parent).
        #[arg(long)]
        root: bool,

        /// Only children of this tension.
        #[arg(long)]
        parent: Option<String>,

        /// Only tensions with deadlines.
        #[arg(long)]
        has_deadline: bool,

        /// Show tensions changed since (e.g., "today", "yesterday", "3d", "2026-03-10").
        #[arg(long)]
        changed: Option<String>,

        /// Only tensions with active structural signals (overdue, containment, critical path, sequencing pressure, drift).
        #[arg(long)]
        signals: bool,

        /// Sort field: urgency, name, deadline, created, updated, position.
        #[arg(long, default_value = "urgency")]
        sort: String,

        /// Reverse sort order.
        #[arg(long, short)]
        reverse: bool,

        /// Show results as hierarchy.
        #[arg(long)]
        tree: bool,

        /// Expanded detail per tension.
        #[arg(long)]
        long: bool,

        /// Search by content (FrankenSearch hybrid retrieval). Ranks results by relevance.
        #[arg(long)]
        search: Option<String>,
    },

    /// Display the tension forest as a tree.
    #[command(after_help = "\
Examples:
  werk tree                          Full active tension forest
  werk tree 10                       Subtree under tension #10
  werk tree --all                    Include resolved/released
  werk tree --json                   Structured output for scripts")]
    Tree {
        /// Tension ID or prefix (show subtree under this tension).
        id: Option<String>,

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

        /// Append field vitals summary.
        #[arg(long)]
        stats: bool,
    },

    /// Field-level summaries, aggregates, and analysis.
    ///
    /// Default: field vitals. Use flags to add sections.
    /// Replaces ground, health, insights, and trajectory as separate commands.
    #[command(after_help = "\
Examples:
  werk stats                         Field vitals (compact summary)
  werk stats --temporal              Approaching deadlines, critical path, pressure
  werk stats --attention             Where energy went across root tensions
  werk stats --changes               Epochs, resolutions, new tensions, reality shifts
  werk stats --trajectory            Trajectory distribution, urgency collisions
  werk stats --engagement            Field frequency, most/least engaged
  werk stats --drift                 Horizon drift patterns
  werk stats --health                Data integrity checks
  werk stats --health --repair       Purge noop mutations
  werk stats --all                   Everything")]
    Stats {
        /// Show approaching deadlines, critical path, sequencing pressure, containment violations.
        #[arg(long)]
        temporal: bool,

        /// Show mutation distribution across root tensions and branches.
        #[arg(long)]
        attention: bool,

        /// Show epochs, resolutions, new tensions, reality shifts.
        #[arg(long)]
        changes: bool,

        /// Show trajectory distribution and urgency collisions.
        #[arg(long)]
        trajectory: bool,

        /// Show field frequency, most/least engaged tensions.
        #[arg(long)]
        engagement: bool,

        /// Show horizon drift patterns.
        #[arg(long)]
        drift: bool,

        /// Show data integrity checks (noop mutations, orphans).
        #[arg(long)]
        health: bool,

        /// Show all sections.
        #[arg(long)]
        all: bool,

        /// Time window in days for windowed sections.
        #[arg(long, default_value = "7")]
        days: i64,

        /// Repair: purge noop mutations (requires --health).
        #[arg(long)]
        repair: bool,

        /// Skip confirmation prompt (for --repair).
        #[arg(long)]
        yes: bool,
    },

    /// Trajectory projections (superseded by stats --trajectory).
    ///
    /// Per-tension mode (with ID) still unique to this command.
    #[command(hide = true)]
    Trajectory {
        /// Tension ID or prefix (omit for field-wide projection).
        id: Option<String>,

        /// Show urgency collision windows.
        #[arg(long)]
        collisions: bool,
    },

    /// Structural context JSON (superseded by show --json / list --json).
    #[command(hide = true, after_help = "\
Examples:
  werk context 42                    Full context for one tension
  werk context --all                 Context for all active tensions
  werk context --urgent              Context for urgent tensions only")]
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

    /// Launch the web interface (browser-based structural dynamics instrument).
    Serve {
        /// Port to listen on.
        #[arg(short, long, default_value = "3749")]
        port: u16,
    },

    /// Destroy the current workspace (deletes the .werk/ directory).
    #[command(after_help = "\
Examples:
  werk nuke                          Show what would be deleted (dry run)
  werk nuke --confirm                Actually delete the workspace
  werk nuke --global --confirm       Delete global workspace (~/.werk/)")]
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
