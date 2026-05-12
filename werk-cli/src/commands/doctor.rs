//! `werk doctor` — diagnose and (optionally) repair the workspace.
//!
//! R-003. The top-level entry point for the self-healing surface. Every
//! verb here either:
//!   - reads workspace state and writes an append-only run artifact under
//!     `.werk/.doctor/runs/<ULID>/`, OR
//!   - mutates workspace state strictly through `werk_core::doctor_run::DoctorRun`,
//!     which records a verbatim backup, before/after BLAKE3 hashes, and an
//!     `actions.jsonl` entry per mutation.
//!
//! See `werk__doctor_workspace/analysis/repair_specs/r-003-doctor-cli.md`
//! for the contract (exit codes, JSON envelopes, capabilities schema).
//!
//! The existing `werk stats --health [--repair] [--yes]` surface routes
//! through the helpers in this module so the two paths share a single
//! source of truth.
//!
//! ## Exit codes (canonical)
//!
//! ```text
//! 0  healthy             1  findings_present     2  partial_fix
//! 3  fix_failed_rolled_back                      4  refused_unsafe
//! 5  concurrency_lost    6  online_required
//! 64 usage_error         66 no_input             73 cannot_create_output
//! 74 io_error
//! ```
//!
//! ## Known limitations (pass-3 substrate; tracked for follow-up PRs)
//!
//! - **No writer-ticket lock.** Two concurrent `werk doctor --fix`
//!   invocations both succeed; the last `finalize` wins the `latest`
//!   symlink. Real exit-5 `concurrency_lost` behavior needs fsqlite
//!   writer-ticket integration.
//! - **Detect/fix TOCTOU.** `count_noop_mutations` and
//!   `purge_noop_mutations` are independent statements; a concurrent
//!   MCP writer can drift the count between them.
//! - **WAL checkpoint forced before backup (pass 5).** Every triplet
//!   backup is now preceded by `PRAGMA wal_checkpoint(TRUNCATE);` so
//!   the on-disk `werk.db` reflects all committed bytes at copy time.
//!   Failures are non-fatal — the WAL sidecar is still copied, so
//!   replay-on-restore remains correct.
//! - **`undo` is crash-resumable (pass 5).** Each individual file
//!   restore is still tempfile + rename (per-file atomic). A
//!   `restore_in_progress` journal marker is written before the loop
//!   and cleared on success; if a crash interrupts the sequence the
//!   marker remains, the user re-runs `werk doctor undo <run-id>`,
//!   and the restore replays idempotently (restore-from-backup is the
//!   same regardless of how many times it runs). True multi-file
//!   atomicity is not achievable on POSIX without rolling our own
//!   journal-replay; the marker pattern gives the same user-visible
//!   guarantee for ≥ 99% of failure modes (mid-loop SIGKILL, power
//!   loss, OOM).

use crate::error::WerkError;
use crate::output::Output;
use chrono::Utc;
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use std::io::{IsTerminal, Write};
use std::path::Path;
use werk_core::Store;
use werk_core::doctor_run::{ACTION_OPS, ActionRecord, DOCTOR_CONTRACT_VERSION, DoctorRun};
use werk_core::horizon::Horizon;
use werk_core::store::PreferEdge;
use werk_shared::workspace::Workspace;

// ── Doctor exit codes (R-003 §2) ─────────────────────────────────────

pub const EXIT_HEALTHY: i32 = 0;
pub const EXIT_FINDINGS_PRESENT: i32 = 1;
pub const EXIT_PARTIAL_FIX: i32 = 2;
pub const EXIT_FIX_FAILED: i32 = 3;
pub const EXIT_REFUSED_UNSAFE: i32 = 4;
pub const EXIT_CONCURRENCY_LOST: i32 = 5;
pub const EXIT_ONLINE_REQUIRED: i32 = 6;
pub const EXIT_USAGE: i32 = 64;
pub const EXIT_NO_INPUT: i32 = 66;
pub const EXIT_CANNOT_CREATE_OUTPUT: i32 = 73;
pub const EXIT_IO_ERROR: i32 = 74;

// ── Clap surface ─────────────────────────────────────────────────────

/// Top-level `werk doctor` command.
///
/// `DoctorArgs` holds the read-only / `--fix` invocation flags. The
/// subcommand verbs (`undo`, `capabilities`, `ls`, …) are an enum below.
#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Apply repairs. Without this flag the doctor only diagnoses.
    #[arg(long)]
    pub fix: bool,

    /// With `--fix`, print the plan and exit 0 without mutating.
    #[arg(long, requires = "fix")]
    pub dry_run: bool,

    /// Skip the interactive purge confirmation. Required on non-TTY.
    #[arg(long)]
    pub yes: bool,

    /// Mega-command: returns `{summary, findings, actions_planned,
    /// recommended_command, capabilities_command}` in one JSON envelope.
    /// Mutually exclusive with `--fix`.
    #[arg(long, conflicts_with = "fix")]
    pub robot_triage: bool,

    /// Stable machine-readable output (alias of the global `--json`).
    #[arg(long)]
    pub robot: bool,

    /// Restrict to a subset of subsystems (comma-separated).
    #[arg(long, value_name = "LIST")]
    pub only: Option<String>,

    /// Show only findings new since this run-id.
    #[arg(long, value_name = "RUN_ID")]
    pub since: Option<String>,

    /// Print expanded evidence for one finding-id and exit.
    #[arg(long, value_name = "FINDING_ID")]
    pub explain: Option<String>,

    /// Conflict policy for `prune_duplicate_parent_edges` (R-005). Without
    /// this flag the fixer is soft-refused: the finding is reported but
    /// no edge is deleted. `oldest` keeps the contains-edge with the
    /// smallest ULID; `newest` keeps the largest (last-write-wins).
    #[arg(long, value_name = "POLICY", value_parser = ["oldest", "newest"])]
    pub prefer: Option<String>,

    /// Opt-in for `null_violating_child_horizon` (R-005). Nulling a
    /// child's horizon discards user temporal commitment, so the fixer
    /// is soft-refused unless this flag is passed.
    #[arg(long)]
    pub apply_horizon_fix: bool,

    /// Subcommand verb.
    #[command(subcommand)]
    pub verb: Option<DoctorVerb>,
}

/// Subcommand verbs for `werk doctor <verb>`.
#[derive(Debug, Subcommand)]
pub enum DoctorVerb {
    /// Restore from a prior run's backups.
    Undo {
        /// Run id (ULID) or the literal `latest`.
        target: String,
        /// Print the plan, do not restore.
        #[arg(long)]
        dry_run: bool,
    },
    /// Print the doctor's machine-readable contract.
    Capabilities,
    /// Cheap one-line liveness summary for CI scheduling.
    Health,
    /// Paste-ready agent handbook (markdown).
    RobotDocs,
    /// List runs under `.werk/.doctor/runs/`.
    Ls,
    /// Diff the latest run against `<ref>` (default: penultimate run).
    Diff {
        #[arg(value_name = "REF")]
        reference: Option<String>,
    },
    /// Prune old run directories. Both flags required.
    Gc {
        #[arg(long, value_name = "ISO8601_DATE")]
        before: Option<String>,
        #[arg(long)]
        yes: bool,
    },
    /// Copy `.werk/backups/*` to `~/.werk/backups/<slug>/` so the recovery
    /// substrate survives `werk nuke`. COPY-only: source is read-only.
    EvacuateBackups {
        /// Override the destination root (default: `~/.werk/backups/`).
        #[arg(long, value_name = "PATH")]
        dest: Option<std::path::PathBuf>,
        /// Print the plan without copying.
        #[arg(long)]
        dry_run: bool,
    },
}

// ── Capabilities (compile-time table) ────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Capabilities {
    pub schema_version: u32,
    pub doctor_contract_version: u32,
    pub werk_version: &'static str,
    pub detectors: Vec<DetectorSpec>,
    pub fixers: Vec<FixerSpec>,
    pub exit_codes: serde_json::Value,
    pub env_vars: Vec<&'static str>,
    pub subsystems: Vec<&'static str>,
    pub run_artifact_layout: serde_json::Value,
    pub compat_aliases: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DetectorSpec {
    pub id: &'static str,
    pub subsystem: &'static str,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_source: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserved_for: Option<&'static str>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FixerSpec {
    pub id: &'static str,
    pub detector: &'static str,
    pub available: bool,
    pub op: &'static str,
    pub inverse: &'static str,
    pub backs_up: Vec<&'static str>,
}

pub fn capabilities() -> Capabilities {
    Capabilities {
        schema_version: DOCTOR_CONTRACT_VERSION,
        doctor_contract_version: DOCTOR_CONTRACT_VERSION,
        werk_version: env!("CARGO_PKG_VERSION"),
        detectors: vec![
            DetectorSpec {
                id: "noop_mutations",
                subsystem: "store",
                available: true,
                description: Some(
                    "Count rows in store mutations where old_value = new_value (currently scoped to field='position')",
                ),
                evidence_source: Some("werk-core/src/store.rs:1191"),
                reserved_for: None,
            },
            DetectorSpec {
                id: "singleParent",
                subsystem: "edges",
                available: true,
                description: Some(
                    "Every tension has at most one `contains` edge pointing at it (Quint singleParent).",
                ),
                evidence_source: Some("specs/werk.qnt:393"),
                reserved_for: None,
            },
            DetectorSpec {
                id: "noSelfEdges",
                subsystem: "edges",
                available: true,
                description: Some("No edge has from_id == to_id (Quint noSelfEdges)."),
                evidence_source: Some("specs/werk.qnt:399"),
                reserved_for: None,
            },
            DetectorSpec {
                id: "edgesValid",
                subsystem: "edges",
                available: true,
                description: Some(
                    "Both endpoints of every edge reference an existing tension (Quint edgesValid).",
                ),
                evidence_source: Some("specs/werk.qnt:403"),
                reserved_for: None,
            },
            DetectorSpec {
                id: "siblingPositionsUnique",
                subsystem: "edges",
                available: true,
                description: Some(
                    "Among children connected by a contains-edge to the same parent, no two share a non-NULL position (Quint siblingPositionsUnique).",
                ),
                evidence_source: Some("specs/werk.qnt:418"),
                reserved_for: None,
            },
            DetectorSpec {
                id: "noContainmentViolations",
                subsystem: "edges",
                available: true,
                description: Some(
                    "Horizon containment: for each contains-edge with both horizons set, child.horizon <= parent.horizon (Quint noContainmentViolations).",
                ),
                evidence_source: Some("specs/werk.qnt:436"),
                reserved_for: None,
            },
            DetectorSpec {
                id: "undoneSubsetOfCompleted",
                subsystem: "gestures",
                available: true,
                description: Some(
                    "Every non-NULL gestures.undone_gesture_id references an existing gestures row (Quint undoneSubsetOfCompleted).",
                ),
                evidence_source: Some("specs/werk.qnt:450"),
                reserved_for: None,
            },
        ],
        fixers: vec![
            FixerSpec {
                id: "purge_noop_mutations",
                detector: "noop_mutations",
                available: true,
                op: "purge_noop_mutations",
                inverse: "restore_db_from_backup",
                backs_up: vec![".werk/werk.db", ".werk/werk.db-wal", ".werk/werk.db-shm"],
            },
            FixerSpec {
                id: "prune_duplicate_parent_edges",
                detector: "singleParent",
                available: true,
                op: "prune_duplicate_parent_edges",
                inverse: "restore_db_from_backup",
                backs_up: vec![".werk/werk.db", ".werk/werk.db-wal", ".werk/werk.db-shm"],
            },
            FixerSpec {
                id: "delete_self_edges",
                detector: "noSelfEdges",
                available: true,
                op: "delete_self_edges",
                inverse: "restore_db_from_backup",
                backs_up: vec![".werk/werk.db", ".werk/werk.db-wal", ".werk/werk.db-shm"],
            },
            FixerSpec {
                id: "delete_dangling_edges",
                detector: "edgesValid",
                available: true,
                op: "delete_dangling_edges",
                inverse: "restore_db_from_backup",
                backs_up: vec![".werk/werk.db", ".werk/werk.db-wal", ".werk/werk.db-shm"],
            },
            FixerSpec {
                id: "null_colliding_sibling_positions",
                detector: "siblingPositionsUnique",
                available: true,
                op: "null_colliding_sibling_positions",
                inverse: "restore_db_from_backup",
                backs_up: vec![".werk/werk.db", ".werk/werk.db-wal", ".werk/werk.db-shm"],
            },
            FixerSpec {
                id: "null_violating_child_horizon",
                detector: "noContainmentViolations",
                available: true,
                op: "null_violating_child_horizon",
                inverse: "restore_db_from_backup",
                backs_up: vec![".werk/werk.db", ".werk/werk.db-wal", ".werk/werk.db-shm"],
            },
            FixerSpec {
                id: "null_dangling_undo_gestures",
                detector: "undoneSubsetOfCompleted",
                available: true,
                op: "null_dangling_undo_gestures",
                inverse: "restore_db_from_backup",
                backs_up: vec![".werk/werk.db", ".werk/werk.db-wal", ".werk/werk.db-shm"],
            },
            // Meta-op: emitted only when the post-fix safety harness
            // sees a Quint-invariant violation and the run self-rolls
            // back. Listed here so explain/triage can name it; the op
            // is never invoked by a fixer.
            FixerSpec {
                id: "safety_harness_rollback",
                detector: "(meta)",
                available: true,
                op: "safety_harness_rollback",
                inverse: "restore_db_from_backup",
                backs_up: vec![".werk/werk.db", ".werk/werk.db-wal", ".werk/werk.db-shm"],
            },
        ],
        exit_codes: serde_json::json!({
            "0": "healthy",
            "1": "findings_present",
            "2": "partial_fix",
            "3": "fix_failed_rolled_back",
            "4": "refused_unsafe",
            "5": "concurrency_lost",
            "6": "online_required",
            "64": "usage_error",
            "66": "no_input",
            "73": "cannot_create_output",
            "74": "io_error",
        }),
        env_vars: vec!["NO_COLOR", "WERK_WORKSPACE"],
        subsystems: vec!["store", "edges", "gestures"],
        run_artifact_layout: serde_json::json!({
            "root": ".werk/.doctor/runs/<ULID>/",
            "files": ["report.json", "report.md", "actions.jsonl", "backups/", "stderr.log"],
            "latest_symlink": ".werk/.doctor/latest",
        }),
        compat_aliases: serde_json::json!({
            "werk stats --health": "werk doctor --only=store",
            "werk stats --health --repair --yes": "werk doctor --fix --only=store --yes",
        }),
    }
}

// ── Unified detector/fixer helpers (shared with `stats --health`) ────

/// One detector finding. Read-only output — never mutated by the doctor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub subsystem: String,
    pub severity: String,
    pub message: String,
    pub count: usize,
}

