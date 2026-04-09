//! Hook system for werk — execute shell commands on lifecycle events.
//!
//! Three layers:
//! - **HookRunner**: executes hooks by name. Supports chains (multiple commands
//!   per event), category hooks (`pre_mutation`, `post_mutation`), wildcards
//!   (`post_*`), and filters (`parent:N`).
//! - **HookBridge**: subscribes to the EventBus and fires post-hooks automatically.
//!   Adding a new Event variant makes it hookable with zero wiring.
//! - **HookEvent**: the JSON payload sent to hook scripts via stdin.
//!
//! Pre-hooks remain at the command level (CLI/MCP check before calling Store).
//! Post-hooks fire automatically via the bridge after Store emits events.

use crate::config::Config;
use chrono::{DateTime, Utc};
use sd_core::events::{Event, EventBus};
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// ============================================================================
// Hook Event Payload
// ============================================================================

/// Event payload sent to hooks via stdin as JSON.
///
/// This is a superset payload — all fields present, nullable where not applicable.
/// Hook scripts receive this and filter on what they care about.
#[derive(Debug, Clone, Serialize)]
pub struct HookEvent {
    pub event: String,
    pub category: String,
    pub timestamp: DateTime<Utc>,
    pub tension_id: Option<String>,
    pub tension_desired: Option<String>,
    pub current_reality: Option<String>,
    pub parent_id: Option<String>,
    pub field: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

impl HookEvent {
    /// Build a HookEvent from an sd-core Event.
    ///
    /// The Event's serde representation carries all the typed data.
    /// We flatten it into the hook payload format for backward compatibility.
    pub fn from_event(event: &Event) -> Self {
        let hook_name = event.hook_name().to_string();
        let category = event.category().to_string();
        let timestamp = event.timestamp();
        let tension_id = event.tension_id().map(|s| s.to_string());

        match event {
            Event::TensionCreated {
                desired,
                actual,
                parent_id,
                ..
            } => Self {
                event: hook_name,
                category,
                timestamp,
                tension_id,
                tension_desired: Some(desired.clone()),
                current_reality: Some(actual.clone()),
                parent_id: parent_id.clone(),
                field: None,
                old_value: None,
                new_value: None,
            },
            Event::RealityConfronted {
                old_actual,
                new_actual,
                ..
            } => Self {
                event: hook_name,
                category,
                timestamp,
                tension_id,
                tension_desired: None,
                current_reality: Some(new_actual.clone()),
                parent_id: None,
                field: Some("actual".to_string()),
                old_value: Some(old_actual.clone()),
                new_value: Some(new_actual.clone()),
            },
            Event::DesireRevised {
                old_desired,
                new_desired,
                ..
            } => Self {
                event: hook_name,
                category,
                timestamp,
                tension_id,
                tension_desired: Some(new_desired.clone()),
                current_reality: None,
                parent_id: None,
                field: Some("desired".to_string()),
                old_value: Some(old_desired.clone()),
                new_value: Some(new_desired.clone()),
            },
            Event::TensionResolved {
                final_desired,
                final_actual,
                ..
            } => Self {
                event: hook_name,
                category,
                timestamp,
                tension_id,
                tension_desired: Some(final_desired.clone()),
                current_reality: Some(final_actual.clone()),
                parent_id: None,
                field: Some("status".to_string()),
                old_value: Some("Active".to_string()),
                new_value: Some("Resolved".to_string()),
            },
            Event::TensionReleased {
                desired, actual, ..
            } => Self {
                event: hook_name,
                category,
                timestamp,
                tension_id,
                tension_desired: Some(desired.clone()),
                current_reality: Some(actual.clone()),
                parent_id: None,
                field: Some("status".to_string()),
                old_value: Some("Active".to_string()),
                new_value: Some("Released".to_string()),
            },
            Event::TensionDeleted {
                desired, actual, ..
            } => Self {
                event: hook_name,
                category,
                timestamp,
                tension_id,
                tension_desired: Some(desired.clone()),
                current_reality: Some(actual.clone()),
                parent_id: None,
                field: None,
                old_value: None,
                new_value: None,
            },
            Event::StructureChanged {
                old_parent_id,
                new_parent_id,
                ..
            } => Self {
                event: hook_name,
                category,
                timestamp,
                tension_id,
                tension_desired: None,
                current_reality: None,
                parent_id: new_parent_id.clone(),
                field: Some("parent_id".to_string()),
                old_value: old_parent_id.clone(),
                new_value: new_parent_id.clone(),
            },
            Event::HorizonChanged {
                old_horizon,
                new_horizon,
                ..
            } => Self {
                event: hook_name,
                category,
                timestamp,
                tension_id,
                tension_desired: None,
                current_reality: None,
                parent_id: None,
                field: Some("horizon".to_string()),
                old_value: old_horizon.clone(),
                new_value: new_horizon.clone(),
            },
            Event::UrgencyThresholdCrossed {
                old_urgency,
                new_urgency,
                threshold,
                crossed_above,
                ..
            } => Self {
                event: hook_name,
                category,
                timestamp,
                tension_id,
                tension_desired: None,
                current_reality: None,
                parent_id: None,
                field: Some("urgency".to_string()),
                old_value: Some(old_urgency.to_string()),
                new_value: Some(format!(
                    "{}:threshold={}:above={}",
                    new_urgency, threshold, crossed_above
                )),
            },
            Event::HorizonDriftDetected {
                drift_type,
                change_count,
                ..
            } => Self {
                event: hook_name,
                category,
                timestamp,
                tension_id,
                tension_desired: None,
                current_reality: None,
                parent_id: None,
                field: Some("horizon_drift".to_string()),
                old_value: None,
                new_value: Some(format!("{:?}:{}", drift_type, change_count)),
            },
            Event::NoteTaken {
                tension_id,
                text,
                ..
            } => Self {
                event: hook_name,
                category,
                timestamp,
                tension_id: Some(tension_id.clone()),
                tension_desired: None,
                current_reality: None,
                parent_id: None,
                field: Some("note".to_string()),
                old_value: None,
                new_value: Some(text.clone()),
            },
            Event::NoteRetracted {
                tension_id,
                text,
                ..
            } => Self {
                event: hook_name,
                category,
                timestamp,
                tension_id: Some(tension_id.clone()),
                tension_desired: None,
                current_reality: None,
                parent_id: None,
                field: Some("note".to_string()),
                old_value: Some(text.clone()),
                new_value: None,
            },
            Event::GestureUndone {
                gesture_id,
                undo_gesture_id,
                reversed_mutation_count,
                ..
            } => Self {
                event: hook_name,
                category,
                timestamp,
                tension_id: None,
                tension_desired: None,
                current_reality: None,
                parent_id: None,
                field: Some("gesture".to_string()),
                old_value: Some(gesture_id.clone()),
                new_value: Some(format!(
                    "{}:reversed={}",
                    undo_gesture_id, reversed_mutation_count
                )),
            },
        }
    }

