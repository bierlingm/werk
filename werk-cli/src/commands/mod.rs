//! CLI commands for werk-cli.
//!
//! Each subcommand is defined with clap derive macros and implemented
//! in its own module file.

pub mod add;
pub mod batch;
pub mod compose_up;
pub mod config;
pub mod daemon;
pub mod desire;
pub mod doctor;
pub mod epoch;
pub mod field;
pub mod flush;
pub mod hold;
pub mod hooks;
pub mod horizon;
pub mod init;
pub mod list;
pub mod log;
pub mod merge;
pub mod move_cmd;
pub mod note;
pub mod nuke;
pub mod position;
pub mod reality;
pub mod recur;
pub mod release;
pub mod reopen;
pub mod resolve;
pub mod rm;
pub mod serve;
pub mod show;
pub mod sigil;
pub mod snooze;
pub mod spaces;
pub mod split;
pub mod stats;
pub mod tree;
pub mod undo;

use batch::BatchCommand;
use clap::Subcommand;
use werk_shared::{AnalysisThresholds, Config, SignalThresholds, Workspace};

/// Convert AnalysisThresholds to werk-core's ProjectionThresholds.
pub fn to_projection_thresholds(a: &AnalysisThresholds) -> werk_core::ProjectionThresholds {
    werk_core::ProjectionThresholds {
        pattern_window_seconds: a.pattern_window_days * 86400,
        neglect_frequency_threshold: a.neglect_frequency,
        oscillation_gap_variance: a.oscillation_variance,
        resolution_gap_threshold: a.resolution_gap,
    }
}

/// Load signal thresholds from workspace config, falling back to defaults.
///
/// Used by main.rs for CLI flag defaults (before a command discovers its own workspace).
pub fn load_signal_thresholds() -> SignalThresholds {
    Workspace::discover()
        .ok()
        .and_then(|ws| Config::load(&ws).ok())
        .map(|c| SignalThresholds::load(&c))
        .unwrap_or_default()
}

/// Load signal thresholds from an already-discovered workspace.
pub fn signal_thresholds_from(workspace: &Workspace) -> SignalThresholds {
    Config::load(workspace)
        .ok()
        .map(|c| SignalThresholds::load(&c))
        .unwrap_or_default()
}

/// Load analysis thresholds from an already-discovered workspace.
pub fn analysis_thresholds_from(workspace: &Workspace) -> AnalysisThresholds {
    Config::load(workspace)
        .ok()
        .map(|c| AnalysisThresholds::load(&c))
        .unwrap_or_default()
}

/// Read a parsed config value (local workspace first, falling back to global),
/// or the given fallback if the key is unset or parses wrong. Resolves level
/// labels (e.g. `"a week"` → `"7"`) before parsing.
pub fn config_default<T: std::str::FromStr>(key: &str, fallback: T) -> T {
    use werk_shared::config_registry::resolve_value;
    let config = Workspace::discover()
        .ok()
        .and_then(|ws| Config::load(&ws).ok())
        .or_else(|| Config::load_global().ok());
    config
        .and_then(|c| c.get(key).cloned())
        .map(|v| resolve_value(key, &v))
        .and_then(|v| v.parse().ok())
        .unwrap_or(fallback)
}