/// Aggregate result of running the `store` subsystem detectors.
#[derive(Debug, Clone, Default)]
pub struct StoreFindings {
    pub noop_mutations: usize,
}

impl StoreFindings {
    pub fn into_findings(&self) -> Vec<Finding> {
        let mut out = Vec::new();
        if self.noop_mutations > 0 {
            out.push(Finding {
                id: "fm-store-noop-mutations-non-position-fields".to_string(),
                subsystem: "store".to_string(),
                severity: "low".to_string(),
                message: format!(
                    "{} noop mutation row(s) in store_mutations",
                    self.noop_mutations
                ),
                count: self.noop_mutations,
            });
        }
        out
    }
}

/// Run the `store` subsystem detectors (read-only).
pub fn run_store_detect(store: &werk_core::Store) -> Result<StoreFindings, WerkError> {
    let noop = store.count_noop_mutations().map_err(WerkError::CoreError)?;
    Ok(StoreFindings {
        noop_mutations: noop,
    })
}

/// Result of running the `store` subsystem fixers under an active `DoctorRun`.
#[derive(Debug, Clone, Default)]
pub struct StoreFixResult {
    pub purged: usize,
}

/// Run the `store` subsystem fixers, routing every mutation through the
/// supplied `DoctorRun`. This is the single source of truth shared by both
/// `werk doctor --fix --only=store` and the legacy `werk stats --health --repair`.
pub fn run_store_fix(
    store: &werk_core::Store,
    workspace: &Workspace,
    run: &mut DoctorRun,
    findings: &StoreFindings,
) -> Result<StoreFixResult, WerkError> {
    if findings.noop_mutations == 0 {
        return Ok(StoreFixResult::default());
    }
    let db_path = workspace.db_path();
    // Best-effort WAL checkpoint so the backup triplet is self-consistent
    // (see record_db_intent rationale). Failures are non-fatal — the WAL
    // file is still copied alongside `werk.db` so replay is possible.
    if let Err(e) = store.wal_checkpoint_truncate() {
        eprintln!(
            "[doctor] wal_checkpoint(TRUNCATE) skipped: {} (backup will include WAL bytes)",
            e
        );
    }
    // fsqlite uses a `werk.db-wal` sidecar (and may use `-shm`). To get a
    // self-consistent backup we must back up the triplet — restoring the
    // main file alone while leaving a stale WAL on disk would replay the
    // WAL against the restored base and lose the rollback.
    let before_hash = run.record_backup(&db_path).map_err(WerkError::CoreError)?;
    for ext in ["werk.db-wal", "werk.db-shm"] {
        let sidecar = db_path.with_file_name(ext);
        if sidecar.exists() {
            run.record_backup(&sidecar).map_err(WerkError::CoreError)?;
        }
    }
    // Crash-safety: write the action **before** the mutation so a SIGKILL
    // between backup-success and mutation-success doesn't leave a silently
    // mutated DB whose `actions.jsonl` is empty (the undo path would then
    // refuse to roll back even though a backup exists). `after_hash` is a
    // best-effort post-hoc field; undo only needs `before_hash` + backup.
    let target = db_path
        .strip_prefix(workspace.root())
        .unwrap_or(&db_path)
        .display()
        .to_string();
    let intent = ActionRecord {
        timestamp: Utc::now().to_rfc3339(),
        op: "purge_noop_mutations".to_string(),
        target: target.clone(),
        before_hash: Some(before_hash),
        after_hash: None,
        note: Some("intent recorded; mutation pending".to_string()),
    };
    debug_assert!(
        ACTION_OPS.contains(&intent.op.as_str()),
        "op `{}` not in ACTION_OPS table — capabilities will drift",
        intent.op
    );
    run.record_action(intent).map_err(WerkError::CoreError)?;
    let purged_count = store.purge_noop_mutations().map_err(WerkError::CoreError)?;
    let after_hash = std::fs::read(&db_path)
        .ok()
        .map(|b| blake3::hash(&b).to_hex().to_string());
    // Append a completion record so the action log reflects reality (the
    // undo path replays in reverse and tolerates this trailing record;
    // duplicate `op` rows on the same target are idempotent under undo
    // because restore is a file replace, not a state delta).
    let completion = ActionRecord {
        timestamp: Utc::now().to_rfc3339(),
        op: "purge_noop_mutations".to_string(),
        target,
        before_hash: None,
        after_hash,
        note: Some(format!("purged {} noop mutation row(s)", purged_count)),
    };
    run.record_action(completion).map_err(WerkError::CoreError)?;
    Ok(StoreFixResult {
        purged: purged_count,
    })
}

// ── Quint-invariant detectors / fixers (R-005) ───────────────────────

/// Aggregate result of running the `edges` subsystem detectors.
#[derive(Debug, Clone, Default)]
pub struct EdgesFindings {
    pub multi_parent: Vec<werk_core::DoctorMultiParentRow>,
    pub self_edges: Vec<werk_core::DoctorEdgeRow>,
    pub dangling_edges: Vec<werk_core::DoctorEdgeRow>,
    pub sibling_collisions: Vec<werk_core::DoctorSiblingCollisionRow>,
    /// Children whose horizon exceeds parent's. The CLI does the
    /// `Horizon::parse` comparison; the store returns raw pairs.
    pub horizon_violations: Vec<HorizonViolation>,
    /// Pairs where either side's horizon string failed to parse — surfaced
    /// as a separate low-severity finding (not a containment violation).
    pub horizon_unparseable: Vec<HorizonUnparseable>,
}

#[derive(Debug, Clone)]
pub struct HorizonViolation {
    pub parent_id: String,
    pub child_id: String,
    pub parent_horizon: String,
    pub child_horizon: String,
}

#[derive(Debug, Clone)]
pub struct HorizonUnparseable {
    pub parent_id: String,
    pub child_id: String,
    pub raw_parent: String,
    pub raw_child: String,
    pub error: String,
}

impl EdgesFindings {
    pub fn is_empty(&self) -> bool {
        self.multi_parent.is_empty()
            && self.self_edges.is_empty()
            && self.dangling_edges.is_empty()
            && self.sibling_collisions.is_empty()
            && self.horizon_violations.is_empty()
            && self.horizon_unparseable.is_empty()
    }

    pub fn into_findings(&self) -> Vec<Finding> {
        let mut out = Vec::new();
        for v in &self.multi_parent {
            out.push(Finding {
                id: "fm-edges-multi-parent".to_string(),
                subsystem: "edges".to_string(),
                severity: "high".to_string(),
                message: format!(
                    "tension {} has {} contains-edges (singleParent violated)",
                    v.tension_id,
                    v.parent_edge_ids.len()
                ),
                count: v.parent_edge_ids.len(),
            });
        }
        if !self.self_edges.is_empty() {
            out.push(Finding {
                id: "fm-edges-self-loop".to_string(),
                subsystem: "edges".to_string(),
                severity: "high".to_string(),
                message: format!("{} self-referencing edge(s)", self.self_edges.len()),
                count: self.self_edges.len(),
            });
        }
        if !self.dangling_edges.is_empty() {
            out.push(Finding {
                id: "fm-edges-dangling".to_string(),
                subsystem: "edges".to_string(),
                severity: "high".to_string(),
                message: format!(
                    "{} edge(s) reference missing tension(s)",
                    self.dangling_edges.len()
                ),
                count: self.dangling_edges.len(),
            });
        }
        for c in &self.sibling_collisions {
            out.push(Finding {
                id: "fm-edges-sibling-position-collision".to_string(),
                subsystem: "edges".to_string(),
                severity: "medium".to_string(),
                message: format!(
                    "parent {} has {} children at position {}",
                    c.parent_id,
                    c.child_ids.len(),
                    c.position
                ),
                count: c.child_ids.len(),
            });
        }
        for v in &self.horizon_violations {
            out.push(Finding {
                id: "fm-edges-horizon-containment".to_string(),
                subsystem: "edges".to_string(),
                severity: "medium".to_string(),
                message: format!(
                    "child {}'s horizon {} exceeds parent {}'s horizon {}",
                    v.child_id, v.child_horizon, v.parent_id, v.parent_horizon
                ),
                count: 1,
            });
        }
        for u in &self.horizon_unparseable {
            out.push(Finding {
                id: "fm-edges-horizon-unparseable".to_string(),
                subsystem: "edges".to_string(),
                severity: "low".to_string(),
                message: format!(
                    "could not parse horizon on edge {}->{}: {}",
                    u.parent_id, u.child_id, u.error
                ),
                count: 1,
            });
        }
        out
    }
}

/// Aggregate result of running the `gestures` subsystem detectors.
#[derive(Debug, Clone, Default)]
pub struct GesturesFindings {
    pub dangling_undo: Vec<werk_core::DoctorDanglingUndoRow>,
}

impl GesturesFindings {
    pub fn is_empty(&self) -> bool {
        self.dangling_undo.is_empty()
    }

    pub fn into_findings(&self) -> Vec<Finding> {
        let mut out = Vec::new();
        if !self.dangling_undo.is_empty() {
            out.push(Finding {
                id: "fm-gestures-undone-dangling".to_string(),
                subsystem: "gestures".to_string(),
                severity: "medium".to_string(),
                message: format!(
                    "{} gesture(s) reference a non-existent undone_gesture_id",
                    self.dangling_undo.len()
                ),
                count: self.dangling_undo.len(),
            });
        }
        out
    }
}

/// Read-only detector pass for the `edges` subsystem. All six Quint
/// edge invariants in one entry point.
pub fn run_edges_detect(store: &Store) -> Result<EdgesFindings, WerkError> {
    let multi_parent = store
        .list_multi_parent_violations()
        .map_err(WerkError::CoreError)?;
    let self_edges = store.list_self_edges().map_err(WerkError::CoreError)?;
    let dangling_edges = store.list_dangling_edges().map_err(WerkError::CoreError)?;
    let sibling_collisions = store
        .list_sibling_position_collisions()
        .map_err(WerkError::CoreError)?;
    let horizon_pairs = store
        .list_horizon_pairs_for_contains_edges()
        .map_err(WerkError::CoreError)?;
    let mut horizon_violations = Vec::new();
    let mut horizon_unparseable = Vec::new();
    for p in horizon_pairs {
        match (Horizon::parse(&p.parent_horizon), Horizon::parse(&p.child_horizon)) {
            (Ok(ph), Ok(ch)) => {
                if ch > ph {
                    horizon_violations.push(HorizonViolation {
                        parent_id: p.parent_id,
                        child_id: p.child_id,
                        parent_horizon: p.parent_horizon,
                        child_horizon: p.child_horizon,
                    });
                }
            }
            (Err(e), _) => {
                horizon_unparseable.push(HorizonUnparseable {
                    parent_id: p.parent_id,
                    child_id: p.child_id,
                    raw_parent: p.parent_horizon,
                    raw_child: p.child_horizon,
                    error: format!("parent: {}", e),
                });
            }
            (_, Err(e)) => {
                horizon_unparseable.push(HorizonUnparseable {
                    parent_id: p.parent_id,
                    child_id: p.child_id,
                    raw_parent: p.parent_horizon,
                    raw_child: p.child_horizon,
                    error: format!("child: {}", e),
                });
            }
        }
    }
    Ok(EdgesFindings {
        multi_parent,
        self_edges,
        dangling_edges,
        sibling_collisions,
        horizon_violations,
        horizon_unparseable,
    })
}

/// Read-only detector pass for the `gestures` subsystem.
pub fn run_gestures_detect(store: &Store) -> Result<GesturesFindings, WerkError> {
    let dangling_undo = store
        .list_dangling_undo_gestures()
        .map_err(WerkError::CoreError)?;
    Ok(GesturesFindings { dangling_undo })
}

#[derive(Debug, Clone, Default)]
pub struct EdgesFixResult {
    pub multi_parent_pruned: usize,
    pub multi_parent_soft_refused: usize,
    pub self_edges_deleted: usize,
    pub dangling_edges_deleted: usize,
    pub sibling_positions_nulled: usize,
    pub horizon_violations_nulled: usize,
    pub horizon_soft_refused: usize,
    pub parent_ids_reconciled: usize,
}

