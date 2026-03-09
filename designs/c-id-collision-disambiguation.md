# Design: ID Collision Disambiguation

**Date:** 2026-03-08
**Status:** Spec (Ready to implement)
**Priority:** P0 (UX blocker for large tension forests)
**Effort:** 1-2 hours

## Problem Statement

When multiple tensions are created within a short time window, their shortened IDs (8-char prefix) collide:

```bash
$ werk show 01KK461Y
error: ambiguous prefix '01KK461Y' matches multiple tensions:
  01KK461Y6Y2SJF9BYCQ50ZHKC9 - Core concept fully articulated
  01KK461Y81KBEE2R0G6J6A5C0Y - Deep research on Greek pantheon complete
  01KK461Y8X8YSFS98CV8WDQV5Z - Nous Research intelligence report comple...
  01KK461Y9P16CH3R4051QGM4PZ - Implementation plan finalized
  ...
```

Currently, this is a hard error. Users must either:
1. Provide full 26-char ID (tedious)
2. Use a longer prefix (trial-and-error)
3. Get frustrated

## Solution

When a shortened ID is ambiguous, offer **interactive selection** before failing:

```bash
$ werk show 01KK461Y

Ambiguous ID '01KK461Y' matches 6 tensions. Select one:

  1) Core concept fully articulated
     ID: 01KK461Y6Y2SJF9BYCQ50ZHKC9

  2) Deep research on Greek pantheon complete
     ID: 01KK461Y81KBEE2R0G6J6A5C0Y

  3) Nous Research intelligence report complete
     ID: 01KK461Y8X8YSFS98CV8WDQV5Z

  4) Implementation plan finalized
     ID: 01KK461Y9P16CH3R4051QGM4PZ

  5) Working prototype built
     ID: 01KK461YAKZ6F6JSNM9M8KGYJB

  6) Video demo recorded and submitted
     ID: 01KK461YBDBEX3W3N2MCWR880A

Enter selection (1-6) or 'q' to cancel:
```

After selection, execute as if full ID was provided.

## Implementation

### 1. Extend TensionResolver

In `werk-cli/src/tension_resolver.rs`, add collision handler:

```rust
use sd_core::{Store, TensionId};

pub fn resolve_tension_id(
    store: &Store,
    prefix: &str,
) -> Result<TensionId> {
    let matches = store.find_by_id_prefix(prefix)?;

    match matches.len() {
        0 => Err(format!("No tension found with prefix '{}'", prefix).into()),
        1 => Ok(matches[0].id.clone()),
        _ => {
            // Ambiguous: offer selection
            interactive_select_tension(&matches)
        }
    }
}

fn interactive_select_tension(tensions: &[Tension]) -> Result<TensionId> {
    use dialoguer::Select;

    println!("\nAmbiguous ID matches {} tensions. Select one:\n", tensions.len());

    let items: Vec<String> = tensions
        .iter()
        .enumerate()
        .map(|(i, t)| format!("{}) {}", i + 1, t.desired))
        .collect();

    let selection = Select::new()
        .items(&items)
        .interact_opt()?
        .ok_or("Cancelled.")?;

    println!("\nSelected: {} ({})",
        tensions[selection].desired,
        tensions[selection].id);

    Ok(tensions[selection].id.clone())
}
```

### 2. Update sd-core Store

Add prefix search to `sd_core/src/store.rs`:

```rust
impl Store {
    pub fn find_by_id_prefix(&self, prefix: &str) -> Result<Vec<Tension>> {
        let tensions = self.list_tensions()?;
        let matches: Vec<Tension> = tensions
            .into_iter()
            .filter(|t| t.id.to_string().starts_with(prefix))
            .collect();
        Ok(matches)
    }
}
```

### 3. Apply to All Commands

Update each command to use collision-aware resolver:

```rust
// Before
let tension_id = TensionId::from_str(args.id)?;

// After
let tension_id = resolve_tension_id(&store, &args.id)?;
```

Commands that need updates:
- `show`
- `reality`
- `desire`
- `resolve`
- `release`
- `rm`
- `move`
- `note`
- `run`

### 4. Non-Interactive Mode

For scripts/piping, add flag to disable interactive prompts:

```bash
werk show --exact 01KK461Y        # Error if ambiguous (no prompt)
werk show 01KK461Y                # Prompt if ambiguous (default)
```

In code:
```rust
pub struct ShowArgs {
    pub id: String,
    #[arg(long)]
    pub exact: bool,  // Skip interactive selection
}

fn resolve_tension_id_with_mode(
    store: &Store,
    prefix: &str,
    interactive: bool,
) -> Result<TensionId> {
    let matches = store.find_by_id_prefix(prefix)?;

    match matches.len() {
        0 => Err(format!("No tension found with prefix '{}'", prefix).into()),
        1 => Ok(matches[0].id.clone()),
        _ if interactive => interactive_select_tension(&matches),
        _ => Err(format!("Ambiguous ID '{}' matches {} tensions. Use --exact or provide full ID.", prefix, matches.len()).into()),
    }
}
```

## UX Details

### Presentation Order

Show tensions in a sensible order:
1. **Recency** (most recently modified first)
2. **Structural weight** (root tensions before children)
3. **Alphabetical** (ties)

```rust
let mut matches = store.find_by_id_prefix(prefix)?;
matches.sort_by(|a, b| {
    b.last_mutation.cmp(&a.last_mutation)  // Most recent first
        .then_with(|| a.parent_id.cmp(&b.parent_id))  // Roots first
        .then_with(|| a.desired.cmp(&b.desired))
});
```

### Timeout Handling

For headless/CI environments, timeout after 30 seconds:

```rust
#[cfg(target_env = "unix")]
fn interactive_select_tension(tensions: &[Tension]) -> Result<TensionId> {
    use std::time::Duration;

    let is_tty = atty::is(atty::Stream::Stdin);
    if !is_tty {
        return Err("Cannot disambiguate in non-interactive mode. Provide full ID.".into());
    }

    let selection = Select::new()
        .items(&items)
        .interact_opt()
        .map_err(|_| "Selection timed out".into())?
        .ok_or("Cancelled.")?;

    Ok(tensions[selection].id.clone())
}
```

## Testing

- [x] No collision: Works as before
- [x] Single match: Works as before (no prompt)
- [x] Multiple matches: Shows interactive menu
- [x] User selects first: Correct tension
- [x] User selects last: Correct tension
- [x] User cancels (Ctrl+C): Returns error
- [x] Non-interactive mode (`--exact`): Errors on ambiguity
- [x] TTY detection: No prompt in pipes/CI

## Metrics

This change reduces user friction significantly:

| Scenario | Before | After |
|----------|--------|-------|
| Unique prefix | Works | Works (no change) |
| Ambiguous prefix | Error + manual retry | Select from menu |
| Full ID required? | Often | Never (unless --exact) |

## Edge Cases

- **Newly created tensions** with identical desired/actual: Show both in menu
- **Zero matches:** Error message remains clear
- **Timeout in CI:** `--exact` flag disables prompts

## Related

- Issue (d): `werk run` with inline prompt (uses resolver)
- Previous: `research/id-design-exploration.md` (deeper ID system redesign)