/// Like [`config_default`] but for String-valued keys where an empty string
/// should be treated as "not set" (e.g. `editor.command`).
pub fn config_default_string(key: &str, fallback: &str) -> String {
    let config = Workspace::discover()
        .ok()
        .and_then(|ws| Config::load(&ws).ok())
        .or_else(|| Config::load_global().ok());
    config
        .and_then(|c| c.get(key).cloned())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

/// CLI subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize a workspace (creates .werk/ directory with werk.db).
    Init {
        /// Use global workspace (~/.werk/) instead of local.
        #[arg(short, long)]
        global: bool,
    },

    /// Get or set configuration values. Run without subcommand to list all values.
    Config {
        /// Config subcommand (omit to list all values).
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
    #[command(
        name = "compose",
        after_help = "\
Examples:
  werk compose \"Product launch\" \"Components ready\" 42 43 44
  werk compose \"Q2 goals\" \"Planning complete\" 10 13 15"
    )]
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
    #[command(
        alias = "deadline",
        after_help = "\
Examples:
  werk horizon 42                    Show current horizon and urgency
  werk horizon 42 2026-06            Set deadline to June 2026
  werk horizon 42 none               Clear the deadline"
    )]
    Horizon {
        /// Tension ID or prefix.
        id: String,

        /// New horizon value (e.g., "2026-05", or "none" to clear).
        /// If omitted, displays current horizon with urgency.
        value: Option<String>,
    },

    /// Display tension details.
    ///
    /// Default output is a structured briefing: identity, situation, signals,
    /// theory of closure (children organized by frontier zone), recent notes,
    /// and activity. Section flags expand specific areas; --full expands all.
    #[command(after_help = "\
Examples:
  werk show 42                       Briefing view
  werk show 42 --brief               Card view (identity + situation only)
  werk show 42 --notes               Expand all notes to full text
  werk show 42 --route               Theory of closure with per-child detail
  werk show 42 --notes --activity    Compose section expansions
  werk show 42 --full                Everything expanded (present epoch)
  werk show 42 --history             Restore complete cross-epoch history
  werk show --json 42                Structured output for scripts")]
    Show {
        /// Tension ID or prefix (4+ characters).
        id: String,

        /// Card view: identity + situation + signals only. No children, notes, or activity.
        #[arg(long, short)]
        brief: bool,

        /// Expand all notes to full text (default shows recent notes truncated).
        #[arg(long, short)]
        notes: bool,

        /// Theory of closure focused: children with per-child detail (reality, deadline, urgency).
        #[arg(long, short)]
        route: bool,

        /// Full mutation history (default shows last 10).
        #[arg(long, short)]
        activity: bool,

        /// Expand all epochs with desire/reality snapshots.
        #[arg(long, short)]
        epochs: bool,

        /// Structural neighborhood: ancestors, siblings, engagement metrics.
        #[arg(long, short)]
        context: bool,

        /// Everything expanded: all notes, all activity, all epochs, ancestors, siblings, engagement.
        #[arg(long, short)]
        full: bool,

        /// Restore the complete cross-epoch picture: done children, notes, and activity
        /// from before the present epoch boundary. Default scopes these to the present
        /// epoch (the timespan since the most recent desire/reality change).
        #[arg(long = "history", short = 'H')]
        history: bool,
    },

    /// Render a sigil (SVG artifact) from a scope.
    #[command(after_help = "\