#[derive(Debug, Clone, Default)]
pub struct GesturesFixResult {
    pub dangling_undo_nulled: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct EdgesFixOptions {
    /// `--prefer` policy (None = soft-refuse multi-parent fixer).
    pub prefer: Option<PreferEdge>,
    /// `--apply-horizon-fix` (false = soft-refuse horizon fixer).
    pub apply_horizon_fix: bool,
}

/// Helper: ensure the DB triplet is backed up exactly once per run and
/// record one intent ActionRecord for `op` against `werk.db`. The intent
/// record's `before_hash` is computed FRESH at call time (not cached) so
/// each fixer's hash reflects the live state immediately before its
/// mutation.
fn record_db_intent(
    store: &Store,
    run: &mut DoctorRun,
    workspace: &Workspace,
    op: &str,
    note: &str,
) -> Result<(), WerkError> {
    let db_path = workspace.db_path();
    // Force a WAL checkpoint (best-effort) so the on-disk `werk.db`
    // bytes reflect all committed writes BEFORE we snapshot. Without
    // this, the live `werk.db` can lag the `-wal` sidecar and the
    // three-file copy can be internally inconsistent under replay.
    // Failures are surfaced as a stderr note; a stale-WAL backup is
    // still correct (WAL is included in the triplet), just larger.
    if let Err(e) = store.wal_checkpoint_truncate() {
        eprintln!(
            "[doctor] wal_checkpoint(TRUNCATE) skipped: {} (backup will include WAL bytes)",
            e
        );
    }
    // `before_hash` in the intent record is the hash that `cmd_undo`
    // verifies against AFTER restoring from `backups/`. The restored
    // file's bytes equal the SNAPSHOT bytes captured by record_backup_once,
    // so before_hash must equal the snapshot hash — not a fresh
    // hash_current of the live file (which can diverge from the snapshot
    // when fsqlite housekeeping touches bytes between the copy and the
    // fresh read). Per-fixer freshness is provided by the completion
    // record's after_hash field, which captures post-mutation state.
    let snapshot_hash = run
        .record_backup_once(&db_path)
        .map_err(WerkError::CoreError)?;
    for ext in ["werk.db-wal", "werk.db-shm"] {
        let sidecar = db_path.with_file_name(ext);
        if sidecar.exists() {
            run.record_backup_once(&sidecar)
                .map_err(WerkError::CoreError)?;
        }
    }
    let target = db_path
        .strip_prefix(workspace.root())
        .unwrap_or(&db_path)
        .display()
        .to_string();
    let intent = ActionRecord {
        timestamp: Utc::now().to_rfc3339(),
        op: op.to_string(),
        target,
        before_hash: Some(snapshot_hash),
        after_hash: None,
        note: Some(note.to_string()),
    };
    debug_assert!(
        ACTION_OPS.contains(&intent.op.as_str()),
        "op `{}` not in ACTION_OPS table — capabilities will drift",
        intent.op
    );
    run.record_action(intent).map_err(WerkError::CoreError)?;
    Ok(())
}

/// Helper: append a completion record for `op` after a mutation.
fn record_db_completion(
    run: &mut DoctorRun,
    workspace: &Workspace,
    op: &str,
    note: String,
) -> Result<(), WerkError> {
    let db_path = workspace.db_path();
    let after_hash = DoctorRun::hash_current(&db_path).ok().flatten();
    let target = db_path
        .strip_prefix(workspace.root())
        .unwrap_or(&db_path)
        .display()
        .to_string();
    let completion = ActionRecord {
        timestamp: Utc::now().to_rfc3339(),
        op: op.to_string(),
        target,
        before_hash: None,
        after_hash,
        note: Some(note),
    };
    run.record_action(completion).map_err(WerkError::CoreError)?;
    Ok(())
}

/// Run the `edges` subsystem fixers. Each per-fixer mutation is
/// journaled with its own intent/completion pair (sharing one backup
/// for the DB triplet). Soft-refused fixers emit no actions but their
/// findings remain visible.
pub fn run_edges_fix(
    store: &Store,
    workspace: &Workspace,
    run: &mut DoctorRun,
    findings: &EdgesFindings,
    options: EdgesFixOptions,
) -> Result<EdgesFixResult, WerkError> {
    let mut result = EdgesFixResult::default();
    if findings.is_empty() {
        return Ok(result);
    }

    // 1. singleParent — soft-refused unless --prefer is passed.
    if !findings.multi_parent.is_empty() {
        if let Some(prefer) = options.prefer {
            record_db_intent(
                store,
                run,
                workspace,
                "prune_duplicate_parent_edges",
                &format!(
                    "intent recorded; pruning {} multi-parent tension(s) with prefer={:?}",
                    findings.multi_parent.len(),
                    prefer
                ),
            )?;
            let pr = store
                .doctor_prune_duplicate_parent_edges(prefer)
                .map_err(WerkError::CoreError)?;
            result.multi_parent_pruned = pr.deleted_edge_ids.len();
            result.parent_ids_reconciled += pr.parent_ids_reconciled;
            record_db_completion(
                run,
                workspace,
                "prune_duplicate_parent_edges",
                format!(
                    "pruned {} duplicate contains-edge(s) across {} tension(s); reconciled parent_id on {}",
                    pr.deleted_edge_ids.len(),
                    pr.affected_tension_ids.len(),
                    pr.parent_ids_reconciled
                ),
            )?;
        } else {
            result.multi_parent_soft_refused = findings.multi_parent.len();
        }
    }

    // 2. noSelfEdges — always applied.
    if !findings.self_edges.is_empty() {
        record_db_intent(
            store,
            run,
            workspace,
            "delete_self_edges",
            &format!(
                "intent recorded; deleting {} self-edge(s)",
                findings.self_edges.len()
            ),
        )?;
        let sr = store
            .doctor_delete_self_edges()
            .map_err(WerkError::CoreError)?;
        result.self_edges_deleted = sr.deleted;
        result.parent_ids_reconciled += sr.parent_ids_reconciled;
        record_db_completion(
            run,
            workspace,
            "delete_self_edges",
            format!(
                "deleted {} self-referencing edge(s); reconciled parent_id on {}",
                sr.deleted, sr.parent_ids_reconciled
            ),
        )?;
    }

    // 3. edgesValid — always applied.
    if !findings.dangling_edges.is_empty() {
        record_db_intent(
            store,
            run,
            workspace,
            "delete_dangling_edges",
            &format!(
                "intent recorded; deleting {} dangling edge(s)",
                findings.dangling_edges.len()
            ),
        )?;
        let dr = store
            .doctor_delete_dangling_edges()
            .map_err(WerkError::CoreError)?;
        result.dangling_edges_deleted = dr.deleted;
        result.parent_ids_reconciled += dr.parent_ids_reconciled;
        record_db_completion(
            run,
            workspace,
            "delete_dangling_edges",
            format!(
                "deleted {} dangling edge(s); reconciled parent_id on {}",
                dr.deleted, dr.parent_ids_reconciled
            ),
        )?;
    }

    // 4. siblingPositionsUnique — always applied.
    if !findings.sibling_collisions.is_empty() {
        record_db_intent(
            store,
            run,
            workspace,
            "null_colliding_sibling_positions",
            &format!(
                "intent recorded; resolving {} position-collision group(s)",
                findings.sibling_collisions.len()
            ),
        )?;
        let sf = store
            .doctor_null_colliding_sibling_positions()
            .map_err(WerkError::CoreError)?;
        result.sibling_positions_nulled = sf.nulled.len();
        record_db_completion(
            run,
            workspace,
            "null_colliding_sibling_positions",
            format!(
                "nulled position on {} child tension(s) across {} parent(s)",
                sf.nulled.len(),
                sf.parent_count
            ),
        )?;
    }

    // 5. noContainmentViolations — soft-refused unless --apply-horizon-fix.
    if !findings.horizon_violations.is_empty() {
        if options.apply_horizon_fix {
            let targets: Vec<String> = findings
                .horizon_violations
                .iter()
                .map(|v| v.child_id.clone())
                .collect();
            record_db_intent(
                store,
                run,
                workspace,
                "null_violating_child_horizon",
                &format!(
                    "intent recorded; nulling horizon on {} child tension(s)",
                    targets.len()
                ),
            )?;
            let pairs = store
                .doctor_null_violating_child_horizons(&targets)
                .map_err(WerkError::CoreError)?;
            result.horizon_violations_nulled = pairs.len();
            record_db_completion(
                run,
                workspace,
                "null_violating_child_horizon",
                format!(
                    "nulled horizon on {} child tension(s); transitive violations not auto-resolved (re-run if needed)",
                    pairs.len()
                ),
            )?;
        } else {
            result.horizon_soft_refused = findings.horizon_violations.len();
        }
    }

    Ok(result)
}

/// Run the `gestures` subsystem fixers.
pub fn run_gestures_fix(
    store: &Store,
    workspace: &Workspace,
    run: &mut DoctorRun,
    findings: &GesturesFindings,
) -> Result<GesturesFixResult, WerkError> {
    let mut result = GesturesFixResult::default();
    if findings.dangling_undo.is_empty() {
        return Ok(result);
    }
    let targets: Vec<String> = findings
        .dangling_undo
        .iter()
        .map(|r| r.gesture_id.clone())
        .collect();
    record_db_intent(
        store,
        run,
        workspace,
        "null_dangling_undo_gestures",
        &format!(
            "intent recorded; nulling undone_gesture_id on {} phantom undo-gesture(s)",
            targets.len()
        ),
    )?;
    let count = store
        .doctor_null_dangling_undo_gestures(&targets)
        .map_err(WerkError::CoreError)?;
    result.dangling_undo_nulled = count;
    record_db_completion(
        run,
        workspace,
        "null_dangling_undo_gestures",
        format!("nulled undone_gesture_id on {} phantom undo-gesture(s)", count),
    )?;
    Ok(result)
}

// ── Safety harness (W-1) ─────────────────────────────────────────────
//
// W-1 enforcement (per analysis/safety_envelope.md): after `cmd_fix`
// completes its subsystem fixer passes, it re-detects every Quint
// invariant via `run_edges_detect` / `run_gestures_detect` and compares
// the per-violator key sets to the pre-fix snapshot. Any NEWLY
// introduced violator (one that didn't exist pre-fix) triggers an
// in-flight rollback via `safety_harness_rollback` and finalizes the
// run with EXIT_FIX_FAILED. The per-id SET diff (rather than a coarse
// "any violation of invariant X" check) ensures a soft-refused
// fixer's pre-existing residuals are correctly distinguished from
// post-fix-introduced ones.
//
// TOCTOU: a concurrent MCP writer adding a violator between detector
// passes is attributed to this run and triggers rollback. Documented
// as pass-3 limitation #2.

/// In-flight rollback: replay the run's own backups in place. Used by
/// `cmd_fix` when the post-fix safety harness reports a violation.
/// Each per-file replacement is journaled with its own intent/completion
/// record so a SIGKILL mid-rollback leaves a recoverable trail.
fn safety_harness_rollback(
    run: &mut DoctorRun,
    workspace: &Workspace,
    violated_ids: &[String],
) -> Result<usize, WerkError> {
    let db_path = workspace.db_path();
    let mut restored = 0usize;
    let candidates: Vec<std::path::PathBuf> = ["werk.db", "werk.db-wal", "werk.db-shm"]
        .into_iter()
        .map(|name| db_path.with_file_name(name))
        .collect();
    for live_path in &candidates {
        let rel = live_path
            .strip_prefix(workspace.root())
            .unwrap_or(live_path)
            .to_path_buf();
        let backup_path = run.run_dir().join("backups").join(&rel);
        let rel_str = rel.display().to_string();
        if !backup_path.exists() {
            // No backup for this sidecar. If a live sidecar exists, it
            // was created by fsqlite DURING the fix run and must be
            // removed — leaving it would let fsqlite replay a stale WAL
            // against the restored base on next open (mirrors cmd_undo's
            // stale-sidecar handling). Journaled so cmd_undo can replay.
            if live_path.exists() {
                let before_now =
                    DoctorRun::hash_current(live_path).map_err(WerkError::CoreError)?;
                let intent = ActionRecord {
                    timestamp: Utc::now().to_rfc3339(),
                    op: "safety_harness_rollback".to_string(),
                    target: rel_str.clone(),
                    before_hash: before_now,
                    after_hash: None,
                    note: Some(format!(
                        "W-1 violated post-fix: [{}]; removing stale sidecar {} (no backup)",
                        violated_ids.join(","),
                        rel_str
                    )),
                };
                run.record_action(intent).map_err(WerkError::CoreError)?;
                std::fs::remove_file(live_path).map_err(|e| {
                    WerkError::IoError(format!(
                        "rollback remove stale sidecar {}: {}",
                        rel_str, e
                    ))
                })?;
                let completion = ActionRecord {
                    timestamp: Utc::now().to_rfc3339(),
                    op: "safety_harness_rollback".to_string(),
                    target: rel_str,
                    before_hash: None,
                    after_hash: None,
                    note: Some("removed stale sidecar".to_string()),
                };
                run.record_action(completion).map_err(WerkError::CoreError)?;
                restored += 1;
            }
            continue;
        }
        // before_hash for the rollback's intent record is the SNAPSHOT
        // hash — what the restored bytes will be — NOT the pre-rollback
        // live hash. cmd_undo's restore_target verifies post-restore
        // bytes against before_hash; setting before_hash to the
        // pre-rollback hash would fail verification on subsequent
        // cmd_undo replay (review H8).
        let snapshot_bytes = std::fs::read(&backup_path).map_err(|e| {
            WerkError::IoError(format!("read backup {}: {}", rel_str, e))
        })?;
        let snapshot_hash = blake3::hash(&snapshot_bytes).to_hex().to_string();
        let intent = ActionRecord {
            timestamp: Utc::now().to_rfc3339(),
            op: "safety_harness_rollback".to_string(),
            target: rel_str.clone(),
            before_hash: Some(snapshot_hash.clone()),
            after_hash: None,
            note: Some(format!(
                "W-1 violated post-fix: [{}]; restoring {}",
                violated_ids.join(","),
                rel_str
            )),
        };
        run.record_action(intent).map_err(WerkError::CoreError)?;
        // Atomic tempfile + rename in place.
        let tmp = live_path.with_extension(format!(
            "{}.harness.tmp.{}",
            live_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("partial"),
            std::process::id()
        ));
        std::fs::copy(&backup_path, &tmp).map_err(|e| {
            WerkError::IoError(format!("rollback copy {}: {}", rel_str, e))
        })?;
        std::fs::rename(&tmp, live_path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            WerkError::IoError(format!("rollback rename {}: {}", rel_str, e))
        })?;
        let completion = ActionRecord {
            timestamp: Utc::now().to_rfc3339(),
            op: "safety_harness_rollback".to_string(),
            target: rel_str,
            before_hash: None,
            after_hash: Some(snapshot_hash),
            note: Some("restored from in-flight run backup".to_string()),
        };
        run.record_action(completion).map_err(WerkError::CoreError)?;
        restored += 1;
    }
    Ok(restored)
}

