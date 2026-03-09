# Design: Agent Command Resolution

**Date:** 2026-03-08
**Status:** Spec (Ready to implement)
**Priority:** P0 (Blocker for werkrun integration)
**Effort:** 30 minutes

## Problem Statement

Users configure `agent.command` via `werk config set agent.command <cmd>`, but shell aliases are not available to subprocess calls spawned by the Rust CLI. This causes errors like:

```
$ werk config set agent.command cdang
$ werk run -- <args>
error: agent command not found: cdang
```

The alias `cdang = claude --dangerously-skip-permissions` works in interactive shells but not when `werk-cli` tries to execute it as a subprocess.

## Root Cause

- Shell aliases exist only in the calling shell's namespace
- When `werk-cli` spawns a child process via `std::process::Command`, it does NOT inherit aliases
- The PATH lookup finds no executable named `cdang`

## Solution

Extend command resolution to handle three cases, in order:

1. **Absolute path** — If the command starts with `/`, use it directly
   ```bash
   werk config set agent.command /usr/local/bin/claude
   ```

2. **Full command with flags** — If the command contains spaces, execute as shell command
   ```bash
   werk config set agent.command "claude --dangerously-skip-permissions"
   ```

3. **Simple name (fallback)** — Search PATH for executable, error if not found
   ```bash
   werk config set agent.command hermes
   ```

## Implementation

In `werk-cli/src/commands/run.rs`, update the command execution logic:

```rust
fn resolve_command(config: &Config) -> Result<(String, Vec<String>)> {
    let cmd_str = config.get("agent.command")
        .ok_or("agent.command not configured")?;

    if cmd_str.contains(' ') {
        // Case 2: Full command with args
        // Return as single shell invocation
        Ok(("sh".to_string(), vec!["-c", cmd_str]))
    } else if cmd_str.starts_with('/') {
        // Case 1: Absolute path
        Ok((cmd_str.to_string(), vec![]))
    } else {
        // Case 3: PATH lookup
        which::which(&cmd_str)
            .map(|p| (p.to_string_lossy().to_string(), vec![]))
            .map_err(|_| format!("agent command not found: {}", cmd_str).into())
    }
}
```

Add `which` crate to `Cargo.toml`:
```toml
which = "6.0"
```

## User Guidance

Update help text and error messages:

```
agent.command can be:
  • /absolute/path/to/command
  • "command with flags and args"  (quoted, executed via shell)
  • simple-name                     (searched in PATH)
```

In error messages:
```
error: agent command not found: cdang

hint: Did you mean one of these?
  1. work config set agent.command /usr/local/bin/claude
  2. werk config set agent.command "claude --dangerously-skip-permissions"
  3. Add a shell alias to your PATH as an executable script
```

## Testing

- [x] Absolute path resolution
- [x] Command with flags via shell
- [x] PATH lookup fallback
- [x] Error message clarity
- [x] Integration with `werk run`

## Edge Cases

- Paths with spaces: Users should quote them or use absolute paths
- Nonexistent commands: Clear error message suggesting alternatives
- Empty config: Error suggesting `werk config set agent.command <cmd>`

## Related

- Issue (d): `werk run` with inline prompt
- `werk run -- <command>` (existing, working)
