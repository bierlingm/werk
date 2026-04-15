//! `werk serve` — launch the Axum-backed web interface.
//!
//! Workspace selection is exclusive across three flags:
//! - `--global` / `-g` — target `~/.werk/` regardless of CWD
//! - `--daemon-target` — read the active path from `~/.werk/config.toml`
//!   (written by `werk daemon point`). This is what the installed launchd
//!   plist / systemd unit uses so that `werk daemon point` persists across
//!   daemon restarts and `daemon install --force` reinstalls without losing
//!   the operator's workspace selection.
//! - `--workspace-path <PATH>` — explicit path (for scripts and tests)
//!
//! Port selection is exclusive across two flags:
//! - `--port <N>` — single fixed port (default: `DEFAULT_PORT`)
//! - `--port-range start-end` — scan inclusively and bind the first free port
//!
//! On successful bind, writes the chosen port to `<werk_dir>/daemon.port` for
//! CLI introspection (`werk daemon status`). The browser extension rediscovers
//! via port probing since it can't read filesystem paths from the sandbox.

use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};

use werk_shared::Workspace;
use werk_shared::daemon_workspaces;

use crate::commands::{config_default, config_default_string};
use crate::error::WerkError;

// Re-export so call sites under `werk::commands::serve::*` keep working.
pub use werk_shared::daemon_net::{DEFAULT_PORT, DEFAULT_PORT_RANGE, PORT_FILE_NAME};

pub fn cmd_serve(
    port: Option<u16>,
    port_range: Option<String>,
    host: Option<String>,
    global: bool,
    daemon_target: bool,
    workspace_path: Option<PathBuf>,
) -> Result<(), WerkError> {
    let workspace = resolve_workspace(global, daemon_target, workspace_path)?;

    let host = host.unwrap_or_else(|| config_default_string("serve.host", "127.0.0.1"));
    let ip: IpAddr = host
        .parse()
        .map_err(|e| WerkError::IoError(format!("invalid host '{host}': {e}")))?;

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| WerkError::IoError(format!("failed to create runtime: {}", e)))?;

    rt.block_on(async {
        let (listener, bound_port) = match port_range.as_deref() {
            Some(range_str) => {
                let (start, end) = parse_range(range_str)?;
                bind_in_range(ip, start, end).await?
            }
            None => {
                let p = port.unwrap_or_else(|| config_default("serve.port", DEFAULT_PORT));
                let addr = SocketAddr::new(ip, p);
                let listener = tokio::net::TcpListener::bind(addr)
                    .await
                    .map_err(|e| WerkError::IoError(format!("bind {addr}: {e}")))?;
                (listener, p)
            }
        };

        write_port_file(workspace.werk_dir(), bound_port);

        let display_host = if host == "127.0.0.1" || host == "0.0.0.0" {
            "localhost"
        } else {
            host.as_str()
        };
        eprintln!("werk web → http://{display_host}:{bound_port}");
        eprintln!("           workspace: {}", workspace.root().display());

        werk_web::serve_on(workspace.root().to_path_buf(), listener)
            .await
            .map_err(|e| WerkError::IoError(e.to_string()))
    })
}

fn resolve_workspace(
    global: bool,
    daemon_target: bool,
    workspace_path: Option<PathBuf>,
) -> Result<Workspace, WerkError> {
    if global {
        return Workspace::global().map_err(|e| WerkError::IoError(e.to_string()));
    }
    if let Some(path) = workspace_path {
        return open_workspace_at(&path);
    }
    if daemon_target {
        let path = daemon_workspaces::active_path()?;
        return open_workspace_at(&path);
    }
    Workspace::discover().map_err(|e| WerkError::IoError(e.to_string()))
}

fn open_workspace_at(path: &Path) -> Result<Workspace, WerkError> {
    if !path.join(".werk").exists() {
        return Err(WerkError::IoError(format!(
            "{} is not a werk workspace (no .werk/ inside)",
            path.display()
        )));
    }
    Workspace::discover_from(path).map_err(|e| WerkError::IoError(e.to_string()))
}

/// Parse a "start-end" range string. End is inclusive.
pub fn parse_range(s: &str) -> Result<(u16, u16), WerkError> {
    let (start, end) = s
        .split_once('-')
        .ok_or_else(|| WerkError::IoError(format!("invalid port range '{s}': expected start-end")))?;
    let start: u16 = start
        .trim()
        .parse()
        .map_err(|e| WerkError::IoError(format!("invalid range start '{start}': {e}")))?;
    let end: u16 = end
        .trim()
        .parse()
        .map_err(|e| WerkError::IoError(format!("invalid range end '{end}': {e}")))?;
    if end < start {
        return Err(WerkError::IoError(format!(
            "invalid port range: end {end} < start {start}"
        )));
    }
    Ok((start, end))
}

/// Try to bind every port in [start, end] inclusive, returning the listener
/// on the first that succeeds along with the port number.
async fn bind_in_range(
    ip: IpAddr,
    start: u16,
    end: u16,
) -> Result<(tokio::net::TcpListener, u16), WerkError> {
    let mut last_err = None;
    for port in start..=end {
        let addr = SocketAddr::new(ip, port);
        match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => return Ok((listener, port)),
            Err(e) => last_err = Some((port, e)),
        }
    }
    let msg = match last_err {
        Some((p, e)) => format!("no free port in {start}-{end} (last tried {p}: {e})"),
        None => format!("empty range {start}-{end}"),
    };
    Err(WerkError::IoError(msg))
}

fn write_port_file(werk_dir: &Path, port: u16) {
    let path = werk_dir.join(PORT_FILE_NAME);
    if let Err(e) = std::fs::write(&path, format!("{port}\n")) {
        eprintln!("warning: failed to write {}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_range_ok() {
        assert_eq!(parse_range("3749-3759").unwrap(), (3749, 3759));
        assert_eq!(parse_range("8000-8000").unwrap(), (8000, 8000));
    }

    #[test]
    fn test_parse_range_reversed() {
        assert!(parse_range("3759-3749").is_err());
    }

    #[test]
    fn test_parse_range_malformed() {
        assert!(parse_range("3749").is_err());
        assert!(parse_range("abc-def").is_err());
    }
}