Examples:
  werk sigil 2
  werk sigil 2 --logic contemplative
  werk sigil 2 --seed 7 --out /tmp/sigil.svg
  werk sigil 2 --save")]
    Sigil {
        /// Scope addresses (tension IDs or short codes). Multiple inputs build a union scope.
        #[arg(num_args = 0..)]
        scope: Vec<String>,

        /// Logic preset name or path to a .toml file.
        #[arg(long)]
        logic: Option<String>,

        /// Override the deterministic seed.
        #[arg(long)]
        seed: Option<u64>,

        /// Write SVG to the given path.
        #[arg(long)]
        out: Option<std::path::PathBuf>,

        /// Save to archive and record metadata in the sigils table.
        #[arg(long)]
        save: bool,

        /// Resolve and report without rendering or persisting.
        #[arg(long)]
        dry_run: bool,
    },

    /// Update the current reality of a tension.
    ///
    /// Reality updates are epoch boundaries — they snapshot the ending delta
    /// before applying the new reality. Use --no-epoch for minor corrections
    /// that don't warrant a new delta.
    ///
    /// Opens $EDITOR if value is omitted and a TTY is available.
    /// Fails with an error if no value and no TTY (non-interactive use).
    #[command(
        alias = "actual",
        after_help = "\
Examples:
  werk reality 42 \"Draft complete, 60k words\"
  werk reality 42 --no-epoch \"Fix typo in status\"
  werk reality 42                    Opens $EDITOR with current value"
    )]
    Reality {
        /// Tension ID or prefix.
        id: String,

        /// New reality. Required in non-interactive contexts (no TTY).
        /// Opens $EDITOR if omitted and a terminal is available.
        value: Option<String>,

        /// Skip epoch creation (for minor corrections).
        #[arg(long)]
        no_epoch: bool,

        /// Include a compact post-mutation view in JSON output.
        /// (Human output always shows the echo.)
        #[arg(long)]
        show_after: bool,
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

        /// Include a compact post-mutation view in JSON output.
        /// (Human output always shows the echo.)
        #[arg(long)]
        show_after: bool,
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

        /// Include a compact post-mutation view in JSON output.
        /// (Human output always shows the echo.)
        #[arg(long)]
        show_after: bool,
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

        /// Include a compact post-mutation view in JSON output.
        /// (Human output always shows the echo.)
        #[arg(long)]
        show_after: bool,
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

    /// Undo a gesture (reverse all its mutations).
    #[command(after_help = "\
Examples:
  werk undo 01JWAB1234           Undo gesture by ID
  werk undo --last               Undo the most recent gesture
  werk undo --last --dry-run     Preview what would be undone")]
    Undo {
        /// Gesture ID to undo.
        gesture_id: Option<String>,

        /// Undo the most recent gesture.
        #[arg(long)]
        last: bool,

        /// Preview without making changes.
        #[arg(long)]
        dry_run: bool,
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

    /// Split a tension into N new tensions with provenance tracking.
    ///
    /// Creates N new tensions, each with a `split_from` provenance edge
    /// pointing to the source. The source is resolved by default.
    #[command(after_help = "\
Examples:
  werk split 25 \"API concern\" \"UI concern\"
  werk split 25 \"A\" \"B\" \"C\"                Three-way split
  werk split 25 \"A\" \"B\" --assign 30=1       Child #30 to successor 1
  werk split 25 \"A\" \"B\" --children-to-parent
  werk split 25 \"A\" \"B\" --keep              Source stays active
  werk split 25 \"A\" \"B\" --dry-run           Preview")]
    Split {
        /// Source tension ID or prefix.
        id: String,

        /// Desired states for new tensions (at least 2).
        #[arg(required = true, num_args = 2..)]
        desires: Vec<String>,

        /// Assign children: CHILD_SHORT_CODE=TARGET_NUM (e.g., 30=1).
        #[arg(long, num_args = 1..)]
        assign: Vec<String>,

        /// Float all children to source's parent.
        #[arg(long)]
        children_to_parent: bool,

        /// Move all children to successor N.
        #[arg(long)]
        children_to: Option<usize>,

        /// Keep source active (don't resolve).
        #[arg(long)]
        keep: bool,

        /// Release source instead of resolving.
        #[arg(long)]
        release: bool,

        /// Hold source (unpositioned) instead of resolving.
        #[arg(long)]
        hold: bool,

        /// Preview without making changes.
        #[arg(long)]
        dry_run: bool,
    },

    /// Merge tensions with provenance tracking.
    ///
    /// Asymmetric: `--into <id>` — one survives, the other is absorbed.
    /// Symmetric: `--as "desire"` — both absorbed into a new tension.
    #[command(after_help = "\
