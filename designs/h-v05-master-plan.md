# werk v0.5 — Master Plan

**Date:** 2026-03-13
**Status:** Plan complete
**Scope:** Three integrated systems: TUI, CLI, and Agent Hooks
**Companion plans:** `calm-wandering-crab.md` (projection engine sd-core implementation detail), `g-tui-ideas-v05.md` (research + idea evaluation)

---

## Part I: World-Class TUI

### Design Philosophy

The TUI is the daily instrument. It's not a dashboard you glance at — it's the surface through which a practitioner engages with structural dynamics. Every pixel serves the confrontation between desired state and current reality.

**Three principles:**

1. **Density without clutter.** Show the maximum useful information at every terminal size. No empty views. No dead-end screens. Every row of pixels earns its space.
2. **Cursor, not viewport.** The user always has a position — something selected, something highlighted. j/k moves the cursor. The screen follows the cursor. Never the other way around.
3. **Two-and-a-half views.** Dashboard (list), Detail (depth), Tree (structure). No more. Everything else is a panel, overlay, or inline section. The user never gets lost.

### View Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│ werk  12 active  3▲ urgent  2⚠ neglected    ▸Ship feature 92%  │  ← Status Bar (1 row, always)
├──────────────────────────┬──────────────────────────────────────┤
│ ^ URGENT                 │  Ship feature                  a8f2 │
│   [C] → ↓ Ship feature 3d│                                     │
│   [A] → ~ Fix auth bug 1d│  Desired   Ship the feature by v2   │
│ * ACTIVE                 │  Actual    API complete, UI 80%      │
│   [A] ○ — Write docs     │  Horizon   Mar 16 (3d remaining)    │
│   [G] ○ — Plan Q2 roadmap│                                     │
│ ! NEGLECTED              │  Phase     Completion                │
│   [A] ↔ ⇌ Upd. branding │  Movement  → Advancing              │
│                          │  Magnitude █████████░ 0.82           │
│                          │  Urgency   █████████░ 92%            │
│                          │  Forecast  On track — ~Mar 15        │
│                          │                                      │
│                          │  Trajectory  ↓ Resolving             │
│                          │  Gap now     ■■■■■■□□□□  0.62        │
│                          │  Gap +1w     ■■■■■□□□□□  0.55        │
│                          │  Gap +1m     ■■■□□□□□□□  0.34        │
│                          │                                      │
│                          │  Next      Unblock downstream — 2    │
│                          │            children blocked          │
│                          │                                      │
│                          │  History (5)                         │
│                          │  2h ago    actual: "API done" → "API │
│                          │            complete, UI 80%"         │
│                          │  1d ago    horizon: Mar 20 → Mar 16  │
│                          │                                      │
│                          │  Children (2)                        │
│                          │  a9c1  [A] → ↓ Deploy staging        │
│                          │  b2d4  [G] ○ — Write release notes   │
├──────────────────────────┴──────────────────────────────────────┤
│ j/k  Enter detail  Tab tree  a add  r/d edit  z snooze  :  /?  │  ← Hints (1 row)
└─────────────────────────────────────────────────────────────────┘
```

Dashboard row format: `[Phase] Movement Trajectory Desired Horizon`. Trajectory indicators: `↓` resolving (green), `—` stalling (dim), `~` drifting (yellow), `⇌` oscillating (red). Computed from the projection engine on a 5-minute cache cycle.

#### Responsive Layout Tiers

| Width | Layout | Detail |
|-------|--------|--------|
| <60 cols | Single pane | Dashboard only. Enter → full-screen Detail. |
| 60-119 cols | Single pane | Dashboard with richer columns (sparkline, urgency bar). Enter → full-screen Detail. |
| 120+ cols | Split pane | Dashboard left (40%), Detail right (60%). Cursor movement auto-loads detail. No Enter/Esc cycling. |

#### The Three Views

**Dashboard** (default):
- Status bar (1 row): app name, counts, top 2 urgent tensions (right-aligned)
- Tier-grouped tension list with section headers (URGENT / ACTIVE / NEGLECTED)
- Section headers are non-selectable, cursor skips them
- Columns adapt to width (see existing responsive logic, keep it)
- When split pane active: detail pane on right auto-updates on cursor move

**Detail** (Enter from Dashboard, or right pane in split mode):
- Title bar with desired + short ID
- Sections: Info → Dynamics (always full, no verbose toggle) → Forecast → Trajectory (projected gap progression + risks) → Next Action (lever) → Siblings → History → Children → Agent Response (if present)
- j/k moves a cursor between sections and items within sections (not raw scroll)
- Enter on a child → push to nav stack, load that child's detail
- Enter on a sibling → navigate to it
- Esc → pop nav stack (or back to Dashboard)

**Tree** (Tab from Dashboard):
- Same status bar
- Hierarchical forest with expand/collapse (using ftui Tree widget)
- Expand/collapse with `l`/`h` (vim-style: right expands, left collapses)
- Enter → opens Detail for selected node
- Tab → back to Dashboard
- Persists expand/collapse state to `.werk/tree-state.json`

#### Eliminated Views

| Old View | Where It Goes |
|----------|---------------|
| Focus | Removed. Detail view shows everything Focus showed, plus more. |
| Neighborhood | Absorbed into Detail as "Siblings" section between Dynamics and History. |
| Timeline | Becomes a toggleable bottom panel in Dashboard (`T` to toggle). 5-10 rows. |
| DynamicsSummary | Becomes a section in the command palette `:health` or a modal overlay (`D`). |
| Agent | Agent responses shown inline in Detail view as a collapsible section. Agent mutation checkboxes operate in a sub-mode within Detail. |

#### Input System

```
Priority stack (highest to lowest):
1. Command Palette (if open)     — absorbs all keys
2. Search overlay (if active)    — absorbs all keys except Esc
3. Lever overlay (if showing)    — Esc/L to dismiss
4. Help overlay (if showing)     — Esc/? to dismiss
5. Input modes:
   a. TextInput                  — Enter submit, Esc cancel
   b. Confirm                   — y/n
   c. MovePicker                — j/k/Enter/Esc
   d. Review                   — NEW: guided review mode
