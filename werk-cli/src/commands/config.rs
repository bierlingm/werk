//! Config command handler for werk-cli.
//!
//! The Config struct and TOML logic live in werk-shared.
//! This module contains only the CLI command handler.

pub use werk_shared::config::{Config, ConfigValue};
use werk_shared::error::{Result, WerkError};

/// Config command handler.
pub fn cmd_config(
    output: &crate::output::Output,
    command: Option<&super::ConfigCommand>,
) -> Result<()> {
    use werk_shared::workspace::Workspace;
    use serde::Serialize;

    /// JSON output structure for config set.
    #[derive(Serialize)]
    struct ConfigSetResult {
        key: String,
        value: String,
        path: String,
    }

    /// JSON output structure for config get (single key).
    #[derive(Serialize)]
    struct ConfigGetResult {
        key: String,
        value: String,
    }

    /// JSON output structure for config get (list all).
    #[derive(Serialize)]
    struct ConfigListResult {
        path: String,
        values: std::collections::BTreeMap<String, String>,
    }

    /// JSON output structure for config path.
    #[derive(Serialize)]
    struct ConfigPathResult {
        local_path: Option<String>,
        local_exists: bool,
        global_path: String,
        global_exists: bool,
        active: String,
    }

    // If no subcommand, show usage hint
    let command = match command {
        Some(cmd) => cmd,
        None => {
            return Err(WerkError::InvalidInput(
                "config requires a subcommand. Use 'werk config get <key>' or 'werk config set <key> <value>'".to_string(),
            ));
        }
    };

    match command {
        super::ConfigCommand::Set { key, value } => {
            // Validate key is not empty
            if key.is_empty() {
                return Err(WerkError::InvalidInput(
                    "config key cannot be empty".to_string(),
                ));
            }

            // Try to find a local workspace first, fall back to global
            let workspace_result = Workspace::discover();
            let mut config = match workspace_result {
                Ok(ws) => Config::load(&ws)?,
                Err(_) => {
                    // No local workspace - use global config
                    Config::load_global()?
                }
            };

            // Set the value
            config.set(key, value.clone());

            // Save
            config.save()?;

            // Output
            let path = config
                .path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            if output.is_structured() {
                let result = ConfigSetResult { key: key.clone(), value: value.clone(), path };
                output
                    .print_structured(&result)
                    .map_err(WerkError::IoError)?;
            } else {
                output
                    .success(&format!("Set {} = {}", key, value))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }

            Ok(())
        }
        super::ConfigCommand::Get { key } => {
            // Try to find a local workspace first, fall back to global
            let workspace_result = Workspace::discover();
            let config = match workspace_result {
                Ok(ws) => Config::load(&ws)?,
                Err(_) => Config::load_global()?,
            };

            match key {
                Some(k) if !k.is_empty() => {
                    // Single key lookup
                    match config.get(&k) {
                        Some(value) => {
                            if output.is_structured() {
                                let result = ConfigGetResult {
                                    key: k.clone(),
                                    value: value.clone(),
                                };
                                output.print_structured(&result).map_err(WerkError::IoError)?;
                            } else {
                                println!("{} = {}", k, value);
                            }
                            Ok(())
                        }
                        None => Err(WerkError::ConfigError(format!(
                            "config key '{}' not found", k
                        ))),
                    }
                }
                _ => {
                    // No key: list all config values
                    let path_str = config.path()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    if output.is_structured() {
                        let result = ConfigListResult {
                            path: path_str,
                            values: config.values().clone(),
                        };
                        output.print_structured(&result).map_err(WerkError::IoError)?;
                    } else {
                        println!("# {}", path_str);
                        if config.values().is_empty() {
                            println!("  (no values set)");
                        } else {
                            for (k, v) in config.values() {
                                println!("{} = {}", k, v);
                            }
                        }
                    }
                    Ok(())
                }
            }
        }
        super::ConfigCommand::Path => {
            let home = dirs::home_dir()
                .ok_or_else(|| WerkError::IoError("cannot determine home directory".to_string()))?;
            let global_path = home.join(".werk").join("config.toml");
            let global_exists = global_path.exists();

            let workspace_result = Workspace::discover();
            let (local_path, local_exists) = match &workspace_result {
                Ok(ws) => {
                    let p = ws.config_path();
                    let exists = p.exists();
                    (Some(p.display().to_string()), exists)
                }
                Err(_) => (None, false),
            };

            let active = if local_exists {
                "local"
            } else if global_exists {
                "global"
            } else {
                "none"
            };

            if output.is_structured() {
                let result = ConfigPathResult {
                    local_path,
                    local_exists,
                    global_path: global_path.display().to_string(),
                    global_exists,
                    active: active.to_string(),
                };
                output.print_structured(&result).map_err(WerkError::IoError)?;
            } else {
                if let Some(ref lp) = local_path {
                    println!("Local:  {}  {}", lp, if local_exists { "(active)" } else { "(not found)" });
                }
                println!("Global: {}  {}", global_path.display(), if global_exists { if local_path.is_none() || !local_exists { "(active)" } else { "(exists)" } } else { "(not found)" });
            }

            Ok(())
        }
    }
}
