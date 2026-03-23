# Troubleshooting & Common Patterns

## "werk: command not found"
```bash
cargo install --path werk-cli   # if building from source
# or ensure ~/.cargo/bin is in PATH
```

## "No workspace found"
```bash
cd /your/project
werk init
```
werk walks UP from the current directory to find `.werk/`. Run `werk init` in the directory you want as your workspace root.

## "ambiguous prefix matches multiple tensions"
Use the short code (e.g., `#23`) or a longer ULID prefix. `werk show <partial>` will list all matches if ambiguous.

## Common Structural Patterns

### The Oscillator
**Shape**: tendency = oscillating, advance then retreat repeated.
**What's happening**: the person hits discomfort and retreats. The trace zigzags around the same ground.
**What to ask**: "What happens in the moment you turn back?"

### The Neglector
**Shape**: active parent, untended children, no updates on children for 14+ days.
**What's happening**: declared a theory of closure but isn't executing it.
**What to ask**: "Are these still the right children? Or has the theory changed?"

### The Starter
**Shape**: many germination-phase tensions, few in assimilation or completion.
**What's happening**: loves beginning, avoids the middle.
**What to ask**: "What would it feel like to have only three tensions?"

### The Postponer
**Shape**: horizon drift = repeated postponement, multiple deadline changes.
**What's happening**: the deadline keeps moving because commitment keeps retreating.
**What to ask**: "What would change if the horizon was immovable?"

### The Loner
**Shape**: all root tensions, no children, no decomposition.
**What's happening**: thinking in abstractions, not in actionable steps.
**What to ask**: "What would need to be true next week for this to advance?"

### Overreach
**Shape**: many active children, few resolved, parent moving but children starving.
**What's happening**: the theory of closure is too ambitious for the available energy.
**What to ask**: "Which three of these children would close the gap if completed?"