    /// Legacy factory: build a mutation HookEvent manually (for pre-hooks at command level).
    pub fn mutation(
        tension_id: &str,
        tension_desired: &str,
        current_reality: Option<&str>,
        parent_id: Option<&str>,
        field: &str,
        old_value: Option<&str>,
        new_value: &str,
    ) -> Self {
        Self {
            event: "mutation".to_string(),
            category: "mutation".to_string(),
            timestamp: Utc::now(),
            tension_id: Some(tension_id.to_string()),
            tension_desired: Some(tension_desired.to_string()),
            current_reality: current_reality.map(|s| s.to_string()),
            parent_id: parent_id.map(|s| s.to_string()),
            field: Some(field.to_string()),
            old_value: old_value.map(|s| s.to_string()),
            new_value: Some(new_value.to_string()),
        }
    }

    /// Legacy factory: build a status change HookEvent manually (for pre-hooks).
    pub fn status_change(
        tension_id: &str,
        tension_desired: &str,
        current_reality: Option<&str>,
        parent_id: Option<&str>,
        new_status: &str,
    ) -> Self {
        Self {
            event: new_status.to_lowercase(),
            category: "status_change".to_string(),
            timestamp: Utc::now(),
            tension_id: Some(tension_id.to_string()),
            tension_desired: Some(tension_desired.to_string()),
            current_reality: current_reality.map(|s| s.to_string()),
            parent_id: parent_id.map(|s| s.to_string()),
            field: Some("status".to_string()),
            old_value: Some("Active".to_string()),
            new_value: Some(new_status.to_string()),
        }
    }

