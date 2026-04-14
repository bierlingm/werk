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

    // No subcommand → list all values (equivalent to `werk config get`).
    let fallback;
    let command = match command {
        Some(cmd) => cmd,
        None => {
            fallback = super::ConfigCommand::Get { key: None };
            &fallback
        }
    };

    match command {
        super::ConfigCommand::Set { key, value } => {
            if key.is_empty() {
                return Err(WerkError::InvalidInput(
                    "config key cannot be empty".to_string(),
                ));
            }

            // Typed validation for registry keys — rejects "abc" for an Int
            // key and canonicalizes forms like "YES" → "true" for Bool.
            let entry = werk_shared::config_registry::lookup(key);
            let canonical_value = match entry {
                Some(entry) => werk_shared::config_registry::validate(entry.kind, value)
                    .map_err(|e| WerkError::InvalidInput(format!("{key}: {e}")))?,
                None => value.clone(),
            };

            let workspace_result = Workspace::discover();
            let mut config = match workspace_result {
                Ok(ws) => Config::load(&ws)?,
                Err(_) => Config::load_global()?,
            };

            // Synthetic keys cascade to other keys and are not stored themselves.
            // analysis.sensitivity = "sharp" writes four analysis.projection.* keys.
            if entry.map(|e| e.kind.is_synthetic()).unwrap_or(false) {
                let Some(bundle) = werk_shared::config_registry::cascade_for(key, &canonical_value) else {
                    return Err(WerkError::InvalidInput(format!(
                        "no cascade defined for {key} = {canonical_value}"
                    )));
                };
                for (k, v) in bundle {
                    config.set(k, (*v).to_string());
                }
                config.save()?;

                if output.is_structured() {
                    output
                        .print_structured(&serde_json::json!({
                            "key": key,
                            "value": canonical_value,
                            "synthetic": true,
                            "cascaded": bundle.iter().map(|(k, v)| serde_json::json!({
                                "key": k, "value": v
                            })).collect::<Vec<_>>(),
                        }))
                        .map_err(WerkError::IoError)?;
                } else {
                    output
                        .success(&format!("Set {key} = {canonical_value} · {} key{} updated", bundle.len(), if bundle.len() == 1 { "" } else { "s" }))
                        .map_err(|e| WerkError::IoError(e.to_string()))?;
                    let palette = output.palette();
                    for (k, v) in bundle {
                        println!("  {} {k} = {v}", palette.chrome("·"));
                    }
                }
                return Ok(());
            }

            let old_value = config.get(key).cloned();
            config.set(key, canonical_value.clone());
            config.save()?;

            let path = config
                .path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            if output.is_structured() {
                let result = ConfigSetResult {
                    key: key.clone(),
                    value: canonical_value.clone(),
                    path,
                };
                output
                    .print_structured(&result)
                    .map_err(WerkError::IoError)?;
            } else {
                // Transition line: "· key: old → new" when changing, "✓ Set key = new" on first set.
                let palette = output.palette();
                match old_value {
                    Some(old) if old != canonical_value => {
                        println!(
                            "{} {}: {} {} {}",
                            palette.warning("·"),
                            key,
                            palette.chrome(&old),
                            palette.chrome("→"),
                            canonical_value,
                        );
                    }
                    Some(_) => {
                        println!("  {key}: {canonical_value} (unchanged)");
                    }
                    None => {
                        output
                            .success(&format!("Set {key} = {canonical_value}"))
                            .map_err(|e| WerkError::IoError(e.to_string()))?;
                    }
                }
            }

            Ok(())
        }
        super::ConfigCommand::Unset { key } => {
            if key.is_empty() {
                return Err(WerkError::InvalidInput(
                    "config key cannot be empty".to_string(),
                ));
            }

            let workspace_result = Workspace::discover();
            let mut config = match workspace_result {
                Ok(ws) => Config::load(&ws)?,
                Err(_) => Config::load_global()?,
            };

            let old_value = config.get(key).cloned();
            if old_value.is_none() {
                return Err(WerkError::ConfigError(format!(
                    "config key '{key}' is not set"
                )));
            }

            config.remove(key);
            config.save()?;

            let palette = output.palette();
            if output.is_structured() {
                #[derive(serde::Serialize)]
                struct UnsetResult<'a> {
                    key: &'a str,
                    removed: String,
                }
                output
                    .print_structured(&UnsetResult {
                        key,
                        removed: old_value.unwrap_or_default(),
                    })
                    .map_err(WerkError::IoError)?;
            } else {
                let old = old_value.unwrap_or_default();
                let default_hint = werk_shared::config_registry::lookup(key)
                    .map(|e| format!("  (default {})", e.default))
                    .unwrap_or_default();
                println!(
                    "{} unset {}: {}{}",
                    palette.warning("·"),
                    key,
                    palette.chrome(&old),
                    palette.chrome(&default_hint),
                );
            }

            Ok(())
        }
        super::ConfigCommand::Reset { target } => {
            use werk_shared::config_registry::{keys_with_prefix, lookup as lookup_key, REGISTRY};

            let workspace_result = Workspace::discover();
            let mut config = match workspace_result {
                Ok(ws) => Config::load(&ws)?,
                Err(_) => Config::load_global()?,
            };

            // Target is either: None (all), an exact registry key, or a
            // dotted prefix matching one or more registry keys.
            let keys_to_reset: Vec<&'static str> = match target {
                None => REGISTRY.iter().map(|k| k.key).collect(),
                Some(t) => {
                    if let Some(entry) = lookup_key(t) {
                        vec![entry.key]
                    } else {
                        let matched: Vec<&'static str> = keys_with_prefix(t).map(|k| k.key).collect();
                        if matched.is_empty() {
                            return Err(WerkError::InvalidInput(format!(
                                "'{t}' is not a registry key or a known prefix. \
                                 Try `werk config` to see all keys, or \
                                 `werk config unset <key>` for hooks/unknowns."
                            )));
                        }
                        matched
                    }
                }
            };

            // Collect what will change so we can report it.
            let mut cleared: Vec<(String, String)> = Vec::new();
            for key in &keys_to_reset {
                if let Some(old) = config.get(key) {
                    cleared.push(((*key).to_string(), old.clone()));
                    config.remove(key);
                }
            }

            if cleared.is_empty() {
                let scope = target.as_deref().unwrap_or("(all registry keys)");
                if output.is_structured() {
                    #[derive(serde::Serialize)]
                    struct ResetResult<'a> {
                        scope: &'a str,
                        cleared: Vec<(String, String)>,
                    }
                    output
                        .print_structured(&ResetResult { scope, cleared })
                        .map_err(WerkError::IoError)?;
                } else {
                    println!("  {scope}: already at defaults");
                }
                return Ok(());
            }

            config.save()?;

            if output.is_structured() {
                #[derive(serde::Serialize)]
                struct ResetResult<'a> {
                    scope: &'a str,
                    cleared: Vec<(String, String)>,
                }
                let scope = target.as_deref().unwrap_or("all");
                output
                    .print_structured(&ResetResult { scope, cleared })
                    .map_err(WerkError::IoError)?;
            } else {
                let palette = output.palette();
                let label = target
                    .as_deref()
                    .unwrap_or("all registry keys");
                output
                    .success(&format!(
                        "Reset {label}: {} key{} to defaults",
                        cleared.len(),
                        if cleared.len() == 1 { "" } else { "s" },
                    ))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
                for (k, old) in &cleared {
                    let default = werk_shared::config_registry::lookup(k)
                        .map(|e| e.default)
                        .unwrap_or("?");
                    println!(
                        "  {} {k}: {} {} {}",
                        palette.warning("·"),
                        palette.chrome(old),
                        palette.chrome("→"),
                        default,
                    );
                }
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
                    // Single key lookup. Registry-known keys render with
                    // their levels, gloss, and default annotation; unknown
                    // keys just show key=value.
                    let stored = config.get(&k).cloned();
                    let entry = werk_shared::config_registry::lookup(&k);
                    let effective = stored
                        .clone()
                        .unwrap_or_else(|| entry.map(|e| e.default.to_string()).unwrap_or_default());

                    if stored.is_none() && entry.is_none() {
                        return Err(WerkError::ConfigError(format!("config key '{k}' not found")));
                    }

                    if output.is_structured() {
                        let resolved = werk_shared::config_registry::resolve_value(&k, &effective);
                        let label = werk_shared::config_registry::label_for(&k, &effective);
                        output
                            .print_structured(&serde_json::json!({
                                "key": k,
                                "value": effective,
                                "resolved": resolved,
                                "label": label,
                                "is_set": stored.is_some(),
                                "default": entry.map(|e| e.default),
                                "gloss": entry.map(|e| e.gloss),
                                "levels": entry.map(|e| e.kind.labels()
                                    .iter().map(|(n, v)| serde_json::json!({"name": n, "value": v}))
                                    .collect::<Vec<_>>()).unwrap_or_default(),
                            }))
                            .map_err(WerkError::IoError)?;
                    } else {
                        render_key_detail(output, &k, stored.as_deref(), entry);
                    }
                    Ok(())
                }
                _ => {
                    // No key: list all config values, grouped by framework.
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
                        render_grouped(output, &path_str, config.values());
                    }
                    Ok(())
                }
            }
        }
        super::ConfigCommand::Edit => {
            if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                return Err(WerkError::InvalidInput(
                    "edit requires a TTY. Use `werk config set <key> <value>` \
                     or `werk config import <path>` in scripts."
                        .into(),
                ));
            }

            let workspace_result = Workspace::discover();
            let before = match workspace_result.as_ref() {
                Ok(ws) => Config::load(ws)?,
                Err(_) => Config::load_global()?,
            };
            let path = before
                .path()
                .ok_or_else(|| WerkError::IoError("no config path resolved".into()))?
                .to_path_buf();

            // Ensure file exists so $EDITOR has something to open.
            if !path.exists() {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| WerkError::IoError(format!("create config dir: {e}")))?;
                }
                std::fs::write(&path, "")
                    .map_err(|e| WerkError::IoError(format!("create config file: {e}")))?;
            }

            let editor = super::config_default_string(
                "editor.command",
                &std::env::var("EDITOR").unwrap_or_else(|_| {
                    if cfg!(windows) { "notepad".into() } else { "vi".into() }
                }),
            );
            let status = std::process::Command::new(&editor)
                .arg(&path)
                .status()
                .map_err(|e| WerkError::IoError(format!("launch {editor}: {e}")))?;
            if !status.success() {
                return Err(WerkError::IoError(format!(
                    "editor {editor} exited with status {status}"
                )));
            }

            // Reload and diff.
            let after = Config::load_from_path(&path)?;
            let diff = diff_configs(before.values(), after.values());
            render_diff_report(output, &diff, "edited")?;
            Ok(())
        }
        super::ConfigCommand::Diff => {
            let workspace_result = Workspace::discover();
            let config = match workspace_result.as_ref() {
                Ok(ws) => Config::load(ws)?,
                Err(_) => Config::load_global()?,
            };
            let diff = diff_vs_defaults(config.values());
            render_diff_report(output, &diff, "drift from defaults")?;
            Ok(())
        }
        super::ConfigCommand::Export { path } => {
            let workspace_result = Workspace::discover();
            let config = match workspace_result.as_ref() {
                Ok(ws) => Config::load(ws)?,
                Err(_) => Config::load_global()?,
            };

            let values = config.values();
            let hash = fnv1a_values(values);
            let count = values.len();

            // Build a preset file: header comment + unflattened TOML.
            let toml_body = values_to_toml_string(values)?;
            let now = chrono::Utc::now();
            let header = format!(
                "# werk config preset\n\
                 # exported: {}\n\
                 # keys: {}\n\
                 # fnv1a: {:016x}\n\n",
                werk_shared::format_timestamp(now, now),
                count,
                hash,
            );
            let file_body = format!("{header}{toml_body}");

            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        WerkError::IoError(format!(
                            "create export dir {}: {e}",
                            parent.display()
                        ))
                    })?;
                }
            }
            std::fs::write(path, file_body).map_err(|e| {
                WerkError::IoError(format!("write {}: {e}", path.display()))
            })?;

            if output.is_structured() {
                output
                    .print_structured(&serde_json::json!({
                        "path": path.display().to_string(),
                        "keys": count,
                        "fnv1a": format!("{:016x}", hash),
                    }))
                    .map_err(WerkError::IoError)?;
            } else {
                output
                    .success(&format!(
                        "Exported {count} key{} to {}",
                        if count == 1 { "" } else { "s" },
                        path.display(),
                    ))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
                println!("  fnv1a: {:016x}", hash);
            }
            Ok(())
        }
        super::ConfigCommand::Import { path, merge } => {
            if !path.exists() {
                return Err(WerkError::IoError(format!(
                    "import source not found: {}",
                    path.display()
                )));
            }
            let incoming = Config::load_from_path(path)?;

            // Validate every registry key in the incoming config against its Kind.
            let mut errors: Vec<String> = Vec::new();
            for (k, v) in incoming.values() {
                if let Some(entry) = werk_shared::config_registry::lookup(k) {
                    if let Err(msg) = werk_shared::config_registry::validate(entry.kind, v) {
                        errors.push(format!("{k}: {msg}"));
                    }
                }
            }
            if !errors.is_empty() {
                return Err(WerkError::InvalidInput(format!(
                    "import validation failed ({} error{}):\n  {}",
                    errors.len(),
                    if errors.len() == 1 { "" } else { "s" },
                    errors.join("\n  "),
                )));
            }

            // Resolve destination + load current.
            let workspace_result = Workspace::discover();
            let mut config = match workspace_result.as_ref() {
                Ok(ws) => Config::load(ws)?,
                Err(_) => Config::load_global()?,
            };
            let before = config.values().clone();

            if !merge {
                // Replace semantics: drop every existing key first.
                let keys: Vec<String> = config.values().keys().cloned().collect();
                for k in keys {
                    config.remove(&k);
                }
            }
            for (k, v) in incoming.values() {
                // Canonicalize registry keys to their Kind's normal form.
                let value = match werk_shared::config_registry::lookup(k) {
                    Some(entry) => {
                        werk_shared::config_registry::validate(entry.kind, v).unwrap_or_else(|_| v.clone())
                    }
                    None => v.clone(),
                };
                config.set(k, value);
            }
            config.save()?;

            let diff = diff_configs(&before, config.values());
            let mode_label = if *merge { "merged" } else { "imported" };
            render_diff_report(output, &diff, mode_label)?;
            Ok(())
        }
        super::ConfigCommand::Begin => {
            let workspace_result = Workspace::discover();
            let config = match workspace_result.as_ref() {
                Ok(ws) => Config::load(ws)?,
                Err(_) => Config::load_global()?,
            };
            let session_path = session_file_path(&config)?;
            if session_path.exists() {
                let existing = read_session(&session_path)?;
                return Err(WerkError::InvalidInput(format!(
                    "a config session is already active: {} ({} · started {}). \
                     Use `werk config status`, `commit`, or `abort`.",
                    existing.id,
                    pluralize(existing.snapshot.len(), "key", "keys"),
                    existing.started_at,
                )));
            }
            let session = ConfigSession {
                id: format!("g:{}", ulid::Ulid::new()),
                started_at: chrono::Utc::now().to_rfc3339(),
                snapshot: config.values().clone(),
            };
            write_session(&session_path, &session)?;

            if output.is_structured() {
                output
                    .print_structured(&session)
                    .map_err(WerkError::IoError)?;
            } else {
                output
                    .success(&format!("Started config session {}", session.id))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
                println!(
                    "  snapshot: {} · make changes, then `werk config commit -m \"...\"` or `abort`",
                    pluralize(session.snapshot.len(), "key", "keys"),
                );
            }
            Ok(())
        }
        super::ConfigCommand::Status => {
            let workspace_result = Workspace::discover();
            let config = match workspace_result.as_ref() {
                Ok(ws) => Config::load(ws)?,
                Err(_) => Config::load_global()?,
            };
            let session_path = session_file_path(&config)?;
            let session = match session_path.exists() {
                true => read_session(&session_path)?,
                false => {
                    return Err(WerkError::InvalidInput(
                        "no active config session. `werk config begin` to start one.".into(),
                    ));
                }
            };
            let diff = diff_configs(&session.snapshot, config.values());
            if output.is_structured() {
                output
                    .print_structured(&serde_json::json!({
                        "session_id": session.id,
                        "started_at": session.started_at,
                        "changes": diff,
                    }))
                    .map_err(WerkError::IoError)?;
            } else {
                println!(
                    "  {}  started {}",
                    output.palette().structure(&session.id),
                    output.palette().chrome(&session.started_at),
                );
                render_diff_report(output, &diff, "pending")?;
            }
            Ok(())
        }
        super::ConfigCommand::Commit { message } => {
            let workspace_result = Workspace::discover();
            let config = match workspace_result.as_ref() {
                Ok(ws) => Config::load(ws)?,
                Err(_) => Config::load_global()?,
            };
            let session_path = session_file_path(&config)?;
            let session = match session_path.exists() {
                true => read_session(&session_path)?,
                false => {
                    return Err(WerkError::InvalidInput(
                        "no active config session. Use `werk config set/unset` directly, \
                         or `werk config begin` to start a session."
                            .into(),
                    ));
                }
            };
            let diff = diff_configs(&session.snapshot, config.values());
            if diff.is_empty() {
                let _ = std::fs::remove_file(&session_path);
                output
                    .success(&format!("Closed {} with no changes", session.id))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
                return Ok(());
            }

            // Append audit entry.
            let audit_path = session_path.with_file_name("config-sessions.jsonl");
            let audit_entry = serde_json::json!({
                "id": session.id,
                "started_at": session.started_at,
                "committed_at": chrono::Utc::now().to_rfc3339(),
                "message": message,
                "changes": diff,
            });
            {
                use std::io::Write;
                let mut f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&audit_path)
                    .map_err(|e| WerkError::IoError(format!("open audit log: {e}")))?;
                writeln!(f, "{audit_entry}")
                    .map_err(|e| WerkError::IoError(format!("write audit entry: {e}")))?;
            }
            let _ = std::fs::remove_file(&session_path);

            if output.is_structured() {
                output
                    .print_structured(&audit_entry)
                    .map_err(WerkError::IoError)?;
            } else {
                let palette = output.palette();
                output
                    .success(&format!(
                        "Committed {} · {} · {}",
                        session.id,
                        pluralize(diff.len(), "change", "changes"),
                        message.as_deref().unwrap_or("(no message)"),
                    ))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
                println!("  {}", palette.chrome(&format!("audit: {}", audit_path.display())));
                println!(
                    "  {}",
                    palette.chrome("revert: werk config abort is only valid inside a session"),
                );
            }
            Ok(())
        }
        super::ConfigCommand::Abort => {
            let workspace_result = Workspace::discover();
            let mut config = match workspace_result.as_ref() {
                Ok(ws) => Config::load(ws)?,
                Err(_) => Config::load_global()?,
            };
            let session_path = session_file_path(&config)?;
            let session = match session_path.exists() {
                true => read_session(&session_path)?,
                false => {
                    return Err(WerkError::InvalidInput(
                        "no active config session to abort.".into(),
                    ));
                }
            };

            // Revert: clear current, replay snapshot.
            let keys: Vec<String> = config.values().keys().cloned().collect();
            for k in keys {
                config.remove(&k);
            }
            for (k, v) in &session.snapshot {
                config.set(k, v.clone());
            }
            config.save()?;
            let _ = std::fs::remove_file(&session_path);

            let reverted = diff_configs(&session.snapshot, config.values()); // should be empty
            if output.is_structured() {
                output
                    .print_structured(&serde_json::json!({
                        "aborted": session.id,
                        "restored": session.snapshot.len(),
                        "still_divergent": reverted,
                    }))
                    .map_err(WerkError::IoError)?;
            } else {
                output
                    .success(&format!(
                        "Aborted {} · restored {} to snapshot",
                        session.id,
                        pluralize(session.snapshot.len(), "key", "keys"),
                    ))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
            Ok(())
        }
        super::ConfigCommand::Preset { command } => {
            use werk_shared::config_registry::{preset, PRESETS};
            match command {
                super::PresetCommand::List => {
                    if output.is_structured() {
                        output
                            .print_structured(&serde_json::json!({
                                "presets": PRESETS.iter().map(|p| serde_json::json!({
                                    "name": p.name,
                                    "description": p.description,
                                    "keys": p.values.len(),
                                })).collect::<Vec<_>>(),
                            }))
                            .map_err(WerkError::IoError)?;
                    } else {
                        let palette = output.palette();
                        let name_width = PRESETS.iter().map(|p| p.name.len()).max().unwrap_or(8);
                        for p in PRESETS {
                            println!(
                                "  {:<nw$}  {}",
                                palette.structure(p.name),
                                palette.chrome(p.description),
                                nw = name_width,
                            );
                        }
                        println!();
                        println!("  {}", palette.chrome("Apply: werk config preset apply <name>"));
                        println!("  {}", palette.chrome("Inspect: werk config preset show <name>"));
                    }
                    Ok(())
                }
                super::PresetCommand::Show { name } => {
                    let Some(p) = preset(name) else {
                        return Err(WerkError::InvalidInput(format!(
                            "no preset named '{name}'. Try `werk config preset list`."
                        )));
                    };
                    if output.is_structured() {
                        output
                            .print_structured(&serde_json::json!({
                                "name": p.name,
                                "description": p.description,
                                "values": p.values.iter().map(|(k, v)| serde_json::json!({
                                    "key": k, "value": v
                                })).collect::<Vec<_>>(),
                            }))
                            .map_err(WerkError::IoError)?;
                    } else {
                        let palette = output.palette();
                        println!(
                            "{}  {}",
                            palette.structure(p.name),
                            palette.chrome(p.description),
                        );
                        let key_width = p.values.iter().map(|(k, _)| k.len()).max().unwrap_or(8);
                        for (k, v) in p.values {
                            println!("  {:<kw$}  {}", k, v, kw = key_width);
                        }
                    }
                    Ok(())
                }
                super::PresetCommand::Apply { name } => {
                    let Some(p) = preset(name) else {
                        return Err(WerkError::InvalidInput(format!(
                            "no preset named '{name}'. Try `werk config preset list`."
                        )));
                    };
                    let workspace_result = Workspace::discover();
                    let mut config = match workspace_result {
                        Ok(ws) => Config::load(&ws)?,
                        Err(_) => Config::load_global()?,
                    };
                    let before = config.values().clone();

                    // Apply each value. Resolve synthetic cascades at the
                    // registry level — same logic the Set handler uses.
                    for (key, value) in p.values {
                        let entry = werk_shared::config_registry::lookup(key);
                        let canonical = match entry {
                            Some(e) => werk_shared::config_registry::validate(e.kind, value)
                                .map_err(|e| WerkError::InvalidInput(format!("{key}: {e}")))?,
                            None => (*value).to_string(),
                        };
                        if entry.map(|e| e.kind.is_synthetic()).unwrap_or(false) {
                            if let Some(bundle) = werk_shared::config_registry::cascade_for(key, &canonical) {
                                for (k, v) in bundle {
                                    config.set(k, (*v).to_string());
                                }
                            }
                        } else {
                            config.set(key, canonical);
                        }
                    }
                    config.save()?;

                    let diff = diff_configs(&before, config.values());
                    render_diff_report(output, &diff, &format!("applied preset \"{}\"", p.name))?;
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

/// Session marker: one active session per workspace, persisted as JSON
/// alongside config.toml. Multi-process: creating a session when one exists
/// errors with a clear message pointing at status/commit/abort.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ConfigSession {
    id: String,
    started_at: String,
    snapshot: std::collections::BTreeMap<String, String>,
}

fn session_file_path(config: &Config) -> Result<std::path::PathBuf> {
    let config_path = config
        .path()
        .ok_or_else(|| WerkError::IoError("no config path resolved".into()))?;
    let parent = config_path
        .parent()
        .ok_or_else(|| WerkError::IoError("config path has no parent".into()))?;
    Ok(parent.join("config-session.json"))
}

fn read_session(path: &std::path::Path) -> Result<ConfigSession> {
    let body = std::fs::read_to_string(path)
        .map_err(|e| WerkError::IoError(format!("read session: {e}")))?;
    serde_json::from_str(&body)
        .map_err(|e| WerkError::IoError(format!("parse session: {e}")))
}

fn write_session(path: &std::path::Path, session: &ConfigSession) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| WerkError::IoError(format!("create session dir: {e}")))?;
    }
    let body = serde_json::to_string_pretty(session)
        .map_err(|e| WerkError::IoError(format!("serialize session: {e}")))?;
    std::fs::write(path, body)
        .map_err(|e| WerkError::IoError(format!("write session: {e}")))?;
    Ok(())
}

