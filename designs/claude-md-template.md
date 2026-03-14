## werk — Structural Tension Management

This project uses `werk` for tracking structural tensions (gaps between desired and actual states).

### Reading state
- `werk list --json` — all active tensions with dynamics
- `werk show <id> --json` — full detail for one tension
- `werk context <id>` — full context with family and dynamics
- `werk tree` — hierarchical view
- `werk health --json` — system health summary
- `werk diff --since yesterday --json` — recent changes
- `werk trajectory --json` — structural projections

### Modifying state
- `werk add "desired" "actual" [--parent ID] [--horizon +2w]` — create
- `werk reality <id> "new actual"` — update reality
- `werk resolve <id>` — mark resolved
- `werk note <id> "observation"` — add note

### Agent-assisted
- `werk run <id> "prompt"` — one-shot agent analysis
- `werk run --system "prompt"` — system-wide analysis
- `werk run <id> --decompose` — break into sub-tensions
- `werk batch apply mutations.yaml` — apply structured mutations

### Conventions
- Before starting work, check `werk list --urgent`
- After milestones, update the relevant tension's reality
- Model sub-tasks as child tensions
