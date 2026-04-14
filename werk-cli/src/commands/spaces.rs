//! `werk spaces` — manage the registry of named werk spaces.
//!
//! The registry is a `[workspaces]` table in `~/.werk/config.toml`. See
//! `werk_shared::registry` for the schema and CRUD primitives.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use werk_shared::Workspace;
use werk_shared::registry::{GLOBAL_NAME, RegisteredWorkspace, Registry, global_entry};

use crate::commands::SpacesCommand;
use crate::error::WerkError;
use crate::output::Output;

/// Directories `scan` skips wholesale. Most are language/build artifacts; a
/// few are macOS noise that's never a real workspace.
const SCAN_EXCLUDES: &[&str] = &[
    ".git",
    ".cache",
    ".cargo",
    ".gradle",
    ".npm",
    ".rustup",
    ".Trash",
    ".venv",
    "Library",
    "Pictures",
    "Music",
    "__pycache__",
    "build",
    "dist",
    "node_modules",
    "target",
    "venv",
];

pub fn cmd_spaces(output: &Output, command: SpacesCommand) -> Result<(), WerkError> {
    match command {
        SpacesCommand::List { path } => list(output, path),
        SpacesCommand::Register { name, path } => register(output, &name, &path),
        SpacesCommand::Unregister { name } => unregister(output, &name),
        SpacesCommand::Create { name, path } => create(output, &name, &path),
        SpacesCommand::Scan { depth, register_all } => scan(output, depth, register_all),
        SpacesCommand::Rename { old, new } => rename(output, &old, &new),
    }
}

