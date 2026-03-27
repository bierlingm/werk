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

    /// JSON output structure for config get.
    #[derive(Serialize)]
    struct ConfigGetResult {
        key: String,
        value: String,
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
            // Validate key is not empty
            if key.is_empty() {
                return Err(WerkError::InvalidInput(
                    "config key cannot be empty".to_string(),
                ));
            }

            // Try to find a local workspace first, fall back to global
            let workspace_result = Workspace::discover();
            let config = match workspace_result {
                Ok(ws) => Config::load(&ws)?,
                Err(_) => {
                    // No local workspace - use global config
                    Config::load_global()?
                }
            };

            // Get the value
            match config.get(key) {
                Some(value) => {
                    if output.is_structured() {
                        let result = ConfigGetResult {
                            key: key.clone(),
                            value: value.clone(),
                        };
                        output
                            .print_structured(&result)
                            .map_err(WerkError::IoError)?;
                    } else {
                        println!("{} = {}", key, value);
                    }
                    Ok(())
                }
                None => Err(WerkError::ConfigError(format!(
                    "config key '{}' not found",
                    key
                ))),
            }
        }
    }
}