6. Normal mode (view-specific)   — full keybinding map
```

#### Master Keybinding Map (Normal Mode)

**Universal (all views):**
| Key | Action |
|-----|--------|
| `j`/`k` or `↓`/`↑` | Move cursor down/up |
| `Tab` | Cycle: Dashboard → Tree → Dashboard |
| `Shift+Tab` | Cycle backward |
| `:` | Command palette |
| `/` | Search (persistent — Enter commits filter, Esc clears) |
| `?` | Help overlay |
| `q` / `Ctrl+C` | Quit |
| `u` | Undo last action (5-second window) |
| `!`/`@`/`#` | Jump to 1st/2nd/3rd most urgent |

**Dashboard + Tree (list contexts):**
| Key | Action |
|-----|--------|
| `Enter` | Open detail (or navigate into, in split mode) |
| `f` | Cycle filter (Active → All → Resolved → Released) |
| `Space` | Quick toggle: Active ↔ All |
| `a` | Add tension (single-line: `desired [horizon] [| actual]`) |
| `c` | Create child of selected |
| `p` | Create parent of selected |
| `r` | Update reality |
| `d` | Update desire |
| `n` | Add note |
| `h` | Set horizon (supports `+2w`, `eom`, absolute dates) |
| `z` | Snooze (prompted for wake date) |
| `Z` | Show/hide snoozed tensions |
| `R` | Resolve (with what-if preview → confirm with R again) |
| `X` | Release (with what-if preview → confirm with X again) |
| `m` | Move/reparent |
| `g` | Agent (decompose or advise) |
| `L` | Lever detail overlay |
| `T` | Toggle timeline bottom panel (Dashboard only) |
| `D` | Health summary overlay |
| `Ctrl+R` | Start morning review ritual |

**Detail view (additional):**
| Key | Action |
|-----|--------|
| `Esc` | Back (pop nav stack or return to Dashboard) |
| `Enter` | Navigate into selected child/sibling |
| `Del` | Delete (with confirm) |
| `w` | Reflect (TextArea) |
| `Y` | Set recurrence interval |
| `v` | What-if preview for current tension |

**Tree view (additional):**
| Key | Action |
|-----|--------|
| `l` / `→` | Expand node |
| `h` / `←` | Collapse node |
| `Enter` | Open detail |
| `Esc` | Back to Dashboard |

#### State Architecture

```rust
pub struct WerkApp {
    // Core
    engine: DynamicsEngine,
    tensions: Vec<TensionRow>,
    filter: Filter,
    active_view: View,     // Dashboard | Detail | TreeView
    input_mode: InputMode,
    toasts: Vec<Toast>,
    lever: Option<LeverResult>,
    pending_undo: Option<UndoAction>,

    // Projection engine (cached, recomputed every 5 minutes)
    field_projection: Option<FieldProjection>,
    last_projection_time: Option<DateTime<Utc>>,

    // View-specific state (sub-structs)
    dashboard: DashboardState,
    detail: DetailState,
    tree: TreeState,
    agent: AgentState,
    search: SearchState,
    review: ReviewState,
    reflect: ReflectState,

    // Widgets (owned by app, rendered by views)
    palette: CommandPalette,
    text_input: TextInput,
    search_input: TextInput,
}

pub struct DashboardState {
    table_state: RefCell<TableState>,
    panel: DashboardPanel,       // None | Timeline | Health
    split_mode: bool,            // auto-detected from width
}

pub struct DetailState {
    tension: Option<Tension>,
    dynamics: Option<ComputedDynamics>,  // full, not DetailDynamics subset
    mutations: Vec<MutationDisplay>,
    children: Vec<TensionRow>,
    siblings: Vec<TensionRow>,
    parent: Option<Tension>,
    ancestors: Vec<(String, String)>,
    nav_stack: Vec<String>,
    cursor: DetailCursor,        // which section/item is focused
    forecast: Option<ForecastResult>,
    projection: Option<TensionProjection>,  // from projection engine
}

pub struct TreeState {
    tree_widget_state: RefCell<TreeWidgetState>,  // ftui Tree state
    expanded: HashSet<String>,   // persisted to .werk/tree-state.json
}
```

#### New Features Integrated into TUI

