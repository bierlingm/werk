//! Hook management CLI commands.
//!
//! werk hooks list [--verbose]
//! werk hooks add <event> <command> [--filter <filter>]
//! werk hooks rm <event> [command]
//! werk hooks test <event> [--tension ID]
//! werk hooks log [--tail N]
//! werk hooks run <name> [--tension ID]
//! werk hooks install [--git] [hook-name...]

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use serde::Serialize;
use werk_shared::{Config, GitHooks, HookEvent, HookRunner, ShippedHooks};

// ============================================================================
// JSON output types
// ============================================================================

#[derive(Serialize)]
struct HookListEntry {
    event: String,
    commands: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<String>,
    scope: String,
}

#[derive(Serialize)]
struct HookListResult {
    hooks: Vec<HookListEntry>,
}

#[derive(Serialize)]
struct HookTestResult {
    hook_name: String,
    command: String,
    success: bool,
    stdout: String,
    stderr: String,
    duration_ms: u64,
}

#[derive(Serialize)]
struct ShippedHookInfo {
    name: String,
    event: String,
    description: String,
    installed: bool,
}

// ============================================================================
// Command handlers
// ============================================================================

pub fn cmd_hooks_list(output: &Output, verbose: bool) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let config = Config::load(&workspace).unwrap_or_default();
    let global_config = Config::load_global().ok();

    let mut entries = Vec::new();

    // Collect from global
    if let Some(ref gc) = global_config {
        collect_hook_entries(gc, "global", &mut entries);
    }

    // Collect from workspace
    collect_hook_entries(&config, "workspace", &mut entries);

    if output.is_structured() {
        output
            .print_structured(&HookListResult { hooks: entries })
            .map_err(WerkError::IoError)?;
    } else if entries.is_empty() {
        println!("No hooks configured.");
        println!();
        println!("Add a hook:");
        println!("  werk hooks add post_tension_resolved ./notify.sh");
        println!("  werk hooks add post_* ./log-all.sh");
        println!();
        println!("Install shipped defaults:");
        println!("  werk hooks install flush auto-stage");
    } else {
        println!("Configured hooks:");
        println!();
        for entry in &entries {
            let cmds = entry.commands.join(", ");
            if verbose {
                println!(
                    "  {} → {} [{}]{}",
                    entry.event,
                    cmds,
                    entry.scope,
                    entry
                        .filter
                        .as_ref()
                        .map(|f| format!(" (filter: {})", f))
                        .unwrap_or_default()
                );
            } else {
                println!("  {} → {}", entry.event, cmds);
            }
        }
        println!();
        println!("{} hook(s) configured", entries.len());
    }

    Ok(())
}

fn collect_hook_entries(config: &Config, scope: &str, entries: &mut Vec<HookListEntry>) {
    for (key, value) in config.values() {
        if let Some(hook_name) = key.strip_prefix("hooks.") {
            if hook_name.contains('.') {
                continue; // skip sub-keys like hooks.X.filter
            }
            let commands = parse_command_value(value);
            let filter_key = format!("{}.filter", key);
            let filter = config.get(&filter_key).cloned();

            entries.push(HookListEntry {
                event: hook_name.to_string(),
                commands,
                filter,
                scope: scope.to_string(),
            });
        }
    }
}

