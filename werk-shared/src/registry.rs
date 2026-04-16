//! Workspace registry — the `[workspaces]` section of `~/.werk/config.toml`.
//!
//! Maps human-chosen names to absolute workspace paths:
//!
//! ```toml
//! [workspaces.werk]
//! path = "/Users/me/code/werk"
//!
//! [workspaces.journal]
//! path = "/Users/me/journal"
//! ```
//!
//! The name "global" is reserved and always resolves to `~/.werk/`. Registry
//! entries persist across sessions and survive `werk daemon` restarts; the
//! daemon's `recent_workspaces` list is a separate, ephemeral concern.
//!
//! This module is consumed by:
//! - `werk spaces {list, register, unregister, rename, scan}` (#251)
//! - `werk -w <name>` flag resolution (#250)
//! - the `/api/workspaces` endpoint and the werk-tab switcher (#276)

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::Config;
use crate::error::{Result, WerkError};

/// Reserved name that always resolves to `~/.werk/`.
pub const GLOBAL_NAME: &str = "global";

/// Top-level config key prefix for registry entries.
const PREFIX: &str = "workspaces.";

/// A workspace registered under a chosen name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredWorkspace {
    pub name: String,
    pub path: PathBuf,
}

/// In-memory view of the `[workspaces]` config table.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    entries: BTreeMap<String, PathBuf>,
}

impl Registry {
    /// Load the registry from `~/.werk/config.toml`.
    ///
    /// Missing config or missing section yields an empty registry; never errors
    /// for the common "first run" case.
    pub fn load() -> Result<Self> {
        let cfg = load_global_config()?;
        Ok(Self::from_config(&cfg))
    }

    /// Build a Registry view from an already-loaded Config.
    pub fn from_config(cfg: &Config) -> Self {
        let mut entries = BTreeMap::new();
        for (key, value) in cfg.values() {
            // We're looking for keys like "workspaces.<name>.path".
            let Some(rest) = key.strip_prefix(PREFIX) else {
                continue;
            };
            let Some((name, field)) = rest.split_once('.') else {
                continue;
            };
            if field != "path" {
                continue;
            }
            if name == GLOBAL_NAME {
                // Ignore any explicit "global" entry — it's reserved.
                continue;
            }
            entries.insert(name.to_string(), PathBuf::from(value));
        }
        Self { entries }
    }

    /// Persist the registry to `~/.werk/config.toml`, replacing every
    /// `workspaces.*` key. Other keys are left untouched.
    pub fn save(&self) -> Result<()> {
        let mut cfg = load_global_config()?;
        // Drop every existing workspaces.* key, then write fresh ones.
        let stale: Vec<String> = cfg
            .values()
            .keys()
            .filter(|k| k.starts_with(PREFIX))
            .cloned()
            .collect();
        for key in stale {
            cfg.remove(&key);
        }
        for (name, path) in &self.entries {
            cfg.set(
                &format!("{PREFIX}{name}.path"),
                path.display().to_string(),
            );
        }
        cfg.save().map_err(|e| WerkError::IoError(e.to_string()))
    }

    /// All registered workspaces. Reserved "global" is not included; query
    /// `resolve("global")` if you want it.
    pub fn list(&self) -> Vec<RegisteredWorkspace> {
        self.entries
            .iter()
            .map(|(name, path)| RegisteredWorkspace {
                name: name.clone(),
                path: path.clone(),
            })
            .collect()
    }

    /// Look up a workspace by name. Resolves "global" to `~/.werk/` even when
    /// the caller hasn't loaded the registry yet.
    pub fn get(&self, name: &str) -> Option<RegisteredWorkspace> {
        if name == GLOBAL_NAME {
            return global_entry().ok();
        }
        self.entries.get(name).map(|path| RegisteredWorkspace {
            name: name.to_string(),
            path: path.clone(),
        })
    }

    /// Find a registered workspace by absolute path.
    pub fn find_by_path(&self, path: &Path) -> Option<RegisteredWorkspace> {
        if let Ok(global) = global_entry() {
            if path == global.path {
                return Some(global);
            }
        }
        self.entries
            .iter()
            .find(|(_, p)| p.as_path() == path)
            .map(|(name, path)| RegisteredWorkspace {
                name: name.clone(),
                path: path.clone(),
            })
    }

    /// Register a workspace. Returns the canonical entry that was stored.
    ///
    /// Errors when the name is reserved, malformed, or already in use, and
    /// when the path doesn't contain a `.werk/` directory.
    pub fn register(&mut self, name: &str, path: &Path) -> Result<RegisteredWorkspace> {
        validate_name(name)?;
        let abs = canonicalize_workspace(path)?;
        if self.entries.contains_key(name) {
            return Err(WerkError::IoError(format!(
                "name '{name}' is already registered (use `werk spaces rename` or `unregister`)"
            )));
        }
        if let Some(existing) = self.find_by_path(&abs) {
            return Err(WerkError::IoError(format!(
                "{} is already registered as '{}'",
                abs.display(),
                existing.name
            )));
        }
        self.entries.insert(name.to_string(), abs.clone());
        Ok(RegisteredWorkspace {
            name: name.to_string(),
            path: abs,
        })
    }