// ── Top-level dispatch ───────────────────────────────────────────────

/// Entry point invoked by `main.rs` for `Commands::Doctor`. Returns the
/// canonical doctor exit code. `main.rs` is responsible for `process::exit`
/// after this returns so Drop order is preserved.
pub fn cmd_doctor(output: &Output, args: DoctorArgs) -> Result<i32, WerkError> {
    // Verb dispatch (subcommand wins over read/fix flags when both present).
    if let Some(verb) = args.verb {
        return match verb {
            DoctorVerb::Capabilities => cmd_capabilities(output),
            DoctorVerb::Health => cmd_health(output),
            DoctorVerb::RobotDocs => cmd_robot_docs(),
            DoctorVerb::Ls => cmd_ls(output),
            DoctorVerb::Diff { reference } => cmd_diff(output, reference),
            DoctorVerb::Gc { before, yes } => cmd_gc(output, before, yes),
            DoctorVerb::Undo { target, dry_run } => cmd_undo(output, target, dry_run),
            DoctorVerb::EvacuateBackups { dest, dry_run } => {
                cmd_evacuate_backups(output, dest, dry_run)
            }
        };
    }

    if let Some(finding_id) = args.explain {
        return cmd_explain(output, &finding_id);
    }

    // Flag-based dispatch.
    if args.robot_triage {
        return cmd_robot_triage(output, args.only.as_deref());
    }

    if args.fix {
        // `value_parser` on DoctorArgs.prefer restricts values to
        // {"oldest","newest"} at parse time; unknown values would be
        // rejected by clap with a usage error before we reach this
        // dispatch (review S5). The match below is exhaustive for the
        // accepted set.
        let prefer = match args.prefer.as_deref() {
            None => None,
            Some("oldest") => Some(PreferEdge::Oldest),
            Some("newest") => Some(PreferEdge::Newest),
            Some(_unreachable) => unreachable!("clap value_parser restricts --prefer"),
        };
        return cmd_fix(
            output,
            args.dry_run,
            args.yes,
            args.only.as_deref(),
            args.robot,
            EdgesFixOptions {
                prefer,
                apply_horizon_fix: args.apply_horizon_fix,
            },
        );
    }

    cmd_diagnose(
        output,
        args.only.as_deref(),
        args.since.as_deref(),
        args.robot,
    )
}

// ── Diagnose (read-only) ─────────────────────────────────────────────

fn cmd_diagnose(
    output: &Output,
    only: Option<&str>,
    since: Option<&str>,
    robot: bool,
) -> Result<i32, WerkError> {
    let workspace = match Workspace::discover() {
        Ok(w) => w,
        Err(_) => {
            emit_refusal(output, "no workspace discovered (cd into a workspace)");
            return Ok(EXIT_REFUSED_UNSAFE);
        }
    };
    let subsystems = parse_only(only)?;
    let store = workspace.open_store()?;
    let run = DoctorRun::start(workspace.root()).map_err(|e| {
        emit_refusal(output, &format!("cannot create run dir: {}", e));
        WerkError::IoError(e.to_string())
    })?;

    let mut findings = Vec::new();
    if subsystems.includes_store() {
        let store_findings = run_store_detect(&store)?;
        findings.extend(store_findings.into_findings());
    }
    if subsystems.includes_edges() {
        let edges_findings = run_edges_detect(&store)?;
        findings.extend(edges_findings.into_findings());
    }
    if subsystems.includes_gestures() {
        let gestures_findings = run_gestures_detect(&store)?;
        findings.extend(gestures_findings.into_findings());
    }

    // `--since` filter applies to stdout only; the run artifact still holds
    // the full lossless `findings` list.
    let visible_findings: Vec<Finding> = if let Some(ref_id) = since {
        let prior = load_run_report(&workspace, ref_id)?;
        let prior_ids: std::collections::HashSet<String> = prior
            .findings
            .iter()
            .filter_map(|v| v.get("id").and_then(|s| s.as_str()).map(String::from))
            .collect();
        findings
            .iter()
            .filter(|f| !prior_ids.contains(&f.id))
            .cloned()
            .collect()
    } else {
        findings.clone()
    };

    // Exit code reports what the USER sees on stdout. With `--since`,
    // findings already present in the prior run are filtered out, so the
    // visible set is the truth — exit 1 only when there's something new.
    // (The lossless `findings` list still goes into report.json.)
    let exit_code = if visible_findings.is_empty() {
        EXIT_HEALTHY
    } else {
        EXIT_FINDINGS_PRESENT
    };

    let summary = if visible_findings.is_empty() {
        if findings.is_empty() {
            "no findings".to_string()
        } else {
            format!("no new findings since {}", since.unwrap_or(""))
        }
    } else {
        format!("{} finding(s)", visible_findings.len())
    };

    let findings_json: Vec<serde_json::Value> = findings
        .iter()
        .map(|f| serde_json::to_value(f).unwrap_or(serde_json::Value::Null))
        .collect();

    let report = run
        .finalize(exit_code, &summary, findings_json)
        .map_err(WerkError::CoreError)?;

    let envelope = serde_json::json!({
        "schema_version": DOCTOR_CONTRACT_VERSION,
        "verb": "doctor",
        "run_id": report.run_id,
        "exit_code": exit_code,
        "data": {
            "summary": summary,
            "findings": visible_findings,
            "action_count": 0_usize,
        }
    });

    print_envelope(output, robot, &envelope, |_| {
        if visible_findings.is_empty() {
            println!("Healthy.");
        } else {
            println!("{} finding(s):", visible_findings.len());
            for f in &visible_findings {
                println!("  [{}] {}: {}", f.severity, f.id, f.message);
            }
            eprintln!("Run `werk doctor --fix --yes` to repair.");
        }
    });

    Ok(exit_code)
}

// ── Fix ──────────────────────────────────────────────────────────────

fn cmd_fix(
    output: &Output,
    dry_run: bool,
    yes: bool,
    only: Option<&str>,
    robot: bool,
    options: EdgesFixOptions,
) -> Result<i32, WerkError> {
    if !yes && !std::io::stdin().is_terminal() {
        emit_refusal(output, "--yes required for --fix on non-TTY");
        return Ok(EXIT_USAGE);
    }

    let workspace = match Workspace::discover() {
        Ok(w) => w,
        Err(_) => {
            emit_refusal(output, "no workspace discovered");
            return Ok(EXIT_REFUSED_UNSAFE);
        }
    };
    let subsystems = parse_only(only)?;
    let store = workspace.open_store()?;

    // Detect first across all requested subsystems. Dry-run uses this
    // result without starting a DoctorRun so the run-id is null.
    let store_findings = if subsystems.includes_store() {
        run_store_detect(&store)?
    } else {
        StoreFindings::default()
    };
    let edges_findings = if subsystems.includes_edges() {
        run_edges_detect(&store)?
    } else {
        EdgesFindings::default()
    };
    let gestures_findings = if subsystems.includes_gestures() {
        run_gestures_detect(&store)?
    } else {
        GesturesFindings::default()
    };

    let mut findings: Vec<Finding> = Vec::new();
    findings.extend(store_findings.into_findings());
    findings.extend(edges_findings.into_findings());
    findings.extend(gestures_findings.into_findings());

    let mut actions_planned = Vec::new();
    if store_findings.noop_mutations > 0 {
        actions_planned.push(serde_json::json!({
            "op": "purge_noop_mutations",
            "target": ".werk/werk.db",
            "fixer": "purge_noop_mutations",
        }));
    }
    if !edges_findings.multi_parent.is_empty() && options.prefer.is_some() {
        actions_planned.push(serde_json::json!({
            "op": "prune_duplicate_parent_edges",
            "target": ".werk/werk.db",
            "fixer": "prune_duplicate_parent_edges",
        }));
    }
    if !edges_findings.self_edges.is_empty() {
        actions_planned.push(serde_json::json!({
            "op": "delete_self_edges",
            "target": ".werk/werk.db",
            "fixer": "delete_self_edges",
        }));
    }
    if !edges_findings.dangling_edges.is_empty() {
        actions_planned.push(serde_json::json!({
            "op": "delete_dangling_edges",
            "target": ".werk/werk.db",
            "fixer": "delete_dangling_edges",
        }));
    }
    if !edges_findings.sibling_collisions.is_empty() {
        actions_planned.push(serde_json::json!({
            "op": "null_colliding_sibling_positions",
            "target": ".werk/werk.db",
            "fixer": "null_colliding_sibling_positions",
        }));
    }
    if !edges_findings.horizon_violations.is_empty() && options.apply_horizon_fix {
        actions_planned.push(serde_json::json!({
            "op": "null_violating_child_horizon",
            "target": ".werk/werk.db",
            "fixer": "null_violating_child_horizon",
        }));
    }
    if !gestures_findings.dangling_undo.is_empty() {
        actions_planned.push(serde_json::json!({
            "op": "null_dangling_undo_gestures",
            "target": ".werk/werk.db",
            "fixer": "null_dangling_undo_gestures",
        }));
    }

    // Soft-refused fixers — surface in dry-run output so users know why.
    let mut soft_refused = Vec::new();
    if !edges_findings.multi_parent.is_empty() && options.prefer.is_none() {
        soft_refused.push(serde_json::json!({
            "fixer": "prune_duplicate_parent_edges",
            "reason": "policy required: pass --prefer=oldest or --prefer=newest",
            "count": edges_findings.multi_parent.len(),
        }));
    }
    if !edges_findings.horizon_violations.is_empty() && !options.apply_horizon_fix {
        soft_refused.push(serde_json::json!({
            "fixer": "null_violating_child_horizon",
            "reason": "opt-in required: pass --apply-horizon-fix",
            "count": edges_findings.horizon_violations.len(),
        }));
    }

    if dry_run {
        let envelope = serde_json::json!({
            "schema_version": DOCTOR_CONTRACT_VERSION,
            "verb": "doctor",
            "run_id": serde_json::Value::Null,
            "exit_code": EXIT_HEALTHY,
            "data": {
                "dry_run": true,
                "findings": findings,
                "actions_planned": actions_planned,
                "soft_refused": soft_refused,
            }
        });
        print_envelope(output, robot, &envelope, |_| {
            println!("Plan:");
            if actions_planned.is_empty() {
                println!("  (no actions)");
            } else {
                for a in &actions_planned {
                    println!("  {}", a);
                }
            }
            if !soft_refused.is_empty() {
                eprintln!("Soft-refused fixers (re-run with the named flag):");
                for s in &soft_refused {
                    eprintln!("  {}", s);
                }
            }
        });
        return Ok(EXIT_HEALTHY);
    }

    // Interactive confirmation on TTY when --yes wasn't passed and there
    // is actually something destructive to do.
    if !yes && !actions_planned.is_empty() && std::io::stdin().is_terminal() {
        eprint!(
            "Apply {} fixer action(s)? [y/N] ",
            actions_planned.len()
        );
        let _ = std::io::stderr().flush();
        let mut input = String::new();
        let accepted = std::io::stdin().read_line(&mut input).is_ok()
            && input.trim().eq_ignore_ascii_case("y");
        if !accepted {
            emit_refusal(output, "user declined");
            return Ok(EXIT_REFUSED_UNSAFE);
        }
    }

    let mut run = DoctorRun::start(workspace.root()).map_err(|e| {
        emit_refusal(output, &format!("cannot create run dir: {}", e));
        WerkError::IoError(e.to_string())
    })?;

    let findings_json: Vec<serde_json::Value> = findings
        .iter()
        .map(|f| serde_json::to_value(f).unwrap_or(serde_json::Value::Null))
        .collect();

    // Run the three subsystem fixer passes in order. Each may early-return
    // with no actions when its findings group is empty.
    let store_fix = match run_store_fix(&store, &workspace, &mut run, &store_findings) {
        Ok(r) => r,
        Err(e) => {
            let _ = run.finalize(
                EXIT_FIX_FAILED,
                format!("store fix failed: {}", e),
                findings_json.clone(),
            );
            emit_refusal(output, &format!("store fixer error: {}", e));
            return Ok(EXIT_FIX_FAILED);
        }
    };
    let edges_fix = match run_edges_fix(&store, &workspace, &mut run, &edges_findings, options) {
        Ok(r) => r,
        Err(e) => {
            let _ = run.finalize(
                EXIT_FIX_FAILED,
                format!("edges fix failed: {}", e),
                findings_json.clone(),
            );
            emit_refusal(output, &format!("edges fixer error: {}", e));
            return Ok(EXIT_FIX_FAILED);
        }
    };
    let gestures_fix = match run_gestures_fix(&store, &workspace, &mut run, &gestures_findings) {
        Ok(r) => r,
        Err(e) => {
            let _ = run.finalize(
                EXIT_FIX_FAILED,
                format!("gestures fix failed: {}", e),
                findings_json.clone(),
            );
            emit_refusal(output, &format!("gestures fixer error: {}", e));
            return Ok(EXIT_FIX_FAILED);
        }
    };

    // W-1 safety harness: re-detect every Quint invariant. A residual
    // violation triggers an in-flight rollback to the run's backups.
    //
    // Soft-refuse handling: a pre-existing violator that the soft-refused
    // fixer chose not to touch is EXPECTED to persist post-fix. To
    // distinguish "expected residual" from "newly introduced by an
    // applied fixer", we compare per-tension-id sets pre vs post. Only
    // truly NEW violators trigger rollback. The coarse-by-invariant-name
    // filter (replaced by this code) could mask a fresh violation that
    // happened to share the same invariant name as a soft-refused one
    // (review H4).
    let post_edges = match run_edges_detect(&store) {
        Ok(v) => v,
        Err(e) => {
            let _ = run.finalize(
                EXIT_FIX_FAILED,
                format!("safety harness failed to re-detect: {}", e),
                findings_json.clone(),
            );
            emit_refusal(output, &format!("safety harness error: {}", e));
            return Ok(EXIT_FIX_FAILED);
        }
    };
    let post_gestures = match run_gestures_detect(&store) {
        Ok(v) => v,
        Err(e) => {
            let _ = run.finalize(
                EXIT_FIX_FAILED,
                format!("safety harness failed to re-detect: {}", e),
                findings_json.clone(),
            );
            emit_refusal(output, &format!("safety harness error: {}", e));
            return Ok(EXIT_FIX_FAILED);
        }
    };

    let pre_mp_ids: std::collections::HashSet<String> = edges_findings
        .multi_parent
        .iter()
        .map(|v| v.tension_id.clone())
        .collect();
    let pre_self_ids: std::collections::HashSet<String> =
        edges_findings.self_edges.iter().map(|e| e.id.clone()).collect();
    let pre_dangling_ids: std::collections::HashSet<String> = edges_findings
        .dangling_edges
        .iter()
        .map(|e| e.id.clone())
        .collect();
    let pre_sib_keys: std::collections::HashSet<(String, i64)> = edges_findings
        .sibling_collisions
        .iter()
        .map(|c| (c.parent_id.clone(), c.position))
        .collect();
    let pre_horizon_keys: std::collections::HashSet<(String, String)> = edges_findings
        .horizon_violations
        .iter()
        .map(|v| (v.parent_id.clone(), v.child_id.clone()))
        .collect();
    let pre_undo_ids: std::collections::HashSet<String> = gestures_findings
        .dangling_undo
        .iter()
        .map(|r| r.gesture_id.clone())
        .collect();

    let mut harness_violations: Vec<String> = Vec::new();
    if post_edges
        .multi_parent
        .iter()
        .any(|v| !pre_mp_ids.contains(&v.tension_id))
    {
        harness_violations.push("singleParent".to_string());
    }
    if post_edges.self_edges.iter().any(|e| !pre_self_ids.contains(&e.id)) {
        harness_violations.push("noSelfEdges".to_string());
    }
    if post_edges
        .dangling_edges
        .iter()
        .any(|e| !pre_dangling_ids.contains(&e.id))
    {
        harness_violations.push("edgesValid".to_string());
    }
    if post_edges
        .sibling_collisions
        .iter()
        .any(|c| !pre_sib_keys.contains(&(c.parent_id.clone(), c.position)))
    {
        harness_violations.push("siblingPositionsUnique".to_string());
    }
    if post_edges
        .horizon_violations
        .iter()
        .any(|v| !pre_horizon_keys.contains(&(v.parent_id.clone(), v.child_id.clone())))
    {
        harness_violations.push("noContainmentViolations".to_string());
    }
    if post_gestures
        .dangling_undo
        .iter()
        .any(|r| !pre_undo_ids.contains(&r.gesture_id))
    {
        harness_violations.push("undoneSubsetOfCompleted".to_string());
    }

    if !harness_violations.is_empty() {
        // W-1 rollback. Journal per-file intent/completion records.
        // Propagate errors (review H2): a partial rollback must not be
        // silently absorbed — the user needs to know manual recovery is
        // required.
        let rollback_result = safety_harness_rollback(&mut run, &workspace, &harness_violations);
        let summary = match &rollback_result {
            Ok(n) => format!(
                "W-1 violated post-fix: [{}]; rolled back {} file(s)",
                harness_violations.join(","),
                n
            ),
            Err(e) => format!(
                "W-1 violated post-fix: [{}]; ROLLBACK FAILED ({}); manual recovery required from .werk/.doctor/runs/{}/backups/",
                harness_violations.join(","),
                e,
                run.run_id()
            ),
        };
        let _ = run.finalize(EXIT_FIX_FAILED, &summary, findings_json);
        let envelope = serde_json::json!({
            "schema_version": DOCTOR_CONTRACT_VERSION,
            "verb": "doctor",
            "exit_code": EXIT_FIX_FAILED,
            "data": {
                "summary": summary,
                "rolled_back_invariants": harness_violations,
                "rollback_ok": rollback_result.is_ok(),
            }
        });
        print_envelope(output, robot, &envelope, |_| {
            eprintln!("doctor: {}", summary);
        });
        return Ok(EXIT_FIX_FAILED);
    }

    // Sum the actual mutation row counts so the summary reflects work
    // done. `purged` was previously collapsed to 1/0 (review H5).
    let total_actions = store_fix.purged
        + edges_fix.multi_parent_pruned
        + edges_fix.self_edges_deleted
        + edges_fix.dangling_edges_deleted
        + edges_fix.sibling_positions_nulled
        + edges_fix.horizon_violations_nulled
        + gestures_fix.dangling_undo_nulled;
    let exit_code = if !soft_refused.is_empty() && total_actions > 0 {
        EXIT_PARTIAL_FIX
    } else if !soft_refused.is_empty() {
        EXIT_FINDINGS_PRESENT
    } else {
        EXIT_HEALTHY
    };

    let summary = if total_actions == 0 && soft_refused.is_empty() {
        "no actions taken".to_string()
    } else if soft_refused.is_empty() {
        format!("applied {} fixer mutation(s)", total_actions)
    } else {
        format!(
            "applied {} fixer mutation(s); {} soft-refused finding group(s) remain",
            total_actions,
            soft_refused.len()
        )
    };
    let report = run
        .finalize(exit_code, &summary, findings_json)
        .map_err(WerkError::CoreError)?;

    let envelope = serde_json::json!({
        "schema_version": DOCTOR_CONTRACT_VERSION,
        "verb": "doctor",
        "run_id": report.run_id,
        "exit_code": exit_code,
        "data": {
            "summary": summary,
            "findings": findings,
            "actions_taken": report.action_count,
            "purged": store_fix.purged,
            "edges": {
                "multi_parent_pruned": edges_fix.multi_parent_pruned,
                "self_edges_deleted": edges_fix.self_edges_deleted,
                "dangling_edges_deleted": edges_fix.dangling_edges_deleted,
                "sibling_positions_nulled": edges_fix.sibling_positions_nulled,
                "horizon_violations_nulled": edges_fix.horizon_violations_nulled,
                "parent_ids_reconciled": edges_fix.parent_ids_reconciled,
            },
            "gestures": {
                "dangling_undo_nulled": gestures_fix.dangling_undo_nulled,
            },
            "soft_refused": soft_refused,
        }
    });
    print_envelope(output, robot, &envelope, |_| {
        println!("{}", summary);
        eprintln!("Run id: {}", report.run_id);
    });
    Ok(exit_code)
}

