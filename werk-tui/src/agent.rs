//! Agent execution for one-shot mode.

use std::io::Write;
use std::process::{Command, Stdio};

/// Execute a one-shot agent command and capture output.
///
/// The agent command is resolved from config (e.g. "hermes chat -Q -q").
/// The prompt is passed as the last CLI argument AND piped to stdin.
pub fn execute_agent_oneshot(agent_cmd: &str, prompt: &str) -> Result<String, String> {
    let parts: Vec<&str> = agent_cmd.split_whitespace().collect();
    if parts.is_empty() {
        return Err("empty agent command".to_string());
    }

    // Shell command: sh -c '$cmd "$1"' -- "$prompt"
    let shell_cmd = format!("{} \"$1\"", agent_cmd);
    let mut child = Command::new("sh")
        .args(["-c", &shell_cmd, "--", prompt])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn agent: {}", e))?;

    // Also pipe prompt to stdin as fallback
    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(prompt.as_bytes());
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("agent failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "agent exited {}: {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
