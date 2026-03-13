use std::io::Write;

/// Execute agent command and capture its stdout.
///
/// This runs on a background thread via Cmd::task().
pub fn execute_agent_capture(agent_cmd: &str, prompt: &str) -> std::result::Result<String, String> {
    let (program, args) = resolve_agent_command(agent_cmd)?;

    let mut child = std::process::Command::new(&program)
        .args(&args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                format!("agent command not found: {}", program)
            } else {
                format!("failed to spawn agent: {}", e)
            }
        })?;

    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(prompt.as_bytes());
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("failed to read agent output: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "agent command failed (exit {}): {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Resolve an agent command string into (program, args).
pub fn resolve_agent_command(cmd: &str) -> std::result::Result<(String, Vec<String>), String> {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return Err("agent command is empty".to_string());
    }

    if cmd.starts_with('/') {
        if !std::path::Path::new(cmd).exists() {
            return Err(format!("agent command not found at path: {}", cmd));
        }
        Ok((cmd.to_string(), vec![]))
    } else if cmd.contains(' ') {
        Ok((
            "sh".to_string(),
            vec!["-c".to_string(), cmd.to_string()],
        ))
    } else {
        match which::which(cmd) {
            Ok(path) => Ok((path.to_string_lossy().to_string(), vec![])),
            Err(_) => Err(format!("agent command not found: {}", cmd)),
        }
    }
}