    /// Legacy factory: build a create HookEvent manually (for pre-hooks).
    pub fn create(
        tension_id: &str,
        tension_desired: &str,
        current_reality: Option<&str>,
        parent_id: Option<&str>,
    ) -> Self {
        Self {
            event: "create".to_string(),
            category: "create".to_string(),
            timestamp: Utc::now(),
            tension_id: Some(tension_id.to_string()),
            tension_desired: Some(tension_desired.to_string()),
            current_reality: current_reality.map(|s| s.to_string()),
            parent_id: parent_id.map(|s| s.to_string()),
            field: None,
            old_value: None,
            new_value: None,
        }
    }
}

// ============================================================================
// Hook Log Entry
// ============================================================================

/// A record of a hook execution, stored in the hook log.
#[derive(Debug, Clone, Serialize)]
pub struct HookLogEntry {
    pub timestamp: DateTime<Utc>,
    pub hook_name: String,
    pub command: String,
    pub event_type: String,
    pub tension_id: Option<String>,
    pub success: bool,
    pub duration_ms: u64,
    pub stderr: Option<String>,
}

// ============================================================================
// Hook Filter
// ============================================================================

/// A filter that restricts when a hook fires.
#[derive(Debug, Clone)]
pub enum HookFilter {
    /// Only fire for tensions with this parent ID (short code).
    Parent(String),
    /// Only fire for tensions with this status.
    Status(String),
}

impl HookFilter {
    /// Parse a filter string like "parent:42" or "status:active".
    pub fn parse(s: &str) -> Option<Self> {
        let (key, value) = s.split_once(':')?;
        match key {
            "parent" => Some(HookFilter::Parent(value.to_string())),
            "status" => Some(HookFilter::Status(value.to_string())),
            _ => None,
        }
    }

    /// Check if a HookEvent matches this filter.
    pub fn matches(&self, event: &HookEvent) -> bool {
        match self {
            HookFilter::Parent(pid) => event
                .parent_id
                .as_ref()
                .map(|p| p.contains(pid.as_str()))
                .unwrap_or(false),
            HookFilter::Status(status) => event
                .new_value
                .as_ref()
                .map(|v| v.eq_ignore_ascii_case(status))
                .unwrap_or(false),
        }
    }
}

// ============================================================================
// Hook Config Entry
// ============================================================================

/// A single hook configuration: one or more commands with optional filters.
#[derive(Debug, Clone)]
pub struct HookEntry {
    pub commands: Vec<String>,
    pub filters: Vec<HookFilter>,
}

// ============================================================================
// Hook Runner
// ============================================================================

/// Executes hooks based on configuration.
///
/// Supports:
/// - **Specific hooks**: `post_tension_resolved = "./notify.sh"`
/// - **Category hooks**: `pre_mutation = "./validate.sh"` (fires for any mutation)
/// - **Wildcards**: `post_* = "./log.sh"` (fires for all post events)
/// - **Chains**: `post_tension_resolved = ["./a.sh", "./b.sh"]` (ordered execution)
/// - **Filters**: via structured config (future — parsed from extended TOML)
///
/// Execution order for a given event: wildcard → category → specific.
/// Within each level, commands execute in chain order.
pub struct HookRunner {
    /// Map from hook name pattern to commands.
    /// Keys: "pre_mutation", "post_tension_resolved", "post_*", etc.
    hooks: HashMap<String, HookEntry>,
    /// Hook execution log (in-memory, latest N entries).
    log: Arc<Mutex<Vec<HookLogEntry>>>,
    /// Maximum log entries to keep in memory.
    max_log_entries: usize,
}

impl HookRunner {
    /// Create from workspace and global configs.
    ///
    /// Global hooks fire first, then workspace hooks. Both are merged.
    pub fn from_configs(global: Option<&Config>, workspace: &Config) -> Self {
        let mut hooks = HashMap::new();

        // Load global hooks first
        if let Some(global_config) = global {
            Self::load_hooks_from_config(global_config, &mut hooks);
        }

        // Load workspace hooks (append to chains, don't replace)
        Self::load_hooks_from_config(workspace, &mut hooks);

        Self {
            hooks,
            log: Arc::new(Mutex::new(Vec::new())),
            max_log_entries: 1000,
        }
    }

    /// Create from a single Config (backward compatible).
    pub fn from_config(config: &Config) -> Self {
        let mut hooks = HashMap::new();
        Self::load_hooks_from_config(config, &mut hooks);
        Self {
            hooks,
            log: Arc::new(Mutex::new(Vec::new())),
            max_log_entries: 1000,
        }
    }

    fn load_hooks_from_config(config: &Config, hooks: &mut HashMap<String, HookEntry>) {
        for (key, value) in config.values() {
            if let Some(hook_name) = key.strip_prefix("hooks.") {
                // Skip sub-keys like hooks.post_mutation.filter
                if hook_name.contains('.') {
                    continue;
                }

                let commands = Self::parse_commands(value);
                if commands.is_empty() {
                    continue;
                }

                // Check for associated filter
                let filter_key = format!("{}.filter", key);
                let filters = config
                    .get(&filter_key)
                    .and_then(|f| HookFilter::parse(f))
                    .into_iter()
                    .collect();

                let entry = hooks.entry(hook_name.to_string()).or_insert_with(|| {
                    HookEntry {
                        commands: Vec::new(),
                        filters: Vec::new(),
                    }
                });
                entry.commands.extend(commands);
                if entry.filters.is_empty() {
                    entry.filters = filters;
                }
            }
        }
    }