fn list(output: &Output, with_path: bool) -> Result<(), WerkError> {
    let reg = Registry::load()?;
    let entries = reg.list();
    let global = global_entry()?;

    if output.is_json() {
        let json = serde_json::json!({
            "global": entry_to_json(&global),
            "registered": entries.iter().map(entry_to_json).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
        return Ok(());
    }

    let mut rows: Vec<(String, String)> =
        vec![(global.name.clone(), global.path.display().to_string())];
    for e in &entries {
        let right = if with_path {
            e.path.display().to_string()
        } else {
            e.path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string()
        };
        rows.push((e.name.clone(), right));
    }
    print_two_col(&rows);
    if entries.is_empty() {
        println!();
        println!("(no other spaces registered)");
        println!("`werk spaces register <name> <path>` to add one");
        println!("`werk spaces scan` to find existing .werk/ dirs under your home");
    }
    Ok(())
}

fn register(output: &Output, name: &str, path: &Path) -> Result<(), WerkError> {
    let mut reg = Registry::load()?;
    let entry = reg.register(name, path)?;
    reg.save()?;
    let _ = output.success(&format!(
        "registered '{}' → {}",
        entry.name,
        entry.path.display()
    ));
    Ok(())
}

fn unregister(output: &Output, name: &str) -> Result<(), WerkError> {
    let mut reg = Registry::load()?;
    let removed = reg.unregister(name)?;
    if removed {
        reg.save()?;
        let _ = output.success(&format!("unregistered '{name}'"));
    } else {
        let _ = output.info(&format!("'{name}' was not registered (nothing to do)"));
    }
    Ok(())
}

fn create(output: &Output, name: &str, path: &Path) -> Result<(), WerkError> {
    Workspace::init(path, false).map_err(|e| WerkError::IoError(e.to_string()))?;
    register(output, name, path)
}

fn rename(output: &Output, old: &str, new: &str) -> Result<(), WerkError> {
    let mut reg = Registry::load()?;
    reg.rename(old, new)?;
    reg.save()?;
    let _ = output.success(&format!("renamed '{old}' → '{new}'"));
    Ok(())
}

fn scan(output: &Output, max_depth: usize, register_all: bool) -> Result<(), WerkError> {
    let home = dirs::home_dir()
        .ok_or_else(|| WerkError::IoError("cannot determine home directory".into()))?;
    let mut reg = Registry::load()?;

    let mut found: Vec<PathBuf> = Vec::new();
    walk(&home, max_depth, 0, &mut found);

    let global_path = home.clone();
    let mut registered: Vec<(String, PathBuf)> = Vec::new();
    let mut unregistered: Vec<PathBuf> = Vec::new();

    for werk_dir in found {
        let Some(root) = werk_dir.parent() else {
            continue;
        };
        let root = root.to_path_buf();
        if root == global_path {
            // ~/.werk → "global"; not interesting here.
            continue;
        }
        if let Some(existing) = reg.find_by_path(&root) {
            registered.push((existing.name, root));
        } else {
            unregistered.push(root);
        }
    }

    if output.is_json() {
        let json = serde_json::json!({
            "registered": registered.iter().map(|(name, path)| {
                serde_json::json!({"name": name, "path": path.display().to_string()})
            }).collect::<Vec<_>>(),
            "unregistered": unregistered.iter().map(|p| {
                let derived = derive_name(p);
                serde_json::json!({"path": p.display().to_string(), "suggested_name": derived})
            }).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
        return Ok(());
    }

    println!("registered ({}):", registered.len());
    let reg_rows: Vec<(String, String)> = registered
        .iter()
        .map(|(n, p)| (n.clone(), p.display().to_string()))
        .collect();
    print_two_col_indent(&reg_rows, 2);
    println!();
    println!("unregistered ({}):", unregistered.len());
    let unreg_rows: Vec<(String, String)> = unregistered
        .iter()
        .map(|p| (format!("?{}", derive_name(p)), p.display().to_string()))
        .collect();
    print_two_col_indent(&unreg_rows, 2);
    if !unregistered.is_empty() && !register_all {
        println!();
        println!("re-run with --register-all to add all unregistered hits");
    }

    if register_all && !unregistered.is_empty() {
        println!();
        println!("registering...");
        let mut taken: HashSet<String> =
            reg.list().into_iter().map(|e| e.name).collect();
        taken.insert(GLOBAL_NAME.to_string());
        for path in &unregistered {
            let name = unique_name(&derive_name(path), &taken);
            match reg.register(&name, path) {
                Ok(e) => {
                    println!("  ✓ {} → {}", e.name, e.path.display());
                    taken.insert(e.name);
                }
                Err(e) => {
                    println!("  ✗ {}: {e}", path.display());
                }
            }
        }
        reg.save()?;
    }

    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────

/// Print rows as a two-column table. Column 1 width adapts to the widest
/// entry plus 2-space gutter; never less than 12.
fn print_two_col(rows: &[(String, String)]) {
    print_two_col_indent(rows, 0);
}

fn print_two_col_indent(rows: &[(String, String)], indent: usize) {
    let width = rows.iter().map(|(a, _)| a.len()).max().unwrap_or(12).max(12) + 2;
    let pad = " ".repeat(indent);
    for (a, b) in rows {
        println!("{pad}{a:<width$}{b}", a = a, b = b, width = width);
    }
}


fn walk(dir: &Path, max_depth: usize, current_depth: usize, out: &mut Vec<PathBuf>) {
    if current_depth > max_depth {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if SCAN_EXCLUDES.contains(&name) {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        if name == ".werk" {
            // Found one. Don't recurse into it.
            out.push(path);
            continue;
        }
        walk(&path, max_depth, current_depth + 1, out);
    }
}

/// Derive a registry name from a workspace path. Lowercase basename with any
/// non-ascii-alnum-or-dash collapsed to '-'. Empty results fall back to "ws".
fn derive_name(path: &Path) -> String {
    let basename = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut out = String::with_capacity(basename.len());
    for c in basename.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-').trim_matches('_').to_string();
    if trimmed.is_empty() || trimmed == GLOBAL_NAME {
        return "ws".to_string();
    }
    trimmed
}

/// Resolve a name collision by appending -2, -3, … until unique.
fn unique_name(base: &str, taken: &HashSet<String>) -> String {
    if !taken.contains(base) {
        return base.to_string();
    }
    let mut n = 2;
    loop {
        let candidate = format!("{base}-{n}");
        if !taken.contains(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

fn entry_to_json(e: &RegisteredWorkspace) -> serde_json::Value {
    serde_json::json!({
        "name": e.name,
        "path": e.path.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_name_basic() {
        assert_eq!(derive_name(Path::new("/foo/werk")), "werk");
        assert_eq!(derive_name(Path::new("/foo/Bar Baz")), "bar-baz");
        assert_eq!(derive_name(Path::new("/foo/desk-werk")), "desk-werk");
    }

    #[test]
    fn test_derive_name_global_falls_back() {
        // The literal name "global" is reserved.
        assert_eq!(derive_name(Path::new("/.werk-but-named/global")), "ws");
    }

    #[test]
    fn test_derive_name_empty_falls_back() {
        assert_eq!(derive_name(Path::new("/")), "ws");
        assert_eq!(derive_name(Path::new("///")), "ws");
    }

    #[test]
    fn test_unique_name() {
        let mut taken = HashSet::new();
        assert_eq!(unique_name("werk", &taken), "werk");
        taken.insert("werk".to_string());
        assert_eq!(unique_name("werk", &taken), "werk-2");
        taken.insert("werk-2".to_string());
        assert_eq!(unique_name("werk", &taken), "werk-3");
    }
}