1. **Split pane** — auto-activates at width >= 120, detail auto-loads on cursor move
2. **Tier grouping** — section headers in dashboard list
3. **Resolution forecasting** — "Forecast" line in Detail dynamics section
4. **Structural projection engine** — per-tension trajectory classification (Resolving/Stalling/Drifting/Oscillating), projected gap progression at 1w/1m/3m horizons, urgency collision detection, and trajectory-aware lever scoring. Trajectory indicators on every dashboard row. Full projection section in Detail. Cached on 5-minute cycle. See `calm-wandering-crab.md` for full sd-core implementation plan.
5. **Snooze** — `z` to snooze, `Z` to show snoozed, auto-resurface on tick
6. **Composite auto-resolution** — parent resolves when all children resolve
7. **Recurring tensions** — `Y` to set recurrence, auto-recreate on resolve
8. **What-if preview** — resolve/release shows cascade preview before confirming
9. **Morning review** — `Ctrl+R` starts guided walkthrough of urgent/neglected/stagnant
10. **Behavioral insights** — `:insights` in command palette shows pattern digest (includes urgency collision warnings from projection engine)
11. **Undo** — `u` undoes last action within 5 seconds
12. **Persistent search** — `/` opens overlay, Enter commits filter, Esc clears
13. **Single-line quick-add** — `a` prompts: `desired [horizon] [| actual]`
14. **Relative horizons** — `+2w`, `+3m`, `eom`, `eoq`, `eoy`
15. **Filesystem watch** — external changes auto-reload (no stale data)
16. **Inline agent responses** — agent output shown in Detail, not separate view
17. **Tree expand/collapse** — `h`/`l` with persisted state

---

## Part II: Massively Useful CLI

### Design Philosophy

The CLI is the scripting surface. It's for: (1) quick one-off commands from the shell, (2) piping and composition with Unix tools, (3) agent consumption and mutation, (4) automation scripts and cron jobs. It is NOT a human daily-driver — that's the TUI.

**Three principles:**

1. **`--json` everywhere.** Every command produces structured JSON when `--json` is passed. No exceptions. Agents and scripts consume JSON. Humans get pretty output by default.
2. **Composable primitives.** Each command does one thing. Complex workflows compose commands with pipes and scripts.
3. **Exit codes are semantic.** 0 = success, 1 = user error (bad input, not found), 2 = internal error. Scripts can branch on exit codes without parsing output.

### Command Taxonomy

Organized into three tiers:

#### Tier 1: Lifecycle (Create / Destroy)

```bash
# Create
werk add "Ship the feature" "Have a design doc" --horizon +2w --parent 01KK
werk add "Ship the feature" "Have a design doc"    # minimal
werk add "Ship the feature"                        # desire-only (actual defaults to empty)

# Delete
werk rm 01KK                                      # reparents children to root
werk rm 01KK --cascade                             # deletes entire subtree
werk rm 01KK --json                                # returns { deleted: [...ids] }

# Workspace
werk init [--global]                               # create .werk/
werk nuke [--confirm]                              # destroy .werk/
```

**New: `werk add` improvements:**
- Single positional arg creates desire-only tension (actual defaults to empty string)
- `--horizon` accepts relative formats: `+2w`, `+3m`, `eom`, `eoy`
- `--recur +1w` marks as recurring on creation
- `--decompose` triggers agent auto-decomposition after creation
- `--json` returns `{ "id": "...", "short_id": "..." }`

#### Tier 2: Orchestration (Mutate / Transform)

```bash
# Update fields
werk reality 01KK "API complete, UI 80% done"     # update actual
werk reality 01KK                                  # opens $EDITOR
werk desire 01KK "Ship v2 with analytics"          # update desired
werk desire 01KK                                   # opens $EDITOR
werk horizon 01KK +2w                              # set horizon (relative)
werk horizon 01KK 2026-04-01                       # set horizon (absolute)
werk horizon 01KK --clear                          # remove horizon

# Status transitions
werk resolve 01KK                                  # mark resolved
werk release 01KK --reason "deprioritized"         # mark released
werk reopen 01KK                                   # NEW: reactivate resolved/released

# Structural changes
werk move 01KK --parent 02BB                       # reparent under 02BB
werk move 01KK --root                              # make root (clear parent)

# Notes
werk note 01KK "Discussed in standup, on track"    # add note to tension
werk note "General workspace observation"          # workspace-level note

# Snooze
werk snooze 01KK +3d                               # NEW: hide until date
werk snooze 01KK --clear                           # NEW: unsnooze

# Recurrence
werk recur 01KK +1w                                # NEW: set recurrence
werk recur 01KK --clear                            # NEW: remove recurrence
```

**New commands:**
- `werk reopen` — reactivate a resolved/released tension (status → Active)
- `werk snooze` — set/clear snooze date
- `werk recur` — set/clear recurrence interval

#### Tier 3: Query / Read