fn pluralize(n: usize, singular: &str, plural: &str) -> String {
    format!("{n} {}", if n == 1 { singular } else { plural })
}

/// FNV-1a hash of the sorted `key=value` pairs. Used as regression-detection
/// metadata in exported presets — two exports of the same effective config
/// hash to the same 16-hex-digit number, regardless of import order.
fn fnv1a_values(values: &std::collections::BTreeMap<String, String>) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    // BTreeMap iteration is sorted → deterministic.
    for (k, v) in values {
        for b in k.bytes().chain(std::iter::once(b'=')).chain(v.bytes()).chain(std::iter::once(b'\n')) {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    hash
}

/// Serialize a flat dot-notation map to a nested TOML document. Delegates to
/// `Config`'s existing unflatten logic by round-tripping through a tempfile,
/// which keeps all the existing serialization tests as the single source of truth.
fn values_to_toml_string(
    values: &std::collections::BTreeMap<String, String>,
) -> Result<String> {
    // Build a Config with the values, save to a tempfile, read it back.
    let temp = std::env::temp_dir().join(format!("werk_export_{}.toml", ulid::Ulid::new()));
    // load_from_path on a missing file returns a Config with `path` set and
    // empty values — the cleanest way to borrow Config's save() logic.
    let mut config = Config::load_from_path(&temp)?;
    for (k, v) in values {
        config.set(k, v.clone());
    }
    config.save()?;
    let body = std::fs::read_to_string(&temp)
        .map_err(|e| WerkError::IoError(format!("read temp export: {e}")))?;
    let _ = std::fs::remove_file(&temp);
    Ok(body)
}

/// One change in a config diff. `before == None` = added; `after == None` = removed.
#[derive(Debug, serde::Serialize)]
struct DiffEntry {
    key: String,
    before: Option<String>,
    after: Option<String>,
}

/// Compute the diff between two flat config maps. Only returns keys that
/// actually differ. Order: registry order first (so diffs group naturally),
/// then unregistered keys alphabetically.
fn diff_configs(
    before: &std::collections::BTreeMap<String, String>,
    after: &std::collections::BTreeMap<String, String>,
) -> Vec<DiffEntry> {
    use werk_shared::config_registry::{lookup, REGISTRY};
    let mut out: Vec<DiffEntry> = Vec::new();
    // Registry keys first, in registry order.
    for entry in REGISTRY {
        let b = before.get(entry.key);
        let a = after.get(entry.key);
        if b != a {
            out.push(DiffEntry {
                key: entry.key.into(),
                before: b.cloned(),
                after: a.cloned(),
            });
        }
    }
    // Non-registry keys in union, alphabetical.
    let mut extras: std::collections::BTreeSet<&String> = std::collections::BTreeSet::new();
    for k in before.keys().chain(after.keys()) {
        if lookup(k).is_none() {
            extras.insert(k);
        }
    }
    for k in extras {
        let b = before.get(k);
        let a = after.get(k);
        if b != a {
            out.push(DiffEntry {
                key: k.clone(),
                before: b.cloned(),
                after: a.cloned(),
            });
        }
    }
    out
}

/// Diff a live config against the registry defaults. Non-registry keys
/// (hooks, unknowns) appear as "added" entries since defaults would be None.
fn diff_vs_defaults(values: &std::collections::BTreeMap<String, String>) -> Vec<DiffEntry> {
    use werk_shared::config_registry::{lookup, REGISTRY};
    let mut out: Vec<DiffEntry> = Vec::new();
    for entry in REGISTRY {
        if let Some(user) = values.get(entry.key) {
            if user != entry.default {
                out.push(DiffEntry {
                    key: entry.key.into(),
                    before: Some(entry.default.into()),
                    after: Some(user.clone()),
                });
            }
        }
    }
    let mut extras: Vec<(&String, &String)> = values
        .iter()
        .filter(|(k, _)| lookup(k).is_none())
        .collect();
    extras.sort_by_key(|(k, _)| k.as_str());
    for (k, v) in extras {
        out.push(DiffEntry {
            key: k.clone(),
            before: None,
            after: Some(v.clone()),
        });
    }
    out
}

fn render_diff_report(
    output: &crate::output::Output,
    diff: &[DiffEntry],
    label: &str,
) -> Result<()> {
    let palette = output.palette();
    if output.is_structured() {
        output
            .print_structured(&serde_json::json!({
                "label": label,
                "changes": diff,
            }))
            .map_err(WerkError::IoError)?;
        return Ok(());
    }

    if diff.is_empty() {
        println!("  no changes ({label})");
        return Ok(());
    }

    let header = match diff.len() {
        1 => format!("1 change ({label})"),
        n => format!("{n} changes ({label})"),
    };
    output
        .success(&header)
        .map_err(|e| WerkError::IoError(e.to_string()))?;

    let key_width = diff.iter().map(|e| e.key.len()).max().unwrap_or(1);
    for entry in diff {
        let marker = palette.warning("·");
        let (b, a) = (
            entry.before.as_deref().unwrap_or("—"),
            entry.after.as_deref().unwrap_or("—"),
        );
        println!(
            "  {marker} {key:<kw$}  {b_col} {arrow} {a}",
            key = entry.key,
            kw = key_width,
            b_col = palette.chrome(b),
            arrow = palette.chrome("→"),
            a = a,
        );
    }
    Ok(())
}

/// Render the full config surface grouped by framework. Registry keys are
/// shown always (defaults as ghosts); user-set values render over them.
/// Unregistered user keys land in an "Other" group; hooks land under "Hooks".
fn render_grouped(
    output: &crate::output::Output,
    path_str: &str,
    values: &std::collections::BTreeMap<String, String>,
) {
    use werk_shared::config_registry::{groups, is_hook_key, keys_in_group, lookup, REGISTRY};

    let palette = output.palette();

    // Count modifications: user-set values that differ from registry default,
    // plus any user keys outside the registry (hooks, unknowns).
    let mut modified = 0usize;
    for (k, v) in values.iter() {
        match lookup(k) {
            Some(entry) if entry.default == v => {}
            Some(_) => modified += 1,
            None => modified += 1, // hooks, unknowns — presence = user intent
        }
    }

    // Header.
    let header_left = palette.structure("werk · config");
    let header_mid = palette.chrome(&format!(" · {path_str}"));
    let header_right = if modified == 0 {
        palette.chrome("all defaults")
    } else if modified == 1 {
        palette.warning("1 modified")
    } else {
        palette.warning(&format!("{modified} modified"))
    };
    println!("{header_left}{header_mid}       {header_right}");
    println!("{}", palette.chrome(&"─".repeat(65)));

    // Column widths across registry keys + any user-set keys.
    let key_width = REGISTRY.iter().map(|k| k.key.len())
        .chain(values.keys().map(String::len))
        .max()
        .unwrap_or(20);
    // Value column reserves room for `label (value)` shapes so level
    // keys and raw keys align.
    let value_width = REGISTRY.iter()
        .map(|k| {
            let base = values.get(k.key).map(|s| s.as_str()).unwrap_or(k.default);
            if k.kind.has_levels() {
                let label = werk_shared::config_registry::label_for(k.key, base)
                    .unwrap_or(base);
                let resolved = werk_shared::config_registry::resolve_value(k.key, base);
                label.len() + 3 + resolved.len()
            } else {
                base.len()
            }
        })
        .chain(values.values().map(String::len))
        .max()
        .unwrap_or(8)
        .max(5);

    // Registry keys grouped by top-level namespace (`flush`, `signals`, …).
    for group in groups() {
        println!();
        println!("  {}", palette.structure(group));
        for entry in keys_in_group(group) {
            if entry.kind.is_synthetic() {
                // Synthetic: infer value from the cascade map.
                let inferred = werk_shared::config_registry::infer_synthetic(entry.key, values);
                let displayed = inferred.unwrap_or("custom");
                let user_value = if inferred.is_some() && inferred != Some(entry.default) {
                    Some(displayed.to_string())
                } else {
                    None
                };
                render_row(
                    &palette,
                    entry.key,
                    entry.default,
                    Some(entry.default),
                    entry.gloss,
                    user_value.as_ref(),
                    key_width,
                    value_width,
                );
            } else {
                render_row(&palette, entry.key, entry.default, Some(entry.default),
                    entry.gloss, values.get(entry.key), key_width, value_width);
            }
        }
    }

    // Hook definitions — dynamic, only if any user hooks are set. Separate
    // from the registry `hooks` group which holds CLI preferences like
    // `hooks.log_tail`.
    let hook_keys: Vec<_> = values.iter()
        .filter(|(k, _)| is_hook_key(k))
        .collect();
    if !hook_keys.is_empty() {
        println!();
        println!(
            "  {}  {}",
            palette.structure("hook definitions"),
            palette.chrome("— event → command"),
        );
        for (k, v) in hook_keys {
            render_row(&palette, k, v, None, "", Some(v), key_width, value_width);
        }
    }

    // Unknowns — anything user-set that isn't a registry key or a hook.
    let other_keys: Vec<_> = values.iter()
        .filter(|(k, _)| lookup(k).is_none() && !is_hook_key(k))
        .collect();
    if !other_keys.is_empty() {
        println!();
        println!(
            "  {}  {}",
            palette.structure("other"),
            palette.chrome("— keys not in the registry"),
        );
        for (k, v) in other_keys {
            render_row(&palette, k, v, None, "", Some(v), key_width, value_width);
        }
    }

    // Footer hint.
    println!();
    println!("{}", palette.chrome(&"─".repeat(65)));
    println!(
        "{}",
        palette.chrome(
            "Change: werk config set <key> <value>   Reset: werk config reset <key>"
        ),
    );
}

/// Render one line of the grouped config view.
///
/// - `default`: the fallback value to render when `user_value` is None.
/// - `registry_default`: the registry's declared default, if the key is
///    registered — used to decide whether a user-set value is modified and
///    to emit the `[default X]` annotation.
#[allow(clippy::too_many_arguments)]
fn render_row(
    palette: &werk_shared::cli_display::Palette,
    key: &str,
    default: &str,
    registry_default: Option<&str>,
    gloss: &str,
    user_value: Option<&String>,
    key_width: usize,
    value_width: usize,
) {
    let is_modified = match (user_value, registry_default) {
        (Some(v), Some(d)) => v != d,
        (Some(_), None) => true, // hooks, unknowns — presence = modified
        (None, _) => false,
    };

    let gutter = if is_modified {
        palette.warning("·")
    } else {
        " ".to_string()
    };

    let displayed_value = user_value.map(String::as_str).unwrap_or(default);

    // Level-annotated display: "balanced (0.5)" when the value matches a
    // known label (or its backing value). Only for registered keys.
    let (main_display, level_hint) = match werk_shared::config_registry::lookup(key) {
        Some(entry) if entry.kind.is_synthetic() => {
            // StringEnum — value is a label-name itself, no backing value.
            // Render just the name (or "custom" if unknown).
            let is_known = entry.kind.enum_names().iter().any(|n| *n == displayed_value);
            (displayed_value.to_string(), if is_known { None } else { Some("custom".into()) })
        }
        Some(entry) if entry.kind.has_levels() => {
            let label = werk_shared::config_registry::label_for(key, displayed_value);
            match label {
                Some(name) if name == displayed_value => {
                    // Stored form is the label itself — show label, chrome value.
                    let resolved = werk_shared::config_registry::resolve_value(key, displayed_value);
                    (format!("{name}"), Some(resolved))
                }
                Some(name) => {
                    // Stored form is a raw that matches a label — show label + raw.
                    (name.to_string(), Some(displayed_value.to_string()))
                }
                None => {
                    // Raw custom value — show as-is with a (custom) tag.
                    (displayed_value.to_string(), Some("custom".to_string()))
                }
            }
        }
        _ => (displayed_value.to_string(), None),
    };

    let value_str = if user_value.is_none() {
        palette.chrome(&main_display)
    } else {
        main_display.clone()
    };
    let level_hint_str = match level_hint {
        Some(h) => palette.chrome(&format!(" ({h})")),
        None => String::new(),
    };
    let combined_value = format!("{value_str}{level_hint_str}");
    // Strip ANSI for width calculation.
    let combined_display_len = strip_ansi(&combined_value).chars().count();
    let value_padding = " ".repeat(value_width.saturating_sub(combined_display_len));

    let gloss_part = if !gloss.is_empty() {
        palette.chrome(gloss)
    } else {
        String::new()
    };

    let default_annotation = match registry_default {
        Some(d) if is_modified && !d.is_empty() => palette.chrome(&format!("  [default {d}]")),
        Some("") if is_modified => palette.chrome("  [default unset]"),
        _ => String::new(),
    };

    println!(
        "  {gutter} {key:<kw$}  {combined_value}{value_padding}  {gloss_part}{default_annotation}",
        kw = key_width,
    );
}

/// Render the detail view for `werk config get <key>` — key, value, label,
/// levels table, default annotation, gloss. For non-registry keys, reduces
/// to `key = value`.
fn render_key_detail(
    output: &crate::output::Output,
    key: &str,
    stored: Option<&str>,
    entry: Option<&werk_shared::config_registry::ConfigKey>,
) {
    let palette = output.palette();

    // For synthetic keys, the effective value is inferred from the current
    // config (not stored). Non-synthetic: effective = stored, or default.
    let config_values = werk_shared::Workspace::discover()
        .ok()
        .and_then(|ws| Config::load(&ws).ok())
        .map(|c| c.values().clone())
        .unwrap_or_default();
    let effective_owned: String = match entry {
        Some(e) if e.kind.is_synthetic() => werk_shared::config_registry::infer_synthetic(key, &config_values)
            .map(String::from)
            .unwrap_or_else(|| "custom".to_string()),
        _ => stored.map(String::from).unwrap_or_else(|| entry.map(|e| e.default.to_string()).unwrap_or_default()),
    };
    let effective = effective_owned.as_str();

    // Header line: `key = value_display`.
    let value_display = match entry {
        Some(e) if e.kind.is_synthetic() => effective.to_string(),
        Some(e) if e.kind.has_levels() => {
            let label = werk_shared::config_registry::label_for(key, effective);
            let resolved = werk_shared::config_registry::resolve_value(key, effective);
            match label {
                Some(name) if name == effective => format!("{name} ({resolved})"),
                Some(name) => format!("{name} ({effective})"),
                None => format!("{effective} (custom)"),
            }
        }
        _ => effective.to_string(),
    };
    println!("{key} = {value_display}");

    let Some(entry) = entry else {
        return;
    };

    // Levels row.
    if entry.kind.is_synthetic() {
        let names = entry.kind.enum_names();
        let rendered: Vec<String> = names
            .iter()
            .map(|name| {
                if *name == effective {
                    palette.structure(name)
                } else {
                    palette.chrome(name)
                }
            })
            .collect();
        println!("  {} {}", palette.chrome("levels:"), rendered.join(palette.chrome(" · ").as_str()));
    } else if entry.kind.has_levels() {
        let labels = entry.kind.labels();
        let rendered: Vec<String> = labels
            .iter()
            .map(|(name, value)| {
                if entry.default == *name
                    || Some(*name) == werk_shared::config_registry::label_for(key, effective)
                {
                    palette.structure(&format!("{name} ({value})"))
                } else {
                    palette.chrome(&format!("{name} ({value})"))
                }
            })
            .collect();
        println!("  {} {}", palette.chrome("levels:"), rendered.join(palette.chrome(" · ").as_str()));
    }

    // Default annotation.
    if entry.kind.is_synthetic() {
        if effective != entry.default {
            println!("  {}", palette.chrome(&format!("[default {}]", entry.default)));
        }
    } else if stored.is_some() && stored != Some(entry.default) {
        println!("  {}", palette.chrome(&format!("[default {}]", entry.default)));
    } else if stored.is_none() {
        println!("  {}", palette.chrome("(unset — showing default)"));
    }

    // Gloss.
    if !entry.gloss.is_empty() {
        println!("  {}", palette.chrome(entry.gloss));
    }
}

/// Strip ANSI escape sequences for width calculations. Minimal — handles
/// the CSI sequences emitted by `owo_colors`.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_esc = false;
    for c in s.chars() {
        if in_esc {
            if c == 'm' || c.is_alphabetic() {
                in_esc = false;
            }
            continue;
        }
        if c == '\u{1b}' {
            in_esc = true;
            continue;
        }
        out.push(c);
    }
    out
}
