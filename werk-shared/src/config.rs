//! Config file management for werk.
//!
//! Handles reading and writing `.werk/config.toml` with dot-notation keys.

use crate::error::{Result, WerkError};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Configuration value types.
///
/// TOML values can be strings, integers, booleans, arrays, or tables.
/// For simplicity, we store all values as strings in the config file,
/// but we preserve the TOML representation when reading/writing.
pub type ConfigValue = String;

/// In-memory representation of a config file.
///
/// Uses BTreeMap for deterministic ordering (useful for serialization).
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Path to the config file.
    path: Option<PathBuf>,
    /// Key-value pairs (dot-notation keys like "agent.command").
    values: BTreeMap<String, ConfigValue>,
}

impl Config {
    /// Load config from a workspace.
    ///
    /// If the config file doesn't exist, returns an empty config.
    /// If the config file is malformed, returns an error.
    pub fn load(workspace: &crate::workspace::Workspace) -> Result<Self> {
        let path = workspace.config_path();
        Self::load_from_path(&path)
    }

    /// Load config from global workspace.
    ///
    /// Falls back to ~/.werk/config.toml if no local workspace exists.
    pub fn load_global() -> Result<Self> {
        let home = dirs::home_dir()
            .ok_or_else(|| WerkError::IoError("cannot determine home directory".to_string()))?;
        let global_config = home.join(".werk").join("config.toml");
        Self::load_from_path(&global_config)
    }

    /// Load config from a specific path.
    ///
    /// If the file doesn't exist, returns an empty config.
    /// If the file is malformed, returns a descriptive error.
    pub fn load_from_path(path: &std::path::Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self {
                path: Some(path.to_path_buf()),
                values: BTreeMap::new(),
            });
        }

        let content = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                WerkError::PermissionDenied(format!("{}", path.display()))
            } else {
                WerkError::IoError(format!("failed to read config file: {}", e))
            }
        })?;

        // Parse as TOML
        let toml_value: toml::Value = toml::from_str(&content).map_err(|e| {
            WerkError::ConfigError(format!(
                "malformed config file at {}: {}",
                path.display(),
                e
            ))
        })?;

        // Flatten to dot-notation keys
        let values = flatten_toml(&toml_value);

        Ok(Self {
            path: Some(path.to_path_buf()),
            values,
        })
    }

    /// Get a config value by key.
    ///
    /// Key uses dot notation: "agent.command", "display.theme", etc.
    /// Returns None if the key doesn't exist.
    pub fn get(&self, key: &str) -> Option<&ConfigValue> {
        self.values.get(key)
    }

    /// Set a config value.
    ///
    /// Key uses dot notation: "agent.command", "display.theme", etc.
    /// If the key doesn't exist, it's created.
    /// If it exists, the value is overwritten.
    pub fn set(&mut self, key: &str, value: ConfigValue) {
        self.values.insert(key.to_string(), value);
    }

    /// Remove a config key.
    pub fn remove(&mut self, key: &str) {
        self.values.remove(key);
    }

    /// Save the config to disk.
    ///
    /// Creates parent directories if needed.
    /// Creates the file if it doesn't exist.
    pub fn save(&self) -> Result<()> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| WerkError::IoError("config path not set, cannot save".to_string()))?;

        // Create parent directories if needed
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent).map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    WerkError::PermissionDenied(format!("{}", parent.display()))
                } else {
                    WerkError::IoError(format!("failed to create config directory: {}", e))
                }
            })?;
        }

        // Convert back to TOML structure
        let toml_value = unflatten_toml(&self.values);

        // Serialize to TOML
        let content = toml::to_string_pretty(&toml_value)
            .map_err(|e| WerkError::IoError(format!("failed to serialize config: {}", e)))?;

        // Write to file
        std::fs::write(path, content).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                WerkError::PermissionDenied(format!("{}", path.display()))
            } else {
                WerkError::IoError(format!("failed to write config file: {}", e))
            }
        })?;

        Ok(())
    }

    /// Get all values.
    pub fn values(&self) -> &BTreeMap<String, ConfigValue> {
        &self.values
    }

    /// Get the path to the config file.
    pub fn path(&self) -> Option<&std::path::Path> {
        self.path.as_deref()
    }
}

/// Flatten a TOML value into dot-notation keys.
///
/// Example:
///   [agent]
///   command = "echo test"
///   timeout = 30
///
/// Becomes:
///   {"agent.command": "echo test", "agent.timeout": "30"}
fn flatten_toml(value: &toml::Value) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    flatten_toml_recursive(value, "", &mut result);
    result
}