```bash
# Single tension
werk show 01KK                                    # human-readable detail
werk show 01KK --json                              # full JSON with dynamics
werk show 01KK --dynamics                          # dynamics only (human)
werk show 01KK --forecast                          # NEW: resolution forecast

# Lists
werk list                                          # active tensions, one per line
werk list --all                                    # include resolved/released
werk list --urgent                                 # NEW: urgent tier only
werk list --neglected                              # NEW: neglected tier only
werk list --stagnant                               # NEW: stagnant movement only
werk list --snoozed                                # NEW: snoozed tensions
werk list --phase G                                # NEW: filter by phase
werk list --sort urgency                           # NEW: sort control
werk list --json                                   # JSON array

# Tree
werk tree                                          # active tension forest
werk tree --all                                    # include resolved
werk tree --json                                   # JSON tree structure

# Context (for agents)
werk context 01KK                                  # full JSON context
werk context 01KK --family                         # NEW: include full subtree context
werk context --all                                 # NEW: all tensions context
werk context --urgent                              # NEW: only urgent tensions

# Analytics
werk health                                        # NEW: phase/movement/alert summary
werk health --json                                 # NEW: structured health data
werk insights                                      # NEW: behavioral pattern digest
werk insights --days 30                             # NEW: custom window
werk insights --json                               # NEW: structured insights

# Diff
werk diff                                          # NEW: what changed today
werk diff --since yesterday                        # NEW: changes since date
werk diff --since "3 days ago"                     # NEW: relative
werk diff --json                                   # NEW: structured diff

# Structural projection
werk trajectory 01KK                               # NEW: per-tension trajectory + gap progression
werk trajectory 01KK --json                        # NEW: structured projection data
werk trajectory                                    # NEW: full-field structural funnel
werk trajectory --collisions                       # NEW: urgency collision windows
werk trajectory --json                             # NEW: full projection JSON (for agents)
```

**New commands:**
- `werk list` — replaces the need for `tree` in flat-list scenarios, with rich filtering
- `werk health` — CLI version of the health/dynamics summary
- `werk insights` — behavioral pattern analysis from mutation history
- `werk diff` — changelog: what happened in a time window
- `werk context --all` / `--urgent` — bulk context for agents that need system-wide awareness
- `werk trajectory [id]` — structural projection: per-tension trajectory or full-field projection
- `werk trajectory --collisions` — upcoming urgency collision windows

#### Tier 4: Agent Integration (detailed in Part III)

```bash
# One-shot (existing, improved)
werk run 01KK "What should I focus on?"            # agent with prompt

# Interactive (existing)
werk run 01KK -- claude --dangerously-skip-permissions

# New modes
werk run 01KK --decompose                          # auto-decomposition prompt
werk run 01KK --review                             # review prompt (is reality current?)
werk run --system "Review all urgent tensions"     # NEW: system-wide agent context
werk run --lever                                   # NEW: run agent on lever tension

# Batch
werk batch apply mutations.json                    # NEW: apply mutations from file
werk batch apply -                                 # NEW: apply mutations from stdin
```

### Output Format Standards

**Human output** (default):
```
$ werk show 01KK
Ship the feature                                      01KK461Y
─────────────────────────────────────────────────────────────────
  Desired   Ship the feature by v2 launch
  Actual    API complete, UI 80% done
  Status    Active          Created  3 days ago
  Horizon   Mar 16 (3d remaining)

  Phase     Completion      Movement  → Advancing
  Magnitude █████████░ 0.82
  Urgency   █████████░ 92%
  Forecast  On track — resolving ~Mar 15
  Trajectory  ↓ Resolving   Gap: 0.82 → 0.55 (1w) → 0.34 (1m)

  Recent (3)
  2h ago    actual: "API done" → "API complete, UI 80%"
  1d ago    horizon: Mar 20 → Mar 16
  3d ago    created

  Children (2)
  a9c1  [A] → Deploy staging
  b2d4  [G] ○ Write release notes
```

**JSON output** (`--json`):
```json
{
  "id": "01KK461YBDBEX3W3N2MCWR880A",
  "short_id": "01KK461Y",
  "desired": "Ship the feature by v2 launch",
  "actual": "API complete, UI 80% done",
  "status": "Active",
  "created_at": "2026-03-10T14:30:00Z",
  "horizon": { "value": "2026-03-16", "kind": "Day", "range": { "start": "...", "end": "..." } },
  "parent_id": null,
  "dynamics": {
    "phase": "Completion",
    "movement": "Advancing",
    "magnitude": 0.82,
    "urgency": 0.92,
    "forecast": { "on_track": true, "estimated_date": "2026-03-15", "velocity_ratio": 1.12 },
    "oscillation": null,
    "conflict": null,
    "neglect": null,
    "resolution": { "velocity": 0.0042, "trend": "Steady", "is_sufficient": true },
    "horizon_drift": { "type": "Tightening", "changes": 1 },
    "compensating_strategy": null,
    "assimilation_depth": "Deep",
    "orientation": "Creative"
  },
  "projection": {
    "trajectory": "Resolving",
    "current_gap": 0.82,
    "projected_gap_1w": 0.55,
    "projected_gap_1m": 0.34,
    "projected_gap_3m": 0.08,
    "will_resolve": true,
    "time_to_resolution_seconds": 3628800,
    "oscillation_risk": false,
    "neglect_risk": false,
    "engagement_trend": "accelerating"
  },
  "children": [ { "id": "...", "desired": "Deploy staging", "phase": "A", "urgency": 0.5 } ],
  "mutations": [ { "timestamp": "...", "field": "actual", "old": "...", "new": "..." } ]
}
```

### Pipe-Friendly Patterns

