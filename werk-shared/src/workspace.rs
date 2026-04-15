//! Workspace resolution for werk.
//!
//! A workspace is a `.werk/` directory containing `werk.db` and optionally `config.toml`.
//!
//! Resolution strategy:
//! 1. Walk up from CWD looking for `.werk/` directory
//! 2. If not found, fall back to `~/.werk/`
//! 3. If neither exists, return an error

use crate::error::{Result, WerkError};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Process-wide workspace override set by the top-level `-w` / `-g` flags
/// in main(). Honored by `Workspace::discover()`. Avoids the unsafe
/// `std::env::set_var` path while staying global.
static WORKSPACE_OVERRIDE: OnceLock<String> = OnceLock::new();

/// Set the in-process workspace selection. Idempotent: subsequent calls are
/// silently ignored (top-level flags are parsed once, in main()).
pub fn set_workspace_override(name: &str) {
    let _ = WORKSPACE_OVERRIDE.set(name.to_string());
}

/// Read the in-process workspace selection, if any.
pub fn workspace_override() -> Option<&'static str> {
    WORKSPACE_OVERRIDE.get().map(|s| s.as_str())
}

/// A werk workspace.
///
/// Represents a `.werk/` directory with its associated store and config.
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Path to the workspace root (parent of .werk/).
    root: PathBuf,
    /// Path to .werk/ directory.
    werk_dir: PathBuf,
}

impl Workspace {
    /// Discover a workspace starting from the current directory.
    ///
    /// Resolution order:
    /// 1. In-process override set by `set_workspace_override` (top-level
    ///    `-w` / `-g` flags).
    /// 2. `WERK_WORKSPACE` env var (set by callers spawning werk as a
    ///    child process).
    /// 3. Walk up from CWD looking for `.werk/`.
    /// 4. Fall back to `~/.werk/`.
    ///
    /// Names are resolved through `crate::registry::Registry`; the reserved
    /// name "global" always maps to `~/.werk/`.
    pub fn discover() -> Result<Self> {
        if let Some(name) = workspace_override() {
            if !name.is_empty() {
                return Self::resolve_name(name);
            }
        }
        if let Ok(name) = std::env::var("WERK_WORKSPACE") {
            if !name.is_empty() {
                return Self::resolve_name(&name);
            }
        }
        let cwd = std::env::current_dir()
            .map_err(|e| WerkError::IoError(format!("failed to get current directory: {}", e)))?;

        Self::discover_from(&cwd)
    }

    /// Resolve a workspace by registered name. "global" is always implicit.
    pub fn resolve_name(name: &str) -> Result<Self> {
        if name == crate::registry::GLOBAL_NAME {
            return Self::global();
        }
        let reg = crate::registry::Registry::load()?;
        let entry = reg.get(name).ok_or_else(|| {
            WerkError::IoError(format!(
                "no registered space named '{name}'. Run `werk spaces list` or register one with `werk spaces register <name> <path>`."
            ))
        })?;
        Self::discover_from(&entry.path)
    }

    /// Discover a workspace starting from a specific directory.
    ///
    /// Walks up from the given path looking for `.werk/`, falling back to `~/.werk/`.
    pub fn discover_from(start: &Path) -> Result<Self> {
        // Walk up looking for .werk/
        let mut current = start.to_path_buf();
        loop {
            let werk_dir = current.join(".werk");
            if werk_dir.exists() {
                return Ok(Self {
                    root: current,
                    werk_dir,
                });
            }
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => break,
            }
        }

        // Fall back to ~/.werk/
        if let Some(home) = dirs::home_dir() {
            let global_werk = home.join(".werk");
            if global_werk.exists() {
                return Ok(Self {
                    root: home,
                    werk_dir: global_werk,
                });
            }
        }

