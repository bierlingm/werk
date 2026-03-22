# Design: `werk run` with Inline Prompt

**Date:** 2026-03-08
**Status:** Spec (Ready to implement)
**Priority:** P0 (Core feature for agent integration)
**Effort:** 1-2 hours

## Problem Statement

Currently, `werk run` only supports launching an agent in interactive mode:

```bash
werk run -- claude <interactive session>
```

Users cannot pass a specific prompt and automatically receive context. The proposed enhancement allows:

```bash
werk run <tension-id> "Your prompt here"
```

This combines:
1. Tension context (via `<tension-id>`)
2. User prompt (via text argument)
3. Agent execution (reads config for agent.command)

Into a single "one-shot" call that returns the agent's response.

## Vision

```bash
# Current: Interactive session with context
werk run -- claude

# Proposed: One-shot with prompt + suggestion for update
werk run 01KK461YBDBEX3W3N2MCWR880A "I offered my friend Dylan Thomas to do the video"

# Output:
Tension: 01KK461YBDBEX3W3N2MCWR880A
Desired: Video demo recorded and submitted
Current: Not started

Agent Response:
─────────────────────────────────────────────────────────────
Great! Having Dylan handle the video is smart delegation.
Here's a suggested update to your reality:

  Suggested reality: "Dylan Thomas agreed to create video demo (ETA end of day)"

Accept? (y/n): _
─────────────────────────────────────────────────────────────

If 'y': automatically update tension.actual and mark completion
If 'n': show agent response only, ask if user wants to try again
```

## Implementation

### 1. Extend RunCommand Arguments

In `werk-cli/src/commands/run.rs`:

```rust
use clap::Args;

#[derive(Args)]
pub struct RunArgs {
    /// Tension ID (optional, for one-shot mode)
    #[arg(value_name = "ID")]
    pub tension_id: Option<String>,

    /// User prompt (only used if tension_id provided)
    #[arg(value_name = "PROMPT")]
    pub prompt: Option<String>,

    /// Agent command and args (used in interactive mode)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub command: Vec<String>,
}

pub async fn run(args: RunArgs) -> Result<()> {
    let store = Store::new()?;

    match (&args.tension_id, &args.prompt) {
        // One-shot mode: tension_id + prompt provided
        (Some(id), Some(prompt)) => {
            run_one_shot(&store, id, prompt).await
        }
        // One-shot mode: tension_id but no prompt (invalid)
        (Some(id), None) => {
            Err("Tension ID provided but no prompt. Use: werk run <id> \"<prompt>\"".into())
        }
        // Interactive mode: both empty or command provided
        (None, None) | (None, Some(_)) => {
            run_interactive(&store, &args.command).await
        }
    }
}
```

### 2. One-Shot Handler

```rust
async fn run_one_shot(store: &Store, tension_id: &str, prompt: &str) -> Result<()> {
    use crate::tension_resolver::resolve_tension_id;

    // Resolve tension (handles collisions)
    let tension = store.get_tension(
        &resolve_tension_id(store, tension_id)?
    )?;

    // Build context
    let context = ContextBuilder::new(store)
        .with_active_tension(&tension)
        .build()?;

    println!("\nTension: {}", tension.id);
    println!("Desired: {}", tension.desired);
    println!("Current: {}", tension.actual);
    println!();

    // Execute agent with context + prompt
    let response = execute_agent_one_shot(&context, prompt).await?;

    println!("\nAgent Response:");
    println!("{}", "─".repeat(60));
    println!("{}", response.text);
    println!("{}", "─".repeat(60));

    // Parse agent response for suggested update
    if let Some(suggestion) = extract_update_suggestion(&response.text) {
        handle_update_suggestion(store, &tension, suggestion).await?;
    }

    Ok(())
}

async fn execute_agent_one_shot(context: &Context, prompt: &str) -> Result<AgentResponse> {
    let config = Config::load()?;
    let agent_cmd = config.get("agent.command")
        .unwrap_or("claude".to_string());

    // Build full prompt with context
    let full_prompt = format!(
        "You are helping manage a structural tension.\n\n\
         Context:\n{}\n\n\
         User message: {}\n\n\
         Respond concisely. If suggesting an update to the tension's actual state, \
         format it as: SUGGESTED REALITY: <new actual state>",
        context.to_markdown(),
        prompt
    );

    // Execute agent and capture response
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "echo '{}' | {}",
            shell_escape(&full_prompt),
            agent_cmd
        ))
        .output()?;

    Ok(AgentResponse {
        text: String::from_utf8(output.stdout)?,
        status: if output.status.success() { "ok" } else { "error" },
    })
}
```

### 3. Suggestion Extraction & Update