Examples:
  werk merge 18 19 --into 18                   #18 survives, #19 absorbed
  werk merge 18 19 --into 18 --desire \"updated\"  Update survivor's desire
  werk merge 18 19 --as \"combined concern\"     Both absorbed into new
  werk merge 18 19 --into 18 --dry-run         Preview")]
    Merge {
        /// First tension ID.
        id1: String,

        /// Second tension ID.
        id2: String,

        /// Asymmetric: surviving tension ID (must be id1 or id2).
        #[arg(long)]
        into: Option<String>,

        /// Symmetric: desire for the new merged tension.
        #[arg(long, name = "as")]
        as_desire: Option<String>,

        /// Update survivor's desire (asymmetric mode).
        #[arg(long)]
        desire: Option<String>,

        /// Assign children: CHILD_SHORT_CODE=TARGET (e.g., 30=survivor).
        #[arg(long, num_args = 1..)]
        assign: Vec<String>,

        /// Float absorbed tension's children to its parent.
        #[arg(long)]
        children_to_parent: bool,

        /// Preview without making changes.
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
  werk position 42 3                 Set as third in sequence
  werk position --renumber 10        Compact children of #10 to 1..N
  werk position --renumber 10 --dry-run
                                     Preview the renumber map")]
    Position {
        /// Tension ID or prefix.
        id: Option<String>,

        /// Position number (1-based, higher = earlier in sequence).
        n: Option<i32>,

        /// Compact positions among the children of this parent so they
        /// run 1..N preserving relative order. Held siblings stay held.
        #[arg(long, value_name = "PARENT_ID", conflicts_with_all = ["id", "n"])]
        renumber: Option<String>,

        /// Preview the change without mutating.
        #[arg(long)]
        dry_run: bool,
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

    /// List tensions with filtering and sorting.
    ///
    /// The general-purpose query engine. Use flags to filter, sort, and
    /// format. The sole query surface for the field.
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
        /// List a different kind of record (e.g., "sigil").
        #[arg(long)]
        kind: Option<String>,

        /// Include resolved and released tensions.
        #[arg(long)]
        all: bool,

        /// Filter by status (active, resolved, released).
        #[arg(long)]
        status: Option<String>,

        /// Only overdue tensions (deadline passed, still active).
        #[arg(long)]
        overdue: bool,

        /// Deadline within N days, or overdue. Accepts integer (14), relative
        /// (+2w, +1m), or named (a week, two weeks, a month). Default: config.
        #[arg(long)]
        approaching: Option<Option<String>>,

        /// No mutations in N days. Accepts integer (14), relative (+2w, +1m),
        /// or named (a few days, a week, two weeks). Default: config.
        #[arg(long)]
        stale: Option<Option<String>>,

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
        /// Defaults to `list.default_sort` in config (fallback: `urgency`).
        #[arg(long)]
        sort: Option<String>,

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

        /// Force the compact single-line layout (v1.5). The rich
        /// two-line-root layout is the default in interactive
        /// terminals wider than 80 columns.
        #[arg(long)]
        compact: bool,
    },

    /// Field-level summaries, aggregates, and analysis.
    ///
    /// Default: field vitals. Use flags to add sections.
    /// The sole analysis surface for the field.
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

        /// Time window for windowed sections. Accepts integer (7), relative
        /// (+2w, +1m), or named (today, a week, two weeks, a month).
        /// Defaults to `stats.default_window_days` in config.
        #[arg(long)]
        days: Option<String>,

        /// Repair: purge noop mutations (requires --health).
        #[arg(long)]
        repair: bool,

        /// Skip confirmation prompt (for --repair).
        #[arg(long)]
        yes: bool,
    },

    /// The aggregate command center across every registered space.
    ///
    /// Field-scope counterpart to `werk stats`: shows vitals for every
    /// registered space pooled together, tagged by space. With --attention,
    /// adds the three pooled exception bands (overdue / next-up / held).
    ///
    /// Every number shown is the workspace's own standard — the field view
    /// doesn't infer cross-space priorities. A stale registry entry (lost
    /// `.werk/`) is skipped silently and reported at the end.
    #[command(after_help = "\
