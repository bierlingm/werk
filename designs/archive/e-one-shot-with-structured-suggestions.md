# Design: One-Shot Command with Structured Suggestions

**Date:** 2026-03-08
**Status:** Concept (Future enhancement to design d)
**Priority:** P1 (High-value, can follow directly after design d)
**Effort:** 2-3 hours

## Problem Statement

In design (d), we proposed `werk run <id> "<prompt>"` which returns agent advice and optionally suggests a single `.actual` update.

However, users often need richer interactions:

```
Current reality: "Not started"
Desired: "Video demo recorded and submitted"

Prompt: "I've delegated this to Dylan. What should we track now?"

Agent Response:
"Smart delegation! Track Dylan's progress separately:
  1. Create child tension: 'Dylan video quality acceptable'
  2. Update this to: 'Delegated to Dylan Thomas (ETA Saturday)'
  3. Add note: 'Dylan email confirmed Tuesday'"
```

Currently, the agent gives advice in prose, and the user must manually:
1. Create a child tension
2. Update this one
3. Add a note

**Vision:** Let the agent suggest *structured* mutations to the entire tension forest, which the user can accept/modify/apply.

## Solution: Structured Suggestion Format

Instead of free prose, agents in "one-shot" mode receive a prompt asking them to return **structured suggestions** as YAML:

```
---
# Suggested mutations to the user's tension forest
mutations:
  - action: "update_actual"
    tension_id: "<parent-id>"  # This tension
    new_value: "Delegated to Dylan Thomas (ETA Saturday)"
    reasoning: "Tracks delegation explicitly"

  - action: "create_child"
    parent_id: "<parent-id>"
    desired: "Dylan's video meets quality standards"
    actual: "Waiting for Dylan to confirm progress"
    reasoning: "Separate track for delegated work"

  - action: "add_note"
    tension_id: "<parent-id>"
    text: "Dylan confirmed via email Tuesday; ETA Saturday EOD"

  - action: "update_status"
    tension_id: "<parent-id>"
    new_status: "Active"
    reasoning: "Still driving the outcome (monitoring Dylan)"

response: |
  Smart delegation approach! Here's what I'd suggest:

  1. Track Dylan's work as a child tension so you can monitor
  2. Update this tension to show it's delegated
  3. Note the communication and timeline

  This keeps you connected to the outcome while respecting Dylan's autonomy.
---
```

## Implementation

### 1. Agent Prompt Enhancement

In `execute_agent_one_shot()` from design (d), extend instructions:

```rust
let full_prompt = format!(
    "You are helping manage a structural tension.\n\n\
     Context:\n{}\n\n\
     User message: {}\n\n\
     \
     IMPORTANT: Respond in YAML format with two sections:\n\
     1. 'mutations' array: suggested changes to the tension forest\n\
     2. 'response' string: your advice in prose\n\n\
     Supported mutation actions:\n\
     - update_actual: {{tension_id, new_value, reasoning}}\n\
     - create_child: {{parent_id, desired, actual, reasoning}}\n\
     - add_note: {{tension_id, text}}\n\
     - update_status: {{tension_id, new_status, reasoning}}\n\
     - update_desired: {{tension_id, new_value, reasoning}}\n\n\
     Only suggest mutations you're confident about. \
     If nothing should change, return empty mutations: [].\n\n\
     Example:\n\
     ---\n\
     mutations:\n\
       - action: update_actual\n\
         tension_id: 01KK461Y6Y2SJF9BYCQ50ZHKC9\n\
         new_value: \"Research phase complete\"\n\
         reasoning: \"Progress has been made\"\n\n\
     response: |\n\
       Solid work on the research.\n\
     ---",
    context.to_markdown(),
    prompt
);
```

### 2. Structured Response Parsing

Create new module `werk-cli/src/agent_response.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredResponse {
    pub mutations: Vec<Mutation>,
    pub response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Mutation {
    UpdateActual {
        tension_id: String,
        new_value: String,
        reasoning: String,
    },
    CreateChild {
        parent_id: String,
        desired: String,
        actual: String,
        reasoning: String,
    },
    AddNote {
        tension_id: String,
        text: String,
    },
    UpdateStatus {
        tension_id: String,
        new_status: String,
        reasoning: String,
    },
    UpdateDesired {
        tension_id: String,
        new_value: String,
        reasoning: String,
    },
}

impl StructuredResponse {
    pub fn from_yaml(text: &str) -> Result<Self> {
        serde_yaml::from_str(text)
            .map_err(|e| format!("Failed to parse agent response: {}", e).into())
    }
}
```

### 3. Response Handler

```rust
async fn handle_structured_suggestions(
    store: &Store,
    tension: &Tension,
    response: StructuredResponse,
) -> Result<()> {
    // Show human-readable advice
    println!("\nAgent Response:");
    println!("{}", "─".repeat(60));
    println!("{}", response.response);
    println!("{}", "─".repeat(60));

    // Show suggested mutations
    if response.mutations.is_empty() {
        println!("\n(No structural changes suggested)");
        return Ok(());
    }

    println!("\nSuggested Changes:\n");
    display_mutations(&response.mutations);

    // Ask user to accept/review
    let action = dialoguer::Select::new()
        .items(&["Apply all", "Review each", "Cancel"])
        .default(0)
        .interact()?;

    match action {
        0 => apply_mutations(store, tension, response.mutations).await,
        1 => review_mutations_interactively(store, tension, response.mutations).await,
        _ => {
            println!("Cancelled.");
            Ok(())
        }
    }
}

fn display_mutations(mutations: &[Mutation]) {
    for (i, mutation) in mutations.iter().enumerate() {
        println!("{}. {}", i + 1, mutation.summary());
    }
}

impl Mutation {
    fn summary(&self) -> String {
        match self {
            Mutation::UpdateActual { new_value, .. } =>
                format!("Update actual state to: \"{}\"", new_value),
            Mutation::CreateChild { desired, .. } =>
                format!("Create child tension: \"{}\"", desired),
            Mutation::AddNote { text, .. } =>
                format!("Add note: \"{}\"", text),
            Mutation::UpdateStatus { new_status, .. } =>
                format!("Mark as: {}", new_status),
            Mutation::UpdateDesired { new_value, .. } =>
                format!("Update desired to: \"{}\"", new_value),
        }
    }
}
```

