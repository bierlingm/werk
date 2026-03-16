//! Watch command handler — The Daimon.
//!
//! A background daemon that monitors tension dynamics and invokes the agent
//! when structurally significant thresholds cross. Turns werk from a passive
//! tool into an instrument that watches while you work.

use crate::commands::config::Config;
use crate::dynamics::compute_all_dynamics;
use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use chrono::{DateTime, Utc};
use sd_core::{DynamicsEngine, TensionStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ============================================================================
// Data types
// ============================================================================

/// A snapshot of dynamics state for a single tension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensionSnapshot {
    pub tension_id: String,
    pub desired: String,
    pub phase: String,
    pub tendency: String,
    pub has_conflict: bool,
    pub has_neglect: bool,
    pub oscillation_reversals: usize,
    pub has_resolution: bool,
    pub status: String,
    pub horizon_drift: String,
    pub timestamp: DateTime<Utc>,
}

/// The full snapshot store: all tensions at last check.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SnapshotStore {
    pub tensions: HashMap<String, TensionSnapshot>,
    pub last_check: Option<DateTime<Utc>>,
}

/// Trigger types that can fire an agent check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    NeglectOnset,
    ConflictDetected,
    OscillationSpike,
    HorizonBreach,
    PhaseTransition,
    Stagnation,
    Resolution,
}

impl std::fmt::Display for TriggerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TriggerType::NeglectOnset => write!(f, "neglect_onset"),
            TriggerType::ConflictDetected => write!(f, "conflict_detected"),
            TriggerType::OscillationSpike => write!(f, "oscillation_spike"),
            TriggerType::HorizonBreach => write!(f, "horizon_breach"),
            TriggerType::PhaseTransition => write!(f, "phase_transition"),
            TriggerType::Stagnation => write!(f, "stagnation"),
            TriggerType::Resolution => write!(f, "resolution"),
        }
    }
}

/// A detected threshold crossing.
#[derive(Debug, Clone)]
pub struct ThresholdCrossing {
    pub tension_id: String,
    pub tension_desired: String,
    pub trigger: TriggerType,
    pub previous_summary: String,
    pub current_summary: String,
}

/// A pending insight written by the watch daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingInsight {
    pub tension_id: String,
    pub trigger: String,
    pub timestamp: DateTime<Utc>,
    pub response: String,
    pub mutations: Vec<serde_yaml::Value>,
    pub reviewed: bool,
    pub tension_desired: String,
}

/// Cooldown tracker: (tension_id, trigger_type) -> last fire time.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CooldownStore {
    pub entries: HashMap<String, DateTime<Utc>>,
}

impl CooldownStore {
    fn key(tension_id: &str, trigger: &TriggerType) -> String {
        format!("{}:{}", tension_id, trigger)
    }

    pub fn is_cooled_down(&self, tension_id: &str, trigger: &TriggerType, cooldown_minutes: i64) -> bool {
        let key = Self::key(tension_id, trigger);
        match self.entries.get(&key) {
            Some(last_fire) => {
                let elapsed = Utc::now().signed_duration_since(*last_fire).num_minutes();
                elapsed >= cooldown_minutes
            }
            None => true,
        }
    }

    pub fn record(&mut self, tension_id: &str, trigger: &TriggerType) {
        let key = Self::key(tension_id, trigger);
        self.entries.insert(key, Utc::now());
    }
}

/// Watch history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub tension_id: String,
    pub trigger: String,
    pub summary: String,
}

/// Watch history log.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WatchHistory {
    pub entries: Vec<HistoryEntry>,
}

// ============================================================================
// File I/O helpers
// ============================================================================

fn watch_dir(workspace: &Workspace) -> PathBuf {
    workspace.werk_dir().join("watch")
}

fn snapshots_path(workspace: &Workspace) -> PathBuf {
    watch_dir(workspace).join("snapshots.json")
}

fn cooldowns_path(workspace: &Workspace) -> PathBuf {
    watch_dir(workspace).join("cooldowns.json")
}

fn history_path(workspace: &Workspace) -> PathBuf {
    watch_dir(workspace).join("history.json")
}

fn pending_dir(workspace: &Workspace) -> PathBuf {
    watch_dir(workspace).join("pending")
}

fn pid_path(workspace: &Workspace) -> PathBuf {
    watch_dir(workspace).join("daemon.pid")
}