    /// Parse a value as either a single command or a TOML array of commands.
    fn parse_commands(value: &str) -> Vec<String> {
        let trimmed = value.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Parse as array: ["./a.sh", "./b.sh"]
            let inner = &trimmed[1..trimmed.len() - 1];
            inner
                .split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            vec![trimmed.to_string()]
        }
    }

    /// Create a no-op runner (no hooks configured).
    pub fn noop() -> Self {
        Self {
            hooks: HashMap::new(),
            log: Arc::new(Mutex::new(Vec::new())),
            max_log_entries: 1000,
        }
    }

    /// Get all configured hook names.
    pub fn configured_hooks(&self) -> Vec<&str> {
        self.hooks.keys().map(|s| s.as_str()).collect()
    }

    /// Get the commands for a specific hook name.
    pub fn get_commands(&self, hook_name: &str) -> Option<&[String]> {
        self.hooks.get(hook_name).map(|e| e.commands.as_slice())
    }

    /// Collect all hook names that should fire for a given event.
    ///
    /// Order: wildcard (`post_*`) → category (`post_mutation`) → specific (`post_tension_resolved`).
    /// For pre-hooks: `pre_*` → `pre_mutation` → `pre_tension_resolved`.
    fn matching_hooks(&self, prefix: &str, event: &HookEvent) -> Vec<(String, Vec<String>)> {
        let mut result = Vec::new();

        let wildcard_key = format!("{}*", prefix);
        let category_key = format!("{}{}", prefix, event.category);
        let specific_key = format!("{}{}", prefix, event.event);

        // Backward compat: old hook names map to new ones
        // post_create → post_tension_created, post_resolve → post_tension_resolved, etc.
        let legacy_keys = Self::legacy_hook_names(prefix, &event.event);

        for key in [&wildcard_key, &category_key, &specific_key]
            .into_iter()
            .chain(legacy_keys.iter())
        {
            if let Some(entry) = self.hooks.get(key.as_str()) {
                // Check filters
                if entry.filters.iter().all(|f| f.matches(event)) && !entry.commands.is_empty() {
                    result.push((key.clone(), entry.commands.clone()));
                }
            }
        }

        result
    }

    /// Map old hook names to new event-based names for backward compatibility.
    fn legacy_hook_names(prefix: &str, event_name: &str) -> Vec<String> {
        let mut names = Vec::new();
        // Old: post_resolve → matches tension_resolved
        // Old: post_release → matches tension_released
        // Old: post_create → matches tension_created
        match event_name {
            "tension_resolved" => names.push(format!("{}resolve", prefix)),
            "tension_released" => names.push(format!("{}release", prefix)),
            "tension_created" => names.push(format!("{}create", prefix)),
            _ => {}
        }
        names
    }

