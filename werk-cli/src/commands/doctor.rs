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
//! - **WAL checkpoint not forced before backup.** The fix path backs up
//!   `werk.db` + `werk.db-wal` + `werk.db-shm` as independent file
//!   copies; bringing the triplet under a single `BEGIN EXCLUSIVE` is a
//!   follow-up.
//! - **`undo` is not transactional.** A failure midway through restoring
//!   leaves the workspace partially restored; the run directory's
//!   backups remain available for manual recovery.

use crate::error::WerkError;
use crate::output::Output;
use chrono::Utc;
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use std::io::{IsTerminal, Write};
use std::path::Path;
use werk_core::doctor_run::{ACTION_OPS, ActionRecord, DOCTOR_CONTRACT_VERSION, DoctorRun};
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
                available: false,
                description: None,
                evidence_source: None,
                reserved_for: Some("R-005"),
            },
            DetectorSpec {
                id: "noSelfEdges",
                subsystem: "edges",
                available: false,
                description: None,
                evidence_source: None,
                reserved_for: Some("R-005"),
            },
            DetectorSpec {
                id: "edgesValid",
                subsystem: "edges",
                available: false,
                description: None,
                evidence_source: None,
                reserved_for: Some("R-005"),
            },
            DetectorSpec {
                id: "siblingPositionsUnique",
                subsystem: "edges",
                available: false,
                description: None,
                evidence_source: None,
                reserved_for: Some("R-005"),
            },
            DetectorSpec {
                id: "noContainmentViolations",
                subsystem: "edges",
                available: false,
                description: None,
                evidence_source: None,
                reserved_for: Some("R-005"),
            },
            DetectorSpec {
                id: "undoneSubsetOfCompleted",
                subsystem: "gestures",
                available: false,
                description: None,
                evidence_source: None,
                reserved_for: Some("R-005"),
            },
        ],
        fixers: vec![FixerSpec {
            id: "purge_noop_mutations",
            detector: "noop_mutations",
            available: true,
            op: "purge_noop_mutations",
            inverse: "restore_db_from_backup",
            backs_up: vec![".werk/werk.db", ".werk/werk.db-wal", ".werk/werk.db-shm"],
        }],
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
        return cmd_fix(
            output,
            args.dry_run,
            args.yes,
            args.only.as_deref(),
            args.robot,
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
    // Other subsystems (edges, gestures): reserved for R-005. No-op for now.

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

    // Detect first (the chokepoint contract: detect-then-fix).
    // For dry-run we do this WITHOUT starting a DoctorRun, so a dry-run
    // never promotes `.werk/.doctor/latest` (which would corrupt a
    // subsequent `werk doctor undo latest`).
    let store_findings = if subsystems.includes_store() {
        run_store_detect(&store)?
    } else {
        StoreFindings::default()
    };

    let findings = store_findings.into_findings();
    let mut actions_planned = Vec::new();
    if store_findings.noop_mutations > 0 {
        actions_planned.push(serde_json::json!({
            "op": "purge_noop_mutations",
            "target": ".werk/werk.db",
            "fixer": "purge_noop_mutations",
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
        });
        return Ok(EXIT_HEALTHY);
    }

    // Interactive confirmation on TTY when --yes wasn't passed and there
    // is actually something destructive to do. Skipped silently when
    // there's nothing to fix (the call would still produce action_count=0
    // and no mutation, but the prompt would be noise).
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

    let fix_result = match run_store_fix(&store, &workspace, &mut run, &store_findings) {
        Ok(r) => r,
        Err(e) => {
            // Best-effort: finalize with exit 3; the run dir contains
            // backups under the chokepoint contract so a subsequent
            // `werk doctor undo <run-id>` can roll back manually.
            let _ = run.finalize(
                EXIT_FIX_FAILED,
                format!("fix failed: {}", e),
                findings.iter().map(|f| serde_json::to_value(f).unwrap_or(serde_json::Value::Null)).collect(),
            );
            emit_refusal(output, &format!("fixer error: {}", e));
            return Ok(EXIT_FIX_FAILED);
        }
    };

    let summary = if fix_result.purged > 0 {
        format!("purged {} noop mutation row(s)", fix_result.purged)
    } else {
        "no actions taken".to_string()
    };
    let report = run
        .finalize(
            EXIT_HEALTHY,
            &summary,
            findings.iter().map(|f| serde_json::to_value(f).unwrap_or(serde_json::Value::Null)).collect(),
        )
        .map_err(WerkError::CoreError)?;

    let envelope = serde_json::json!({
        "schema_version": DOCTOR_CONTRACT_VERSION,
        "verb": "doctor",
        "run_id": report.run_id,
        "exit_code": EXIT_HEALTHY,
        "data": {
            "summary": summary,
            "findings": findings,
            "actions_taken": report.action_count,
            "purged": fix_result.purged,
        }
    });
    print_envelope(output, robot, &envelope, |_| {
        println!("{}", summary);
        eprintln!("Run id: {}", report.run_id);
    });
    Ok(EXIT_HEALTHY)
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
        "singleParent" | "noSelfEdges" | "edgesValid" | "siblingPositionsUnique"
        | "noContainmentViolations" | "undoneSubsetOfCompleted" => Some(serde_json::json!({
            "id": finding_id,
            "subsystem": if finding_id == "undoneSubsetOfCompleted" { "gestures" } else { "edges" },
            "severity": "unknown",
            "summary": "Quint invariant — reserved capability, runtime detector lands in R-005.",
            "available": false,
            "reserved_for": "R-005",
            "specs": "specs/werk.qnt:458-472",
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
    let _subsystems = parse_only(only)?; // currently unused; future R-005 fanout
    let store = workspace.open_store()?;
    let run = DoctorRun::start(workspace.root()).map_err(|e| {
        emit_refusal(output, &format!("cannot create run dir: {}", e));
        WerkError::IoError(e.to_string())
    })?;
    let store_findings = run_store_detect(&store)?;
    let findings = store_findings.into_findings();
    let mut actions_planned = Vec::new();
    if store_findings.noop_mutations > 0 {
        actions_planned.push(serde_json::json!({
            "op": "purge_noop_mutations",
            "target": ".werk/werk.db",
            "fixer": "purge_noop_mutations",
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
            "noSelfEdges",
            "edgesValid",
            "siblingPositionsUnique",
            "noContainmentViolations",
            "undoneSubsetOfCompleted",
        ];
        let caps = capabilities();
        for d in &caps.detectors {
            // R-005-reserved detectors are addressed by their Quint id.
            // The available detector(s) currently map to fm-* ids via
            // their Finding::id, not the detector spec id. Both are listed.
            if !KNOWN_TO_EXPLAIN.contains(&d.id) {
                panic!(
                    "capabilities declares detector `{}` but cmd_explain has no spec for it",
                    d.id
                );
            }
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