    /// Remove a registration. Returns true if something was removed.
    pub fn unregister(&mut self, name: &str) -> Result<bool> {
        if name == GLOBAL_NAME {
            return Err(WerkError::IoError(
                "cannot unregister the reserved 'global' name".into(),
            ));
        }
        Ok(self.entries.remove(name).is_some())
    }

    /// Rename a registration. Errors when the source doesn't exist or the
    /// destination is already taken.
    pub fn rename(&mut self, old: &str, new: &str) -> Result<()> {
        if old == GLOBAL_NAME || new == GLOBAL_NAME {
            return Err(WerkError::IoError(
                "cannot rename to or from reserved 'global' name".into(),
            ));
        }
        validate_name(new)?;
        if !self.entries.contains_key(old) {
            return Err(WerkError::IoError(format!("no workspace named '{old}'")));
        }
        if self.entries.contains_key(new) {
            return Err(WerkError::IoError(format!(
                "name '{new}' is already in use"
            )));
        }
        let path = self.entries.remove(old).unwrap(); // ubs:ignore — checked above
        self.entries.insert(new.to_string(), path);
        Ok(())
    }
}

/// Return the always-implicit global workspace entry.
pub fn global_entry() -> Result<RegisteredWorkspace> {
    let home = dirs::home_dir()
        .ok_or_else(|| WerkError::IoError("cannot determine home directory".into()))?;
    Ok(RegisteredWorkspace {
        name: GLOBAL_NAME.to_string(),
        path: home,
    })
}

fn canonicalize_workspace(path: &Path) -> Result<PathBuf> {
    let abs = std::fs::canonicalize(path).map_err(|e| {
        WerkError::IoError(format!("cannot resolve {}: {e}", path.display()))
    })?;
    if !abs.join(".werk").exists() {
        return Err(WerkError::IoError(format!(
            "{} is not a werk workspace (no .werk/ inside)",
            abs.display()
        )));
    }
    Ok(abs)
}

/// Minimum length for a space name. Single-char names would collide with
/// address sigils (`g:`, `s:`), so we require ≥2 characters — every
/// registered name is addressable via the `name:code` cross-space syntax.
pub const MIN_NAME_LEN: usize = 2;

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(WerkError::IoError("name cannot be empty".into()));
    }
    if name.len() < MIN_NAME_LEN {
        return Err(WerkError::IoError(format!(
            "name '{name}' is too short; minimum {MIN_NAME_LEN} characters (single-char names collide with address sigils)"
        )));
    }
    if name == GLOBAL_NAME {
        return Err(WerkError::IoError(format!(
            "'{GLOBAL_NAME}' is a reserved name"
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(WerkError::IoError(format!(
            "name '{name}' contains invalid characters; use [a-z0-9_-] only"
        )));
    }
    if name.starts_with('-') || name.starts_with('_') {
        return Err(WerkError::IoError(format!(
            "name '{name}' cannot start with '-' or '_'"
        )));
    }
    Ok(())
}

fn load_global_config() -> Result<Config> {
    let home = dirs::home_dir()
        .ok_or_else(|| WerkError::IoError("cannot determine home directory".into()))?;
    let werk_dir = home.join(".werk");
    let path = werk_dir.join("config.toml");
    if !path.exists() {
        std::fs::create_dir_all(&werk_dir).map_err(|e| {
            WerkError::IoError(format!("create {}: {e}", werk_dir.display()))
        })?;
        std::fs::write(&path, "")
            .map_err(|e| WerkError::IoError(format!("touch {}: {e}", path.display())))?;
    }
    Config::load_from_path(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_ok() {
        assert!(validate_name("werk").is_ok());
        assert!(validate_name("desk-werk").is_ok());
        assert!(validate_name("a1_b2").is_ok());
        assert!(validate_name("ab").is_ok()); // minimum valid length
    }

    #[test]
    fn test_validate_name_empty() {
        assert!(validate_name("").is_err());
    }

    #[test]
    fn test_validate_name_too_short() {
        assert!(validate_name("g").is_err()); // would collide with g: sigil
        assert!(validate_name("s").is_err()); // would collide with s: sigil
        assert!(validate_name("x").is_err()); // any single char
    }

    #[test]
    fn test_validate_name_global_reserved() {
        assert!(validate_name("global").is_err());
    }

    #[test]
    fn test_validate_name_invalid_chars() {
        assert!(validate_name("foo.bar").is_err());
        assert!(validate_name("foo bar").is_err());
        assert!(validate_name("foo/bar").is_err());
    }

    #[test]
    fn test_validate_name_leading_special() {
        assert!(validate_name("-foo").is_err());
        assert!(validate_name("_foo").is_err());
    }

    #[test]
    fn test_from_config_picks_workspace_keys() {
        let mut cfg = Config::default();
        cfg.set("workspaces.werk.path", "/abs/werk".into());
        cfg.set("workspaces.journal.path", "/abs/journal".into());
        cfg.set("daemon.workspace_path", "/abs/werk".into()); // ignored
        cfg.set("workspaces.global.path", "/should/be/ignored".into()); // reserved

        let reg = Registry::from_config(&cfg);
        let names: Vec<_> = reg.list().into_iter().map(|w| w.name).collect();
        assert_eq!(names, vec!["journal", "werk"]);
    }
}