    /// Execute a pre-hook check. Returns Ok(true) if allowed, Ok(false) if blocked.
    ///
    /// Pre-hooks run synchronously. Any hook in the chain returning non-zero blocks.
    pub fn run_pre_hook(&self, event_name: &str, event: &HookEvent) -> Result<bool, String> {
        let mut check_event = event.clone();
        check_event.event = event_name.to_string();

        let matches = self.matching_hooks("pre_", &check_event);
        for (hook_name, commands) in matches {
            for command in &commands {
                let (success, _stdout, stderr, duration) = Self::execute_command(command, event);
                self.record_log(&hook_name, command, event, success, duration, &stderr);
                if !success {
                    eprintln!("Hook '{}' blocked: {}", hook_name, stderr.trim());
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// Execute post-hooks for an event (fire-and-forget).
    pub fn run_post_hooks(&self, event: &HookEvent) {
        let matches = self.matching_hooks("post_", event);
        for (hook_name, commands) in matches {
            for command in &commands {
                let (success, _stdout, stderr, duration) = Self::execute_command(command, event);
                self.record_log(&hook_name, command, event, success, duration, &stderr);
                if !success {
                    eprintln!("Warning: hook '{}' failed: {}", hook_name, stderr.trim());
                }
            }
        }
    }

    /// Execute a single hook command, returning (success, stdout, stderr, duration_ms).
    fn execute_command(command: &str, event: &HookEvent) -> (bool, String, String, u64) {
        let event_json = match serde_json::to_string(event) {
            Ok(j) => j,
            Err(e) => {
                return (
                    false,
                    String::new(),
                    format!("failed to serialize hook event: {}", e),
                    0,
                )
            }
        };

        let start = std::time::Instant::now();

        let result = std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(event_json.as_bytes()).ok();
                }
                child.wait_with_output()
            });

        let duration = start.elapsed().as_millis() as u64;

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                (output.status.success(), stdout, stderr, duration)
            }
            Err(e) => (false, String::new(), format!("spawn failed: {}", e), duration),
        }
    }

    fn record_log(
        &self,
        hook_name: &str,
        command: &str,
        event: &HookEvent,
        success: bool,
        duration_ms: u64,
        stderr: &str,
    ) {
        let entry = HookLogEntry {
            timestamp: Utc::now(),
            hook_name: hook_name.to_string(),
            command: command.to_string(),
            event_type: event.event.clone(),
            tension_id: event.tension_id.clone(),
            success,
            duration_ms,
            stderr: if stderr.is_empty() {
                None
            } else {
                Some(stderr.to_string())
            },
        };

        if let Ok(mut log) = self.log.lock() {
            log.push(entry);
            while log.len() > self.max_log_entries {
                log.remove(0);
            }
        }
    }

    /// Get the hook execution log.
    pub fn log_entries(&self) -> Vec<HookLogEntry> {
        self.log.lock().map(|l| l.clone()).unwrap_or_default()
    }

    // === Legacy convenience methods (used by existing CLI pre-hook code) ===

    /// Run pre_mutation hook. Returns false if blocked.
    pub fn pre_mutation(&self, event: &HookEvent) -> bool {
        self.run_pre_hook("mutation", event).unwrap_or(true)
    }

    /// Run post_mutation hook (fire-and-forget).
    pub fn post_mutation(&self, event: &HookEvent) {
        self.run_post_hooks(event);
    }

    /// Run post_resolve hook.
    pub fn post_resolve(&self, event: &HookEvent) {
        self.run_post_hooks(event);
    }

    /// Run post_release hook.
    pub fn post_release(&self, event: &HookEvent) {
        self.run_post_hooks(event);
    }

    /// Run post_create hook.
    pub fn post_create(&self, event: &HookEvent) {
        self.run_post_hooks(event);
    }

    /// Check if any hooks are configured.
    pub fn has_hooks(&self) -> bool {
        !self.hooks.is_empty()
    }
}

// ============================================================================
// Hook Bridge
// ============================================================================

/// Bridges the EventBus to the hook system.
///
/// Subscribes to the EventBus and automatically fires post-hooks for every
/// emitted event. Adding a new Event variant to the enum makes it hookable
/// with zero additional wiring — the bridge reads `event.hook_name()` to
/// derive the post-hook name.
///
/// Pre-hooks are NOT handled by the bridge — they remain at the command level
/// because they need to block before the Store mutation happens.
pub struct HookBridge {
    _subscription: sd_core::events::SubscriptionHandle,
}

impl HookBridge {
    /// Create a bridge that subscribes to the given EventBus and fires
    /// post-hooks using the given HookRunner.
    ///
    /// The runner is wrapped in an Arc so the subscription callback can
    /// reference it safely.
    pub fn new(bus: &EventBus, runner: Arc<HookRunner>) -> Self {
        let subscription = bus.subscribe(move |event: &Event| {
            let hook_event = HookEvent::from_event(event);
            runner.run_post_hooks(&hook_event);
        });
        Self {
            _subscription: subscription,
        }
    }
}

/// Handle returned by `Workspace::open_store_with_hooks()`.
///
/// Keeps the HookBridge subscription alive and provides access to the
/// HookRunner for pre-hook checks at the command level.
pub struct HookBridgeHandle {
    pub _bridge: HookBridge,
    pub runner: Arc<HookRunner>,
}

// ============================================================================
// Shipped Default Hooks
// ============================================================================

/// Available shipped hook scripts.
pub struct ShippedHooks;

impl ShippedHooks {
    /// Flush hook: calls `werk flush` after any mutation.
    pub const FLUSH: &'static str = r#"#!/bin/sh
# Shipped hook: flush tensions.json after every mutation
werk flush 2>/dev/null || true
"#;

    /// Readme tree hook: updates README.md tension tree section.
    pub const README_TREE: &'static str = r#"#!/bin/sh
# Shipped hook: update README.md tension tree after mutations
if [ -f README.md ]; then
    tree_output=$(werk tree 2>/dev/null)
    if [ -n "$tree_output" ]; then
        python3 .githooks/update-readme-tree.py 2>/dev/null || true
    fi
fi
"#;