fn ensure_watch_dirs(workspace: &Workspace) -> Result<(), WerkError> {
    let dirs = [watch_dir(workspace), pending_dir(workspace)];
    for d in &dirs {
        if !d.exists() {
            std::fs::create_dir_all(d).map_err(|e| {
                WerkError::IoError(format!("failed to create watch directory: {}", e))
            })?;
        }
    }
    Ok(())
}

fn load_snapshots(workspace: &Workspace) -> SnapshotStore {
    let path = snapshots_path(workspace);
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => SnapshotStore::default(),
        }
    } else {
        SnapshotStore::default()
    }
}

fn save_snapshots(workspace: &Workspace, store: &SnapshotStore) -> Result<(), WerkError> {
    let path = snapshots_path(workspace);
    let content = serde_json::to_string_pretty(store)
        .map_err(|e| WerkError::IoError(format!("failed to serialize snapshots: {}", e)))?;
    std::fs::write(&path, content)
        .map_err(|e| WerkError::IoError(format!("failed to write snapshots: {}", e)))?;
    Ok(())
}

fn load_cooldowns(workspace: &Workspace) -> CooldownStore {
    let path = cooldowns_path(workspace);
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => CooldownStore::default(),
        }
    } else {
        CooldownStore::default()
    }
}

fn save_cooldowns(workspace: &Workspace, store: &CooldownStore) -> Result<(), WerkError> {
    let path = cooldowns_path(workspace);
    let content = serde_json::to_string_pretty(store)
        .map_err(|e| WerkError::IoError(format!("failed to serialize cooldowns: {}", e)))?;
    std::fs::write(&path, content)
        .map_err(|e| WerkError::IoError(format!("failed to write cooldowns: {}", e)))?;
    Ok(())
}

fn load_history(workspace: &Workspace) -> WatchHistory {
    let path = history_path(workspace);
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => WatchHistory::default(),
        }
    } else {
        WatchHistory::default()
    }
}

fn save_history(workspace: &Workspace, history: &WatchHistory) -> Result<(), WerkError> {
    let path = history_path(workspace);
    let content = serde_json::to_string_pretty(history)
        .map_err(|e| WerkError::IoError(format!("failed to serialize history: {}", e)))?;
    std::fs::write(&path, content)
        .map_err(|e| WerkError::IoError(format!("failed to write history: {}", e)))?;
    Ok(())
}

fn write_pending_insight(workspace: &Workspace, insight: &PendingInsight) -> Result<PathBuf, WerkError> {
    let filename = format!(
        "{}_{}.yaml",
        insight.timestamp.format("%Y-%m-%dT%H_%M_%S"),
        &insight.tension_id[..8.min(insight.tension_id.len())],
    );
    let path = pending_dir(workspace).join(&filename);
    let content = serde_yaml::to_string(insight)
        .map_err(|e| WerkError::IoError(format!("failed to serialize insight: {}", e)))?;
    std::fs::write(&path, content)
        .map_err(|e| WerkError::IoError(format!("failed to write pending insight: {}", e)))?;
    Ok(path)
}

/// Load all pending insights from disk.
pub fn load_pending_insights(workspace: &Workspace) -> Vec<(PathBuf, PendingInsight)> {
    let dir = pending_dir(workspace);
    if !dir.exists() {
        return Vec::new();
    }
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "yaml").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(insight) = serde_yaml::from_str::<PendingInsight>(&content) {
                        if !insight.reviewed {
                            results.push((path, insight));
                        }
                    }
                }
            }
        }
    }
    // Sort by timestamp (oldest first)
    results.sort_by_key(|(_, i)| i.timestamp);
    results
}

/// Mark a pending insight as reviewed on disk.
pub fn mark_insight_reviewed(path: &Path) -> Result<(), WerkError> {
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(mut insight) = serde_yaml::from_str::<PendingInsight>(&content) {
            insight.reviewed = true;
            let new_content = serde_yaml::to_string(&insight)
                .map_err(|e| WerkError::IoError(format!("failed to serialize insight: {}", e)))?;
            std::fs::write(path, new_content)
                .map_err(|e| WerkError::IoError(format!("failed to write insight: {}", e)))?;
        }
    }
    Ok(())
}

// ============================================================================
// Snapshot + diff logic
// ============================================================================

