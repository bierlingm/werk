//! launchd backend for `werk daemon`.
//!
//! Writes a user-scoped LaunchAgent to `~/Library/LaunchAgents/dev.werk.daemon.plist`
//! that runs `werk serve --global --port-range <range>`. Bootstraps it into
//! the user's GUI launchd domain so it starts at login and respawns on crash.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::commands::daemon::{
    DAEMON_LABEL, ERR_LOG_FILE, LOG_FILE, ensure_global_workspace, read_port_file,
};
use crate::error::WerkError;
use crate::output::Output;

fn plist_path() -> Result<PathBuf, WerkError> {
    let home = dirs::home_dir()
        .ok_or_else(|| WerkError::IoError("cannot determine home directory".into()))?;
    Ok(home.join("Library/LaunchAgents").join(format!("{DAEMON_LABEL}.plist")))
}

fn uid() -> Result<u32, WerkError> {
    let out = Command::new("id")
        .arg("-u")
        .output()
        .map_err(|e| WerkError::IoError(format!("exec id: {e}")))?;
    if !out.status.success() {
        return Err(WerkError::IoError("id -u failed".into()));
    }
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<u32>()
        .map_err(|e| WerkError::IoError(format!("parse uid: {e}")))
}

fn service_target() -> Result<String, WerkError> {
    Ok(format!("gui/{}/{}", uid()?, DAEMON_LABEL))
}

fn domain_target() -> Result<String, WerkError> {
    Ok(format!("gui/{}", uid()?))
}

pub fn install(
    output: &Output,
    exe: &Path,
    werk_dir: &Path,
    range_str: &str,
    force: bool,
) -> Result<(), WerkError> {
    let plist = plist_path()?;

    if plist.exists() && !force {
        return Err(WerkError::IoError(format!(
            "{} already exists. Pass --force to overwrite, or `werk daemon uninstall` first.",
            plist.display()
        )));
    }

    // Ensure parent dir exists (Library/LaunchAgents may not on a fresh install).
    if let Some(parent) = plist.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| WerkError::IoError(format!("create {}: {e}", parent.display())))?;
    }

    // If already loaded, boot it out before we rewrite + re-bootstrap.
    if is_loaded()? {
        let _ = bootout();
    }

    let contents = render_plist(exe, werk_dir, range_str)?;
    std::fs::write(&plist, contents)
        .map_err(|e| WerkError::IoError(format!("write {}: {e}", plist.display())))?;

    bootstrap(&plist)?;

    let _ = output.success(&format!(
        "installed {DAEMON_LABEL} → {}",
        plist.display()
    ));

    // Small convenience: if the port file lands within a couple seconds, report it.
    if let Some(port) = wait_for_port_file(werk_dir, 2) {
        let _ = output.info(&format!("listening on http://127.0.0.1:{port}"));
    } else {
        let _ = output.info("daemon starting; run `werk daemon status` to confirm.");
    }
    Ok(())
}

pub fn uninstall(output: &Output) -> Result<(), WerkError> {
    let plist = plist_path()?;
    if is_loaded()? {
        bootout()?;
    }
    if plist.exists() {
        std::fs::remove_file(&plist)
            .map_err(|e| WerkError::IoError(format!("remove {}: {e}", plist.display())))?;
    }
    let _ = output.success(&format!("{DAEMON_LABEL} uninstalled."));
    Ok(())
}

pub fn status(output: &Output) -> Result<(), WerkError> {
    let loaded = is_loaded()?;
    let werk_dir = ensure_global_workspace()?;
    let port = read_port_file(&werk_dir);

    let plist = plist_path()?;
    let plist_present = plist.exists();

    if output.is_json() {
        let json = serde_json::json!({
            "loaded": loaded,
            "label": DAEMON_LABEL,
            "plist": plist.display().to_string(),
            "plist_present": plist_present,
            "port": port,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
        return Ok(());
    }

    println!("{DAEMON_LABEL}");
    println!("  plist:   {}", plist.display());
    println!("  loaded:  {}", if loaded { "yes" } else { "no" });
    match port {
        Some(p) => println!("  port:    {p} (http://127.0.0.1:{p})"),
        None => println!("  port:    unknown (no daemon.port file yet)"),
    }
    Ok(())
}

fn render_plist(exe: &Path, werk_dir: &Path, range_str: &str) -> Result<String, WerkError> {
    let log = werk_dir.join(LOG_FILE);
    let err_log = werk_dir.join(ERR_LOG_FILE);
    let exe_s = xml_escape(&exe.display().to_string());
    let range_s = xml_escape(range_str);
    let log_s = xml_escape(&log.display().to_string());
    let err_s = xml_escape(&err_log.display().to_string());

    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{DAEMON_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe_s}</string>
        <string>serve</string>
        <string>--daemon-target</string>
        <string>--port-range</string>
        <string>{range_s}</string>
        <string>--host</string>
        <string>127.0.0.1</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>ProcessType</key>
    <string>Background</string>
    <key>StandardOutPath</key>
    <string>{log_s}</string>
    <key>StandardErrorPath</key>
    <string>{err_s}</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin</string>
    </dict>
</dict>
</plist>
"#,
    ))
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn bootstrap(plist: &Path) -> Result<(), WerkError> {
    let domain = domain_target()?;
    let out = Command::new("launchctl")
        .args(["bootstrap", &domain, &plist.display().to_string()])
        .output()
        .map_err(|e| WerkError::IoError(format!("exec launchctl: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(WerkError::IoError(format!(
            "launchctl bootstrap failed: {}",
            stderr.trim()
        )));
    }
    Ok(())
}

fn bootout() -> Result<(), WerkError> {
    let service = service_target()?;
    // Ignore failure — bootout on a not-loaded service is a no-op for us.
    let _ = Command::new("launchctl")
        .args(["bootout", &service])
        .output();
    Ok(())
}

/// Restart the running daemon job. No-op if not loaded.
pub fn restart() -> Result<(), WerkError> {
    let service = service_target()?;
    let out = Command::new("launchctl")
        .args(["kickstart", "-k", &service])
        .output()
        .map_err(|e| WerkError::IoError(format!("exec launchctl: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(WerkError::IoError(format!(
            "launchctl kickstart failed: {}",
            stderr.trim()
        )));
    }
    Ok(())
}

fn is_loaded() -> Result<bool, WerkError> {
    let service = service_target()?;
    let out = Command::new("launchctl")
        .args(["print", &service])
        .output()
        .map_err(|e| WerkError::IoError(format!("exec launchctl: {e}")))?;
    Ok(out.status.success())
}

fn wait_for_port_file(werk_dir: &Path, secs: u64) -> Option<u16> {
    use std::time::{Duration, Instant};
    let deadline = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < deadline {
        if let Some(p) = read_port_file(werk_dir) {
            return Some(p);
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    None
}