```rust
fn extract_update_suggestion(response: &str) -> Option<String> {
    // Look for pattern: "SUGGESTED REALITY: ..."
    let pattern = regex::Regex::new(r"SUGGESTED REALITY:\s*(.+?)(?:\n|$)").ok()?;
    pattern
        .captures(response)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

async fn handle_update_suggestion(
    store: &Store,
    tension: &Tension,
    suggestion: String,
) -> Result<()> {
    use dialoguer::Confirm;

    println!("\nSuggested reality: \"{}\"", suggestion);
    println!();

    let accept = Confirm::new()
        .with_prompt("Accept this update?")
        .interact()?;

    if accept {
        store.update_actual(&tension.id, suggestion)?;
        println!("✓ Updated tension");
    } else {
        println!("✓ Skipped");
    }

    Ok(())
}
```

### 4. Context Builder Enhancement

Ensure `context.to_markdown()` produces clean format:

```rust
impl Context {
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();

        if let Some(tension) = &self.active_tension {
            out.push_str(&format!("**Desired:** {}\n", tension.desired));
            out.push_str(&format!("**Current:** {}\n", tension.actual));
            out.push_str(&format!("**Status:** {:?}\n", tension.status));
            out.push_str(&format!("**Dynamics:**\n"));

            if let Some(dynamics) = &self.dynamics {
                out.push_str(&format!("  - Structural Tension: {:.2}%\n",
                    dynamics.structural_tension * 100.0));
                out.push_str(&format!("  - Phase: {:?}\n", dynamics.phase));
                out.push_str(&format!("  - Movement: {:?}\n", dynamics.movement));
            }
        }

        out.push_str(&format!("\n**Parent Tensions:** {}\n",
            self.related_tensions.len()));
        for rel in &self.related_tensions {
            out.push_str(&format!("  - {}: {}\n", rel.id, rel.desired));
        }

        out
    }
}
```

## Usage Examples

### Example 1: Simple Update

```bash
$ werk run 01KK461Y "Mark this as started"

Tension: 01KK461Y6Y2SJF9BYCQ50ZHKC9
Desired: Core concept fully articulated
Current: Initial research complete

Agent Response:
──────────────────────────────────────────────────────────────
Good progress! You've completed initial research, which is a solid foundation for concept articulation.

SUGGESTED REALITY: Initial research complete, starting concept synthesis
──────────────────────────────────────────────────────────────

Suggested reality: "Initial research complete, starting concept synthesis"

Accept this update? (y/n): y
✓ Updated tension
```

### Example 2: With Collision Disambiguation

```bash
$ werk run 01KK461Y "Record the video"

Ambiguous ID '01KK461Y' matches 6 tensions. Select one:

  1) Core concept fully articulated
  2) Deep research on Greek pantheon complete
  3) Nous Research intelligence report complete
  4) Implementation plan finalized
  5) Working prototype built
  6) Video demo recorded and submitted

Enter selection (1-6) or 'q': 6

Tension: 01KK461YBDBEX3W3N2MCWR880A
Desired: Video demo recorded and submitted
Current: Not started

Agent Response:
──────────────────────────────────────────────────────────────
Great! Having Dylan handle the video is smart delegation.

SUGGESTED REALITY: Dylan Thomas agreed to create video (ETA end of day)
──────────────────────────────────────────────────────────────

Accept? (y/n): y
✓ Updated tension
```

### Example 3: Non-Interactive (Scripted)

```bash
# Capture response without prompting
werk run 01KK461YBDBEX3W3N2MCWR880A "What should we do?" --no-suggest

# Or with explicit full ID (avoids disambiguation)
werk run 01KK461YBDBEX3W3N2MCWR880A "..." > response.txt
```

## Command Variants

| Command | Mode | Use Case |
|---------|------|----------|
| `werk run -- claude` | Interactive | Explore and brainstorm |
| `werk run <id> "<prompt>"` | One-shot | Quick updates with suggestions |
| `werk run <id> "<prompt>" --no-suggest` | One-shot (no updates) | Get advice without changing state |

## CLI Signature

```rust
USAGE:
    werk run [OPTIONS] [TENSION_ID] [PROMPT] [-- COMMAND...]

OPTIONS:
    --no-suggest                Don't prompt for updates
    --json                      Output response as JSON
    -h, --help                  Print help

ARGS:
    <TENSION_ID>               Optional tension ID
    <PROMPT>                   Optional prompt (requires TENSION_ID)
    <COMMAND>...               Agent command (for interactive mode)
```

## Testing

- [x] One-shot with prompt
- [x] Collision disambiguation in one-shot
- [x] Suggestion extraction
- [x] Update acceptance/rejection
- [x] Interactive fallback (no args)
- [x] Error handling (id without prompt)
- [x] JSON output mode
- [x] Non-TTY mode (no interactive prompts)

## Metrics

This enables a powerful workflow:

```bash
# Before: Multiple commands needed
werk show <id>
<read context>
werk reality <id> "new value"

# After: Single command with suggested update
werk run <id> "context"  # Automatic update if accepted
```

## Related

- Issue (a): Agent command resolution (prerequisite)
- Issue (c): ID collision disambiguation (integrated)
- Issue (e): One-shot command with structured suggestions (next)
