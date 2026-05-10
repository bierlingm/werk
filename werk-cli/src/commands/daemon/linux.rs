//! systemd --user backend for `werk daemon`.
//!
//! Writes a user unit to `~/.config/systemd/user/werk-daemon.service` that
//! runs `werk serve --global --port-range <range>`. Enables + starts it so it
//! survives login sessions (with lingering enabled) and respawns on crash.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::commands::daemon::{
    DAEMON_LABEL, ERR_LOG_FILE, LOG_FILE, ensure_global_workspace, read_port_file,
};
use crate::error::WerkError;
use crate::output::Output;

const UNIT_NAME: &str = "werk-daemon.service";

fn unit_path() -> Result<PathBuf, WerkError> {
    let home = dirs::home_dir()
        .ok_or_else(|| WerkError::IoError("cannot determine home directory".into()))?;
    Ok(home.join(".config/systemd/user").join(UNIT_NAME))
}

pub fn install(
    output: &Output,
    exe: &Path,
    werk_dir: &Path,
    range_str: &str,
    force: bool,
) -> Result<(), WerkError> {
    let unit = unit_path()?;

    if unit.exists() && !force {
        return Err(WerkError::IoError(format!(
            "{} already exists. Pass --force to overwrite, or `werk daemon uninstall` first.",
            unit.display()
        )));
    }

    if let Some(parent) = unit.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| WerkError::IoError(format!("create {}: {e}", parent.display())))?;
    }

    let contents = render_unit(exe, werk_dir, range_str);
    std::fs::write(&unit, contents)
        .map_err(|e| WerkError::IoError(format!("write {}: {e}", unit.display())))?;

    run_systemctl(&["daemon-reload"])?;
    run_systemctl(&["enable", UNIT_NAME])?;
    // Restart handles both "not started" and "already running with stale config" cases.
    run_systemctl(&["restart", UNIT_NAME])?;

    let _ = output.success(&format!("installed {DAEMON_LABEL} → {}", unit.display()));
    let _ = output.info("daemon started. `werk daemon status` to confirm.");
    Ok(())
}

pub fn uninstall(output: &Output) -> Result<(), WerkError> {
    // Best-effort stop + disable; ignore errors because the unit may already be gone.
    let _ = Command::new("systemctl")
        .args(["--user", "stop", UNIT_NAME])
        .output();
    let _ = Command::new("systemctl")
        .args(["--user", "disable", UNIT_NAME])
        .output();

    let unit = unit_path()?;
    if unit.exists() {
        std::fs::remove_file(&unit)
            .map_err(|e| WerkError::IoError(format!("remove {}: {e}", unit.display())))?;
    }
    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output();

    let _ = output.success(&format!("{DAEMON_LABEL} uninstalled."));
    Ok(())
}

pub fn status(output: &Output) -> Result<(), WerkError> {
    let active = is_active();
    let werk_dir = ensure_global_workspace()?;
    let port = read_port_file(&werk_dir);
    let unit = unit_path()?;

    if output.is_json() {
        let json = serde_json::json!({
            "active": active,
            "label": DAEMON_LABEL,
            "unit": unit.display().to_string(),
            "unit_present": unit.exists(),
            "port": port,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
        return Ok(());
    }

    println!("{DAEMON_LABEL}");
    println!("  unit:    {}", unit.display());
    println!("  active:  {}", if active { "yes" } else { "no" });
    match port {
        Some(p) => println!("  port:    {p} (http://127.0.0.1:{p})"),
        None => println!("  port:    unknown (no daemon.port file yet)"),
    }
    Ok(())
}

fn render_unit(exe: &Path, werk_dir: &Path, range_str: &str) -> String {
    let log = werk_dir.join(LOG_FILE);
    let err_log = werk_dir.join(ERR_LOG_FILE);
    format!(
        "[Unit]\n\
         Description=werk background server (global workspace)\n\
         After=network.target\n\
         \n\
         [Service]\n\
         Type=simple\n\
         ExecStart={exe} serve --daemon-target --port-range {range} --host 127.0.0.1\n\
         Restart=on-failure\n\
         RestartSec=3\n\
         StandardOutput=append:{log}\n\
         StandardError=append:{err_log}\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n",
        exe = exe.display(),
        range = range_str,
        log = log.display(),
        err_log = err_log.display(),
    )
}

fn run_systemctl(args: &[&str]) -> Result<(), WerkError> {
    let mut cmd = Command::new("systemctl");
    cmd.arg("--user").args(args);
    let out = cmd
        .output()
        .map_err(|e| WerkError::IoError(format!("exec systemctl: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(WerkError::IoError(format!(
            "systemctl --user {:?} failed: {}",
            args,
            stderr.trim()
        )));
    }
    Ok(())
}

/// Restart the running daemon unit.
pub fn restart() -> Result<(), WerkError> {
    run_systemctl(&["restart", UNIT_NAME])
}

fn is_active() -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", UNIT_NAME])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
