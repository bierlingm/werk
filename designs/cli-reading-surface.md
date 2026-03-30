# CLI Reading Surface Redesign

**Opened:** 2026-03-30
**Status:** Design proposal. Supersedes `ground-mode-redesign.md`.
**Depends on:** Standard of Measurement (sacred core #10), Operative identity, Observational analysis stance (resolved)

---

## The Problem

The CLI has ten reading commands: `show`, `tree`, `list`, `survey`, `diff`, `health`, `ground`, `insights`, `trajectory`, `context`. They overlap significantly:

- `ground` and `insights` both show attention/engagement
- `ground` and `trajectory` both show trajectory analysis
- `ground` and `diff` both show recent changes
- `ground` and `health` both show field statistics
- `list` and `survey` are both "tensions across the field" with different sort axes
- `context` is `show --json` with more structural context

This proliferation happened because each new information need got its own command instead of composing from a smaller set of primitives. The result: users and agents must know which of ten commands to reach for, and several commands return partially redundant information.

The MCP surface mirrors the CLI 1:1 (11 reading tools), multiplying the problem for agents.

---

## The Model

Informed by `br` (beads_rust): one rich query command with filters and sort axes, named presets for common queries, a stats command with `--by` groupings. The key insight is that most "different commands" are really different filter/sort/format configurations over the same data.

For werk, the structural difference is hierarchy (tree, not flat list) and temporal computation (urgency, critical path, not just due dates). The reading surface must support tree views and temporal projections as output modes, not separate commands.

---

## The Four Reading Commands

### `show <id>` — One tension, full detail

**What it does today:** Displays desire, reality, parent, status, deadline, position, closure progress, urgency, signals, frontier, children, activity log. Already rich and well-designed.

**What changes:**

The `context` command's structural context (ancestors, siblings, engagement metrics) folds into `show`:
- `show <id>` — current output (default, human-readable)
- `show <id> --json` — current JSON plus ancestors, siblings, engagement metrics (what `context` returns today)
- `show <id> --full` — text output with ancestors, siblings, engagement metrics appended

`context` is removed as a separate command. `show --json` becomes the agent surface for single-tension reads.

**Flags:**
```
show <id>           # current output
show <id> --full    # expanded: includes ancestors, siblings, engagement
show <id> --json    # structured: full context (replaces context <id>)
```

### `tree [id]` — Hierarchy

**What it does today:** Displays the tension forest (or subtree) as an indented tree with closure ratios, deadline annotations, and signal indicators.

**What changes:** Minimal. Tree is the right command for this — hierarchy is central to werk's identity and doesn't reduce to a list with flags. One addition:

- `tree --stats` — appends the field vitals summary line at the bottom (total/active/resolved/released, positioned/held, activity count). Currently this info lives in `health` and `ground`.

**Flags:**
```
tree [id]           # subtree or full forest
tree --all          # include resolved/released
tree --resolved     # only resolved
tree --released     # only released
tree --stats        # append field vitals
tree --json         # structured output
```

### `list` — The query engine

**What it does today:** Flat list with `--all`, `--urgent`, `--neglected`, `--stagnant`, `--sort`. Weak filtering, one sort axis.

**What it becomes:** The general-purpose tension query. Every "show me tensions matching X, sorted by Y" question routes here. Absorbs `survey`, `diff`, and the current `list` filters.

**Filters (what to include):**
```
--all                  # include resolved/released (default: active only)
--status <s>           # filter by status: active, resolved, released
--overdue              # deadline passed, still active
--approaching [days]   # deadline within N days (default: 14)
--stale [days]         # no mutations in N days (default: 14)
--held                 # unpositioned
--positioned           # in the sequence
--root                 # root tensions only
--parent <id>          # children of a specific tension
--has-deadline         # only tensions with deadlines
--changed [since]      # mutated since (absorbs diff: "today", "yesterday", "3d", date)
```

**Sort (how to order):**
```
--sort <field>         # urgency (default), deadline, name, created, updated, position
--reverse              # flip sort direction
```

**Output modes:**
```
--tree                 # show results as tree (preserve hierarchy)
--long                 # expanded detail per tension
--format <f>           # text (default), json, csv
```

**What this absorbs:**

| Old command | Equivalent |
|-------------|-----------|
| `survey --days 14` | `list --approaching 14 --sort urgency` |
| `survey` (overdue + due soon) | `list --overdue` or `list --approaching` |
| `diff --since today` | `list --changed today` |
| `diff --since "3 days ago"` | `list --changed 3d` |
| `list --urgent` | `list --approaching 7` or `list --sort urgency` |
| `list --neglected` | `list --stale` |
| `list --stagnant` | `list --stale --overdue` |
| `context --all` | `list --json` |
| `context --urgent` | `list --approaching 7 --json` |

**Named presets / aliases:**

`survey` can survive as an alias: `werk survey` = `werk list --approaching 14 --overdue --sort urgency`. Whether to keep it as a convenience command or remove it entirely is a separate decision. The alias costs nothing if the underlying engine is `list`. Same for `diff` as an alias for `list --changed`.

### `stats` — Field aggregates

**What it does:** Replaces `health`, `ground`, `insights`, and `trajectory` as the single command for field-level summaries, aggregates, and analysis.

**Default output (no flags):** Field vitals — the compact summary.
```
Field
  62 active  54 resolved  7 released
  19 deadlined  0 overdue  23 positioned  39 held
  Activity (7d): 47 mutations across 14 tensions
```

**Sections (opt-in via flags):**
```
--temporal             # approaching deadlines, critical path, sequencing pressure, containment violations
--attention [days]     # mutation distribution across root tensions and branches
--changes [days]       # epochs, resolutions, new tensions, reality shifts (categorized)
--trajectory           # trajectory distribution, urgency collisions
--engagement [days]    # field frequency, most/least engaged, trends
--drift                # horizon drift patterns (postponement, oscillation)
--health               # data integrity: noop mutations, orphans, structural alerts
--all                  # everything
```

**Flags:**
```
--days <n>             # time window for windowed sections (default: 7)
--json                 # structured output
--repair               # (with --health) purge noop mutations
--yes                  # (with --repair) skip confirmation
```

**What this absorbs:**

| Old command | Equivalent |
|-------------|-----------|
| `ground` | `stats --all` |
| `ground --days 30` | `stats --all --days 30` |
| `health` | `stats --health` |
| `health --repair` | `stats --health --repair` |
| `insights --days 30` | `stats --attention 30 --engagement 30` |
| `trajectory` | `stats --trajectory` |
| `trajectory --collisions` | `stats --trajectory` (collisions included) |
| `trajectory <id>` | `show <id> --full` (per-tension trajectory in show's engagement section) |

**Section design:**

Each section follows the five categories from the ground mode design exploration (vitals, temporal, attention, changes, analytical) but they are flags on one command, not sections of a mode. The standard of measurement principle applies: vitals and temporal are factual, attention and changes are metric, trajectory and drift are analytical (framed as practice-layer analysis when displayed).

---

## What Dies

| Command | Replacement | Migration |
|---------|-------------|-----------|
| `survey` | `list --approaching --overdue --sort urgency` | Alias or remove |
| `diff` | `list --changed [since]` | Alias or remove |
| `health` | `stats --health` | Direct replacement |
| `ground` | `stats --all` | Direct replacement |
| `insights` | `stats --attention --engagement` | Direct replacement |
| `trajectory` | `stats --trajectory` (field) / `show --full` (per-tension) | Direct replacement |
| `context` | `show --json` (single) / `list --json` (bulk) | Direct replacement |

Seven commands die. Three survive (`show`, `list`, `tree`). One is new (`stats`). Net: 10 → 4.

---

## MCP Surface Implications

The MCP tools should mirror the CLI consolidation:

| Old MCP tool | New MCP tool |
|-------------|-------------|
| `show` | `show` (gains `full` param) |
| `tree` | `tree` (gains `stats` param) |
| `list` | `list` (gains all filter/sort params) |
| `survey` | removed or alias |
| `diff` | removed (use `list` with `changed_since`) |
| `health` | removed (use `stats` with `section: "health"`) |
| `ground` | removed (use `stats`) |
| `insights` | removed (use `stats`) |
| `trajectory` | removed (use `stats` with `section: "trajectory"`) |
| `context` | removed (use `show` or `list` — JSON is always available) |

New MCP tool: `stats` with a `sections` parameter (array of section names).

Agents that previously called `context` now call `show` with `json: true`. Agents that called `ground` or `trajectory` now call `stats` with the relevant section. This is a breaking change for MCP consumers. Document in release notes (#132).

---

## Backward Compatibility

**CLI:** Old command names can be kept as hidden aliases during a transition period, printing a deprecation notice pointing to the new form. Or they can be removed immediately since the user base is small and the mapping is clear.

**MCP:** Breaking change. Old tool names stop working. Document in release notes.

**`--json` output:** The JSON schemas for `list` and `stats` will be new. `show --json` will be a superset of the old `show --json` (gains context fields). `tree --json` unchanged.

---

## Implementation Order

1. **Enrich `list`** — add all filter/sort/format flags. This is the largest piece of work.
2. **Build `stats`** — implement sections one at a time, starting with vitals (trivial) and temporal (highest value).
3. **Enrich `show`** — add `--full` and fold context output into `--json`.
4. **Add `tree --stats`** — small addition.
5. **Wire aliases** — `survey`, `diff` as aliases over `list` for transition.
6. **Remove old commands** — once aliases are proven, remove the originals.
7. **Update MCP** — mirror CLI changes in tool definitions.
8. **Update docs** — README, CLAUDE.md, --help text.

Each step is independently shippable. The old commands keep working until step 6.

---

## Open Questions

1. **Should `survey` and `diff` survive as permanent aliases?** They're memorable names. `survey` especially has conceptual weight in the foundation (the Napoleonic field survey). It could survive as `werk survey` = `werk list --approaching --overdue --sort urgency` forever, as a named perspective rather than a separate command. Cost: one alias. Benefit: vocabulary continuity.

2. **Should `tree` absorb into `list --tree`?** The argument for keeping `tree` separate: hierarchy is central to werk's identity, and `tree` is the most-used reading command. The argument for `list --tree`: fewer top-level commands. Current lean: keep `tree` separate.

3. **Should `stats` show all sections by default or just vitals?** Current lean: just vitals by default, `--all` for everything. Keeps the default output compact and fast. Agents can request specific sections.

4. **What about `health --repair`?** This is a mutation (it deletes noop records), not a read. It's currently hiding in a reading command. It could move to `stats --health --repair` (awkward — a mutation flag on a reading command) or become a separate `repair` command or a flag on `nuke`/system commands. Current lean: keep it on `stats --health --repair` since `--repair` is already established and the awkwardness is contained.