Examples:
  werk field                         Per-space vitals grid + totals
  werk field --attention             Adds pooled overdue / next-up / held
  werk field --json                  Structured output for agents")]
    Field {
        /// Show pooled overdue / next-up / held bands with items tagged `[space:#N]`.
        #[arg(short, long)]
        attention: bool,
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

    /// Manage the registry of named werk spaces.
    ///
    /// The registry lives in `~/.werk/config.toml` under `[workspaces]` and
    /// maps short names to absolute paths. Registered names are accepted
    /// wherever a space identifier is expected — `werk daemon point`, the
    /// in-tab switcher, and (eventually) `werk -w <name>`.
    #[command(after_help = "\
Examples:
  werk spaces list                        Show registered spaces
  werk spaces register desk ~/code/werk   Add a registration
  werk spaces create journal ~/journal    Init + register in one step
  werk spaces scan                        Walk ~/ for unregistered .werk/ dirs
  werk spaces rename desk werk            Rename a registration
  werk spaces unregister journal          Drop a registration")]
    Spaces {
        #[command(subcommand)]
        command: SpacesCommand,
    },

    /// Manage the background `werk serve --global` process.
    ///
    /// Installs an OS-level supervisor (launchd on macOS, systemd --user on Linux)
    /// that keeps the global web API running so the browser extension and other
    /// local consumers always find werk at a known address.
    #[command(after_help = "\
Examples:
  werk daemon install         Install and start the daemon
  werk daemon status          Check if the daemon is running
  werk daemon logs            Tail the daemon log
  werk daemon uninstall       Stop and remove the daemon")]
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },

    /// Launch the web interface (browser-based structural dynamics instrument).
    Serve {
        /// Port to listen on.
        /// Defaults to `serve.port` in config (fallback: 3749).
        #[arg(short, long, conflicts_with = "port_range")]
        port: Option<u16>,

        /// Port range to scan (e.g. "3749-3759"). Binds the first free port.
        /// Used by `werk daemon` so the server stays up when the default port is taken.
        #[arg(long, value_name = "RANGE")]
        port_range: Option<String>,

        /// Bind host.
        /// Defaults to `serve.host` in config (fallback: 127.0.0.1).
        #[arg(long)]
        host: Option<String>,

        /// Target the global workspace (`~/.werk/`) regardless of CWD.
        #[arg(short = 'g', long, conflicts_with_all = ["daemon_target", "workspace_path"])]
        global: bool,

        /// Read the active workspace from `~/.werk/config.toml` (`daemon.workspace_path`).
        /// Used by the supervised `werk daemon` so the in-tab switcher takes effect on restart.
        #[arg(long, conflicts_with_all = ["global", "workspace_path"])]
        daemon_target: bool,

        /// Serve a specific workspace path. Mutually exclusive with --global / --daemon-target.
        #[arg(long, value_name = "PATH", conflicts_with_all = ["global", "daemon_target"])]
        workspace_path: Option<std::path::PathBuf>,
    },

    /// Manage lifecycle hooks.
    ///
    /// Hooks execute shell commands when structural events occur.
    /// The EventBus bridge fires post-hooks automatically; pre-hooks
    /// are checked at command level before mutations.
    #[command(after_help = "\
Examples:
  werk hooks list                          Show configured hooks
  werk hooks add post_tension_resolved ./notify.sh
  werk hooks add post_* ./log-all.sh
  werk hooks rm post_tension_resolved
  werk hooks test post_tension_resolved --tension 42
  werk hooks log --tail 20
  werk hooks install flush auto-stage      Install shipped defaults
  werk hooks install --git                 Set up git pre-commit hook")]
    Hooks {
        #[command(subcommand)]
        command: HooksCommand,
    },

    /// Diagnose and (optionally) repair the workspace's structural state.
    ///
    /// Default (no flags) is read-only and writes only an append-only run
    /// artifact under `.werk/.doctor/runs/<run-id>/`. With `--fix`, every
    /// mutation is backed up verbatim and recorded so
    /// `werk doctor undo <run-id>` can restore byte-for-byte.
    ///
    /// Pass-3 surface (R-003) ships the chokepoint plus one detector
    /// (`noop_mutations`). The six Quint invariant detectors are reserved
    /// in `capabilities --json` and land in R-005.
    #[command(after_help = "\