fn snapshot_tension(engine: &mut DynamicsEngine, tension_id: &str) -> Option<TensionSnapshot> {
    let tension = engine.store().get_tension(tension_id).ok()??;
    let dynamics = compute_all_dynamics(engine, tension_id);

    Some(TensionSnapshot {
        tension_id: tension.id.clone(),
        desired: tension.desired.clone(),
        phase: dynamics.phase.phase.clone(),
        tendency: dynamics.structural_tendency.tendency.clone(),
        has_conflict: dynamics.structural_conflict.is_some(),
        has_neglect: dynamics.neglect.is_some(),
        oscillation_reversals: dynamics.oscillation.as_ref().map(|o| o.reversals).unwrap_or(0),
        has_resolution: dynamics.resolution.is_some(),
        status: tension.status.to_string(),
        horizon_drift: dynamics.horizon_drift.drift_type.clone(),
        timestamp: Utc::now(),
    })
}

fn detect_crossings(
    old: &SnapshotStore,
    new_snapshots: &HashMap<String, TensionSnapshot>,
) -> Vec<ThresholdCrossing> {
    let mut crossings = Vec::new();

    for (id, current) in new_snapshots {
        if let Some(previous) = old.tensions.get(id) {
            // Neglect onset: was false, now true
            if !previous.has_neglect && current.has_neglect {
                crossings.push(ThresholdCrossing {
                    tension_id: id.clone(),
                    tension_desired: current.desired.clone(),
                    trigger: TriggerType::NeglectOnset,
                    previous_summary: "no neglect".to_string(),
                    current_summary: "neglect detected".to_string(),
                });
            }

            // Conflict detected: was false, now true
            if !previous.has_conflict && current.has_conflict {
                crossings.push(ThresholdCrossing {
                    tension_id: id.clone(),
                    tension_desired: current.desired.clone(),
                    trigger: TriggerType::ConflictDetected,
                    previous_summary: "no conflict".to_string(),
                    current_summary: "conflict detected".to_string(),
                });
            }

            // Oscillation spike: reversals increased by 2+
            if current.oscillation_reversals >= previous.oscillation_reversals + 2 {
                crossings.push(ThresholdCrossing {
                    tension_id: id.clone(),
                    tension_desired: current.desired.clone(),
                    trigger: TriggerType::OscillationSpike,
                    previous_summary: format!("{} reversals", previous.oscillation_reversals),
                    current_summary: format!("{} reversals", current.oscillation_reversals),
                });
            }

            // Phase transition
            if previous.phase != current.phase {
                crossings.push(ThresholdCrossing {
                    tension_id: id.clone(),
                    tension_desired: current.desired.clone(),
                    trigger: TriggerType::PhaseTransition,
                    previous_summary: previous.phase.clone(),
                    current_summary: current.phase.clone(),
                });
            }

            // Stagnation: tendency was not Stagnant, now is Stagnant
            if previous.tendency != "Stagnant" && current.tendency == "Stagnant" {
                crossings.push(ThresholdCrossing {
                    tension_id: id.clone(),
                    tension_desired: current.desired.clone(),
                    trigger: TriggerType::Stagnation,
                    previous_summary: previous.tendency.clone(),
                    current_summary: "Stagnant".to_string(),
                });
            }

            // Resolution: status was Active, now Resolved
            if previous.status == "Active" && current.status == "Resolved" {
                crossings.push(ThresholdCrossing {
                    tension_id: id.clone(),
                    tension_desired: current.desired.clone(),
                    trigger: TriggerType::Resolution,
                    previous_summary: "Active".to_string(),
                    current_summary: "Resolved".to_string(),
                });
            }

            // Horizon breach: drift went to Postponement or RepeatedPostponement
            if previous.horizon_drift != "Postponement"
                && previous.horizon_drift != "RepeatedPostponement"
                && (current.horizon_drift == "Postponement"
                    || current.horizon_drift == "RepeatedPostponement")
            {
                crossings.push(ThresholdCrossing {
                    tension_id: id.clone(),
                    tension_desired: current.desired.clone(),
                    trigger: TriggerType::HorizonBreach,
                    previous_summary: previous.horizon_drift.clone(),
                    current_summary: current.horizon_drift.clone(),
                });
            }
        }
        // New tensions: no crossing to detect on first sight
    }

    crossings
}

// ============================================================================
// Agent invocation
// ============================================================================

