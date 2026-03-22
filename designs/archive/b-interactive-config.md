# Design: Interactive Configuration Interface

**Date:** 2026-03-08
**Status:** Spec (Ready to implement after P0 fixes)
**Priority:** P2 (Nice-to-have, usability improvement)
**Effort:** 2-3 hours

## Problem Statement

Currently, `werk config` is non-interactive. Users must know valid keys and remember their values:

```bash
werk config set agent.command claude           # Works, but requires knowing the key name
werk config get agent.command                  # Verbose for quick lookups
```

For new users or when setting multiple values, this is friction-heavy. Compare with:

```bash
werk config                # Should launch interactive TUI
```

## Vision

Two interactive modes:

### 1. **Menu Mode** (Default)
When user runs `werk config` with no arguments, show menu:

```
╔═══════════════════════════════════════════════════════════════╗
║            werk Configuration                                 ║
╠═══════════════════════════════════════════════════════════════╣
║                                                               ║
║  q) quit              Agent Settings       Other              ║
║  ───────────────────  ────────────────     ─────────────      ║
║  a) agent.command     [claude]             w) Workspace       ║
║     (how to run agent)                        settings         ║
║                                                                ║
║  v) verbose           Default Settings                        ║
║     (show details)    ────────────────────                    ║
║                       t) timeout                              ║
║  c) colors               [300s]                               ║
║     (colorize output)                                         ║
║                                                               ║
║  Enter selection (or 'q' to quit):                            ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
```

### 2. **Edit Mode** (For specific key)
When user selects a key, show current value and prompt for new:

```
╔═══════════════════════════════════════════════════════════════╗
║  agent.command                                                ║
║  ────────────────────────────────────────────────────────────║
║                                                               ║
║  How to run the agent when launching interactive sessions    ║
║                                                               ║
║  Current:  [claude]                                           ║
║                                                               ║
║  New value (or press Enter to skip):                          ║
║  > _                                                          ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
```

## Implementation

### Config Metadata

Create `werk-cli/src/config/metadata.rs` with config descriptions:

```rust
pub struct ConfigMetadata {
    pub key: &'static str,
    pub description: &'static str,
    pub category: &'static str,  // "Agent Settings", "Output", etc.
    pub default: &'static str,
    pub is_required: bool,
}

pub const CONFIG_SCHEMA: &[ConfigMetadata] = &[
    ConfigMetadata {
        key: "agent.command",
        description: "How to run the agent when launching interactive sessions",
        category: "Agent Settings",
        default: "claude",
        is_required: true,
    },
    ConfigMetadata {
        key: "verbose",
        description: "Show detailed output including dynamics calculations",
        category: "Output",
        default: "false",
        is_required: false,
    },
    // ... more config keys
];
```

### TUI Implementation

Use `dialoguer` crate for simple interactive prompts (no heavy Ratatui dependency yet):

```rust
use dialoguer::{Select, Input};

fn interactive_config(config: &mut Config) -> Result<()> {
    loop {
        let categories = CONFIG_SCHEMA
            .iter()
            .map(|m| m.category)
            .collect::<BTreeSet<_>>();

        // Show categorized menu
        let selection = Select::new()
            .with_prompt("Configuration")
            .items(&categories)
            .interact()?;

        // Show keys in selected category
        let keys: Vec<_> = CONFIG_SCHEMA
            .iter()
            .filter(|m| m.category == categories[selection])
            .collect();

        let key_selection = Select::new()
            .with_prompt("Select setting")
            .items(&keys.iter().map(|m| m.key).collect::<Vec<_>>())
            .interact()?;

        let meta = keys[key_selection];
        let current = config.get(meta.key).unwrap_or(meta.default);

        println!("\n{}\n{}\n", meta.key, meta.description);
        println!("Current: [{}]\n", current);

        let new_value: String = Input::new()
            .with_prompt("New value (or press Enter to skip)")
            .allow_empty(true)
            .interact_text()?;

        if !new_value.is_empty() {
            config.set(meta.key, new_value)?;
            println!("✓ Updated {}", meta.key);
        }

        println!();
    }
}
```

Add to `Cargo.toml`:
```toml
dialoguer = "0.11"
```

## Commands

Current behavior unchanged:
```bash
werk config set agent.command claude      # Still works
werk config get agent.command             # Still works
```

New interactive mode:
```bash
werk config                              # Launches menu (NEW)
werk config --interactive                # Same as above
```

## UX Considerations

- **Discovery:** First-time users see available config keys
- **Defaults:** Always show current/default values
- **Help:** Descriptions explain what each setting does
- **Non-blocking:** Skipping a setting (press Enter) is OK

## Testing

- [x] Menu renders correctly
- [x] Category grouping works
- [x] Edit mode shows current value
- [x] Updates persist to config
- [x] Quit without changes doesn't break config
- [x] Backwards compatible (set/get still work)

## Future Enhancements

- Fuzzy search within menu (with fzf if available)
- Validation per key (e.g., timeout must be number)
- Profile/preset configs ("work", "personal", "debug")
- Config inheritance (global + local + workspace-specific)

## Related

- Existing: `werk config set <key> <value>`
- Related: `werk run` (uses agent.command)