fn parse_command_value(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
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

pub fn cmd_hooks_add(
    output: &Output,
    event: String,
    command: String,
    filter: Option<String>,
    global: bool,
) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let mut config = if global {
        Config::load_global().unwrap_or_default()
    } else {
        Config::load(&workspace).unwrap_or_default()
    };

    let key = format!("hooks.{}", event);

    // If a hook already exists, convert to chain
    let new_value = if let Some(existing) = config.get(&key) {
        let mut commands = parse_command_value(existing);
        if commands.contains(&command) {
            return Err(WerkError::InvalidInput(format!(
                "Hook '{}' already has command '{}'",
                event, command
            )));
        }
        commands.push(command.clone());
        format!(
            "[{}]",
            commands
                .iter()
                .map(|c| format!("\"{}\"", c))
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        command.clone()
    };

    config.set(&key, new_value);

    if let Some(f) = &filter {
        config.set(&format!("{}.filter", key), f.clone());
    }

    config.save().map_err(|e| WerkError::IoError(e.to_string()))?;

    let scope = if global { "global" } else { "workspace" };
    if output.is_structured() {
        output
            .print_structured(&serde_json::json!({
                "added": true,
                "event": event,
                "command": command,
                "scope": scope,
            }))
            .map_err(WerkError::IoError)?;
    } else {
        println!("✓ Added {} hook: {} [{}]", event, command, scope);
    }

    Ok(())
}

pub fn cmd_hooks_rm(
    output: &Output,
    event: String,
    command: Option<String>,
    global: bool,
) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let mut config = if global {
        Config::load_global().unwrap_or_default()
    } else {
        Config::load(&workspace).unwrap_or_default()
    };

    let key = format!("hooks.{}", event);

    if config.get(&key).is_none() {
        return Err(WerkError::InvalidInput(format!(
            "No hook configured for '{}'",
            event
        )));
    }

    if let Some(cmd) = &command {
        // Remove specific command from chain
        let existing = config.get(&key).cloned().unwrap_or_default();
        let mut commands = parse_command_value(&existing);
        let before_len = commands.len();
        commands.retain(|c| c != cmd);
        if commands.len() == before_len {
            return Err(WerkError::InvalidInput(format!(
                "Command '{}' not found in hook '{}'",
                cmd, event
            )));
        }

        if commands.is_empty() {
            config.remove(&key);
        } else if commands.len() == 1 {
            config.set(&key, commands[0].clone());
        } else {
            config.set(
                &key,
                format!(
                    "[{}]",
                    commands
                        .iter()
                        .map(|c| format!("\"{}\"", c))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
        }
    } else {
        // Remove entire hook
        config.remove(&key);
        // Also remove filter if present
        let filter_key = format!("{}.filter", key);
        config.remove(&filter_key);
    }

    config.save().map_err(|e| WerkError::IoError(e.to_string()))?;

    let scope = if global { "global" } else { "workspace" };
    if output.is_structured() {
        output
            .print_structured(&serde_json::json!({
                "removed": true,
                "event": event,
                "command": command,
                "scope": scope,
            }))
            .map_err(WerkError::IoError)?;
    } else {
        match command {
            Some(cmd) => println!("✓ Removed '{}' from {} hook [{}]", cmd, event, scope),
            None => println!("✓ Removed {} hook [{}]", event, scope),
        }
    }

    Ok(())
}

pub fn cmd_hooks_test(
    output: &Output,
    event: String,
    tension_id: Option<String>,
) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let config = Config::load(&workspace).unwrap_or_default();
    let _runner = HookRunner::from_config(&config);

    // Build a synthetic event
    let hook_event = if let Some(tid) = &tension_id {
        let store = workspace.open_store()?;
        let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
        let resolver = crate::prefix::PrefixResolver::new(tensions);
        let tension = resolver.resolve(tid)?;
        HookEvent::mutation(
            &tension.id,
            &tension.desired,
            Some(&tension.actual),
            tension.parent_id.as_deref(),
            "test",
            Some("test_old"),
            "test_new",
        )
    } else {
        HookEvent::mutation(
            "00000000000000000000000000",
            "test desired state",
            Some("test reality"),
            None,
            "test",
            Some("test_old"),
            "test_new",
        )
    };

    // Find matching hooks
    let key = format!("hooks.{}", event);
    let commands = match config.get(&key) {
        Some(v) => parse_command_value(v),
        None => {
            return Err(WerkError::InvalidInput(format!(
                "No hook configured for '{}'. Add one first:\n  werk hooks add {} ./your-script.sh",
                event, event
            )));
        }
    };

    let mut results = Vec::new();
    for command in &commands {
        let start = std::time::Instant::now();
        let event_json = serde_json::to_string(&hook_event)
            .map_err(|e| WerkError::IoError(format!("serialize: {}", e)))?;

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

        let duration = start.elapsed().as_millis() as u64;

        let (success, stdout, stderr) = match result {
            Ok(out) => (
                out.status.success(),
                String::from_utf8_lossy(&out.stdout).to_string(),
                String::from_utf8_lossy(&out.stderr).to_string(),
            ),
            Err(e) => (false, String::new(), format!("Failed to execute: {}", e)),
        };

        results.push(HookTestResult {
            hook_name: event.clone(),
            command: command.clone(),
            success,
            stdout: stdout.clone(),
            stderr: stderr.clone(),
            duration_ms: duration,
        });

        if !output.is_structured() {
            let status = if success { "✓" } else { "✗" };
            println!("{} {} ({}ms)", status, command, duration);
            if !stdout.is_empty() {
                println!("  stdout: {}", stdout.trim());
            }
            if !stderr.is_empty() {
                println!("  stderr: {}", stderr.trim());
            }
        }
    }

    if output.is_structured() {
        output
            .print_structured(&results)
            .map_err(WerkError::IoError)?;
    }

    Ok(())
}

pub fn cmd_hooks_log(output: &Output, tail: usize) -> Result<(), WerkError> {
    // Hook log is in-memory only during the current process.
    // For persistent logging, use the audit-log shipped hook.
    // Here we check .werk/audit.jsonl if it exists.
    let workspace = Workspace::discover()?;
    let audit_path = workspace.root().join(".werk").join("audit.jsonl");

    if !audit_path.exists() {
        if output.is_structured() {
            output
                .print_structured(&serde_json::json!({ "entries": [], "source": "none" }))
                .map_err(WerkError::IoError)?;
        } else {
            println!("No hook log found.");
            println!();
            println!("Enable audit logging:");
            println!("  werk hooks install audit-log");
        }
        return Ok(());
    }

    let content =
        std::fs::read_to_string(&audit_path).map_err(|e| WerkError::IoError(e.to_string()))?;
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    let start = if lines.len() > tail {
        lines.len() - tail
    } else {
        0
    };
    let recent = &lines[start..];

    if output.is_structured() {
        let entries: Vec<serde_json::Value> = recent
            .iter()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        output
            .print_structured(&serde_json::json!({ "entries": entries, "source": "audit.jsonl" }))
            .map_err(WerkError::IoError)?;
    } else {
        println!(
            "Hook log ({} entries, showing last {}):",
            lines.len(),
            recent.len()
        );
        println!();
        for line in recent {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                let event = v.get("event").and_then(|e| e.as_str()).unwrap_or("?");
                let tid = v
                    .get("tension_id")
                    .and_then(|e| e.as_str())
                    .unwrap_or("-");
                let ts = v
                    .get("timestamp")
                    .and_then(|e| e.as_str())
                    .unwrap_or("?");
                println!("  {} {} tid={}", ts, event, tid);
            } else {
                println!("  {}", line);
            }
        }
    }

    Ok(())
}