Examples:
  werk doctor                          Read-only diagnose
  werk doctor --json                   Machine-readable output
  werk doctor --fix --yes              Repair (skip confirmation)
  werk doctor --dry-run --fix          Print the plan
  werk doctor --robot-triage           One-call JSON triage
  werk doctor undo latest              Roll back the most recent --fix
  werk doctor capabilities --json      What this binary can detect and fix
  werk doctor robot-docs               Paste-ready agent handbook

Exit codes:
  0 healthy          1 findings present     2 partial fix
  3 fix failed       4 refused unsafe       5 lock held
  6 online required  64 usage              66 no input
  73 cannot create   74 io error")]
    Doctor(doctor::DoctorArgs),

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
                | Commands::Split { .. }
                | Commands::Merge { .. }
                | Commands::Snooze { .. }
                | Commands::Recur { .. }
                | Commands::Batch { .. }
                | Commands::Undo { .. }
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

/// Spaces subcommands.
#[derive(Debug, Subcommand)]
pub enum SpacesCommand {
    /// List registered spaces (the global space is always implicit).
    List {
        /// Show full absolute paths instead of basenames.
        #[arg(long)]
        path: bool,
    },

    /// Register an existing workspace under a name.
    Register {
        /// Name to use (a-z, 0-9, dash, underscore; reserved: "global").
        name: String,
        /// Workspace root (parent of `.werk/`).
        path: std::path::PathBuf,
    },

    /// Drop a registration. The workspace files are not touched.
    Unregister { name: String },

    /// Initialize a fresh workspace at `path` and register it under `name`.
    Create {
        name: String,
        path: std::path::PathBuf,
    },

    /// Walk `~/` for `.werk/` directories and report registered vs. unregistered.
    Scan {
        /// Maximum recursion depth (default: 6).
        #[arg(long, default_value_t = 6)]
        depth: usize,

        /// Auto-register every unregistered hit using a derived name.
        #[arg(long)]
        register_all: bool,
    },

    /// Rename a registration. Path is unchanged.
    Rename { old: String, new: String },
}

/// Daemon subcommands.
#[derive(Debug, Subcommand)]
pub enum DaemonCommand {
    /// Install and start the daemon (launchd on macOS, systemd --user on Linux).
    Install {
        /// Port range to scan. Defaults to 3749-3759.
        #[arg(long, value_name = "RANGE")]
        port_range: Option<String>,

        /// Overwrite an existing daemon installation without prompting.
        #[arg(long)]
        force: bool,
    },

    /// Stop and remove the daemon.
    Uninstall,

    /// Show whether the daemon is running and which port it's bound to.
    Status,

    /// Tail the daemon log.
    Logs {
        /// Number of lines to tail.
        #[arg(short = 'n', long, default_value_t = 40)]
        lines: usize,

        /// Follow the log (like `tail -f`).
        #[arg(short, long)]
        follow: bool,
    },

    /// Point the daemon at a different workspace and restart it.
    ///
    /// Accepts a registered name (from `werk spaces register`) or an absolute
    /// path. Persists the selection in `~/.werk/config.toml` and triggers the
    /// supervisor to restart the serve process so the change takes effect
    /// immediately. Use `werk daemon point --global` to return to `~/.werk/`.
    Point {
        /// Registered name or workspace path. Omit with --global.
        target: Option<String>,

        /// Point at the global workspace (`~/.werk/`).
        #[arg(short = 'g', long, conflicts_with = "target")]
        global: bool,
    },
}