// ── Undo ─────────────────────────────────────────────────────────────

fn cmd_undo(output: &Output, target: String, dry_run: bool) -> Result<i32, WerkError> {
    let workspace = match Workspace::discover() {
        Ok(w) => w,
        Err(_) => {
            emit_refusal(output, "no workspace discovered");
            return Ok(EXIT_REFUSED_UNSAFE);
        }
    };
    let doctor_dir = workspace.root().join(".werk").join(".doctor");
    let runs_dir = doctor_dir.join("runs");

    let run_id = if target == "latest" {
        match std::fs::read_link(doctor_dir.join("latest")) {
            Ok(p) => p
                .file_name()
                .and_then(|s| s.to_str())
                .map(String::from)
                .ok_or_else(|| WerkError::InvalidInput("latest symlink unreadable".into()))?,
            Err(_) => {
                emit_refusal(output, "no prior run (latest symlink missing)");
                return Ok(EXIT_NO_INPUT);
            }
        }
    } else {
        target
    };

    let run_dir = runs_dir.join(&run_id);
    if !run_dir.exists() {
        emit_refusal(output, &format!("run {} not found", run_id));
        return Ok(EXIT_NO_INPUT);
    }
    let actions_path = run_dir.join("actions.jsonl");
    let backups_dir = run_dir.join("backups");
    if !actions_path.exists() {
        let envelope = serde_json::json!({
            "schema_version": DOCTOR_CONTRACT_VERSION,
            "verb": "undo",
            "run_id": run_id,
            "exit_code": EXIT_HEALTHY,
            "data": { "restored": [], "note": "no actions recorded" }
        });
        print_envelope(output, false, &envelope, |_| {
            println!("Nothing to undo (no actions recorded).");
        });
        return Ok(EXIT_HEALTHY);
    }

    let contents = std::fs::read_to_string(&actions_path).map_err(|e| {
        WerkError::IoError(format!("read actions.jsonl: {}", e))
    })?;
    let mut actions: Vec<ActionRecord> = Vec::new();
    for line in contents.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let rec: ActionRecord = serde_json::from_str(line).map_err(|e| {
            WerkError::IoError(format!("parse actions.jsonl: {}", e))
        })?;
        actions.push(rec);
    }
    actions.reverse();

    if dry_run {
        let plan: Vec<_> = actions
            .iter()
            .map(|a| {
                serde_json::json!({
                    "op": a.op,
                    "target": a.target,
                    "before_hash": a.before_hash,
                })
            })
            .collect();
        let envelope = serde_json::json!({
            "schema_version": DOCTOR_CONTRACT_VERSION,
            "verb": "undo",
            "run_id": run_id,
            "exit_code": EXIT_HEALTHY,
            "data": { "dry_run": true, "plan": plan }
        });
        print_envelope(output, false, &envelope, |_| {
            println!("Would restore:");
            for a in &actions {
                println!("  {} ({})", a.target, a.op);
            }
        });
        return Ok(EXIT_HEALTHY);
    }

    // Pass-5 undo journaling: write a `restore_in_progress` marker
    // before touching the workspace and remove it on successful
    // completion. If the marker is present at the start of a later undo,
    // the previous undo crashed midway — we surface that and replay the
    // same run-id (idempotent because every per-file step is restore-
    // from-backup, which is the same regardless of how many times it
    // runs). Per-file restores already use tempfile + rename so each
    // individual replacement is crash-safe; the marker closes the loop
    // across the whole sequence.
    let marker_path = run_dir.join("restore_in_progress");
    if marker_path.exists() {
        eprintln!(
            "[doctor] prior undo of run {} did not complete (marker present at {}); replaying.",
            run_id,
            marker_path.display()
        );
    }
    let marker_contents = format!(
        "{{\"run_id\":\"{}\",\"started_at\":\"{}\",\"pid\":{}}}\n",
        run_id,
        Utc::now().to_rfc3339(),
        std::process::id()
    );
    if let Err(e) = std::fs::write(&marker_path, marker_contents.as_bytes()) {
        emit_refusal(
            output,
            &format!("failed to write restore_in_progress marker: {}", e),
        );
        return Ok(EXIT_IO_ERROR);
    }

    let mut restored = Vec::new();
    for a in &actions {
        // Path traversal guard: refuse to touch any target that escapes
        // the workspace. The chokepoint `record_backup` already rejects
        // out-of-tree sources, but a hand-edited `actions.jsonl` could
        // still try to direct undo at `../etc/passwd`.
        if !is_safe_relative(&a.target) {
            emit_refusal(
                output,
                &format!("refusing to restore out-of-tree target: {}", a.target),
            );
            return Ok(EXIT_REFUSED_UNSAFE);
        }
        // Stale-sidecar branch of safety_harness_rollback (R-005): the
        // original action REMOVED a live sidecar that had no backup. On
        // undo there is nothing to restore — replaying as "remove the
        // live file if present" preserves the post-rollback shape.
        let backup_for_target = backups_dir.join(&a.target);
        if a.op == "safety_harness_rollback" && !backup_for_target.exists() {
            let live = workspace.root().join(&a.target);
            if live.exists() {
                if let Err(e) = std::fs::remove_file(&live) {
                    emit_refusal(
                        output,
                        &format!(
                            "failed to remove (undo of stale-sidecar removal) {}: {}",
                            a.target, e
                        ),
                    );
                    return Ok(EXIT_IO_ERROR);
                }
            }
            restored.push(a.target.clone());
            continue;
        }
        if let Err(code) = restore_target(output, workspace.root(), &backups_dir, &a.target, a.before_hash.as_deref()) {
            return Ok(code);
        }
        restored.push(a.target.clone());
        // DB-triplet handling: if this action's target ends in `werk.db`,
        // also restore `werk.db-wal` and `werk.db-shm` from the backup if
        // present, OR remove them from the live tree if not — leaving a
        // stale WAL beside a restored base would corrupt the next open.
        if a.target.ends_with("werk.db") {
            let dir_rel = std::path::Path::new(&a.target)
                .parent()
                .unwrap_or_else(|| std::path::Path::new(""));
            for sidecar in ["werk.db-wal", "werk.db-shm"] {
                let rel = dir_rel.join(sidecar);
                let rel_str = rel.to_string_lossy().into_owned();
                if !is_safe_relative(&rel_str) {
                    continue;
                }
                let backup_path = backups_dir.join(&rel);
                let live_path = workspace.root().join(&rel);
                if backup_path.exists() {
                    if let Err(code) = restore_target(output, workspace.root(), &backups_dir, &rel_str, None) {
                        return Ok(code);
                    }
                    restored.push(rel_str);
                } else if live_path.exists() {
                    // Stale sidecar: backup didn't include it, but a live
                    // file exists. Must remove it — leaving it would let
                    // fsqlite replay a WAL against the restored base.
                    if let Err(e) = std::fs::remove_file(&live_path) {
                        emit_refusal(
                            output,
                            &format!("failed to remove stale sidecar {}: {}", rel_str, e),
                        );
                        return Ok(EXIT_IO_ERROR);
                    }
                }
            }
        }
    }

    // Successful completion: clear the journal marker.
    let _ = std::fs::remove_file(&marker_path);

    let envelope = serde_json::json!({
        "schema_version": DOCTOR_CONTRACT_VERSION,
        "verb": "undo",
        "run_id": run_id,
        "exit_code": EXIT_HEALTHY,
        "data": { "restored": restored, "verified_against": run_id }
    });
    print_envelope(output, false, &envelope, |_| {
        println!("Restored {} target(s) from run {}.", restored.len(), run_id);
    });
    Ok(EXIT_HEALTHY)
}

// ── Capabilities ─────────────────────────────────────────────────────