pub fn cmd_hooks_install(
    output: &Output,
    git: bool,
    hook_names: Vec<String>,
) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;

    if git {
        return cmd_hooks_install_git(output, &workspace);
    }

    if hook_names.is_empty() {
        // Show available shipped hooks
        if output.is_structured() {
            let hooks_dir = workspace.root().join(".werk").join("hooks");
            let infos: Vec<ShippedHookInfo> = ShippedHooks::available()
                .iter()
                .map(|(name, event, desc)| ShippedHookInfo {
                    name: name.to_string(),
                    event: event.to_string(),
                    description: desc.to_string(),
                    installed: hooks_dir.join(format!("{}.sh", name)).exists(),
                })
                .collect();
            output
                .print_structured(&infos)
                .map_err(WerkError::IoError)?;
        } else {
            println!("Available shipped hooks:");
            println!();
            let hooks_dir = workspace.root().join(".werk").join("hooks");
            for (name, event, desc) in ShippedHooks::available() {
                let installed = hooks_dir.join(format!("{}.sh", name)).exists();
                let marker = if installed { " [installed]" } else { "" };
                println!("  {:15} {:15} {}{}", name, event, desc, marker);
            }
            println!();
            println!("Install:");
            println!("  werk hooks install flush auto-stage");
            println!("  werk hooks install --git");
        }
        return Ok(());
    }

    let hooks_dir = workspace.root().join(".werk").join("hooks");
    let mut config = Config::load(&workspace).unwrap_or_default();
    let mut installed = Vec::new();

    for name in &hook_names {
        let content = ShippedHooks::content(name).ok_or_else(|| {
            WerkError::InvalidInput(format!(
                "Unknown shipped hook '{}'. Run 'werk hooks install' to see available hooks.",
                name
            ))
        })?;

        let path = ShippedHooks::install(&hooks_dir, name, content)
            .map_err(|e| WerkError::IoError(format!("failed to install {}: {}", name, e)))?;

        // Register in config
        let event = ShippedHooks::default_event(name).unwrap_or("post_*");
        let config_key = format!("hooks.{}", event);
        let path_str = path.to_string_lossy().to_string();

        if let Some(existing) = config.get(&config_key) {
            let mut commands = parse_command_value(existing);
            if !commands.contains(&path_str) {
                commands.push(path_str.clone());
                config.set(
                    &config_key,
                    format!(
                        "[{}]",
                        commands
                            .iter()
                            .map(|c| format!("\"{}\"", c))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                );
            }
        } else {
            config.set(&config_key, path_str);
        }

        installed.push((name.clone(), path));
    }

    config.save().map_err(|e| WerkError::IoError(e.to_string()))?;

    if output.is_structured() {
        let result: Vec<serde_json::Value> = installed
            .iter()
            .map(|(name, path)| {
                serde_json::json!({
                    "name": name,
                    "path": path.to_string_lossy(),
                    "installed": true,
                })
            })
            .collect();
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        for (name, path) in &installed {
            println!(
                "✓ Installed '{}' → {}",
                name,
                path.to_string_lossy()
            );
        }
    }

    Ok(())
}

fn cmd_hooks_install_git(output: &Output, workspace: &Workspace) -> Result<(), WerkError> {
    let repo_root = workspace.root();

    if GitHooks::is_installed(repo_root) {
        if output.is_structured() {
            output
                .print_structured(&serde_json::json!({
                    "git_hooks": "already_installed",
                    "hooks_path": ".githooks",
                }))
                .map_err(WerkError::IoError)?;
        } else {
            println!("✓ Git hooks already configured (core.hooksPath = .githooks)");
        }
        return Ok(());
    }

    GitHooks::install(repo_root).map_err(|e| WerkError::IoError(e))?;

    if output.is_structured() {
        output
            .print_structured(&serde_json::json!({
                "git_hooks": "installed",
                "hooks_path": ".githooks",
                "pre_commit": ".githooks/pre-commit",
            }))
            .map_err(WerkError::IoError)?;
    } else {
        println!("✓ Git hooks installed:");
        println!("  core.hooksPath = .githooks");
        println!("  .githooks/pre-commit (flush + stage + readme-tree)");
    }

    Ok(())
}