### 4. Interactive Review

```rust
async fn review_mutations_interactively(
    store: &Store,
    tension: &Tension,
    mutations: Vec<Mutation>,
) -> Result<()> {
    use dialoguer::Confirm;

    let mut applied = vec![];

    for (i, mutation) in mutations.iter().enumerate() {
        println!("\nMutation {}/{}:", i + 1, mutations.len());
        println!("  {}", mutation.detailed_description());

        if let Mutation::UpdateActual { reasoning, .. } = mutation {
            println!("  Why: {}", reasoning);
        }

        let accept = Confirm::new()
            .with_prompt("Accept this change?")
            .interact()?;

        if accept {
            applied.push(mutation.clone());
        }
    }

    if !applied.is_empty() {
        println!("\nApplying {} changes...", applied.len());
        apply_mutations(store, tension, applied).await?;
    } else {
        println!("No mutations applied.");
    }

    Ok(())
}

async fn apply_mutations(
    store: &Store,
    tension: &Tension,
    mutations: Vec<Mutation>,
) -> Result<()> {
    for mutation in mutations {
        apply_mutation(store, mutation)?;
    }
    println!("✓ Applied all changes");
    Ok(())
}

fn apply_mutation(store: &Store, mutation: Mutation) -> Result<()> {
    match mutation {
        Mutation::UpdateActual { tension_id, new_value, .. } => {
            let id = resolve_tension_id(store, &tension_id)?;
            store.update_actual(&id, new_value)?;
        }
        Mutation::CreateChild { parent_id, desired, actual, .. } => {
            let parent = resolve_tension_id(store, &parent_id)?;
            store.create_tension_with_parent(&desired, &actual, Some(parent))?;
        }
        Mutation::AddNote { tension_id, text } => {
            let id = resolve_tension_id(store, &tension_id)?;
            store.add_note(&id, text)?;
        }
        Mutation::UpdateStatus { tension_id, new_status, .. } => {
            let id = resolve_tension_id(store, &tension_id)?;
            let status = TensionStatus::from_str(&new_status)?;
            store.update_status(&id, status)?;
        }
        Mutation::UpdateDesired { tension_id, new_value, .. } => {
            let id = resolve_tension_id(store, &tension_id)?;
            store.update_desired(&id, new_value)?;
        }
    }
    Ok(())
}
```

### 5. JSON Output Mode

For scripts/tools, return the entire structure:

```bash
werk run 01KK461Y "..." --json
```

Output:
```json
{
  "tension": {
    "id": "01KK461YBDBEX3W3N2MCWR880A",
    "desired": "Video demo recorded and submitted",
    "actual": "Not started"
  },
  "agent_response": {
    "mutations": [
      {
        "action": "update_actual",
        "tension_id": "01KK461YBDBEX3W3N2MCWR880A",
        "new_value": "Delegated to Dylan Thomas",
        "reasoning": "Clear ownership assignment"
      }
    ],
    "response": "Great decision! ..."
  }
}
```

## Usage Flow

### Example: Delegation with Child Tracking

```bash
$ werk run 01KK461YBDBEX3W3N2MCWR880A "I offered Dylan to do the video"

Tension: 01KK461YBDBEX3W3N2MCWR880A
Desired: Video demo recorded and submitted
Current: Not started

Agent Response:
──────────────────────────────────────────────────────────────
Smart delegation! This keeps you free to focus on writeup.
You should track Dylan's commitment separately so you can
monitor progress without micromanaging.
──────────────────────────────────────────────────────────────

Suggested Changes:

1. Update actual state: "Dylan Thomas agreed (ETA Saturday EOD)"
2. Create child tension: "Dylan's video meets quality standards"
3. Add note: "Confirmed via Telegram Tuesday"

What would you like to do?

  1) Apply all
  2) Review each
  3) Cancel

Enter selection (1-3): 2

Mutation 1/3:
  Update actual to: "Dylan Thomas agreed (ETA Saturday EOD)"
  Why: Clear ownership assignment
  Accept? (y/n): y

Mutation 2/3:
  Create child tension: "Dylan's video meets quality standards"
  Why: Separate tracking for delegated work
  Accept? (y/n): y

Mutation 3/3:
  Add note: "Confirmed via Telegram Tuesday"
  Accept? (y/n): y

✓ Applied 3 changes
```

## Error Handling

If the agent doesn't return valid YAML:

```bash
# Fallback to simple text response
Failed to parse structured response. Showing raw agent output:

"Here's what I'd suggest..."

(No mutations available for one-click application)
```

## Testing

- [x] Valid YAML parsing
- [x] Fallback to text on parse error
- [x] All mutation types working
- [x] Interactive review flow
- [x] "Apply all" bulk action
- [x] JSON output mode
- [x] Child tension creation
- [x] Note addition

## Future: Smart Suggestion Learning

With metrics, we could track:
- Which suggestions users accept/reject (learn patterns)
- Correlation between suggestion type and success (optimize prompt)
- Domain-specific templates (for hiring, project delivery, etc.)

## Related

- Design (d): `werk run` with inline prompt (foundation)
- Design (a): Agent command resolution (prerequisite)
- Metrics collection: Track accepted/rejected suggestions