    /// Auto-stage hook: stages tensions.json and README.md for git.
    pub const AUTO_STAGE: &'static str = r#"#!/bin/sh
# Shipped hook: stage changed files for git after flush
if git rev-parse --git-dir >/dev/null 2>&1; then
    git add tensions.json 2>/dev/null || true
    git add README.md 2>/dev/null || true
fi
"#;

    /// Guard delete hook: blocks deletion of tensions with children.
    pub const GUARD_DELETE: &'static str = r#"#!/bin/sh
# Shipped hook: block deletion of tensions that have children
# Reads HookEvent JSON from stdin
event=$(cat)
event_type=$(echo "$event" | jq -r '.event // empty' 2>/dev/null)
tension_id=$(echo "$event" | jq -r '.tension_id // empty' 2>/dev/null)

if [ "$event_type" = "tension_deleted" ] || [ "$event_type" = "delete" ]; then
    children=$(werk list --parent "$tension_id" --json 2>/dev/null | jq -r 'length' 2>/dev/null)
    if [ "$children" != "0" ] && [ -n "$children" ]; then
        echo "Cannot delete tension with $children active children" >&2
        exit 1
    fi
fi
"#;

    /// Audit log hook: appends to .werk/audit.jsonl.
    pub const AUDIT_LOG: &'static str = r#"#!/bin/sh
# Shipped hook: append every event to .werk/audit.jsonl
cat >> .werk/audit.jsonl
echo "" >> .werk/audit.jsonl
"#;

    /// Install a shipped hook to the workspace hooks directory.
    ///
    /// Returns the path where the hook was written.
    pub fn install(hooks_dir: &std::path::Path, name: &str, content: &str) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(hooks_dir)?;
        let path = hooks_dir.join(format!("{}.sh", name));
        std::fs::write(&path, content)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
        }
        Ok(path)
    }

    /// List available shipped hooks with descriptions.
    pub fn available() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            ("flush", "post_*", "Flush tensions.json after every mutation"),
            ("readme-tree", "post_*", "Update README.md tension tree"),
            ("auto-stage", "post_*", "Stage tensions.json and README.md for git"),
            ("guard-delete", "pre_delete", "Block deletion of tensions with children"),
            ("audit-log", "post_*", "Append events to .werk/audit.jsonl"),
        ]
    }

    /// Get the content of a shipped hook by name.
    pub fn content(name: &str) -> Option<&'static str> {
        match name {
            "flush" => Some(Self::FLUSH),
            "readme-tree" => Some(Self::README_TREE),
            "auto-stage" => Some(Self::AUTO_STAGE),
            "guard-delete" => Some(Self::GUARD_DELETE),
            "audit-log" => Some(Self::AUDIT_LOG),
            _ => None,
        }
    }

    /// Get the default hook event pattern for a shipped hook.
    pub fn default_event(name: &str) -> Option<&'static str> {
        match name {
            "flush" => Some("post_*"),
            "readme-tree" => Some("post_*"),
            "auto-stage" => Some("post_*"),
            "guard-delete" => Some("pre_delete"),
            "audit-log" => Some("post_*"),
            _ => None,
        }
    }
}

// ============================================================================
// Git Integration
// ============================================================================

/// Git hook integration utilities.
pub struct GitHooks;

impl GitHooks {
    /// The content of the generated pre-commit hook.
    pub const PRE_COMMIT: &'static str = r#"#!/bin/sh
# Generated by: werk hooks install --git
# Flushes tension state and stages the result before each commit.

# Flush tension state to JSON
if command -v werk >/dev/null 2>&1; then
    werk flush 2>/dev/null || true
    git add tensions.json 2>/dev/null || true
fi

# Update README tree if the script exists
if [ -f .githooks/update-readme-tree.py ]; then
    python3 .githooks/update-readme-tree.py 2>/dev/null || true
    git add README.md 2>/dev/null || true
fi
"#;