fn flatten_toml_recursive(
    value: &toml::Value,
    prefix: &str,
    result: &mut BTreeMap<String, String>,
) {
    match value {
        toml::Value::Table(table) => {
            for (key, val) in table {
                let new_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                flatten_toml_recursive(val, &new_prefix, result);
            }
        }
        toml::Value::String(s) => {
            result.insert(prefix.to_string(), s.clone());
        }
        toml::Value::Integer(i) => {
            result.insert(prefix.to_string(), i.to_string());
        }
        toml::Value::Float(f) => {
            result.insert(prefix.to_string(), f.to_string());
        }
        toml::Value::Boolean(b) => {
            result.insert(prefix.to_string(), b.to_string());
        }
        toml::Value::Datetime(dt) => {
            result.insert(prefix.to_string(), dt.to_string());
        }
        toml::Value::Array(arr) => {
            // For arrays, we store them as TOML array syntax
            let array_str = arr
                .iter()
                .map(|v| match v {
                    toml::Value::String(s) => format!("\"{}\"", s),
                    toml::Value::Integer(i) => i.to_string(),
                    toml::Value::Float(f) => f.to_string(),
                    toml::Value::Boolean(b) => b.to_string(),
                    toml::Value::Datetime(dt) => dt.to_string(),
                    toml::Value::Array(_) | toml::Value::Table(_) => {
                        // Nested arrays/tables not supported in flatten
                        "[]".to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            result.insert(prefix.to_string(), format!("[{}]", array_str));
        }
    }
}

/// Unflatten dot-notation keys back to a TOML structure.
///
/// Example:
///   {"agent.command": "echo test", "agent.timeout": "30"}
///
/// Becomes:
///   [agent]
///   command = "echo test"
///   timeout = "30"
fn unflatten_toml(values: &BTreeMap<String, String>) -> toml::Value {
    let mut root = toml::map::Map::new();

    for (key, value) in values {
        // Parse the key into segments
        let segments: Vec<&str> = key.split('.').collect();

        // Navigate/create the nested structure
        let mut current = &mut root;
        for (i, segment) in segments.iter().enumerate() {
            if i == segments.len() - 1 {
                // Last segment - set the value
                // Try to parse as appropriate TOML type
                let toml_value = parse_toml_value(value);
                current.insert((*segment).to_string(), toml_value);
            } else {
                // Not the last segment - ensure a table exists
                if !current.contains_key(*segment) {
                    current.insert(
                        (*segment).to_string(),
                        toml::Value::Table(toml::map::Map::new()),
                    );
                }
                // Get mutable reference to the table
                current = current
                    .get_mut(*segment)
                    .and_then(|v| v.as_table_mut())
                    .unwrap(); // ubs:ignore table just inserted above
            }
        }
    }

    toml::Value::Table(root)
}

/// Parse a string value into an appropriate TOML value type.
///
/// Tries in order:
/// 1. Boolean (true/false)
/// 2. Integer
/// 3. Float
/// 4. Array (if starts with [ and ends with ])
/// 5. String (default)
fn parse_toml_value(value: &str) -> toml::Value {
    // Try boolean
    if value == "true" {
        return toml::Value::Boolean(true);
    }
    if value == "false" {
        return toml::Value::Boolean(false);
    }

    // Try integer
    if let Ok(i) = value.parse::<i64>() {
        return toml::Value::Integer(i);
    }

    // Try float
    if let Ok(f) = value.parse::<f64>() {
        return toml::Value::Float(f);
    }

    // Check for array syntax
    let trimmed = value.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        // Parse as TOML array
        if let Ok(parsed) = toml::from_str::<toml::Value>(&format!("arr = {}", trimmed))
            && let Some(arr) = parsed.get("arr")
        {
            return arr.clone();
        }
    }

    // Default to string
    toml::Value::String(value.to_string())
}

// ── Threshold structs ─────────────────────────────────────────────

/// Layer 2 signal thresholds — user-anchored, consumed on standard surfaces.
///
/// These control *when* facts surface (approaching window, stale duration,
/// structural glyph triggers). Configurable via `signals.*` config keys.
/// Three-layer precedence: CLI flag > config > hardcoded default.
#[derive(Debug, Clone)]
pub struct SignalThresholds {
    /// Days until deadline to consider "approaching" (default: 14).
    pub approaching_days: i64,
    /// Urgency value above which a tension is "approaching" (default: 0.5).
    pub approaching_urgency: f64,
    /// Days of inactivity before a tension is "stale" (default: 14).
    pub stale_days: i64,
    /// Betweenness centrality above which HUB glyph fires (default: 0.0001).
    pub hub_centrality: f64,
    /// Descendant count above which REACH glyph fires (default: 5).
    pub reach_descendants: u32,
    /// Desire-reality gap above which DRIFT fires (default: 0.3).
    pub drift_threshold: f64,
}

impl Default for SignalThresholds {
    fn default() -> Self {
        Self {
            approaching_days: 14,
            approaching_urgency: 0.5,
            stale_days: 14,
            hub_centrality: 0.0001,
            reach_descendants: 5,
            drift_threshold: 0.3,
        }
    }
}

impl SignalThresholds {
    /// Load from config, falling back to defaults for missing keys. Level
    /// labels (e.g. `signals.stale.days = "two weeks"`) are resolved to
    /// their underlying numeric value before parsing.
    pub fn load(config: &Config) -> Self {
        use crate::config_registry::resolve_value;
        let defaults = Self::default();
        let read = |key: &str| -> Option<String> { config.get(key).map(|v| resolve_value(key, v)) };
        Self {
            approaching_days: read("signals.approaching.days")
                .and_then(|v| v.parse().ok())
                .unwrap_or(defaults.approaching_days),
            approaching_urgency: read("signals.approaching.urgency")
                .and_then(|v| v.parse().ok())
                .unwrap_or(defaults.approaching_urgency),
            stale_days: read("signals.stale.days")
                .and_then(|v| v.parse().ok())
                .unwrap_or(defaults.stale_days),
            hub_centrality: read("signals.hub.centrality")
                .and_then(|v| v.parse().ok())
                .unwrap_or(defaults.hub_centrality),
            reach_descendants: read("signals.reach.descendants")
                .and_then(|v| v.parse().ok())
                .unwrap_or(defaults.reach_descendants),
            drift_threshold: read("signals.drift.threshold")
                .and_then(|v| v.parse().ok())
                .unwrap_or(defaults.drift_threshold),
        }
    }
}

/// Layer 3 analysis thresholds — instrument-originated, consumed on analytical surfaces.
///
/// These control the projection engine's pattern recognition sensitivity.
/// Configurable via `analysis.projection.*` config keys.
#[derive(Debug, Clone)]
pub struct AnalysisThresholds {
    /// Analysis window for mutation patterns in days (default: 30).
    pub pattern_window_days: i64,
    /// Frequency below this = neglect risk, per day (default: 0.1).
    pub neglect_frequency: f64,
    /// Gap sample variance above this = oscillation risk (default: 0.02).
    pub oscillation_variance: f64,
    /// Gap below this considered "resolved" (default: 0.05).
    pub resolution_gap: f64,
}

impl Default for AnalysisThresholds {
    fn default() -> Self {
        Self {
            pattern_window_days: 30,
            neglect_frequency: 0.1,
            oscillation_variance: 0.02,
            resolution_gap: 0.05,
        }
    }
}

impl AnalysisThresholds {
    /// Load from config, falling back to defaults for missing keys.
    pub fn load(config: &Config) -> Self {
        let defaults = Self::default();
        Self {
            pattern_window_days: config
                .get("analysis.projection.pattern_window_days")
                .and_then(|v| v.parse().ok())
                .unwrap_or(defaults.pattern_window_days),
            neglect_frequency: config
                .get("analysis.projection.neglect_frequency")
                .and_then(|v| v.parse().ok())
                .unwrap_or(defaults.neglect_frequency),
            oscillation_variance: config
                .get("analysis.projection.oscillation_variance")
                .and_then(|v| v.parse().ok())
                .unwrap_or(defaults.oscillation_variance),
            resolution_gap: config
                .get("analysis.projection.resolution_gap")
                .and_then(|v| v.parse().ok())
                .unwrap_or(defaults.resolution_gap),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_load_missing_file() {
        let dir = TempDir::new().unwrap();
        let config = Config::load_from_path(&dir.path().join("config.toml")).unwrap();
        assert!(config.values().is_empty());
    }

    #[test]
    fn test_config_set_and_get() {
        let mut config = Config::default();
        config.set("agent.command", "echo test".to_string());
        assert_eq!(config.get("agent.command"), Some(&"echo test".to_string()));
    }

    #[test]
    fn test_config_get_missing_key() {
        let config = Config::default();
        assert!(config.get("nonexistent").is_none());
    }

    #[test]
    fn test_config_set_overwrites() {
        let mut config = Config::default();
        config.set("agent.command", "echo first".to_string());
        config.set("agent.command", "echo second".to_string());
        assert_eq!(
            config.get("agent.command"),
            Some(&"echo second".to_string())
        );
    }

    #[test]
    fn test_flatten_toml_simple() {
        let toml_str = r#"
command = "echo test"
"#;
        let toml_value: toml::Value = toml::from_str(toml_str).unwrap();
        let flat = flatten_toml(&toml_value);
        assert_eq!(flat.get("command"), Some(&"echo test".to_string()));
    }

    #[test]
    fn test_flatten_toml_nested() {
        let toml_str = r#"
[agent]
command = "echo test"
timeout = 30
"#;
        let toml_value: toml::Value = toml::from_str(toml_str).unwrap();
        let flat = flatten_toml(&toml_value);
        assert_eq!(flat.get("agent.command"), Some(&"echo test".to_string()));
        assert_eq!(flat.get("agent.timeout"), Some(&"30".to_string()));
    }

    #[test]
    fn test_unflatten_toml_simple() {
        let mut values = BTreeMap::new();
        values.insert("command".to_string(), "echo test".to_string());
        let toml_value = unflatten_toml(&values);
        assert_eq!(
            toml_value.get("command"),
            Some(&toml::Value::String("echo test".to_string()))
        );
    }

    #[test]
    fn test_unflatten_toml_nested() {
        let mut values = BTreeMap::new();
        values.insert("agent.command".to_string(), "echo test".to_string());
        values.insert("agent.timeout".to_string(), "30".to_string());
        let toml_value = unflatten_toml(&values);

        let agent = toml_value.get("agent").unwrap().as_table().unwrap();
        assert_eq!(
            agent.get("command"),
            Some(&toml::Value::String("echo test".to_string()))
        );
        assert_eq!(agent.get("timeout"), Some(&toml::Value::Integer(30)));
    }

    #[test]
    fn test_save_and_reload() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        let mut config = Config::load_from_path(&path).unwrap();
        config.set("agent.command", "echo test".to_string());
        config.set("display.theme", "dark".to_string());
        config.save().unwrap();

        // Verify file was created
        assert!(path.exists());

        // Reload and verify
        let reloaded = Config::load_from_path(&path).unwrap();
        assert_eq!(
            reloaded.get("agent.command"),
            Some(&"echo test".to_string())
        );
        assert_eq!(reloaded.get("display.theme"), Some(&"dark".to_string()));
    }

    #[test]
    fn test_malformed_toml_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        std::fs::write(&path, "this is not valid toml [[[[").unwrap();

        let result = Config::load_from_path(&path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, WerkError::ConfigError(_)));
    }

    #[test]
    fn test_parse_toml_value_boolean() {
        let val = parse_toml_value("true");
        assert_eq!(val, toml::Value::Boolean(true));

        let val = parse_toml_value("false");
        assert_eq!(val, toml::Value::Boolean(false));
    }

    #[test]
    fn test_parse_toml_value_integer() {
        let val = parse_toml_value("42");
        assert_eq!(val, toml::Value::Integer(42));
    }

    #[test]
    fn test_parse_toml_value_string() {
        let val = parse_toml_value("hello world");
        assert_eq!(val, toml::Value::String("hello world".to_string()));
    }

    #[test]
    fn test_deeply_nested_key() {
        let mut config = Config::default();
        config.set("level1.level2.level3.value", "deep".to_string());
        assert_eq!(
            config.get("level1.level2.level3.value"),
            Some(&"deep".to_string())
        );
    }

    #[test]
    fn test_roundtrip_preserves_types() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        // Create config with various types
        let mut config = Config::load_from_path(&path).unwrap();
        config.set("string_val", "hello".to_string());
        config.set("int_val", "42".to_string());
        config.set("bool_val", "true".to_string());
        config.set("float_val", "3.14".to_string());
        config.save().unwrap();

        // Verify TOML content has correct types
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("string_val = \"hello\""));
        assert!(content.contains("int_val = 42"));
        assert!(content.contains("bool_val = true"));
        assert!(content.contains("float_val = 3.14"));
    }
}
