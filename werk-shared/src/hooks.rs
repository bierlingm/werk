//! Hook system for werk — execute shell commands on mutation events.

use crate::config::Config;
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Event payload sent to hooks via stdin as JSON.
#[derive(Debug, Clone, Serialize)]
pub struct HookEvent {
    pub event: String,
    pub timestamp: DateTime<Utc>,
    pub tension_id: String,
    pub tension_desired: String,
    pub current_reality: Option<String>,
    pub parent_id: Option<String>,
    pub field: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

impl HookEvent {
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
            timestamp: Utc::now(),
            tension_id: tension_id.to_string(),
            tension_desired: tension_desired.to_string(),
            current_reality: current_reality.map(|s| s.to_string()),
            parent_id: parent_id.map(|s| s.to_string()),
            field: Some(field.to_string()),
            old_value: old_value.map(|s| s.to_string()),
            new_value: Some(new_value.to_string()),
        }
    }

    pub fn status_change(
        tension_id: &str,
        tension_desired: &str,
        current_reality: Option<&str>,
        parent_id: Option<&str>,
        new_status: &str,
    ) -> Self {
        Self {
            event: new_status.to_lowercase(),
            timestamp: Utc::now(),
            tension_id: tension_id.to_string(),
            tension_desired: tension_desired.to_string(),
            current_reality: current_reality.map(|s| s.to_string()),
            parent_id: parent_id.map(|s| s.to_string()),
            field: Some("status".to_string()),
            old_value: Some("Active".to_string()),
            new_value: Some(new_status.to_string()),
        }
    }

    pub fn create(
        tension_id: &str,
        tension_desired: &str,
        current_reality: Option<&str>,
        parent_id: Option<&str>,
    ) -> Self {
        Self {
            event: "create".to_string(),
            timestamp: Utc::now(),
            tension_id: tension_id.to_string(),
            tension_desired: tension_desired.to_string(),
            current_reality: current_reality.map(|s| s.to_string()),
            parent_id: parent_id.map(|s| s.to_string()),
            field: None,
            old_value: None,
            new_value: None,
        }
    }
}

/// Executes hooks based on configuration.
pub struct HookRunner {
    hooks: std::collections::HashMap<String, String>,
}

impl HookRunner {
    /// Create from a Config. Reads all hooks.* keys.
    pub fn from_config(config: &Config) -> Self {
        let mut hooks = std::collections::HashMap::new();
        for key in [
            "pre_mutation",
            "post_mutation",
            "post_resolve",
            "post_release",
            "post_create",
        ] {
            let config_key = format!("hooks.{}", key);
            if let Some(cmd) = config.get(&config_key) {
                hooks.insert(key.to_string(), cmd.clone());
            }
        }
        Self { hooks }
    }

    /// Create a no-op runner (no hooks configured).
    pub fn noop() -> Self {
        Self {
            hooks: std::collections::HashMap::new(),
        }
    }

    /// Execute a hook. Returns Ok(true) if allowed (or no hook), Ok(false) if pre-hook blocked.
    pub fn run_hook(&self, hook_name: &str, event: &HookEvent) -> Result<bool, String> {
        let command = match self.hooks.get(hook_name) {
            Some(cmd) => cmd,
            None => return Ok(true),
        };

        let event_json = serde_json::to_string(event)
            .map_err(|e| format!("failed to serialize hook event: {}", e))?;

        let result = std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    stdin.write_all(event_json.as_bytes()).ok();
                }
                child.wait_with_output()
            });

        match result {
            Ok(output) => {
                if hook_name.starts_with("pre_") {
                    // Pre-hooks can block: exit 0 = allow, non-zero = block
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        eprintln!("Hook '{}' blocked: {}", hook_name, stderr.trim());
                        Ok(false)
                    } else {
                        Ok(true)
                    }
                } else {
                    // Post-hooks are fire-and-forget
                    Ok(true)
                }
            }
            Err(e) => {
                eprintln!("Warning: hook '{}' failed: {}", hook_name, e);
                Ok(true) // Don't block on hook failure
            }
        }
    }

    /// Convenience: run pre_mutation hook. Returns false if blocked.
    pub fn pre_mutation(&self, event: &HookEvent) -> bool {
        self.run_hook("pre_mutation", event).unwrap_or(true)
    }

    /// Convenience: run post_mutation hook (fire-and-forget).
    pub fn post_mutation(&self, event: &HookEvent) {
        self.run_hook("post_mutation", event).ok();
    }

    /// Convenience: run post_resolve hook.
    pub fn post_resolve(&self, event: &HookEvent) {
        self.run_hook("post_resolve", event).ok();
    }

    /// Convenience: run post_release hook.
    pub fn post_release(&self, event: &HookEvent) {
        self.run_hook("post_release", event).ok();
    }

    /// Convenience: run post_create hook.
    pub fn post_create(&self, event: &HookEvent) {
        self.run_hook("post_create", event).ok();
    }

    /// Check if any hooks are configured.
    pub fn has_hooks(&self) -> bool {
        !self.hooks.is_empty()
    }
}

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
        let event = HookEvent::mutation("id1", "desired1", Some("reality1"), Some("parent1"), "actual", Some("old"), "new");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event\":\"mutation\""));
        assert!(json.contains("\"tension_id\":\"id1\""));
    }
}