/// Config subcommands.
#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Get a configuration value (omit key to list all).
    Get {
        /// Configuration key (dot notation, e.g., "agent.command"). Omit to list all.
        key: Option<String>,
    },

    /// Set a configuration value.
    Set {
        /// Configuration key.
        key: String,

        /// Configuration value.
        value: String,
    },

    /// Remove a configuration key (any key, including hooks and unknowns).
    Unset {
        /// Configuration key to remove.
        key: String,
    },

    /// Open `$EDITOR` on the config file directly. On save, report the diff
    /// vs. the pre-edit state. Fails with an error in non-interactive contexts
    /// (no TTY). Use `set`/`unset`/`reset` in scripts.
    Edit,

    /// Show only the keys whose value differs from the registry default,
    /// plus all hook and unknown keys.
    Diff,

    /// Reset registry keys to their defaults.
    ///
    /// With no argument: reset every registered key to its default (hooks
    /// and unknown keys are left alone — use `unset` for those).
    /// With a group name (framing, analysis, action, persistence, display):
    /// reset every key in that group.
    /// With a key name: reset just that one registry key.
    Reset {
        /// Key, group name, or omit for all registry keys.
        target: Option<String>,
    },

    /// Export the current config to a TOML preset file. Includes a header
    /// comment with werk version and an FNV-1a hash of the values for
    /// regression detection on re-import.
    Export {
        /// Destination path. Overwrites if it exists.
        path: std::path::PathBuf,
    },

    /// Import a TOML preset. Every registry key is validated against its
    /// declared Kind before the import is applied. Replaces the current
    /// config (not a merge).
    Import {
        /// Source path.
        path: std::path::PathBuf,

        /// Merge into current config instead of replacing.
        #[arg(long)]
        merge: bool,
    },

    /// Start a config session — snapshots current values. Subsequent changes
    /// are still applied to config.toml live; `abort` reverts to the snapshot.
    Begin,

    /// Show diff between current config and the active session snapshot.
    Status,

    /// Close the active session and record an audit entry.
    Commit {
        /// Message describing the batch of changes.
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Revert config to the session's initial snapshot and discard the session.
    Abort,

    /// Named curated presets — practice stances expressed as config bundles.
    Preset {
        #[command(subcommand)]
        command: PresetCommand,
    },

    /// Show config file path(s).
    Path,
}

/// Subcommands for `werk config preset`.
#[derive(Debug, Subcommand)]
pub enum PresetCommand {
    /// List all shipped presets.
    List,

    /// Show the values a preset would apply.
    Show {
        /// Preset name.
        name: String,
    },

    /// Apply a preset — writes each of its values via the Set handler.
    Apply {
        /// Preset name.
        name: String,
    },
}

/// Hooks subcommands.
#[derive(Debug, Subcommand)]
pub enum HooksCommand {
    /// List configured hooks.
    List {
        /// Show scope and filter details.
        #[arg(short, long)]
        verbose: bool,
    },

    /// Add a hook for an event.
    Add {
        /// Event name (e.g., post_tension_resolved, post_mutation, post_*, pre_delete).
        event: String,

        /// Shell command to execute.
        command: String,

        /// Optional filter (e.g., "parent:42", "status:active").
        #[arg(short, long)]
        filter: Option<String>,

        /// Add to global config (~/.werk/config.toml) instead of workspace.
        #[arg(short, long)]
        global: bool,
    },

    /// Remove a hook (or a specific command from a chain).
    Rm {
        /// Event name (e.g., post_tension_resolved).
        event: String,

        /// Specific command to remove (omit to remove all commands for this event).
        command: Option<String>,

        /// Remove from global config.
        #[arg(short, long)]
        global: bool,
    },

    /// Test a hook by firing it with a synthetic or real event.
    Test {
        /// Event name to test (e.g., post_tension_resolved).
        event: String,

        /// Use a real tension for the test payload.
        #[arg(short, long)]
        tension: Option<String>,
    },

    /// Show hook execution log (from .werk/audit.jsonl).
    Log {
        /// Number of recent entries to show.
        /// Defaults to `hooks.log_tail` in config (fallback: 20).
        #[arg(short, long)]
        tail: Option<usize>,
    },

    /// Install shipped default hooks or git integration.
    Install {
        /// Install git pre-commit hook (.githooks/pre-commit + core.hooksPath).
        #[arg(long)]
        git: bool,

        /// Names of shipped hooks to install (omit to list available).
        #[arg(num_args = 0..)]
        hooks: Vec<String>,
    },
}