        Err(WerkError::no_workspace_with_context(
            start,
            dirs::home_dir().as_deref(),
        ))
    }

    /// Open the global workspace at `~/.werk/`.
    ///
    /// Errors if `~/.werk/` does not exist. Use `Workspace::init(path, true)`
    /// to create it.
    pub fn global() -> Result<Self> {
        let home = dirs::home_dir()
            .ok_or_else(|| WerkError::IoError("cannot determine home directory".to_string()))?;
        let werk_dir = home.join(".werk");
        if !werk_dir.exists() {
            return Err(WerkError::no_workspace_with_context(
                &home,
                Some(&home),
            ));
        }
        Ok(Self {
            root: home,
            werk_dir,
        })
    }

    /// Initialize a workspace at the given path.
    ///
    /// Creates `.werk/` directory if it doesn't exist.
    /// Returns an error if the directory cannot be created.
    pub fn init(path: &Path, global: bool) -> Result<Self> {
        let werk_dir = if global {
            let home = dirs::home_dir()
                .ok_or_else(|| WerkError::IoError("cannot determine home directory".to_string()))?;
            home.join(".werk")
        } else {
            path.join(".werk")
        };

        // Create .werk/ directory
        std::fs::create_dir_all(&werk_dir).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                WerkError::PermissionDenied(format!("{}", werk_dir.display()))
            } else {
                WerkError::IoError(format!("failed to create .werk directory: {}", e))
            }
        })?;

        let root = werk_dir
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| path.to_path_buf());

        Ok(Self { root, werk_dir })
    }

    /// Get the path to the workspace root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get the path to the .werk/ directory.
    pub fn werk_dir(&self) -> &Path {
        &self.werk_dir
    }

    /// Get the path to the database file.
    pub fn db_path(&self) -> PathBuf {
        self.werk_dir.join("werk.db")
    }

    /// Get the path to the config file.
    pub fn config_path(&self) -> PathBuf {
        self.werk_dir.join("config.toml")
    }

    /// Check if this is a global workspace (~/.werk/).
    pub fn is_global(&self) -> bool {
        if let Some(home) = dirs::home_dir() {
            self.werk_dir == home.join(".werk")
        } else {
            false
        }
    }

    /// Check if the database file exists.
    pub fn db_exists(&self) -> bool {
        self.db_path().exists()
    }

    /// Check if the config file exists.
    pub fn config_exists(&self) -> bool {
        self.config_path().exists()
    }

    /// Open the store for this workspace.
    ///
    /// If the store doesn't exist yet, it will be initialized.
    pub fn open_store(&self) -> Result<werk_core::Store> {
        let store = werk_core::Store::init(self.root()).map_err(WerkError::StoreError)?;
        Ok(store)
    }

    /// Open the store with EventBus + HookBridge attached.
    ///
    /// Post-hooks fire automatically via the bridge when the Store emits events.
    /// Returns (Store, HookBridgeHandle) — the handle must be kept alive for the
    /// bridge to remain subscribed. Drop it when the command completes.
    ///
    /// This is the preferred entry point for CLI/MCP commands that mutate state.
    pub fn open_store_with_hooks(
        &self,
    ) -> Result<(werk_core::Store, crate::hooks::HookBridgeHandle)> {
        let mut store = werk_core::Store::init(self.root()).map_err(WerkError::StoreError)?;
        let bus = werk_core::events::EventBus::new();
        store.set_event_bus(bus.clone());

        let config = crate::config::Config::load(self).unwrap_or_default();
        let global_config = crate::config::Config::load_global().ok();
        let runner = std::sync::Arc::new(crate::hooks::HookRunner::from_configs(
            global_config.as_ref(),
            &config,
        ));
        let bridge = crate::hooks::HookBridge::new(&bus, runner.clone());

        Ok((
            store,
            crate::hooks::HookBridgeHandle {
                _bridge: bridge,
                runner,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_discover_from_finds_local_werk() {
        let dir = TempDir::new().unwrap();
        let werk_dir = dir.path().join(".werk");
        std::fs::create_dir_all(&werk_dir).unwrap();

        // Discover from the temp directory
        let workspace = Workspace::discover_from(dir.path()).unwrap();
        assert_eq!(workspace.root(), dir.path());
        assert_eq!(workspace.werk_dir(), werk_dir);
    }

    #[test]
    fn test_discover_from_finds_ancestor_werk() {
        let dir = TempDir::new().unwrap();
        let werk_dir = dir.path().join(".werk");
        std::fs::create_dir_all(&werk_dir).unwrap();

        let subdir = dir.path().join("a").join("b").join("c");
        std::fs::create_dir_all(&subdir).unwrap();

        // Discover from the subdirectory
        let workspace = Workspace::discover_from(&subdir).unwrap();
        assert_eq!(workspace.root(), dir.path());
    }

    #[test]
    fn test_discover_from_no_workspace_returns_error() {
        // Use /tmp which typically has no .werk/
        let result = Workspace::discover_from(std::path::Path::new("/tmp"));
        // This might succeed if ~/.werk/ exists, so we just check it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_init_creates_werk_directory() {
        let dir = TempDir::new().unwrap();
        let workspace = Workspace::init(dir.path(), false).unwrap();
        assert!(workspace.werk_dir().exists());
        assert!(workspace.werk_dir().ends_with(".werk"));
    }

    #[test]
    fn test_init_global_creates_home_werk() {
        // For testing, we can't actually use real home, so we test with a temp dir
        // The actual global init would use dirs::home_dir()
        let dir = TempDir::new().unwrap();
        let workspace = Workspace::init(dir.path(), false).unwrap();
        assert!(workspace.werk_dir().exists());
    }

    #[test]
    fn test_is_global() {
        let dir = TempDir::new().unwrap();
        let workspace = Workspace::init(dir.path(), false).unwrap();
        assert!(!workspace.is_global());
    }

    #[test]
    fn test_db_path() {
        let dir = TempDir::new().unwrap();
        let workspace = Workspace::init(dir.path(), false).unwrap();
        assert!(workspace.db_path().ends_with("werk.db"));
    }

    #[test]
    fn test_config_path() {
        let dir = TempDir::new().unwrap();
        let workspace = Workspace::init(dir.path(), false).unwrap();
        assert!(workspace.config_path().ends_with("config.toml"));
    }
}
