//! Daemon workspace selection persisted in `~/.werk/config.toml`.
//!
//! Two keys:
//! - `daemon.workspace_path` — absolute path to the active workspace root
//!   (parent of `.werk/`). When unset, the daemon falls back to `~/.werk/`.
//! - `daemon.recent_workspaces` — TOML array of paths, most-recent-first,
//!   capped at MAX_RECENTS. Powers the in-tab switcher.

use std::path::{Path, PathBuf};

use crate::Config;
use crate::error::WerkError;
use crate::registry::{self, Registry};

const KEY_ACTIVE: &str = "daemon.workspace_path";
const KEY_RECENTS: &str = "daemon.recent_workspaces";
const MAX_RECENTS: usize = 10;

/// A workspace known to the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceEntry {
    /// Absolute path to the workspace root (parent of `.werk/`).
    pub path: PathBuf,
    /// Display name (basename, with collision suffixes only when needed at render time).
    pub name: String,
    /// True when path resolves to `~/.werk/`.
    pub is_global: bool,
}

impl WorkspaceEntry {
    /// Build an entry from a path. Consults the registry first so that
    /// registered names take precedence over basename-derived ones.
    pub fn from_path(path: PathBuf) -> Self {
        let is_global = is_global_path(&path);
        // Registered names win over any other source of truth.
        if let Ok(reg) = Registry::load() {
            if let Some(found) = reg.find_by_path(&path) {
                return Self {
                    path: found.path,
                    name: found.name,
                    is_global,
                };
            }
        }
        let name = if is_global {
            registry::GLOBAL_NAME.to_string()
        } else {
            path.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| path.display().to_string())
        };
        Self { path, name, is_global }
    }
}

/// Read the active workspace path. Returns the global home if unset.
pub fn active_path() -> Result<PathBuf, WerkError> {
    let cfg = load_global_config()?;
    if let Some(s) = cfg.get(KEY_ACTIVE) {
        let p = PathBuf::from(s);
        if p.join(".werk").exists() {
            return Ok(p);
        }
        // Stale config — fall through to global rather than erroring,
        // so the daemon never wedges on a missing path.
        eprintln!(
            "warning: daemon.workspace_path → {} is not a valid werk workspace; using global",
            p.display()
        );
    }
    home_dir()
}

/// Persist the active workspace and append it to recents.
pub fn set_active(path: &Path) -> Result<(), WerkError> {
    let mut cfg = load_global_config()?;
    cfg.set(KEY_ACTIVE, path.display().to_string());

    let mut recents = read_recents(&cfg);
    let path_string = path.display().to_string();
    recents.retain(|p| p != &path_string);
    recents.insert(0, path_string);
    recents.truncate(MAX_RECENTS);
    cfg.set(KEY_RECENTS, format_array(&recents));

    cfg.save().map_err(|e| WerkError::IoError(e.to_string()))?;
    Ok(())
}

/// Active workspace plus the menu of options to switch between.
///
/// Layering, top to bottom:
/// 1. The active workspace (always first, regardless of source)
/// 2. Every registered workspace (#252's `[workspaces]` table)
/// 3. The global workspace (`~/.werk/`) if not already covered above
/// 4. Recents (`daemon.recent_workspaces`) that aren't already covered
pub fn list() -> Result<(WorkspaceEntry, Vec<WorkspaceEntry>), WerkError> {
    let active = WorkspaceEntry::from_path(active_path()?);
    let cfg = load_global_config()?;

    let mut menu: Vec<WorkspaceEntry> = Vec::new();
    let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

    let push = |entry: WorkspaceEntry, menu: &mut Vec<_>, seen: &mut std::collections::HashSet<_>| {
        if seen.insert(entry.path.clone()) {
            menu.push(entry);
        }
    };

    push(active.clone(), &mut menu, &mut seen);

    if let Ok(reg) = Registry::load() {
        for entry in reg.list() {
            push(
                WorkspaceEntry {
                    path: entry.path.clone(),
                    name: entry.name,
                    is_global: false,
                },
                &mut menu,
                &mut seen,
            );
        }
    }

    let home = home_dir()?;
    push(WorkspaceEntry::from_path(home), &mut menu, &mut seen);

    for raw in read_recents(&cfg) {
        let path = PathBuf::from(raw);
        // Skip recents that no longer point at a real workspace.
        if !path.join(".werk").exists() {
            continue;
        }
        push(WorkspaceEntry::from_path(path), &mut menu, &mut seen);
    }

    Ok((active, menu))
}

fn load_global_config() -> Result<Config, WerkError> {
    let home = home_dir()?;
    let path = home.join(".werk").join("config.toml");
    let mut cfg = Config::load_from_path(&path).map_err(|e| WerkError::IoError(e.to_string()))?;
    // load_from_path leaves path unset when the file doesn't exist; ensure save() works.
    if cfg.path().is_none() {
        // Round-trip via a fresh load that primes the path.
        std::fs::create_dir_all(home.join(".werk"))
            .map_err(|e| WerkError::IoError(format!("create ~/.werk: {e}")))?;
        std::fs::write(&path, "")
            .map_err(|e| WerkError::IoError(format!("touch {}: {e}", path.display())))?;
        cfg = Config::load_from_path(&path).map_err(|e| WerkError::IoError(e.to_string()))?;
    }
    Ok(cfg)
}

fn read_recents(cfg: &Config) -> Vec<String> {
    cfg.get(KEY_RECENTS)
        .map(|s| parse_array(s))
        .unwrap_or_default()
}

/// Parse `["a", "b", "c"]` (TOML array literal stored as a string) into a Vec.
fn parse_array(s: &str) -> Vec<String> {
    let trimmed = s.trim().trim_start_matches('[').trim_end_matches(']');
    if trimmed.trim().is_empty() {
        return Vec::new();
    }
    trimmed
        .split(',')
        .map(|item| {
            item.trim()
                .trim_start_matches('"')
                .trim_end_matches('"')
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn format_array(items: &[String]) -> String {
    let inner: Vec<String> = items
        .iter()
        .map(|s| format!("\"{}\"", s.replace('"', "\\\"")))
        .collect();
    format!("[{}]", inner.join(", "))
}

fn home_dir() -> Result<PathBuf, WerkError> {
    dirs::home_dir().ok_or_else(|| WerkError::IoError("cannot determine home directory".into()))
}

fn is_global_path(path: &Path) -> bool {
    dirs::home_dir().map(|h| path == h).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_array_empty() {
        assert!(parse_array("[]").is_empty());
        assert!(parse_array("[ ]").is_empty());
    }

    #[test]
    fn test_parse_array_one() {
        assert_eq!(parse_array(r#"["/foo"]"#), vec!["/foo"]);
    }

    #[test]
    fn test_parse_array_many() {
        assert_eq!(
            parse_array(r#"["/foo", "/bar/baz", "/q"]"#),
            vec!["/foo", "/bar/baz", "/q"]
        );
    }

    #[test]
    fn test_format_array_roundtrip() {
        let items = vec!["/a".to_string(), "/b/c".to_string()];
        assert_eq!(parse_array(&format_array(&items)), items);
    }
}