    /// Install git hook integration.
    ///
    /// Sets `core.hooksPath` to `.githooks` and writes the pre-commit hook.
    /// Idempotent — succeeds if already configured.
    pub fn install(repo_root: &std::path::Path) -> Result<(), String> {
        let githooks_dir = repo_root.join(".githooks");
        std::fs::create_dir_all(&githooks_dir)
            .map_err(|e| format!("failed to create .githooks/: {}", e))?;

        let pre_commit_path = githooks_dir.join("pre-commit");
        std::fs::write(&pre_commit_path, Self::PRE_COMMIT)
            .map_err(|e| format!("failed to write pre-commit hook: {}", e))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&pre_commit_path, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("failed to set permissions: {}", e))?;
        }

        // Set core.hooksPath
        let output = std::process::Command::new("git")
            .args(["config", "core.hooksPath", ".githooks"])
            .current_dir(repo_root)
            .output()
            .map_err(|e| format!("failed to run git config: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git config failed: {}", stderr.trim()));
        }

        Ok(())
    }

    /// Check if git hook integration is already installed.
    pub fn is_installed(repo_root: &std::path::Path) -> bool {
        let output = std::process::Command::new("git")
            .args(["config", "--get", "core.hooksPath"])
            .current_dir(repo_root)
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let path = String::from_utf8_lossy(&o.stdout).trim().to_string();
                path == ".githooks"
            }
            _ => false,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_runner_noop() {
        let runner = HookRunner::noop();
        assert!(!runner.has_hooks());
        let event = HookEvent::create("test-id", "test desired", None, None);
        assert!(runner.pre_mutation(&event)); // no hook = allowed
    }

    #[test]
    fn test_hook_event_serialization() {
        let event = HookEvent::mutation(
            "id1",
            "desired1",
            Some("reality1"),
            Some("parent1"),
            "actual",
            Some("old"),
            "new",
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event\":\"mutation\""));
        assert!(json.contains("\"tension_id\":\"id1\""));
        assert!(json.contains("\"category\":\"mutation\""));
    }

    #[test]
    fn test_hook_event_from_event() {
        let event = sd_core::events::EventBuilder::tension_created(
            "01ABC".to_owned(),
            "goal".to_owned(),
            "reality".to_owned(),
            None,
            None,
        );
        let hook_event = HookEvent::from_event(&event);
        assert_eq!(hook_event.event, "tension_created");
        assert_eq!(hook_event.category, "create");
        assert_eq!(hook_event.tension_id, Some("01ABC".to_string()));
        assert_eq!(hook_event.tension_desired, Some("goal".to_string()));
    }

    #[test]
    fn test_hook_event_from_reality_confronted() {
        let event = sd_core::events::EventBuilder::reality_confronted(
            "01ABC".to_owned(),
            "old".to_owned(),
            "new".to_owned(),
        );
        let hook_event = HookEvent::from_event(&event);
        assert_eq!(hook_event.event, "reality_confronted");
        assert_eq!(hook_event.category, "mutation");
        assert_eq!(hook_event.field, Some("actual".to_string()));
        assert_eq!(hook_event.old_value, Some("old".to_string()));
        assert_eq!(hook_event.new_value, Some("new".to_string()));
    }

    #[test]
    fn test_parse_commands_single() {
        let cmds = HookRunner::parse_commands("./hook.sh");
        assert_eq!(cmds, vec!["./hook.sh"]);
    }

    #[test]
    fn test_parse_commands_array() {
        let cmds = HookRunner::parse_commands(r#"["./a.sh", "./b.sh"]"#);
        assert_eq!(cmds, vec!["./a.sh", "./b.sh"]);
    }

    #[test]
    fn test_hook_filter_parent() {
        let filter = HookFilter::parse("parent:42").unwrap();
        let mut event = HookEvent::create("id", "desired", None, Some("parent-42-ulid"));
        assert!(filter.matches(&event));
        event.parent_id = Some("other".to_string());
        assert!(!filter.matches(&event));
    }

    #[test]
    fn test_hook_filter_status() {
        let filter = HookFilter::parse("status:resolved").unwrap();
        let mut event = HookEvent::status_change("id", "desired", None, None, "Resolved");
        assert!(filter.matches(&event));
        event.new_value = Some("Active".to_string());
        assert!(!filter.matches(&event));
    }

    #[test]
    fn test_from_config_backward_compat() {
        let mut config = Config::default();
        config.set("hooks.post_mutation", "./old-hook.sh".to_string());
        let runner = HookRunner::from_config(&config);
        assert!(runner.has_hooks());
        assert_eq!(
            runner.get_commands("post_mutation"),
            Some(vec!["./old-hook.sh".to_string()].as_slice())
        );
    }

    #[test]
    fn test_from_config_new_event_names() {
        let mut config = Config::default();
        config.set(
            "hooks.post_tension_resolved",
            "./notify.sh".to_string(),
        );
        let runner = HookRunner::from_config(&config);
        assert!(runner.has_hooks());
        assert_eq!(
            runner.get_commands("post_tension_resolved"),
            Some(vec!["./notify.sh".to_string()].as_slice())
        );
    }

    #[test]
    fn test_from_config_chains() {
        let mut config = Config::default();
        config.set(
            "hooks.post_tension_resolved",
            r#"["./a.sh", "./b.sh"]"#.to_string(),
        );
        let runner = HookRunner::from_config(&config);
        assert_eq!(
            runner.get_commands("post_tension_resolved"),
            Some(vec!["./a.sh".to_string(), "./b.sh".to_string()].as_slice())
        );
    }

    #[test]
    fn test_matching_hooks_specific() {
        let mut config = Config::default();
        config.set(
            "hooks.post_tension_resolved",
            "./specific.sh".to_string(),
        );
        let runner = HookRunner::from_config(&config);
        let event = HookEvent {
            event: "tension_resolved".to_string(),
            category: "status_change".to_string(),
            timestamp: Utc::now(),
            tension_id: Some("id".to_string()),
            tension_desired: None,
            current_reality: None,
            parent_id: None,
            field: None,
            old_value: None,
            new_value: None,
        };
        let matches = runner.matching_hooks("post_", &event);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].0, "post_tension_resolved");
    }

    #[test]
    fn test_matching_hooks_category_and_specific() {
        let mut config = Config::default();
        config.set("hooks.post_mutation", "./category.sh".to_string());
        config.set(
            "hooks.post_reality_confronted",
            "./specific.sh".to_string(),
        );
        let runner = HookRunner::from_config(&config);
        let event = HookEvent {
            event: "reality_confronted".to_string(),
            category: "mutation".to_string(),
            timestamp: Utc::now(),
            tension_id: Some("id".to_string()),
            tension_desired: None,
            current_reality: None,
            parent_id: None,
            field: None,
            old_value: None,
            new_value: None,
        };
        let matches = runner.matching_hooks("post_", &event);
        // Should match both: category (post_mutation) and specific (post_reality_confronted)
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_matching_hooks_wildcard() {
        let mut config = Config::default();
        config.set("hooks.post_*", "./wildcard.sh".to_string());
        let runner = HookRunner::from_config(&config);
        let event = HookEvent {
            event: "tension_created".to_string(),
            category: "create".to_string(),
            timestamp: Utc::now(),
            tension_id: Some("id".to_string()),
            tension_desired: None,
            current_reality: None,
            parent_id: None,
            field: None,
            old_value: None,
            new_value: None,
        };
        let matches = runner.matching_hooks("post_", &event);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].0, "post_*");
    }

    #[test]
    fn test_matching_hooks_legacy_compat() {
        let mut config = Config::default();
        config.set("hooks.post_resolve", "./legacy.sh".to_string());
        let runner = HookRunner::from_config(&config);
        let event = HookEvent {
            event: "tension_resolved".to_string(),
            category: "status_change".to_string(),
            timestamp: Utc::now(),
            tension_id: Some("id".to_string()),
            tension_desired: None,
            current_reality: None,
            parent_id: None,
            field: None,
            old_value: None,
            new_value: None,
        };
        let matches = runner.matching_hooks("post_", &event);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].0, "post_resolve");
    }

    #[test]
    fn test_shipped_hooks_available() {
        let available = ShippedHooks::available();
        assert_eq!(available.len(), 5);
        assert_eq!(available[0].0, "flush");
    }

    #[test]
    fn test_shipped_hooks_content() {
        assert!(ShippedHooks::content("flush").is_some());
        assert!(ShippedHooks::content("nonexistent").is_none());
    }

    #[test]
    fn test_shipped_hooks_install() {
        let dir = tempfile::TempDir::new().unwrap();
        let hooks_dir = dir.path().join("hooks");
        let path = ShippedHooks::install(&hooks_dir, "flush", ShippedHooks::FLUSH).unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("werk flush"));
    }

    #[test]
    fn test_hook_bridge_fires_on_event() {
        let bus = EventBus::new();
        let runner = Arc::new(HookRunner::noop());
        let _bridge = HookBridge::new(&bus, runner);

        // Just verify it doesn't panic — actual hook execution requires shell
        let event = sd_core::events::EventBuilder::tension_created(
            "01ABC".to_owned(),
            "goal".to_owned(),
            "reality".to_owned(),
            None,
            None,
        );
        bus.emit(&event);
    }

    #[test]
    fn test_event_hook_name() {
        let event = sd_core::events::EventBuilder::tension_created(
            "01ABC".to_owned(),
            "g".to_owned(),
            "r".to_owned(),
            None,
            None,
        );
        assert_eq!(event.hook_name(), "tension_created");
        assert!(event.is_commandable());

        let event = sd_core::events::EventBuilder::urgency_threshold_crossed(
            "01ABC".to_owned(),
            0.4,
            0.6,
            0.5,
            true,
        );
        assert_eq!(event.hook_name(), "urgency_threshold_crossed");
        assert!(!event.is_commandable());
    }
}
