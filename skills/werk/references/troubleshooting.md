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
Use a longer prefix. werk uses prefix matching — `01KK` might match many tensions. Use 8-12 characters: `01KKTEJC883X`.

Tip: `werk show <partial>` will list all matches if ambiguous.

## Agent not configured
```bash
werk config set agent.command "hermes chat -Q -q"
```
For Claude: `werk config set agent.command "claude"`

## Agent returns no structured mutations
The agent needs to return YAML between `---` markers at the END of its response. If it doesn't, the response is shown as plain text. This is fine — not every response needs mutations.

To improve mutation quality: ask specific questions ("What should I update?") rather than open-ended ones.

## Common Structural Patterns

### The Oscillator
**Symptoms**: High reversal count, tendency = Oscillating, compensating strategy present.
**What's happening**: The person advances, hits discomfort, retreats. Repeat.
**What to ask**: "What happens in the moment you turn back?"

### The Neglector
**Symptoms**: Active parent, neglected children, no updates on children for 14+ days.
**What's happening**: Declared decomposition but isn't doing the actual work.
**What to ask**: "Are these still the right children? Or have you outgrown this decomposition?"

### The Starter
**Symptoms**: Many germination-phase tensions, few assimilation or completion.
**What's happening**: Loves beginning, avoids the middle.
**What to ask**: "What would it feel like to have only three tensions?"

### The Postponer
**Symptoms**: Horizon drift = repeated postponement, multiple horizon changes.
**What's happening**: The deadline keeps moving because commitment keeps retreating.
**What to ask**: "What would change if the horizon was immovable?"

### The Loner
**Symptoms**: All root tensions, no children, no decomposition.
**What's happening**: Thinking in abstractions, not in actionable steps.
**What to ask**: "What would need to be true next week for this to advance?"
