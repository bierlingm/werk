# werk v1.0 — Session Handoff

## What This Is

A Rust workspace for building an **operative instrument** on top of a **structural dynamics grammar** based on Robert Fritz's creative-process theory. The practitioner tracks structural tensions (gaps between desired state and current reality) and the system computes dynamics from mutation history.

## Architecture (Locked)

```
~/werk/desk/werk/
├── Cargo.toml          # workspace: [sd-core, werk-cli]
├── sd-core/            # Pure grammar library. Zero instrument code.
│   └── src/lib.rs      # Declares: tension, mutation, dynamics, events, store, tree
├── werk-cli/           # The operative instrument binary ("werk")
│   └── src/main.rs     # Placeholder
└── research/           # All planning artifacts
```

**sd-core** = structural dynamics grammar. Pure Fritz. Emits typed events. Never calls instrument code. Extension happens through event subscription and parameter injection.

**werk** = the operative instrument. CLI with subcommands. Each subcommand is a focused tool (some simple actions, some TUI). `werk` is also the workspace concept (`.werk/` directory per project, `~/.sd/` global).

## Data Model (Locked)

Two tables, everything else computed:

- **tensions**: id (ULID), desired, actual, parent_id (nullable → forest topology), created_at, status
- **mutations**: tension_id, timestamp, field, old_value, new_value

Dynamics (lifecycle, conflict, movement, neglect) are **computed from mutation history**, not stored. Thresholds are parameters injected by instruments.

Multiple root tensions allowed. Unattached/loose tensions allowed. Forest, not tree.

## Key Decisions

1. **Grammar = pure Fritz** — zero additions to theory, hooks for extension
2. **Events as extension boundary** — grammar emits, instruments subscribe
3. **Dynamics are computed** — lifecycle stage, structural conflict, movement direction, neglect — all derived
4. **Forest topology** — parent_id nullable, multiple roots, loose tensions as candidates
5. **werk = workspace + CLI** — subcommands for individual capabilities
6. **Oracle = separate process** — not compiled into main binary
7. **Encryption** — age on exported structure files, NOT on the database
8. **Audience** — serious, high-ambition people. No casual-user concessions.

## OPEN: Must Investigate

### fsqlite (FrankenSQLite)
User wants to use Jeffrey Emanuel's tools maximally. Previous assessment said fsqlite isn't production-ready (v0.1.1, in-memory engine, concurrent writes not active, no built-in change tracking replacing mutations). **User explicitly does not trust this assessment.** Re-investigate independently by reading the actual fsqlite source code. The question: can fsqlite's capabilities replace or enhance the mutations table / change tracking?

### Fritz Fidelity
Current dynamics list (lifecycle, conflict, movement, neglect) may be incomplete. Study Fritz's actual theory more deeply. Are there dynamics we're missing? The list should be extensible regardless.

### V3 Plan
V2 plan (research/v2-plan.md) is partially stale — uses "ride" not "werk", has wrong storage assumptions. A V3 plan needs to be written reflecting all final decisions above, then decomposed into beads using the Jeffrey method.

### sd-core Module Stubs
lib.rs declares 6 modules (tension, mutation, dynamics, events, store, tree) but the .rs files don't exist yet.

## Research Artifacts

All in `research/`:
- `synthesis-map.md` — 65+ tools across 13 categories, movement grammars, affordance shaping
- `v1_category_and_seeds.md` — "Operative Instrument" category definition + seed enrichment
- `v1_persona_reports.md` — 10 user persona evaluations of v0.2
- `v1_collection_of_thoughts.md` — User's discovery phase musings (in/on/off, Kelly criterion, cosmic calendar, required action, etc.)
- `v1-plan.md` — V1 plan (historical)
- `v2-plan.md` — V2 plan (partially stale on naming/storage)
- `anima_plan.md` — Self-modeling system design

## The Original Codebase

`~/werk/desk/sc/` — v0.1/v0.2 (~5k lines Rust). Still exists. Has working TUI with 11 phases. The sacred core from persona reports: **practice mode and honest confrontation with current reality**.

## User Identity

- GitHub: `bierlingm` (NOT `moritzbierling`)
- Owns werk.horse domain
- Thinks of this as "the great work" — alchemical, hermetic
- Horse metaphor: harnesses (Claude Code etc.) direct instruments (horses). The rider directs through the harness.
- "I am making mine. That's it."

## Next Steps

1. Independently investigate fsqlite capabilities
2. Deepen Fritz fidelity check
3. Write V3 plan reflecting all locked decisions
4. Decompose into bead graph
5. Begin implementation (sd-core first, then werk-cli)