fn build_watch_prompt(
    crossing: &ThresholdCrossing,
    engine: &mut DynamicsEngine,
    all_tensions: &[sd_core::Tension],
) -> String {
    let tension = all_tensions.iter().find(|t| t.id == crossing.tension_id);
    let context = match tension {
        Some(t) => crate::commands::run::build_context_markdown(engine, t, all_tensions),
        None => String::new(),
    };

    format!(
        "SYSTEM: You are monitoring structural dynamics for the user's tensions.\n\
         A threshold was crossed. Analyze what changed and suggest one action.\n\n\
         TRIGGER: {trigger}\n\
         TENSION: \"{desired}\"\n\
         PREVIOUS STATE: {previous}\n\
         CURRENT STATE: {current}\n\n\
         Context:\n{context}\n\n\
         Respond with a brief observation (2-3 sentences) and optionally suggest one mutation.\n\
         Use YAML format if suggesting a change:\n\
         ---\n\
         mutations:\n\
           - action: add_note\n\
             tension_id: \"{tid}\"\n\
             text: \"your note\"\n\
         response: |\n\
           Your observation here.\n\
         ---\n\n\
         If no change is needed, respond with plain text only.",
        trigger = crossing.trigger,
        desired = crossing.tension_desired,
        previous = crossing.previous_summary,
        current = crossing.current_summary,
        context = context,
        tid = crossing.tension_id,
    )
}

fn invoke_agent_for_crossing(
    workspace: &Workspace,
    engine: &mut DynamicsEngine,
    crossing: &ThresholdCrossing,
    all_tensions: &[sd_core::Tension],
) -> Result<PendingInsight, WerkError> {
    let config = Config::load(workspace)?;
    let agent_cmd = config.get("agent.command").cloned().ok_or_else(|| {
        WerkError::InvalidInput("no agent command configured. Set agent.command in config".to_string())
    })?;

    let prompt = build_watch_prompt(crossing, engine, all_tensions);
    let response_text = crate::commands::run::execute_agent_capture(&agent_cmd, &prompt)?;

    // Try to parse structured YAML from response
    let mut mutations_yaml: Vec<serde_yaml::Value> = Vec::new();
    let response_prose = if let Some(structured) = crate::agent_response::StructuredResponse::from_response(&response_text) {
        // Convert mutations to yaml values
        for m in &structured.mutations {
            if let Ok(val) = serde_yaml::to_value(m) {
                mutations_yaml.push(val);
            }
        }
        structured.response.clone()
    } else {
        response_text.trim().to_string()
    };

    Ok(PendingInsight {
        tension_id: crossing.tension_id.clone(),
        trigger: crossing.trigger.to_string(),
        timestamp: Utc::now(),
        response: response_prose,
        mutations: mutations_yaml,
        reviewed: false,
        tension_desired: crossing.tension_desired.clone(),
    })
}

// ============================================================================
// The watch loop
// ============================================================================

fn run_single_check(
    workspace: &Workspace,
    engine: &mut DynamicsEngine,
) -> Result<usize, WerkError> {
    ensure_watch_dirs(workspace)?;

    let all_tensions = engine.store().list_tensions().map_err(WerkError::StoreError)?;
    let active: Vec<_> = all_tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Active)
        .collect();

    // Load previous snapshots
    let old_snapshots = load_snapshots(workspace);

    // Compute new snapshots for all active tensions
    let mut new_snapshots: HashMap<String, TensionSnapshot> = HashMap::new();
    for t in &active {
        if let Some(snap) = snapshot_tension(engine, &t.id) {
            new_snapshots.insert(t.id.clone(), snap);
        }
    }

    // Detect threshold crossings
    let crossings = detect_crossings(&old_snapshots, &new_snapshots);

    // Load cooldowns
    let mut cooldowns = load_cooldowns(workspace);
    let config = Config::load(workspace)?;
    let cooldown_minutes: i64 = config
        .get("watch.cooldown")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1440); // 24 hours default

    // Filter by cooldown
    let actionable: Vec<_> = crossings
        .into_iter()
        .filter(|c| cooldowns.is_cooled_down(&c.tension_id, &c.trigger, cooldown_minutes))
        .collect();

    let mut insights_written = 0;

    // Invoke agent for each actionable crossing
    for crossing in &actionable {
        eprintln!(
            "i threshold crossed: {} on \"{}\"",
            crossing.trigger,
            werk_shared::truncate(&crossing.tension_desired, 40),
        );

        match invoke_agent_for_crossing(workspace, engine, crossing, &all_tensions) {
            Ok(insight) => {
                let path = write_pending_insight(workspace, &insight)?;
                eprintln!("i   insight written: {}", path.display());
                insights_written += 1;

                // Record cooldown
                cooldowns.record(&crossing.tension_id, &crossing.trigger);

                // Record history
                let mut history = load_history(workspace);
                history.entries.push(HistoryEntry {
                    timestamp: Utc::now(),
                    tension_id: crossing.tension_id.clone(),
                    trigger: crossing.trigger.to_string(),
                    summary: werk_shared::truncate(&insight.response, 80).to_string(),
                });
                // Keep last 100 entries
                if history.entries.len() > 100 {
                    let start = history.entries.len() - 100;
                    history.entries = history.entries[start..].to_vec();
                }
                save_history(workspace, &history)?;
            }
            Err(e) => {
                eprintln!("! agent invocation failed: {}", e);
            }
        }
    }

    // Save new snapshots
    let snapshot_store = SnapshotStore {
        tensions: new_snapshots,
        last_check: Some(Utc::now()),
    };
    save_snapshots(workspace, &snapshot_store)?;
    save_cooldowns(workspace, &cooldowns)?;

    Ok(insights_written)
}