fn cmd_capabilities(output: &Output) -> Result<i32, WerkError> {
    let caps = capabilities();
    if output.is_json() {
        output
            .print_structured(&caps)
            .map_err(WerkError::IoError)?;
    } else {
        let pretty = serde_json::to_string_pretty(&caps).map_err(|e| {
            WerkError::IoError(format!("serialize capabilities: {}", e))
        })?;
        println!("{}", pretty);
    }
    Ok(EXIT_HEALTHY)
}

// ── Health (one-line liveness) ───────────────────────────────────────

fn cmd_health(output: &Output) -> Result<i32, WerkError> {
    let workspace = match Workspace::discover() {
        Ok(w) => w,
        Err(_) => {
            emit_refusal(output, "no workspace discovered");
            return Ok(EXIT_REFUSED_UNSAFE);
        }
    };
    let store = workspace.open_store()?;
    let findings = run_store_detect(&store)?;
    let count = findings.into_findings().len();
    let status = if count == 0 {
        "healthy"
    } else {
        "findings_present"
    };
    let exit = if count == 0 {
        EXIT_HEALTHY
    } else {
        EXIT_FINDINGS_PRESENT
    };
    if output.is_json() {
        let envelope = serde_json::json!({
            "schema_version": DOCTOR_CONTRACT_VERSION,
            "verb": "health",
            "run_id": serde_json::Value::Null,
            "exit_code": exit,
            "data": { "status": status, "finding_count": count }
        });
        let s = serde_json::to_string(&envelope).map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("{}", s);
    } else if count == 0 {
        println!("healthy");
    } else {
        println!("{} finding(s)", count);
    }
    Ok(exit)
}

// ── Robot docs (in-tree paste-ready handbook) ────────────────────────

const ROBOT_DOCS: &str = include_str!("doctor_robot_docs.md");

fn cmd_robot_docs() -> Result<i32, WerkError> {
    print!("{}", ROBOT_DOCS);
    Ok(EXIT_HEALTHY)
}

// ── ls / diff / gc / explain / robot-triage ──────────────────────────

fn cmd_ls(output: &Output) -> Result<i32, WerkError> {
    let workspace = match Workspace::discover() {
        Ok(w) => w,
        Err(_) => {
            emit_refusal(output, "no workspace discovered");
            return Ok(EXIT_REFUSED_UNSAFE);
        }
    };
    let runs_dir = workspace.root().join(".werk").join(".doctor").join("runs");
    if !runs_dir.exists() {
        let envelope = serde_json::json!({
            "schema_version": DOCTOR_CONTRACT_VERSION,
            "verb": "ls",
            "run_id": serde_json::Value::Null,
            "exit_code": EXIT_NO_INPUT,
            "data": { "runs": Vec::<serde_json::Value>::new() }
        });
        print_envelope(output, false, &envelope, |_| {
            eprintln!("No runs yet.");
        });
        return Ok(EXIT_NO_INPUT);
    }
    let mut entries: Vec<_> = std::fs::read_dir(&runs_dir)
        .map_err(|e| WerkError::IoError(e.to_string()))?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    entries.reverse();

    let mut rows = Vec::new();
    for e in &entries {
        let report_path = e.path().join("report.json");
        let row = if let Ok(s) = std::fs::read_to_string(&report_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                serde_json::json!({
                    "run_id": e.file_name().to_string_lossy(),
                    "started_at": v.get("started_at").cloned().unwrap_or(serde_json::Value::Null),
                    "finished_at": v.get("finished_at").cloned().unwrap_or(serde_json::Value::Null),
                    "exit_code": v.get("exit_code").cloned().unwrap_or(serde_json::Value::Null),
                    "action_count": v.get("action_count").cloned().unwrap_or(serde_json::Value::Null),
                    "summary": v.get("summary").cloned().unwrap_or(serde_json::Value::Null),
                })
            } else {
                serde_json::json!({
                    "run_id": e.file_name().to_string_lossy(),
                    "report": "unreadable",
                })
            }
        } else {
            serde_json::json!({
                "run_id": e.file_name().to_string_lossy(),
                "report": "missing",
            })
        };
        rows.push(row);
    }
    let envelope = serde_json::json!({
        "schema_version": DOCTOR_CONTRACT_VERSION,
        "verb": "ls",
        "run_id": serde_json::Value::Null,
        "exit_code": EXIT_HEALTHY,
        "data": { "runs": rows }
    });
    print_envelope(output, false, &envelope, |_| {
        for r in &rows {
            println!(
                "{}  exit={}  actions={}  {}",
                r.get("run_id").and_then(|v| v.as_str()).unwrap_or("?"),
                r.get("exit_code").map(|v| v.to_string()).unwrap_or_default(),
                r.get("action_count").map(|v| v.to_string()).unwrap_or_default(),
                r.get("summary").and_then(|v| v.as_str()).unwrap_or(""),
            );
        }
    });
    Ok(EXIT_HEALTHY)
}

fn cmd_diff(output: &Output, reference: Option<String>) -> Result<i32, WerkError> {
    let workspace = match Workspace::discover() {
        Ok(w) => w,
        Err(_) => {
            emit_refusal(output, "no workspace discovered");
            return Ok(EXIT_REFUSED_UNSAFE);
        }
    };
    let runs_dir = workspace.root().join(".werk").join(".doctor").join("runs");
    if !runs_dir.exists() {
        emit_refusal(output, "no runs to diff");
        return Ok(EXIT_NO_INPUT);
    }
    let mut ids: Vec<String> = std::fs::read_dir(&runs_dir)
        .map_err(|e| WerkError::IoError(e.to_string()))?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    ids.sort();
    if ids.is_empty() {
        emit_refusal(output, "no runs to diff");
        return Ok(EXIT_NO_INPUT);
    }
    let latest = ids.last().cloned().unwrap();
    let ref_id = match reference {
        Some(r) => r,
        None => {
            if ids.len() < 2 {
                emit_refusal(output, "need at least 2 runs (or pass a ref)");
                return Ok(EXIT_NO_INPUT);
            }
            ids[ids.len() - 2].clone()
        }
    };
    let a = load_run_report(&workspace, &ref_id)?;
    let b = load_run_report(&workspace, &latest)?;
    let a_ids: std::collections::HashSet<String> = a
        .findings
        .iter()
        .filter_map(|v| v.get("id").and_then(|s| s.as_str()).map(String::from))
        .collect();
    let b_ids: std::collections::HashSet<String> = b
        .findings
        .iter()
        .filter_map(|v| v.get("id").and_then(|s| s.as_str()).map(String::from))
        .collect();
    let added: Vec<_> = b_ids.difference(&a_ids).cloned().collect();
    let removed: Vec<_> = a_ids.difference(&b_ids).cloned().collect();
    let envelope = serde_json::json!({
        "schema_version": DOCTOR_CONTRACT_VERSION,
        "verb": "diff",
        "run_id": serde_json::Value::Null,
        "exit_code": EXIT_HEALTHY,
        "data": { "from": ref_id, "to": latest, "added": added, "removed": removed }
    });
    print_envelope(output, false, &envelope, |_| {
        println!("From {} to {}:", ref_id, latest);
        for id in &added {
            println!("  + {}", id);
        }
        for id in &removed {
            println!("  - {}", id);
        }
    });
    Ok(EXIT_HEALTHY)
}

fn cmd_gc(output: &Output, before: Option<String>, yes: bool) -> Result<i32, WerkError> {
    let Some(before_str) = before else {
        emit_refusal(output, "--before <date> required");
        return Ok(EXIT_USAGE);
    };
    if !yes {
        emit_refusal(output, "--yes required");
        return Ok(EXIT_USAGE);
    }
    let cutoff = chrono::DateTime::parse_from_rfc3339(&before_str)
        .or_else(|_| {
            chrono::NaiveDate::parse_from_str(&before_str, "%Y-%m-%d").map(|d| {
                d.and_hms_opt(0, 0, 0)
                    .expect("midnight is always a valid time")
                    .and_utc()
                    .fixed_offset()
            })
        })
        .map_err(|e| WerkError::InvalidInput(format!("--before: {}", e)))?;
    // Reject pre-epoch cutoffs so a hostile / typo'd `--before 1969-01-01`
    // can't wrap negative ms into u64::MAX-ish and delete every run.
    let cutoff_ms_i64 = cutoff.timestamp_millis();
    if cutoff_ms_i64 < 0 {
        emit_refusal(output, "--before must be on or after the Unix epoch (1970-01-01)");
        return Ok(EXIT_USAGE);
    }
    let cutoff_ms = cutoff_ms_i64 as u64;

    let workspace = Workspace::discover()?;
    let doctor_dir = workspace.root().join(".werk").join(".doctor");
    let runs_dir = doctor_dir.join("runs");
    if !runs_dir.exists() {
        emit_refusal(output, "no runs to gc");
        return Ok(EXIT_NO_INPUT);
    }

    // Protect the run pointed to by `latest`.
    let latest_run: Option<String> = std::fs::read_link(doctor_dir.join("latest"))
        .ok()
        .and_then(|p| p.file_name().and_then(|s| s.to_str()).map(String::from));

    let mut removed = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    let mut skipped_in_flight: Vec<String> = Vec::new();
    // Defense in depth against racing a concurrent doctor: don't delete
    // any run modified within the last 60 seconds even if its ULID
    // timestamp is older than the cutoff. A doctor that started long ago
    // and is still appending to actions.jsonl would otherwise lose its
    // own backups underneath it.
    let recency_grace_secs: u64 = 60;
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let entries = std::fs::read_dir(&runs_dir).map_err(|e| WerkError::IoError(e.to_string()))?;
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if Some(&name) == latest_run.as_ref() {
            continue;
        }
        // ULID's first 48 bits are the timestamp in ms since epoch.
        let Ok(ulid) = ulid::Ulid::from_string(&name) else {
            continue;
        };
        if ulid.timestamp_ms() >= cutoff_ms {
            continue;
        }
        let path = e.path();
        // Only delete FINALIZED runs (report.json present). An un-finalized
        // run directory belongs to a still-running doctor and removing it
        // would yank that doctor's own backups out from under it.
        if !path.join("report.json").exists() {
            skipped_in_flight.push(name);
            continue;
        }
        // Defense in depth: also skip if the run was touched recently.
        let touched_recently = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| now_secs.saturating_sub(d.as_secs()) < recency_grace_secs)
            .unwrap_or(false);
        if touched_recently {
            skipped_in_flight.push(name);
            continue;
        }
        match std::fs::remove_dir_all(&path) {
            Ok(()) => removed.push(name),
            Err(err) => failures.push(format!("{}: {}", name, err)),
        }
    }

    // Append audit line to history.
    let history_path = doctor_dir.join("scorecard_history.jsonl");
    let line = serde_json::json!({
        "op": "gc",
        "before": before_str,
        "removed": removed,
        "timestamp": Utc::now().to_rfc3339(),
    });
    let mut s = line.to_string();
    s.push('\n');
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&history_path)
    {
        let _ = f.write_all(s.as_bytes());
    }

    let exit = if failures.is_empty() {
        EXIT_HEALTHY
    } else {
        EXIT_PARTIAL_FIX
    };
    let envelope = serde_json::json!({
        "schema_version": DOCTOR_CONTRACT_VERSION,
        "verb": "gc",
        "run_id": serde_json::Value::Null,
        "exit_code": exit,
        "data": {
            "removed": removed,
            "failures": failures,
            "skipped_in_flight": skipped_in_flight,
            "before": before_str,
        }
    });
    print_envelope(output, false, &envelope, |_| {
        println!("Removed {} run(s).", removed.len());
        if !failures.is_empty() {
            eprintln!("Failed to remove {} run(s):", failures.len());
            for f in &failures {
                eprintln!("  {}", f);
            }
        }
    });
    Ok(exit)
}