```bash
# Get IDs of all urgent tensions
werk list --urgent --json | jq -r '.[].id'

# Bulk-update reality for all stagnant tensions
werk list --stagnant --json | jq -r '.[].id' | while read id; do
  werk reality "$id" "No change — still blocked"
done

# Generate a report of what changed this week
werk diff --since "monday" --json | jq '.changes[] | "\(.type): \(.desired)"'

# Feed all urgent tensions to an agent
werk context --urgent | claude "Review these urgent tensions and suggest priorities"

# Morning review script
werk list --urgent --neglected --json | jq -r '.[].id' | while read id; do
  echo "---"
  werk show "$id" --dynamics
  read -p "Update reality? (y/n): " answer
  [ "$answer" = "y" ] && werk reality "$id"
done

# Export tensions to markdown
werk list --all --json | jq -r '.[] | "- [\(.phase)] \(.desired) (\(.status))"'
```

---

## Part III: Agent Hooks Architecture

### Design Philosophy

werk doesn't try to be an agent. werk is the **state layer** that agents read from and write to. The integration surface has three tiers:

1. **CLI as API** — agents call `werk` commands as subprocesses (works with any agent framework)
2. **Structured I/O** — JSON context in, YAML mutations out (the existing `run` / `context` protocol)
3. **Hook system** — pre/post mutation hooks that trigger external processes (NEW)

### Tier 1: CLI as Agent API

Any agent that can execute shell commands can use werk. This already works with Claude Code, Cursor, Aider, or any MCP-equipped tool.

```
Agent calls: werk context 01KK
             → receives JSON with tension + dynamics + family + history
Agent calls: werk reality 01KK "Updated the API endpoints"
             → mutation applied, dynamics recomputed
Agent calls: werk add "Deploy to staging" "Not started" --parent 01KK --horizon +3d
             → child tension created
```

**No special integration needed.** The CLI is the API.

#### Claude Code Integration (via CLAUDE.md)

Place in any project that uses werk:

```markdown
<!-- .claude/CLAUDE.md -->
## werk — Structural Tension Management

This project uses `werk` for tracking structural tensions (gaps between desired and actual states). The workspace is at `.werk/`.

### Reading state
- `werk list --json` — all active tensions with dynamics
- `werk show <id> --json` — full detail for one tension
- `werk context <id>` — full context with family and dynamics (for analysis)
- `werk tree` — hierarchical view of all tensions
- `werk health --json` — system health summary
- `werk diff --since yesterday --json` — recent changes

### Modifying state
- `werk add "desired" "actual" [--parent ID] [--horizon +2w]` — create tension
- `werk reality <id> "new actual state"` — update reality
- `werk desire <id> "new desired state"` — update desire
- `werk resolve <id>` — mark resolved
- `werk note <id> "observation"` — add note
- `werk snooze <id> +3d` — defer a tension

### Agent-assisted analysis
- `werk run <id> "prompt"` — one-shot agent analysis with structured mutations
- `werk insights --json` — behavioral pattern analysis

### Conventions
- Before starting work, run `werk list --urgent` to see what needs attention.
- After completing a significant milestone, update the relevant tension's reality.
- When creating sub-tasks, model them as child tensions of the parent goal.
- Use `werk diff` to understand recent changes before making decisions.
```

Claude Code will naturally discover and use these commands through the CLAUDE.md instructions.

#### Claude Code Hooks (Post-Tool Automation)

Claude Code supports hooks that run after tool executions. Use this to automatically keep werk in sync with development activity:

```json
// .claude/settings.json
{
  "hooks": {
    "post_tool_use": [
      {
        "tool": "Bash",
        "pattern": "git commit",
        "command": "werk note $(werk list --json | jq -r '.[0].id') \"Committed: $(git log -1 --oneline)\""
      }
    ],
    "user_prompt_submit": [
      {
        "command": "echo '---werk-context---' && werk list --urgent --json 2>/dev/null | head -5"
      }
    ]
  }
}
```

This is lightweight: no custom code, just shell commands that feed werk state into Claude's context.

### Tier 2: Structured I/O Protocol (werk run)

The `run` command is the primary agent integration point. It handles the full cycle: context → agent → parse response → apply mutations.

#### One-Shot Mode (Current, Refined)

```bash
werk run 01KK "I just finished the API, what should I update?"
```

Flow:
1. Build markdown context from tension + dynamics + family
2. Append user prompt
3. Append mutation instruction template (YAML format spec)
4. Pipe to `agent.command` via stdin
5. Parse response for YAML mutations between `---` markers
6. Display agent prose response
7. Display each suggested mutation with summary
8. Auto-apply all valid mutations (CLI mode — no interactive confirmation)
9. Record `agent_one_shot` mutation

**Improvements for v0.5:**
- Include resolution forecast in context ("You're behind pace — 60% of required velocity")
- Include structural trajectory in context ("Trajectory: Drifting — gap stable despite engagement. Projected gap at horizon: 0.71")
- Include behavioral insights in context ("This tension has oscillated 3 times")
- Include lever information ("This is the highest-leverage tension in the system")
- Include urgency collision warnings ("This tension's urgency will collide with 2 others in the next 2 weeks")
- Support `--dry-run` to show what would be applied without applying

#### Interactive Mode (Current, Refined)

```bash
werk run 01KK -- claude --dangerously-skip-permissions
```