// ============================================================================
// CLI command
// ============================================================================

pub fn cmd_watch(
    output: &Output,
    daemon: bool,
    stop: bool,
    status: bool,
    pending: bool,
    history: bool,
) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;

    if stop {
        return cmd_watch_stop(output, &workspace);
    }
    if status {
        return cmd_watch_status(output, &workspace);
    }
    if pending {
        return cmd_watch_pending(output, &workspace);
    }
    if history {
        return cmd_watch_history(output, &workspace);
    }
    if daemon {
        return cmd_watch_daemon(output, &workspace);
    }

    // Default: foreground watch loop
    cmd_watch_foreground(output, &workspace)
}

fn cmd_watch_foreground(output: &Output, workspace: &Workspace) -> Result<(), WerkError> {
    ensure_watch_dirs(workspace)?;

    let config = Config::load(workspace)?;
    let interval_minutes: u64 = config
        .get("watch.interval")
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let store = workspace.open_store()?;
    let mut engine = DynamicsEngine::with_store(store);

    let _ = output.info(&format!("watching every {} minutes. Ctrl-C to stop.", interval_minutes));
    let _ = output.info("running initial check...");

    // Initial check
    let n = run_single_check(workspace, &mut engine)?;
    let _ = output.info(&format!("initial check complete. {} insights generated.", n));

    // Loop
    loop {
        std::thread::sleep(std::time::Duration::from_secs(interval_minutes * 60));

        // Re-open store to pick up external changes
        match workspace.open_store() {
            Ok(store) => {
                engine = DynamicsEngine::with_store(store);
                match run_single_check(workspace, &mut engine) {
                    Ok(n) => {
                        let now = Utc::now().format("%H:%M:%S");
                        let _ = output.info(&format!("[{}] check complete. {} insights.", now, n));
                    }
                    Err(e) => {
                        eprintln!("! check failed: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("! failed to open store: {}", e);
            }
        }
    }
}

fn cmd_watch_daemon(output: &Output, workspace: &Workspace) -> Result<(), WerkError> {
    ensure_watch_dirs(workspace)?;

    // Check if already running
    let pid_file = pid_path(workspace);
    if pid_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&pid_file) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                let check = std::process::Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .output();
                if check.map(|o| o.status.success()).unwrap_or(false) {
                    let _ = output.info(&format!("daemon already running (pid {})", pid));
                    return Ok(());
                }
            }
        }
    }

    // Fork: re-execute ourselves with nohup
    let exe = std::env::current_exe()
        .map_err(|e| WerkError::IoError(format!("failed to get current exe: {}", e)))?;

    let child = std::process::Command::new("nohup")
        .args([
            exe.to_str().unwrap_or("werk"),
            "watch",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| WerkError::IoError(format!("failed to spawn daemon: {}", e)))?;

    let pid = child.id();
    std::fs::write(&pid_file, pid.to_string())
        .map_err(|e| WerkError::IoError(format!("failed to write pid file: {}", e)))?;

    let _ = output.info(&format!("daemon started (pid {})", pid));
    Ok(())
}

fn cmd_watch_stop(output: &Output, workspace: &Workspace) -> Result<(), WerkError> {
    let pid_file = pid_path(workspace);
    if !pid_file.exists() {
        let _ = output.info("no daemon running");
        return Ok(());
    }

    if let Ok(content) = std::fs::read_to_string(&pid_file) {
        if let Ok(pid) = content.trim().parse::<u32>() {
            let _ = std::process::Command::new("kill")
                .args([&pid.to_string()])
                .output();
            let _ = output.info(&format!("stopped daemon (pid {})", pid));
        }
    }

    let _ = std::fs::remove_file(&pid_file);
    Ok(())
}