/// R-014: `werk doctor evacuate-backups`.
///
/// Per safety-envelope W-5: `.werk/backups/` lives inside `.werk/` and
/// is destroyed by `werk nuke`. This verb mirrors every regular file
/// under `.werk/backups/` to a stable external location keyed by a
/// per-workspace slug, so the user has somewhere to recover from after
/// nuke. The source tree is never modified — copy semantics, never
/// move. The destination is created if absent.
///
/// Slug derivation: BLAKE3-12-hex of the absolute workspace path +
/// `__` + basename. Collision-resistant across machines.
fn cmd_evacuate_backups(
    output: &Output,
    dest_override: Option<std::path::PathBuf>,
    dry_run: bool,
) -> Result<i32, WerkError> {
    let workspace = match Workspace::discover() {
        Ok(w) => w,
        Err(_) => {
            emit_refusal(output, "no workspace discovered");
            return Ok(EXIT_REFUSED_UNSAFE);
        }
    };
    let src = workspace.root().join(".werk").join("backups");
    if !src.exists() {
        let envelope = serde_json::json!({
            "schema_version": DOCTOR_CONTRACT_VERSION,
            "verb": "evacuate-backups",
            "run_id": serde_json::Value::Null,
            "exit_code": EXIT_NO_INPUT,
            "data": {
                "source": src.display().to_string(),
                "files_copied": 0,
                "destination": serde_json::Value::Null,
                "note": "no .werk/backups/ directory present",
            }
        });
        print_envelope(output, false, &envelope, |_| {
            eprintln!("Nothing to evacuate: {} does not exist.", src.display());
        });
        return Ok(EXIT_NO_INPUT);
    }

    let abs_root = workspace
        .root()
        .canonicalize()
        .unwrap_or_else(|_| workspace.root().to_path_buf());
    let abs_root_str = abs_root.to_string_lossy().into_owned();
    let hash = blake3::hash(abs_root_str.as_bytes()).to_hex().to_string();
    let prefix = &hash[..12];
    let basename = abs_root
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "werk".to_string());
    let slug = format!("{}__{}", prefix, basename);

    let dest_root = match dest_override {
        Some(p) => p,
        None => {
            let home = dirs::home_dir().ok_or_else(|| {
                WerkError::IoError("no home directory; pass --dest <PATH>".to_string())
            })?;
            home.join(".werk").join("backups").join(&slug)
        }
    };

    // Enumerate files (recursive). Symlinks NOT followed: copy-as-file
    // is safer here — we never want to write outside dest_root by
    // chasing a malicious link in `.werk/backups/`.
    fn walk(root: &Path, out: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
        for e in std::fs::read_dir(root)? {
            let e = e?;
            let p = e.path();
            let meta = std::fs::symlink_metadata(&p)?;
            if meta.file_type().is_dir() {
                walk(&p, out)?;
            } else if meta.file_type().is_file() {
                out.push(p);
            }
            // Symlinks and other types are deliberately skipped.
        }
        Ok(())
    }
    let mut files = Vec::new();
    if let Err(e) = walk(&src, &mut files) {
        return Err(WerkError::IoError(format!(
            "failed to enumerate {}: {}",
            src.display(),
            e
        )));
    }
    files.sort();

    let plan: Vec<serde_json::Value> = files
        .iter()
        .map(|p| {
            let rel = p.strip_prefix(&src).unwrap_or(p);
            serde_json::json!({
                "source": p.display().to_string(),
                "dest": dest_root.join(rel).display().to_string(),
            })
        })
        .collect();

    if dry_run {
        let envelope = serde_json::json!({
            "schema_version": DOCTOR_CONTRACT_VERSION,
            "verb": "evacuate-backups",
            "run_id": serde_json::Value::Null,
            "exit_code": EXIT_HEALTHY,
            "data": {
                "source": src.display().to_string(),
                "destination": dest_root.display().to_string(),
                "slug": slug,
                "dry_run": true,
                "files_planned": plan.len(),
                "plan": plan,
            }
        });
        print_envelope(output, false, &envelope, |_| {
            eprintln!(
                "Plan: copy {} file(s) from {} to {} (dry-run, no writes).",
                plan.len(),
                src.display(),
                dest_root.display()
            );
        });
        return Ok(EXIT_HEALTHY);
    }

    // Idempotent destination create.
    std::fs::create_dir_all(&dest_root).map_err(|e| {
        WerkError::IoError(format!(
            "cannot create {}: {}",
            dest_root.display(),
            e
        ))
    })?;
    let mut copied = 0usize;
    let mut skipped = 0usize;
    let mut failures: Vec<serde_json::Value> = Vec::new();
    for p in &files {
        let rel = p.strip_prefix(&src).unwrap_or(p);
        let dest_path = dest_root.join(rel);
        if let Some(parent) = dest_path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            failures.push(serde_json::json!({
                "source": p.display().to_string(),
                "error": format!("mkdir {}: {}", parent.display(), e),
            }));
            continue;
        }
        // If a file with identical bytes already exists, skip — keeps
        // evacuate idempotent across repeated runs.
        if dest_path.exists()
            && let (Ok(src_bytes), Ok(dst_bytes)) =
                (std::fs::read(p), std::fs::read(&dest_path))
            && src_bytes == dst_bytes
        {
            skipped += 1;
            continue;
        }
        match std::fs::copy(p, &dest_path) {
            Ok(_) => copied += 1,
            Err(e) => failures.push(serde_json::json!({
                "source": p.display().to_string(),
                "dest": dest_path.display().to_string(),
                "error": e.to_string(),
            })),
        }
    }
    let exit = if failures.is_empty() {
        EXIT_HEALTHY
    } else {
        EXIT_PARTIAL_FIX
    };
    let envelope = serde_json::json!({
        "schema_version": DOCTOR_CONTRACT_VERSION,
        "verb": "evacuate-backups",
        "run_id": serde_json::Value::Null,
        "exit_code": exit,
        "data": {
            "source": src.display().to_string(),
            "destination": dest_root.display().to_string(),
            "slug": slug,
            "files_seen": files.len(),
            "files_copied": copied,
            "files_skipped_identical": skipped,
            "failures": failures,
        }
    });
    print_envelope(output, false, &envelope, |_| {
        eprintln!(
            "Evacuated {} file(s) ({} unchanged) to {}.",
            copied,
            skipped,
            dest_root.display()
        );
        if !failures.is_empty() {
            eprintln!("Failures:");
            for f in &failures {
                eprintln!("  {}", f);
            }
        }
    });
    Ok(exit)
}

fn cmd_explain(output: &Output, finding_id: &str) -> Result<i32, WerkError> {
    let spec = match finding_id {
        // Accept both the detector id (from capabilities.detectors[].id)
        // and the finding id (from Finding::id) so an agent calling
        // `--explain` with either string lands at the same explanation.
        "noop_mutations" | "fm-store-noop-mutations-non-position-fields" => Some(serde_json::json!({
            "id": finding_id,
            "subsystem": "store",
            "severity": "low",
            "summary": "Rows in store mutations where old_value == new_value indicate redundant writes that bloat the mutations log without changing state.",
            "evidence_source": "werk-core/src/store.rs:1191",
            "fixer": "purge_noop_mutations",
            "command": "werk doctor --fix --yes",
            "inverse": "werk doctor undo <run-id>",
        })),
        "singleParent" | "fm-edges-multi-parent" => Some(serde_json::json!({
            "id": finding_id,
            "subsystem": "edges",
            "severity": "high",
            "summary": "A tension has more than one `contains` edge pointing at it; the forest invariant `singleParent` is violated.",
            "quint_source": "specs/werk.qnt:393",
            "fixer": "prune_duplicate_parent_edges",
            "command": "werk doctor --fix --only=edges --prefer=oldest|newest --yes",
            "inverse": "werk doctor undo <run-id>",
            "policy_note": "Soft-refused by default. Pass --prefer=oldest (smallest ULID) or --prefer=newest (last-write-wins) to choose which contains-edge survives. The fixer reconciles tensions.parent_id in the same transaction.",
        })),
        "noSelfEdges" | "fm-edges-self-loop" => Some(serde_json::json!({
            "id": finding_id,
            "subsystem": "edges",
            "severity": "high",
            "summary": "An edge has from_id == to_id. Self-edges have no semantic meaning and are safe to delete.",
            "quint_source": "specs/werk.qnt:399",
            "fixer": "delete_self_edges",
            "command": "werk doctor --fix --only=edges --yes",
            "inverse": "werk doctor undo <run-id>",
            "policy_note": "Always auto-applied. If the deleted self-edge is a contains-edge, tensions.parent_id is reconciled to NULL.",
        })),
        "edgesValid" | "fm-edges-dangling" => Some(serde_json::json!({
            "id": finding_id,
            "subsystem": "edges",
            "severity": "high",
            "summary": "An edge references a tension that no longer exists. The orphan edge is safe to delete; the surviving endpoint is unaffected.",
            "quint_source": "specs/werk.qnt:403",
            "fixer": "delete_dangling_edges",
            "command": "werk doctor --fix --only=edges --yes",
            "inverse": "werk doctor undo <run-id>",
            "policy_note": "Always auto-applied. tensions.parent_id is reconciled for surviving children whose parent was nuked.",
        })),
        "siblingPositionsUnique" | "fm-edges-sibling-position-collision" => Some(serde_json::json!({
            "id": finding_id,
            "subsystem": "edges",
            "severity": "medium",
            "summary": "Two or more siblings share the same non-NULL position. The Quint invariant requires unique positions among contains-children of the same parent.",
            "quint_source": "specs/werk.qnt:418",
            "fixer": "null_colliding_sibling_positions",
            "command": "werk doctor --fix --only=edges --yes",
            "inverse": "werk doctor undo <run-id>",
            "policy_note": "Always auto-applied. Keeps the child whose contains-edge ULID is smallest; nulls position on the others. Unpositioned children rejoin the unordered tail.",
        })),
        "noContainmentViolations" | "fm-edges-horizon-containment" => Some(serde_json::json!({
            "id": finding_id,
            "subsystem": "edges",
            "severity": "medium",
            "summary": "A child tension's horizon (deadline) exceeds its parent's. Quint requires child.horizon <= parent.horizon.",
            "quint_source": "specs/werk.qnt:436",
            "fixer": "null_violating_child_horizon",
            "command": "werk doctor --fix --only=edges --apply-horizon-fix --yes",
            "inverse": "werk doctor undo <run-id>",
            "policy_note": "Soft-refused by default. Nulling the child's horizon discards user temporal commitment, so opt in with --apply-horizon-fix. Transitive violations are NOT auto-walked; re-run after the first fix to surface any cascaded findings.",
        })),
        "fm-edges-horizon-unparseable" => Some(serde_json::json!({
            "id": finding_id,
            "subsystem": "edges",
            "severity": "low",
            "summary": "A tension's horizon string could not be parsed; the horizon containment detector could not evaluate this edge.",
            "quint_source": "werk-core/src/horizon.rs",
            "fixer": "(none)",
            "command": "manual fix: correct the horizon string via `werk horizon <id> <iso-8601>`",
            "inverse": "(n/a)",
            "policy_note": "No auto-fix available — fix the underlying upstream invariant by hand.",
        })),
        "undoneSubsetOfCompleted" | "fm-gestures-undone-dangling" => Some(serde_json::json!({
            "id": finding_id,
            "subsystem": "gestures",
            "severity": "medium",
            "summary": "A gesture row's undone_gesture_id references a row that doesn't exist. The phantom undo claim is invalid.",
            "quint_source": "specs/werk.qnt:450",
            "fixer": "null_dangling_undo_gestures",
            "command": "werk doctor --fix --only=gestures --yes",
            "inverse": "werk doctor undo <run-id>",
            "policy_note": "Always auto-applied. Nulls undone_gesture_id rather than deleting the row, so inbound FK references from mutations/edges/epochs are preserved.",
        })),
        "safety_harness_rollback" => Some(serde_json::json!({
            "id": finding_id,
            "subsystem": "(meta)",
            "severity": "info",
            "summary": "Internal op emitted only when the post-fix safety harness sees a residual Quint violation. The run self-rolls-back to its first backup; the journal records each per-file restore.",
            "quint_source": "analysis/safety_envelope.md (W-1)",
            "fixer": "(meta)",
            "command": "(internal — never invoked directly by a user)",
            "inverse": "(self-inverse: restoration is idempotent)",
            "policy_note": "If the harness fires, the run finalizes with EXIT_FIX_FAILED. A subsequent `werk doctor undo <run-id>` is safe (no-op) and replays the rollback's own journaled entries.",
        })),
        _ => None,
    };
    if let Some(s) = spec {
        let envelope = serde_json::json!({
            "schema_version": DOCTOR_CONTRACT_VERSION,
            "verb": "explain",
            "run_id": serde_json::Value::Null,
            "exit_code": EXIT_HEALTHY,
            "data": s,
        });
        print_envelope(output, false, &envelope, |_| {
            println!("{}", serde_json::to_string_pretty(&envelope).unwrap_or_default());
        });
        Ok(EXIT_HEALTHY)
    } else {
        emit_refusal(output, &format!("unknown finding id: {}", finding_id));
        Ok(EXIT_NO_INPUT)
    }
}

fn cmd_robot_triage(output: &Output, only: Option<&str>) -> Result<i32, WerkError> {
    let workspace = match Workspace::discover() {
        Ok(w) => w,
        Err(_) => {
            emit_refusal(output, "no workspace discovered");
            return Ok(EXIT_REFUSED_UNSAFE);
        }
    };
    let subsystems = parse_only(only)?;
    let store = workspace.open_store()?;
    let run = DoctorRun::start(workspace.root()).map_err(|e| {
        emit_refusal(output, &format!("cannot create run dir: {}", e));
        WerkError::IoError(e.to_string())
    })?;
    let store_findings = if subsystems.includes_store() {
        run_store_detect(&store)?
    } else {
        StoreFindings::default()
    };
    let edges_findings = if subsystems.includes_edges() {
        run_edges_detect(&store)?
    } else {
        EdgesFindings::default()
    };
    let gestures_findings = if subsystems.includes_gestures() {
        run_gestures_detect(&store)?
    } else {
        GesturesFindings::default()
    };
    let mut findings: Vec<Finding> = Vec::new();
    findings.extend(store_findings.into_findings());
    findings.extend(edges_findings.into_findings());
    findings.extend(gestures_findings.into_findings());
    let mut actions_planned = Vec::new();
    if store_findings.noop_mutations > 0 {
        actions_planned.push(serde_json::json!({
            "op": "purge_noop_mutations",
            "target": ".werk/werk.db",
            "fixer": "purge_noop_mutations",
        }));
    }
    if !edges_findings.multi_parent.is_empty() {
        actions_planned.push(serde_json::json!({
            "op": "prune_duplicate_parent_edges",
            "target": ".werk/werk.db",
            "fixer": "prune_duplicate_parent_edges",
            "requires_flag": "--prefer=oldest|newest",
        }));
    }
    if !edges_findings.self_edges.is_empty() {
        actions_planned.push(serde_json::json!({
            "op": "delete_self_edges",
            "target": ".werk/werk.db",
            "fixer": "delete_self_edges",
        }));
    }
    if !edges_findings.dangling_edges.is_empty() {
        actions_planned.push(serde_json::json!({
            "op": "delete_dangling_edges",
            "target": ".werk/werk.db",
            "fixer": "delete_dangling_edges",
        }));
    }
    if !edges_findings.sibling_collisions.is_empty() {
        actions_planned.push(serde_json::json!({
            "op": "null_colliding_sibling_positions",
            "target": ".werk/werk.db",
            "fixer": "null_colliding_sibling_positions",
        }));
    }
    if !edges_findings.horizon_violations.is_empty() {
        actions_planned.push(serde_json::json!({
            "op": "null_violating_child_horizon",
            "target": ".werk/werk.db",
            "fixer": "null_violating_child_horizon",
            "requires_flag": "--apply-horizon-fix",
        }));
    }
    if !gestures_findings.dangling_undo.is_empty() {
        actions_planned.push(serde_json::json!({
            "op": "null_dangling_undo_gestures",
            "target": ".werk/werk.db",
            "fixer": "null_dangling_undo_gestures",
        }));
    }
    let exit_code = if findings.is_empty() {
        EXIT_HEALTHY
    } else {
        EXIT_FINDINGS_PRESENT
    };
    let summary = if findings.is_empty() {
        "no findings".to_string()
    } else {
        format!(
            "{} finding(s); {} fixable; recommended: werk doctor --fix --yes",
            findings.len(),
            actions_planned.len()
        )
    };
    let report = run
        .finalize(
            exit_code,
            &summary,
            findings.iter().map(|f| serde_json::to_value(f).unwrap_or(serde_json::Value::Null)).collect(),
        )
        .map_err(WerkError::CoreError)?;

    // robot-triage envelope is FLAT (top-level keys are the data); see spec §3.11.
    let envelope = serde_json::json!({
        "schema_version": DOCTOR_CONTRACT_VERSION,
        "verb": "robot-triage",
        "run_id": report.run_id,
        "exit_code": exit_code,
        "summary": summary,
        "findings": findings,
        "actions_planned": actions_planned,
        "recommended_command": if findings.is_empty() {
            "(nothing to do)".to_string()
        } else {
            "werk doctor --fix --yes".to_string()
        },
        "capabilities_command": "werk doctor capabilities --json",
    });
    // robot-triage is always JSON.
    let s = serde_json::to_string(&envelope).map_err(|e| WerkError::IoError(e.to_string()))?;
    println!("{}", s);
    let _ = output;
    Ok(exit_code)
}

