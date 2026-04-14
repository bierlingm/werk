//! `werk daemon` — OS-supervised background `werk serve --global`.
//!
//! Two platform backends: launchd on macOS, systemd --user on Linux. Both
//! install a unit that runs `werk serve --global --port-range <range>` at
//! login, restarts it if it dies, and writes logs to `~/.werk/daemon.log`.
//!
//! The extension discovers the bound port by probing the range — we avoid
//! writing a .port file the extension can't read from the browser sandbox.

use std::path::PathBuf;

use werk_shared::Workspace;

use crate::commands::DaemonCommand;
use crate::commands::serve::{DEFAULT_PORT_RANGE, PORT_FILE_NAME, parse_range};
use crate::error::WerkError;
use crate::output::Output;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

use werk_shared::daemon_workspaces as workspace_config;

/// Label used for the launchd job and systemd unit.
pub const DAEMON_LABEL: &str = "dev.werk.daemon";

/// Log file name under `~/.werk/`.
pub const LOG_FILE: &str = "daemon.log";
pub const ERR_LOG_FILE: &str = "daemon.err.log";

pub fn cmd_daemon(output: &Output, command: DaemonCommand) -> Result<(), WerkError> {
    match command {
        DaemonCommand::Install { port_range, force } => install(output, port_range, force),
        DaemonCommand::Uninstall => uninstall(output),
        DaemonCommand::Status => status(output),
        DaemonCommand::Logs { lines, follow } => logs(output, lines, follow),
        DaemonCommand::Point { target, global } => point(output, target, global),
    }
}

fn point(
    output: &Output,
    target: Option<String>,
    global: bool,
) -> Result<(), WerkError> {
    use werk_shared::registry::Registry;

    if !global && target.is_none() {
        return Err(WerkError::IoError(
            "specify a registered name, a workspace path, or pass --global".into(),
        ));
    }

    let resolved = if global {
        dirs::home_dir()
            .ok_or_else(|| WerkError::IoError("cannot determine home directory".into()))?
    } else {
        let raw = target.unwrap();
        // Try registry first when the input looks like a bare name.
        let looks_like_name = !raw.contains('/') && !raw.starts_with('.') && !raw.starts_with('~');
        if looks_like_name {
            let reg = Registry::load()?;
            if let Some(entry) = reg.get(&raw) {
                entry.path
            } else {
                return Err(WerkError::IoError(format!(
                    "no registered space named '{raw}' (use `werk spaces list` or pass a path)"
                )));
            }
        } else {
            let abs = std::fs::canonicalize(&raw).map_err(|e| {
                WerkError::IoError(format!("cannot resolve {raw}: {e}"))
            })?;
            if !abs.join(".werk").exists() {
                return Err(WerkError::IoError(format!(
                    "{} is not a werk workspace (no .werk/ inside). Run `werk init` there first.",
                    abs.display()
                )));
            }
            abs
        }
    };

    let target = resolved;

    workspace_config::set_active(&target)?;

    #[cfg(target_os = "macos")]
    macos::restart()?;
    #[cfg(target_os = "linux")]
    linux::restart()?;
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = output;
        return Err(WerkError::IoError(
            "werk daemon is only supported on macOS and Linux".into(),
        ));
    }

    let _ = output.success(&format!(
        "daemon now serving {}",
        target.display()
    ));
    Ok(())
}

fn install(output: &Output, port_range: Option<String>, force: bool) -> Result<(), WerkError> {
    let range_str = port_range.unwrap_or_else(|| {
        format!("{}-{}", DEFAULT_PORT_RANGE.0, DEFAULT_PORT_RANGE.1)
    });
    // Validate the range up-front so we fail before touching launchd/systemd.
    let _ = parse_range(&range_str)?;

    let werk_dir = ensure_global_workspace()?;
    let exe = current_exe()?;

    #[cfg(target_os = "macos")]
    {
        macos::install(output, &exe, &werk_dir, &range_str, force)
    }
    #[cfg(target_os = "linux")]
    {
        linux::install(output, &exe, &werk_dir, &range_str, force)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (output, &exe, &werk_dir, &range_str, force);
        Err(WerkError::IoError(
            "werk daemon is only supported on macOS and Linux".into(),
        ))
    }
}

fn uninstall(output: &Output) -> Result<(), WerkError> {
    #[cfg(target_os = "macos")]
    {
        macos::uninstall(output)
    }
    #[cfg(target_os = "linux")]
    {
        linux::uninstall(output)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = output;
        Err(WerkError::IoError(
            "werk daemon is only supported on macOS and Linux".into(),
        ))
    }
}

fn status(output: &Output) -> Result<(), WerkError> {
    #[cfg(target_os = "macos")]
    {
        macos::status(output)
    }
    #[cfg(target_os = "linux")]
    {
        linux::status(output)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = output;
        Err(WerkError::IoError(
            "werk daemon is only supported on macOS and Linux".into(),
        ))
    }
}

fn logs(output: &Output, lines: usize, follow: bool) -> Result<(), WerkError> {
    let werk_dir = ensure_global_workspace()?;
    let log_path = werk_dir.join(LOG_FILE);
    if !log_path.exists() {
        let _ = output.info(&format!(
            "no log yet at {} — has the daemon started?",
            log_path.display()
        ));
        return Ok(());
    }

    let mut args: Vec<String> = Vec::new();
    args.push(format!("-n{}", lines));
    if follow {
        args.push("-f".into());
    }
    args.push(log_path.display().to_string());

    let status = std::process::Command::new("tail")
        .args(&args)
        .status()
        .map_err(|e| WerkError::IoError(format!("failed to exec tail: {e}")))?;
    if !status.success() {
        return Err(WerkError::IoError(format!("tail exited with {status}")));
    }
    Ok(())
}

fn ensure_global_workspace() -> Result<PathBuf, WerkError> {
    let ws =
        Workspace::global().map_err(|e| WerkError::IoError(format!(
            "global workspace (~/.werk/) not found: {e}. Run `werk init --global` first."
        )))?;
    Ok(ws.werk_dir().to_path_buf())
}

fn current_exe() -> Result<PathBuf, WerkError> {
    let exe = std::env::current_exe()
        .map_err(|e| WerkError::IoError(format!("cannot resolve current exe: {e}")))?;
    // Resolve symlinks so the plist/unit holds a stable absolute path.
    Ok(std::fs::canonicalize(&exe).unwrap_or(exe))
}

/// Read the port from `<werk_dir>/daemon.port`, if present.
///
/// Used by `status` as a convenience when the daemon is running and we want
/// to report which port it grabbed. The extension probes the range directly.
pub fn read_port_file(werk_dir: &std::path::Path) -> Option<u16> {
    let path = werk_dir.join(PORT_FILE_NAME);
    std::fs::read_to_string(path).ok().and_then(|s| s.trim().parse().ok())
}