fn cmd_watch_status(output: &Output, workspace: &Workspace) -> Result<(), WerkError> {
    ensure_watch_dirs(workspace)?;

    let config = Config::load(workspace)?;
    let interval: u64 = config.get("watch.interval").and_then(|v| v.parse().ok()).unwrap_or(30);
    let cooldown: u64 = config.get("watch.cooldown").and_then(|v| v.parse().ok()).unwrap_or(1440);

    let snapshots = load_snapshots(workspace);
    let pending = load_pending_insights(workspace);

    // Check daemon status
    let pid_file = pid_path(workspace);
    let daemon_status = if pid_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&pid_file) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                let check = std::process::Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .output();
                if check.map(|o| o.status.success()).unwrap_or(false) {
                    format!("running (pid {})", pid)
                } else {
                    "stopped (stale pid)".to_string()
                }
            } else {
                "unknown".to_string()
            }
        } else {
            "unknown".to_string()
        }
    } else {
        "not running".to_string()
    };

    if output.is_json() {
        let status_json = serde_json::json!({
            "daemon": daemon_status,
            "interval_minutes": interval,
            "cooldown_minutes": cooldown,
            "tensions_tracked": snapshots.tensions.len(),
            "last_check": snapshots.last_check.map(|t| t.to_rfc3339()),
            "pending_insights": pending.len(),
        });
        let _ = output.print_structured(&status_json);
    } else {
        println!("daemon: {}", daemon_status);
        println!("interval: {} minutes", interval);
        println!("cooldown: {} minutes", cooldown);
        println!("tensions tracked: {}", snapshots.tensions.len());
        if let Some(last) = snapshots.last_check {
            let ago = Utc::now().signed_duration_since(last).num_minutes();
            println!("last check: {} ({} minutes ago)", last.format("%Y-%m-%d %H:%M"), ago);
        } else {
            println!("last check: never");
        }
        println!("pending insights: {}", pending.len());
    }

    Ok(())
}

fn cmd_watch_pending(output: &Output, workspace: &Workspace) -> Result<(), WerkError> {
    let pending = load_pending_insights(workspace);

    if output.is_json() {
        let items: Vec<serde_json::Value> = pending
            .iter()
            .map(|(path, insight)| {
                serde_json::json!({
                    "file": path.display().to_string(),
                    "tension_id": insight.tension_id,
                    "tension": insight.tension_desired,
                    "trigger": insight.trigger,
                    "timestamp": insight.timestamp.to_rfc3339(),
                    "response": insight.response,
                    "mutations": insight.mutations.len(),
                })
            })
            .collect();
        let _ = output.print_structured(&serde_json::json!({ "pending": items }));
    } else {
        if pending.is_empty() {
            println!("no pending insights");
            return Ok(());
        }
        println!("{} pending insights:\n", pending.len());
        for (_path, insight) in &pending {
            let ago = Utc::now().signed_duration_since(insight.timestamp).num_minutes();
            println!(
                "  {} on \"{}\"  ({}m ago)",
                insight.trigger,
                werk_shared::truncate(&insight.tension_desired, 30),
                ago,
            );
            if let Some(first_line) = insight.response.lines().next() {
                println!("    {}", werk_shared::truncate(first_line, 70));
            }
            if !insight.mutations.is_empty() {
                println!("    {} suggested mutation(s)", insight.mutations.len());
            }
            println!();
        }
    }

    Ok(())
}

fn cmd_watch_history(output: &Output, workspace: &Workspace) -> Result<(), WerkError> {
    let history = load_history(workspace);

    if output.is_json() {
        let items: Vec<serde_json::Value> = history
            .entries
            .iter()
            .rev()
            .take(20)
            .map(|e| {
                serde_json::json!({
                    "timestamp": e.timestamp.to_rfc3339(),
                    "tension_id": e.tension_id,
                    "trigger": e.trigger,
                    "summary": e.summary,
                })
            })
            .collect();
        let _ = output.print_structured(&serde_json::json!({ "history": items }));
    } else {
        if history.entries.is_empty() {
            println!("no watch history");
            return Ok(());
        }
        println!("recent watch activity:\n");
        for entry in history.entries.iter().rev().take(20) {
            let ago = Utc::now().signed_duration_since(entry.timestamp).num_minutes();
            println!(
                "  [{}m ago] {} on {}",
                ago,
                entry.trigger,
                &entry.tension_id[..8.min(entry.tension_id.len())],
            );
            println!("    {}", werk_shared::truncate(&entry.summary, 70));
            println!();
        }
    }

    Ok(())
}