// ── Helpers ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct OnlySet {
    all: bool,
    items: Vec<String>,
}

impl OnlySet {
    fn includes_store(&self) -> bool {
        self.all || self.items.iter().any(|s| s == "store")
    }
    fn includes_edges(&self) -> bool {
        self.all || self.items.iter().any(|s| s == "edges")
    }
    fn includes_gestures(&self) -> bool {
        self.all || self.items.iter().any(|s| s == "gestures")
    }
}

fn parse_only(s: Option<&str>) -> Result<OnlySet, WerkError> {
    match s {
        None => Ok(OnlySet {
            all: true,
            items: Vec::new(),
        }),
        Some(list) => {
            let items: Vec<String> = list
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let known = ["store", "edges", "gestures"];
            for it in &items {
                if !known.contains(&it.as_str()) {
                    return Err(WerkError::InvalidInput(format!(
                        "--only: unknown subsystem `{}` (known: store, edges, gestures)",
                        it
                    )));
                }
            }
            Ok(OnlySet { all: false, items })
        }
    }
}

fn load_run_report(
    workspace: &Workspace,
    run_id: &str,
) -> Result<werk_core::doctor_run::RunReport, WerkError> {
    let path = workspace
        .root()
        .join(".werk")
        .join(".doctor")
        .join("runs")
        .join(run_id)
        .join("report.json");
    let s = std::fs::read_to_string(&path)
        .map_err(|e| WerkError::IoError(format!("read {}: {}", path.display(), e)))?;
    serde_json::from_str(&s)
        .map_err(|e| WerkError::IoError(format!("parse report.json: {}", e)))
}

fn print_envelope<F: Fn(&Output)>(
    output: &Output,
    robot: bool,
    envelope: &serde_json::Value,
    human: F,
) {
    if output.is_json() || robot {
        match serde_json::to_string(envelope) {
            Ok(s) => println!("{}", s),
            Err(e) => {
                // Don't silently drop JSON output — a consumer parsing
                // stdout would see nothing and have no way to recover.
                // The serialization failure is itself surfaced as a
                // minimal JSON object so the contract holds.
                eprintln!("doctor: failed to serialize envelope: {}", e);
                println!(
                    "{{\"schema_version\":{},\"verb\":\"doctor\",\"exit_code\":{},\"error\":\"envelope_serialization_failed\"}}",
                    DOCTOR_CONTRACT_VERSION, EXIT_IO_ERROR
                );
            }
        }
    } else {
        human(output);
    }
}

/// Refuse paths that contain `..` components, absolute prefixes, or
/// platform-specific escape sequences. The doctor's blast radius is the
/// workspace; an attacker-controlled `actions.jsonl` line should not be
/// able to direct restoration outside it.
fn is_safe_relative(rel: &str) -> bool {
    let p = std::path::Path::new(rel);
    if p.is_absolute() {
        return false;
    }
    for c in p.components() {
        match c {
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => return false,
        }
    }
    true
}

/// Atomic restoration of a single file from `backups_dir/rel` to
/// `workspace_root/rel`. If `expected_hash` is Some, verifies the post-restore
/// BLAKE3 matches. Returns Err(<doctor-exit-code>) on failure so the caller
/// can propagate the right code.
fn restore_target(
    output: &Output,
    workspace_root: &Path,
    backups_dir: &Path,
    rel: &str,
    expected_hash: Option<&str>,
) -> Result<(), i32> {
    let backup_src = backups_dir.join(rel);
    let dest = workspace_root.join(rel);
    if !backup_src.exists() {
        emit_refusal(
            output,
            &format!("backup missing for {} (expected {})", rel, backup_src.display()),
        );
        return Err(EXIT_FIX_FAILED);
    }
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let tmp = dest.with_extension(format!(
        "{}.undo.tmp.{}",
        dest.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("partial"),
        std::process::id()
    ));
    if let Err(e) = std::fs::copy(&backup_src, &tmp) {
        emit_refusal(output, &format!("copy backup: {}", e));
        return Err(EXIT_IO_ERROR);
    }
    if let Err(e) = std::fs::rename(&tmp, &dest) {
        let _ = std::fs::remove_file(&tmp);
        emit_refusal(output, &format!("rename into place: {}", e));
        return Err(EXIT_IO_ERROR);
    }
    if let Some(expected) = expected_hash {
        match std::fs::read(&dest) {
            Ok(bytes) => {
                let actual = blake3::hash(&bytes).to_hex().to_string();
                if actual != expected {
                    emit_refusal(
                        output,
                        &format!(
                            "hash mismatch for {}: expected {}, got {}",
                            rel, expected, actual
                        ),
                    );
                    return Err(EXIT_FIX_FAILED);
                }
            }
            Err(e) => {
                emit_refusal(output, &format!("post-restore read failed: {}", e));
                return Err(EXIT_IO_ERROR);
            }
        }
    }
    Ok(())
}

fn emit_refusal(output: &Output, reason: &str) {
    if output.is_json() {
        let envelope = serde_json::json!({
            "schema_version": DOCTOR_CONTRACT_VERSION,
            "verb": "doctor",
            "exit_code": EXIT_REFUSED_UNSAFE,
            "data": { "error": reason }
        });
        if let Ok(s) = serde_json::to_string(&envelope) {
            println!("{}", s);
        }
    } else {
        eprintln!("doctor: {}", reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_action_ops_match_emitter() {
        let caps = capabilities();
        for fixer in &caps.fixers {
            if !fixer.available {
                continue;
            }
            assert!(
                ACTION_OPS.contains(&fixer.op),
                "fixer `{}` declares op `{}` not in werk_core::doctor_run::ACTION_OPS",
                fixer.id,
                fixer.op
            );
        }
        // And every entry in ACTION_OPS is referenced by at least one available fixer.
        for op in ACTION_OPS {
            assert!(
                caps.fixers.iter().any(|f| f.available && f.op == *op),
                "op `{}` in ACTION_OPS has no matching available fixer in capabilities",
                op
            );
        }
    }

    #[test]
    fn capabilities_werk_version_is_pinned_to_cargo_pkg_version() {
        let caps = capabilities();
        assert_eq!(caps.werk_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn parse_only_known_subsystems() {
        let set = parse_only(Some("store")).unwrap();
        assert!(!set.all);
        assert!(set.includes_store());
        let set = parse_only(Some("store,edges")).unwrap();
        assert!(set.includes_store());
        let set = parse_only(None).unwrap();
        assert!(set.all);
        assert!(set.includes_store());
    }

    #[test]
    fn explain_covers_every_reserved_detector() {
        // Cross-check: every detector declared in `capabilities()` must be
        // recognized by `cmd_explain`. Drift would mean an agent calling
        // `werk doctor --explain <id>` for a slot listed in capabilities
        // gets EXIT_NO_INPUT. The cheapest assertion is to walk the static
        // match arm key set; we encode it as a constant list and assert
        // the union covers every detector id.
        const KNOWN_TO_EXPLAIN: &[&str] = &[
            "noop_mutations",
            "fm-store-noop-mutations-non-position-fields",
            "singleParent",
            "fm-edges-multi-parent",
            "noSelfEdges",
            "fm-edges-self-loop",
            "edgesValid",
            "fm-edges-dangling",
            "siblingPositionsUnique",
            "fm-edges-sibling-position-collision",
            "noContainmentViolations",
            "fm-edges-horizon-containment",
            "fm-edges-horizon-unparseable",
            "undoneSubsetOfCompleted",
            "fm-gestures-undone-dangling",
            "safety_harness_rollback",
        ];
        let caps = capabilities();
        for d in &caps.detectors {
            if !KNOWN_TO_EXPLAIN.contains(&d.id) {
                panic!(
                    "capabilities declares detector `{}` but cmd_explain has no spec for it",
                    d.id
                );
            }
        }
    }

    #[test]
    fn safety_harness_rollback_restores_db_and_journals_per_file() {
        // W-1 contract test: synthesize a fix-then-corrupt situation
        // and assert safety_harness_rollback restores the live triplet
        // from backups and journals per-file intent/completion records.
        // Uses werk-core directly to avoid relying on a deliberately
        // buggy fixer.
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        std::fs::create_dir_all(root.join(".werk")).unwrap();
        // Open a Store, create a tension (so the DB has content), then
        // close. The DB is now in a known state — call this the snapshot.
        {
            let store = werk_core::Store::init_unlocked(&root).unwrap();
            store.create_tension("d", "r").unwrap();
        }
        let workspace = Workspace::discover_from(&root).unwrap();
        let mut run = DoctorRun::start(&root).unwrap();
        // Back up the snapshot.
        run.record_backup_once(&workspace.db_path()).unwrap();
        let wal = workspace.db_path().with_file_name("werk.db-wal");
        if wal.exists() {
            run.record_backup_once(&wal).unwrap();
        }
        // Now simulate a fixer mutation: append junk bytes to werk.db
        // (corrupt it so the post-rollback state is verifiable as
        // "different from corruption, equal to backup").
        let corrupt_bytes = b"\x00CORRUPTION\x00".to_vec();
        let pre_corruption_size = std::fs::metadata(workspace.db_path()).unwrap().len();
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(workspace.db_path())
                .unwrap();
            f.write_all(&corrupt_bytes).unwrap();
        }
        let post_corruption_size = std::fs::metadata(workspace.db_path()).unwrap().len();
        assert!(
            post_corruption_size > pre_corruption_size,
            "corruption should grow the file"
        );

        // Fire the rollback.
        let restored = safety_harness_rollback(&mut run, &workspace, &["singleParent".to_string()])
            .expect("rollback should succeed");
        assert!(restored >= 1, "should restore at least werk.db");

        // Live DB should match the snapshot size (corruption gone).
        let post_rollback_size = std::fs::metadata(workspace.db_path()).unwrap().len();
        assert_eq!(
            post_rollback_size, pre_corruption_size,
            "rollback should restore werk.db to snapshot bytes"
        );

        // actions.jsonl should contain per-file intent + completion
        // records mentioning safety_harness_rollback.
        let actions_path = run.run_dir().join("actions.jsonl");
        let contents = std::fs::read_to_string(&actions_path).unwrap();
        let lines: Vec<&str> = contents.lines().filter(|l| !l.is_empty()).collect();
        assert!(
            lines.iter().any(|l| l.contains("safety_harness_rollback")),
            "actions.jsonl should journal the rollback"
        );
        // At least one intent (before_hash:Some) and one completion (before_hash:null) per file.
        let intent_lines: Vec<_> = lines
            .iter()
            .filter(|l| l.contains("safety_harness_rollback") && l.contains("\"before_hash\":\""))
            .collect();
        let completion_lines: Vec<_> = lines
            .iter()
            .filter(|l| l.contains("safety_harness_rollback") && l.contains("\"before_hash\":null"))
            .collect();
        assert!(
            !intent_lines.is_empty(),
            "should journal at least one rollback intent record"
        );
        assert!(
            !completion_lines.is_empty(),
            "should journal at least one rollback completion record"
        );
    }

    #[test]
    fn parse_only_includes_edges_and_gestures() {
        let set = parse_only(Some("edges")).unwrap();
        assert!(!set.all);
        assert!(set.includes_edges());
        assert!(!set.includes_store());
        let set = parse_only(Some("gestures")).unwrap();
        assert!(set.includes_gestures());
        let set = parse_only(Some("edges,gestures")).unwrap();
        assert!(set.includes_edges());
        assert!(set.includes_gestures());
        assert!(!set.includes_store());
    }

    #[test]
    fn quint_detectors_all_available() {
        let caps = capabilities();
        for id in [
            "singleParent",
            "noSelfEdges",
            "edgesValid",
            "siblingPositionsUnique",
            "noContainmentViolations",
            "undoneSubsetOfCompleted",
        ] {
            let d = caps
                .detectors
                .iter()
                .find(|d| d.id == id)
                .unwrap_or_else(|| panic!("missing detector `{}`", id));
            assert!(d.available, "detector `{}` not available", id);
            assert!(d.reserved_for.is_none(), "detector `{}` still reserved", id);
        }
    }

    #[test]
    fn is_safe_relative_rejects_traversal() {
        assert!(is_safe_relative(".werk/werk.db"));
        assert!(is_safe_relative("nested/path/file"));
        assert!(!is_safe_relative("../etc/passwd"));
        assert!(!is_safe_relative("/etc/passwd"));
        assert!(!is_safe_relative(".werk/../../oops"));
        assert!(!is_safe_relative("a/b/../../../c"));
    }

    #[test]
    fn parse_only_rejects_unknown_subsystem() {
        let err = parse_only(Some("nope")).unwrap_err();
        match err {
            WerkError::InvalidInput(s) => assert!(s.contains("nope")),
            _ => panic!("expected InvalidInput"),
        }
    }
}