Flow:
1. Build full `ContextResult` JSON
2. Pipe JSON to subprocess stdin
3. Set env vars: `WERK_TENSION_ID`, `WERK_CONTEXT`, `WERK_WORKSPACE`
4. Subprocess inherits stdout/stderr (agent can interact with user)
5. On exit, record `agent_session` mutation

**Improvements for v0.5:**
- Set additional env vars: `WERK_LEVER_ID` (highest-leverage tension), `WERK_URGENT_COUNT`, `WERK_NEGLECTED_COUNT`
- Support `--context-file /tmp/ctx.json` to write context to a file instead of stdin (for agents that don't read stdin)
- Support `--mutations-file /tmp/mutations.yaml` to read mutations from a file after agent exits

#### New: System-Wide Agent Mode

```bash
werk run --system "Review all urgent tensions and recommend priorities"
werk run --system --json  # return structured response
```

Instead of a single tension context, provides the full system state:
- All active tensions with dynamics
- The lever recommendation
- Health summary
- Recent behavioral insights
- Top urgency/neglect items

This enables agents to reason about the entire tension forest, not just one tension.

#### New: Decomposition Mode

```bash
werk run 01KK --decompose
# or
werk run 01KK "Break this down into actionable sub-tensions"
```

Uses a specialized prompt template that instructs the agent to return `create_child` mutations. The agent sees the parent tension's context and returns 3-7 sub-tensions as structured YAML.

#### New: Batch Mutation Application

```bash
# Apply mutations from a file
werk batch apply mutations.yaml

# Apply mutations from stdin (pipe from agent)
cat agent_output.yaml | werk batch apply -

# Validate without applying
werk batch validate mutations.yaml
```

Format:
```yaml
mutations:
  - action: update_actual
    tension_id: "01KK461Y"
    new_value: "API complete"
    reasoning: "Milestone reached"
  - action: create_child
    parent_id: "01KK461Y"
    desired: "Deploy to staging"
    actual: "Not started"
    reasoning: "Next step after API completion"
```

This enables any external process to produce mutations and apply them in bulk.

### Tier 3: Hook System (NEW)

Hooks are shell commands that run automatically in response to werk events. They enable external systems to react to state changes without polling.

#### Configuration

```toml
# .werk/config.toml

[hooks]
# Post-mutation hooks: run after any state change
# Receives JSON event on stdin
post_mutation = "~/.werk/hooks/post-mutation.sh"

# Post-resolve hooks: run after a tension is resolved
post_resolve = "~/.werk/hooks/post-resolve.sh"

# Post-create hooks: run after a tension is created
post_create = "~/.werk/hooks/post-create.sh"

# Periodic hooks: run on TUI tick (every 60s)
periodic = "~/.werk/hooks/periodic.sh"

# Pre-mutation hooks: run before a mutation, can block it
# Exit 0 = allow, exit 1 = block (with stderr as reason)
pre_mutation = "~/.werk/hooks/pre-mutation.sh"
```

#### Event Payload (stdin JSON)

```json
{
  "event": "mutation",
  "timestamp": "2026-03-13T14:30:00Z",
  "tension_id": "01KK461YBDBEX3W3N2MCWR880A",
  "tension_desired": "Ship the feature",
  "field": "actual",
  "old_value": "API done",
  "new_value": "API complete, UI 80%",
  "dynamics": {
    "phase": "Completion",
    "movement": "Advancing",
    "urgency": 0.92,
    "forecast_on_track": true
  }
}
```

For status changes:
```json
{
  "event": "resolve",
  "timestamp": "2026-03-13T14:30:00Z",
  "tension_id": "01KK461YBDBEX3W3N2MCWR880A",
  "tension_desired": "Ship the feature",
  "cascade": [
    { "id": "...", "desired": "Parent goal", "auto_resolved": true }
  ]
}
```

#### Example Hook Scripts

**Slack notification on resolve:**
```bash
#!/bin/bash
# ~/.werk/hooks/post-resolve.sh
EVENT=$(cat)
DESIRED=$(echo "$EVENT" | jq -r '.tension_desired')
curl -s -X POST "$SLACK_WEBHOOK" \
  -H 'Content-Type: application/json' \
  -d "{\"text\": \"✓ Resolved: $DESIRED\"}"
```

**Auto-commit on mutation:**
```bash
#!/bin/bash
# ~/.werk/hooks/post-mutation.sh
EVENT=$(cat)
FIELD=$(echo "$EVENT" | jq -r '.field')
DESIRED=$(echo "$EVENT" | jq -r '.tension_desired')
cd "$(echo "$EVENT" | jq -r '.workspace')"
git add .werk/sd.db
git commit -m "werk: $FIELD updated on '$DESIRED'" --no-verify 2>/dev/null
```

**Claude agent auto-review on neglect detection:**
```bash
#!/bin/bash
# ~/.werk/hooks/periodic.sh
# Check for newly neglected tensions and ask Claude for advice
NEGLECTED=$(werk list --neglected --json 2>/dev/null)
COUNT=$(echo "$NEGLECTED" | jq 'length')
if [ "$COUNT" -gt 0 ]; then
  FIRST_ID=$(echo "$NEGLECTED" | jq -r '.[0].id')
  werk run "$FIRST_ID" "This tension has been neglected. Should I update reality, snooze it, or release it?" 2>/dev/null
fi
```

**Pre-mutation validation (prevent deleting tensions with children):**
```bash
#!/bin/bash
# ~/.werk/hooks/pre-mutation.sh
EVENT=$(cat)
ACTION=$(echo "$EVENT" | jq -r '.event')
if [ "$ACTION" = "delete" ]; then
  CHILDREN=$(echo "$EVENT" | jq -r '.children | length')
  if [ "$CHILDREN" -gt 0 ]; then
    echo "Cannot delete tension with $CHILDREN children. Move or resolve them first." >&2
    exit 1
  fi
fi
exit 0
```

#### Hook Execution Model

```rust
pub struct HookRunner {
    config: Config,
}

impl HookRunner {
    /// Execute a hook, piping event JSON to stdin.
    /// Returns Ok(true) if hook succeeded or no hook configured.
    /// Returns Ok(false) if pre-hook blocked the action.
    /// Returns Err on hook execution failure (non-blocking — logged as warning).
    pub fn run_hook(&self, hook_name: &str, event: &HookEvent) -> Result<bool> {
        let command = match self.config.get(&format!("hooks.{}", hook_name)) {
            Some(cmd) => cmd,
            None => return Ok(true),  // no hook configured
        };

        let event_json = serde_json::to_string(event)?;

        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                // Write event JSON to stdin
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    stdin.write_all(event_json.as_bytes()).ok();
                }
                child.wait_with_output()
            });

        match output {
            Ok(out) => {
                if hook_name.starts_with("pre_") {
                    // Pre-hooks can block: exit 1 = block
                    Ok(out.status.success())
                } else {
                    // Post-hooks are fire-and-forget
                    Ok(true)
                }
            }
            Err(e) => {
                eprintln!("warning: hook '{}' failed: {}", hook_name, e);
                Ok(true) // don't block on hook failure
            }
        }
    }
}
```

Integration into the store mutation path:

```rust
// In every mutation function (update_actual, resolve, etc.):
fn update_actual(&mut self, id: &str, value: &str) -> Result<()> {
    let event = HookEvent::mutation(id, "actual", old_value, value);

    // Pre-hook: can block
    if !self.hooks.run_hook("pre_mutation", &event)? {
        return Err(WerkError::HookBlocked(/* stderr message */));
    }

    // Perform mutation
    self.engine.store().update_actual(id, value)?;

    // Post-hooks: fire-and-forget
    self.hooks.run_hook("post_mutation", &event).ok();

    Ok(())
}
```

### Agent Framework Compatibility Matrix

| Framework | Integration Method | Notes |
|-----------|-------------------|-------|
| **Claude Code** | CLAUDE.md + CLI commands | Native. Claude reads docs, calls `werk` commands. Hooks via `.claude/settings.json`. |
| **Claude API (direct)** | `werk run --system` + `werk batch apply` | Pipe system context to Claude API, parse mutations, apply via batch. |
| **Cursor** | Rules file + CLI commands | Same pattern as Claude Code — rules file documents `werk` commands. |
| **Aider** | `.aider.conf.yml` + CLI commands | Configure `werk` as a linting/testing step. |
| **Custom Python agents** | `subprocess.run(["werk", ...])` | CLI is the API. JSON output for parsing. |
| **MCP servers** | `werk context --json` as MCP resource | Expose tensions as MCP resources. Agent queries them via MCP protocol. |
| **n8n / Zapier** | Webhook hooks + CLI | Post-mutation hooks send webhooks. Automation platforms call CLI via SSH/exec. |
| **Cron jobs** | `werk diff --json` + `werk run --lever` | Scheduled scripts that check health and trigger agent reviews. |
| **GitHub Actions** | CLI in CI | `werk list --urgent --json` in PR checks. "These tensions are urgent — consider addressing." |

### MCP Server (Future — Optional)

A dedicated MCP server could expose werk as a tool and resource provider:

```rust
// werk-mcp/src/main.rs — hypothetical
// Tools:
//   werk_list(filter, sort) → tension list
//   werk_show(id) → tension detail
//   werk_add(desired, actual, parent, horizon) → new tension
//   werk_reality(id, value) → update actual
//   werk_resolve(id) → resolve
//   werk_context(id) → full context
//   werk_health() → system health
//   werk_insights(days) → behavioral patterns
//
// Resources:
//   werk://tensions — live list of all tensions
//   werk://tension/{id} — single tension detail
//   werk://health — system health summary
//   werk://lever — current highest-leverage action
```

This is lower priority than the CLI + hooks approach because:
1. The CLI already serves as an excellent API
2. MCP servers require per-framework setup
3. Hooks provide the event-driven integration that MCP resources don't

Build MCP only when there's a framework that benefits from it over CLI.

---

## Implementation Roadmap

### Phase 0: Foundation (1-2 days)

1. Extract view state into sub-structs (mechanical refactor)
2. Remove dead code: `show_resolved`, `ToggleResolved`, `verbose` toggle
3. Add relative horizon parsing (`+2w`, `eom`, etc.)
4. Add `notify` crate for filesystem watching

### Phase 1: Core TUI Rebuild (3-5 days)

5. Implement split pane layout (Grid, auto-detect width)
6. Implement tier-grouped dashboard
7. Implement cursor-based Detail navigation (sections, not scroll)
8. Wire resolution forecasting into Detail dynamics section
9. Add Tab for Dashboard ↔ Tree cycling
10. Collapse secondary views (Neighborhood → Detail siblings, Focus → removed, DynamicsSummary → overlay)
11. Implement single-line quick-add
12. Implement persistent search

### Phase 2: Projection Engine — sd-core (2-3 days)

Full implementation plan in `calm-wandering-crab.md`. Summary:

13. **Mutation pattern extraction** — `extract_mutation_pattern()`: mean interval, frequency, frequency trend, gap trend, gap samples. Foundation for all projection.
14. **Projection primitives** — `project_gap_at()`, `project_frequency_at()`, `estimate_time_to_resolution()`: linear extrapolation from observed patterns, clamped to valid ranges.
15. **Per-tension projection** — `project_tension()` → `TensionProjection`: trajectory classification (Resolving/Stalling/Drifting/Oscillating), projected gap at 1w/1m/3m, will_resolve, oscillation_risk, neglect_risk.
16. **Field-level projection** — `project_field()` → `FieldProjection`: trajectory distribution buckets per horizon, urgency collision detection (2+ tensions with urgency >0.7 in same week window).
17. **Lever enhancement** — add `trajectory_urgency` component to lever scoring (Stalling/Oscillating with approaching horizon = high leverage). Rebalance weights.

### Phase 3: Power Features (3-5 days)

18. Implement snooze (mutation + filter + auto-resurface on tick)
19. Implement composite auto-resolution + horizon inheritance
20. Implement recurring tensions
21. Implement undo (5-second window, toast)
22. Implement what-if preview for resolve/release
23. Implement morning review mode
24. Implement behavioral insights computation (includes urgency collision warnings from projection engine)

### Phase 4: TUI Projection Integration (1-2 days)

25. Add trajectory indicator to dashboard rows (`↓`/`—`/`~`/`⇌` after movement char)
26. Add Trajectory section to Detail view (gap progression bars at 1w/1m/3m, risk flags)
27. Cache `FieldProjection` in WerkApp, recompute on 5-minute cycle in `reload_data()`
28. Populate `trajectory` on each `TensionRow` from cached projection
29. Add `:trajectory` command palette action for field-wide structural funnel overlay (trajectory distribution + urgency collisions)

### Phase 5: CLI Expansion (2-3 days)

30. Add `werk list` with rich filtering (--urgent, --neglected, --stagnant, --phase, --snoozed)
31. Add `werk health` and `werk insights`
32. Add `werk diff`
33. Add `werk trajectory` (per-tension and field-wide projection, --collisions, --json)
34. Add `werk reopen`, `werk snooze`, `werk recur`
35. Add `werk batch apply`
36. Add `werk run --system` and `werk run --decompose`
37. Add `werk context --all` and `werk context --urgent`
38. Include projection data in `werk show --json` and `werk context` output

### Phase 6: Hooks & Integration (1-2 days)

39. Implement HookRunner with pre/post mutation hooks
40. Wire hooks into store mutation path
41. Add hook configuration to config.toml schema
42. Write example hook scripts (Slack, auto-commit, agent review)
43. Write CLAUDE.md template for project integration
44. Test with Claude Code hooks

### Phase 7: Field Resonance — sd-core + TUI + CLI (3-5 days)

Full design in `j-field-resonance.md`. This is the qualitative leap: computing dynamics *between* tensions, not just within them. Turns werk from N independent tension meters into a field dynamics instrument.

45. **Coupling analysis in sd-core** — `compute_resonance()` and `compute_resonance_field()`: cross-correlate mutation timelines to detect constructive resonance (co-advancing), destructive interference (competing), harmonic resonance (phase-locked oscillation), and competitive suppression. O(n² × m), cached on 5-minute cycle alongside FieldProjection.
46. **Resonance groups** — connected components of mutual constructive resonance. These are the *actual* creative fronts, discovered from behavior rather than declared hierarchy.
47. **Net field effect** — per-tension aggregate: positive = field-aligned (advancing this helps the field), negative = field-disruptive (advancing this hurts other work).
48. **Lever cascade multiplier** — replace the current `cascade_potential` (child-count heuristic) with empirical coupling evidence. "Advancing this tension has historically advanced 3 others" beats "this tension has 3 children."
49. **TUI: Resonance section in Detail** — between Dynamics and Forecast. Shows co-advancing partners (`◈`), competing tensions (`◇`), net field effect. Each listed tension is navigable.
50. **TUI: Field Resonance overlay** — `Ctrl+F` or command palette. Shows resonance groups as clusters, interference edges, isolated tensions, field coherence score.
51. **TUI: Dashboard Res column** — optional in wide terminals (140+ cols). Compact: `◈2` / `◇1` / `—`.
52. **CLI: `werk resonance`** — field summary (human), per-tension (`werk resonance 01KK`), `--groups`, `--json`.
53. **Agent context** — include resonance in `werk context` output so agents can reason about field structure.

**Depends on:** Phase 2 (projection engine, for cache infrastructure) and Phase 4 (TUI projection integration, for field-level overlay patterns).

### Phase 8: Polish (ongoing)

54. Tree expand/collapse with persisted state
55. Keyboard macro recording (if ftui supports it)
56. Inline mode for "always-on" dashboard
57. MCP server (if demand warrants)
