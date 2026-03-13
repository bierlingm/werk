# Design F: werk TUI Rebuild

**Date:** 2026-03-13
**Status:** Plan complete, ready for implementation
**Priority:** P0 — This is the product.

---

## Vision

werk becomes a TUI-first application built on FrankenTUI. The CLI remains for scripting and agent integration but is no longer the primary human interface. The TUI is the daily practice instrument — the thing you open, leave running, and interact with throughout your work session.

The TUI makes the full power of sd-core's dynamics engine visible and useful without dumping raw computation at the user. It shows the right information at the right time through focused views, keyboard navigation, and real-time dynamics updates.

---

## Architecture

### Crate Structure

```
werk/
├── sd-core/          # Unchanged — computational grammar
├── werk-tui/         # NEW — FrankenTUI application (primary interface)
├── werk-cli/         # Slimmed — scripting, pipes, agent subprocess
└── werk-shared/      # NEW — shared types between tui and cli
```

### Workspace Cargo.toml

```toml
[workspace]
members = ["sd-core", "werk-tui", "werk-cli", "werk-shared"]
resolver = "2"
```

### werk-tui Dependencies

```toml
[dependencies]
sd-core = { path = "../sd-core" }
werk-shared = { path = "../werk-shared" }
ftui = "0.2"
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### werk-shared Purpose

Extract from werk-cli into werk-shared:
- `workspace.rs` — workspace discovery logic
- `prefix.rs` — ID prefix resolution (minus interactive/dialoguer parts)
- `error.rs` — shared error types
- Config loading

Both werk-tui and werk-cli depend on werk-shared. This eliminates duplication without coupling the two interfaces.

### What Gets Cut from werk-cli

| Item | Action | Reason |
|------|--------|--------|
| `toon-format` dependency | Remove | No consumer. JSON covers machine output. |
| `--toon` flag | Remove | Dead format. |
| `dialoguer` dependency | Remove from CLI, not needed in TUI (ftui has its own input) | Replaced by TUI interaction. |
| `owo-colors` dependency | Remove from CLI | CLI becomes plain/JSON only. Human color output moves to TUI. |
| `TOON` variant in Output | Remove | Simplifies output to Human and JSON. |
| `show --verbose` dynamics wall | Remove | TUI handles detailed dynamics display. CLI `show` becomes concise. |
| Duplicate `truncate` functions | Consolidate into werk-shared | 3 copies → 1. |
| Duplicate `ContextResult` structs | Consolidate into werk-shared | `context.rs` and `run.rs` share one definition. |

### What Stays in werk-cli

The CLI becomes the scripting/automation surface:

| Command | Stays | Notes |
|---------|-------|-------|
| `init` | Yes | Workspace creation (also auto-init in TUI) |
| `add` | Yes | Scriptable tension creation |
| `reality` | Yes | Scriptable update |
| `desire` | Yes | Scriptable update |
| `resolve` | Yes | Scriptable status change |
| `release` | Yes | Scriptable status change |
| `rm` | Yes | Scriptable deletion |
| `move` | Yes | Scriptable reparenting |
| `note` | Yes | Scriptable annotation |
| `tree` | Yes | Quick terminal glance (no dynamics computation) |
| `show` | Yes | Concise single-tension view (5 key dynamics only) |
| `context` | Yes | JSON export for agents |
| `run` | Yes | Agent integration |
| `config` | Yes | Configuration management |
| `nuke` | Yes | Workspace deletion |
| `notes` | Defer | Low value as standalone CLI command |
| `horizon` | Merge into `show` | Horizon info shown inline |

---

## TUI Application Design

### FrankenTUI Model

```rust
use ftui::{Model, Cmd, Frame};
use sd_core::DynamicsEngine;

pub struct WerkApp {
    // Core state
    engine: DynamicsEngine,

    // View state
    active_view: View,
    selected_tension_id: Option<String>,
    tension_list: Vec<TensionRow>,

    // Input state
    input_mode: InputMode,
    input_buffer: String,

    // Notifications
    toasts: Vec<Toast>,
}

pub enum View {
    Dashboard,
    Detail(String),   // tension ID
    TreeView,
    AgentChat(String), // tension ID
}

pub enum InputMode {
    Normal,
    AddTension(AddStep),
    EditReality(String),  // tension ID
    EditDesire(String),
    Search,
    Command,
}

pub enum AddStep {
    Desired,
    Actual,
    Parent,
    Horizon,
}
```

### Message Type

```rust
pub enum Msg {
    // Input events (From<Event>)
    Input(ftui::Event),

    // Navigation
    SelectTension(usize),
    OpenDetail(String),
    SwitchView(View),
    Back,

    // Tension CRUD
    TensionCreated(String),         // new ID
    RealityUpdated(String),         // tension ID
    DesireUpdated(String),
    TensionResolved(String),
    TensionReleased(String),
    TensionDeleted(String),
    TensionMoved(String),
    NoteAdded(String),

    // Dynamics
    DynamicsRecomputed(String),     // tension ID

    // sd-core events (from EventBus)
    DynamicsEvent(sd_core::Event),

    // Agent
    AgentResponseReceived(String, String), // tension_id, response text
    AgentMutationApplied(String),

    // System
    Tick,
    Error(String),
    ToastDismissed(usize),
}
```

### Update Loop

```rust
impl Model for WerkApp {
    type Message = Msg;

    fn init(&mut self) -> Cmd<Msg> {
        // Auto-discover or create workspace
        // Load all tensions
        // Compute initial dynamics
        // Start tick for relative time updates
        Cmd::batch(vec![
            Cmd::msg(Msg::DynamicsRecomputed("*".into())),
            Cmd::tick(Duration::from_secs(60)), // refresh relative times
        ])
    }

    fn update(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::Input(event) => self.handle_input(event),
            Msg::SelectTension(idx) => { /* update selection */ Cmd::none() },
            Msg::OpenDetail(id) => {
                self.active_view = View::Detail(id.clone());
                self.recompute_dynamics(&id)
            },
            Msg::TensionCreated(id) => {
                self.reload_tensions();
                Cmd::msg(Msg::DynamicsRecomputed(id))
            },
            Msg::RealityUpdated(id) => {
                self.reload_tensions();
                self.recompute_dynamics(&id)
            },
            // ... etc
            Msg::Tick => {
                Cmd::tick(Duration::from_secs(60))
            },
            _ => Cmd::none(),
        }
    }

    fn view(&self, frame: &mut Frame<'_>) {
        match &self.active_view {
            View::Dashboard => self.view_dashboard(frame),
            View::Detail(id) => self.view_detail(frame, id),
            View::TreeView => self.view_tree(frame),
            View::AgentChat(id) => self.view_agent(frame, id),
        }
    }
}
```

---

## Views

### 1. Dashboard View (Default)

The view you see when you launch `werk`. Answers: "What should I do right now?"

```
┌─ werk ──────────────────────────────────────────────────────────┐
│                                                                  │
│  ⚠ 2 past horizon  ·  1 neglected  ·  5 active  ·  3 resolved  │
│                                                                  │
│  URGENT                                                          │
│  ▸ [C] → Fix auth middleware          2026-03-14  █████████░ 92% │
│    [A] ↔ Record demo video            2026-03-15  ██████░░░░ 61% │
│                                                                  │
│  ACTIVE                                                          │
│    [G] ○ Write the blog post          2026-03       ░░░░░░░░  8% │
│    [A] → Design new onboarding flow   —             ████░░░░ 45% │
│    [G] ○ Refactor payment module      —             ░░░░░░░░  3% │
│                                                                  │
│  NEGLECTED                                                       │
│    [G] ○ Update API documentation     2026-04       ░░░░░░░░  0% │
│                                                                  │
│──────────────────────────────────────────────────────────────────│
│  j/k navigate  Enter detail  a add  r reality  d desire         │
│  t tree  / search  q quit  ? help                                │
└──────────────────────────────────────────────────────────────────┘
```

**Layout structure:**
- Status bar (top): Badge widgets showing counts by category
- Tension list (center): Grouped by urgency tier, sorted by urgency within tier
- Keybinding bar (bottom): Context-sensitive key hints

**Urgency tiers:**
- URGENT: urgency > 0.75 or past horizon
- ACTIVE: all other active tensions, sorted by urgency (those without horizons last)
- NEGLECTED: detected by neglect dynamic
- (Resolved/Released hidden by default, toggle with `R`)

**Each tension row shows:**
- Selection indicator (`▸` for selected)
- Phase badge: `[G]` Germination, `[A]` Assimilation, `[C]` Completion, `[M]` Momentum
- Movement arrow: `→` Advancing, `↔` Oscillating, `○` Stagnant
- Desired text (truncated to fit)
- Horizon (human-readable: "2026-03-14", "Mar 2026", "—" if none)
- Urgency bar (ProgressBar widget, only when horizon exists)

**Widgets used:** List, Badge, ProgressBar, StatusLine, Panel, Rule

### 2. Detail View

Opened by pressing Enter on a tension in the dashboard.

```
┌─ Detail ────────────────────────────────────────────────────────┐
│                                                                  │
│  Fix auth middleware                                  01KK461Y   │
│                                                                  │
│  Desired  Published blog post on structural dynamics             │
│  Actual   Draft complete, needs editing                          │
│  Status   Active           Created  3 days ago                   │
│  Horizon  2026-03-14       2 days remaining                      │
│                                                                  │
│  ── Dynamics ──────────────────────────────────────────────────  │
│  Phase       Completion        Magnitude  ████████░░ 0.78        │
│  Movement    → Advancing       Urgency    █████████░ 92%         │
│  Conflict    None              Neglect    None                   │
│                                                                  │
│  ── History (last 10) ─────────────────────────────────────────  │
│  3 days ago   [created]   desired="Fix auth middleware"           │
│  2 days ago   [actual]    "Reviewed existing code" → "Draft..."  │
│  5 hours ago  [actual]    "Draft complete" → "Draft, needs ed."  │
│                                                                  │
│  ── Children (2) ──────────────────────────────────────────────  │
│  01KK48  [G] ○ Review token storage                              │
│  01KK4A  [A] → Implement new session handler                     │
│                                                                  │
│──────────────────────────────────────────────────────────────────│
│  Esc back  r reality  d desire  R resolve  X release  n note    │
│  a add-child  m move  Del delete  ? help                         │
└──────────────────────────────────────────────────────────────────┘
```

**Layout:** Vertical split — info panel top, history middle, children bottom.

**Dynamics shown (5 key ones):**
- Phase + Movement (always)
- Magnitude (ProgressBar or numeric)
- Urgency (ProgressBar, only with horizon)
- Conflict (only if detected)
- Neglect (only if detected)

Full dynamics available via `v` (verbose toggle) which adds:
- Oscillation, Resolution, Orientation, CompensatingStrategy, AssimilationDepth, HorizonDrift

**Widgets used:** Panel, Paragraph, ProgressBar, List, Rule, Badge

### 3. Tree View

Full forest visualization. Navigable.

```
┌─ Tree ──────────────────────────────────────────────────────────┐
│                                                                  │
│  ├── [C] → 01KK46  Fix auth middleware              Mar 14  92% │
│  │   ├── [G] ○ 01KK48  Review token storage                     │
│  │   └── [A] → 01KK4A  Implement session handler                │
│  ├── [A] ↔ 01KK4B  Record demo video                Mar 15  61% │
│  ├── [G] ○ 01KK4C  Write the blog post              Mar        │
│  │   └── [G] ○ 01KK4D  Research competitors                     │
│  └── [G] ○ 01KK4E  Design new onboarding flow                   │
│                                                                  │
│  Total: 7  Active: 5  Resolved: 1  Released: 1                  │
│                                                                  │
│──────────────────────────────────────────────────────────────────│
│  j/k navigate  Enter detail  Esc dashboard  f filter  ? help    │
└──────────────────────────────────────────────────────────────────┘
```

**Widgets used:** Tree (ftui's built-in hierarchical widget), Badge, StatusLine

### 4. Agent View

Inline mode: agent output scrolls above, tension context stays pinned below.

```
┌─ Agent: Fix auth middleware ─────────────────────────────────────┐
│                                                                   │
│  Agent: Based on your progress, I suggest updating the reality   │
│  to reflect that the draft is complete. The token storage review │
│  should be tracked as a separate child tension.                  │
│                                                                   │
│  ── Suggested Changes ────────────────────────────────────────── │
│  1. Update actual: "Draft complete, pending review"              │
│  2. Create child: "Token storage audit passed"                   │
│  3. Add note: "Session handler uses JWT, not cookies"            │
│                                                                   │
│  [Apply all]  [Review each]  [Cancel]                            │
│                                                                   │
│──────────────────────────────────────────────────────────────────│
│  Desired  Fix auth middleware                                     │
│  Actual   Draft complete, needs editing                           │
│  Phase    Completion   Movement  → Advancing   Urgency  92%      │
│──────────────────────────────────────────────────────────────────│
│  Enter apply  Tab cycle  Esc cancel  ? help                      │
└──────────────────────────────────────────────────────────────────┘
```

**Widgets used:** LogViewer (scrolling agent output), Panel (pinned context), Modal (for mutation review)

### 5. Inline Input Overlays

When the user presses `a` (add), `r` (reality), `d` (desire), or `n` (note), a TextInput overlay appears at the bottom of the current view. No modal dialogs, no view switches — type and press Enter.

```
│──────────────────────────────────────────────────────────────────│
│  Update reality for "Fix auth middleware":                       │
│  > Draft complete, sent for review_                              │
│──────────────────────────────────────────────────────────────────│
```

For `a` (add tension), a multi-step flow:
```
│  New tension (1/2):                                              │
│  Desired: > Write integration tests for auth_                    │
│──────────────────────────────────────────────────────────────────│
```
Then:
```
│  New tension (2/2):                                              │
│  Actual: > Haven't started_                                      │
│  (Enter to create, Tab to set parent/horizon, Esc to cancel)    │
│──────────────────────────────────────────────────────────────────│
```

**Widgets used:** TextInput, Panel

### 6. Command Palette

Triggered by `/` or `:`. Fuzzy-search over all available actions.

```
│  > res                                                           │
│  ▸ resolve    Mark tension as resolved                           │
│    release    Release tension (let go)                            │
│    reality    Update current state                                │
│──────────────────────────────────────────────────────────────────│
```

**Widgets used:** CommandPalette (ftui built-in)

---

## Keyboard Map

### Global (all views)

| Key | Action |
|-----|--------|
| `q` | Quit |
| `?` | Toggle help overlay |
| `/` or `:` | Open command palette |
| `1` | Switch to Dashboard |
| `2` | Switch to Tree |
| `Ctrl-C` | Quit |

### Dashboard & Tree (list navigation)

| Key | Action |
|-----|--------|
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `Enter` | Open detail view for selected tension |
| `a` | Add new tension (inline input) |
| `r` | Update reality of selected tension |
| `d` | Update desire of selected tension |
| `n` | Add note to selected tension |
| `R` | Toggle showing resolved/released |
| `f` | Cycle filter (all / active / resolved / released) |

### Detail View

| Key | Action |
|-----|--------|
| `Esc` | Back to previous view |
| `r` | Update reality (inline input) |
| `d` | Update desire (inline input) |
| `n` | Add note (inline input) |
| `h` | Set/update horizon (inline input) |
| `R` | Resolve this tension |
| `X` | Release this tension |
| `a` | Add child tension |
| `m` | Move (reparent) — opens tension picker |
| `Del` | Delete (with confirmation) |
| `v` | Toggle verbose dynamics |
| `g` | Open agent view for this tension |

### Input Mode (TextInput active)

| Key | Action |
|-----|--------|
| `Enter` | Submit |
| `Esc` | Cancel |
| `Tab` | Next field (in multi-step add) |

### Agent View

| Key | Action |
|-----|--------|
| `Esc` | Cancel / back to detail |
| `Enter` | Apply selected suggestion |
| `Tab` | Cycle through suggestions |
| `a` | Apply all suggestions |
| `1-9` | Toggle individual suggestion |

---

## DynamicsEngine Integration

### Long-lived Engine

The TUI holds the DynamicsEngine for the entire session. Previous state persists across interactions, enabling real transition event detection.

```rust
impl WerkApp {
    fn recompute_dynamics(&mut self, tension_id: &str) -> Cmd<Msg> {
        // Engine already has previous state from last computation
        let events = self.engine.compute_and_emit(tension_id);

        // Convert sd-core events to TUI messages
        let cmds: Vec<Cmd<Msg>> = events
            .into_iter()
            .map(|e| Cmd::msg(Msg::DynamicsEvent(e)))
            .collect();

        Cmd::batch(cmds)
    }
}
```

### Event → Toast Mapping

When the engine detects a transition, show a Toast notification:

| sd-core Event | Toast |
|---------------|-------|
| `OscillationDetected` | "Oscillation detected on: {desired}" |
| `ResolutionAchieved` | "Resolution achieved: {desired}" |
| `NeglectDetected` | "{desired} is being neglected" |
| `UrgencyThresholdCrossed` | "{desired} is now urgent" |
| `ConflictDetected` | "Conflict between {n} sibling tensions" |
| `LifecycleTransition` | "Phase: {old} → {new}" |
| `HorizonDriftDetected` | "Horizon drifting: {type}" |
| `CompensatingStrategyDetected` | "Compensating strategy: {type}" |

**Widgets used:** Toast, NotificationQueue

---

## Relative Time Display

All timestamps in the TUI use relative time. No RFC3339 anywhere in human-facing output.

```rust
fn relative_time(dt: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let delta = now.signed_duration_since(dt);
    let secs = delta.num_seconds();
    match secs {
        s if s < 60 => "just now".into(),
        s if s < 3600 => format!("{} min ago", s / 60),
        s if s < 86400 => format!("{} hours ago", s / 3600),
        s if s < 604800 => format!("{} days ago", s / 86400),
        s => format!("{} weeks ago", s / 604800),
    }
}
```

This goes in werk-shared since the CLI could use it too.

---

## Auto-Init

When the TUI launches and no workspace is found:

1. Show a welcome screen with a brief explanation
2. Ask: "Create workspace here (.werk/) or globally (~/.werk/)?"
3. Initialize and proceed to empty dashboard
4. Dashboard shows "No tensions yet. Press `a` to create your first."

No `werk init` ceremony required.

---

## Application Entry Point

The `werk` binary becomes the TUI. The CLI becomes `werk-cli` or is accessed via flags.

**Option A (recommended):** Single binary, mode detection.

```rust
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        // Any subcommand → CLI mode
        cli::run();
    } else {
        // No args → TUI mode
        tui::run();
    }
}
```

This means:
- `werk` → launches TUI
- `werk add "..." "..."` → CLI mode
- `werk tree` → CLI mode
- `werk show 01KK` → CLI mode

**Option B:** Two binaries (`werk` for TUI, `werk-cli` for CLI). More separation but worse ergonomics.

**Recommendation:** Option A. One binary. The TUI is the default, the CLI is the fallback for scripting and non-interactive use.

### Implementation of Option A

Keep werk-cli as the package that builds the `werk` binary. Add `werk-tui` as a library dependency. The binary detects whether to launch TUI or dispatch CLI commands.

```toml
# werk-cli/Cargo.toml
[dependencies]
sd-core = { path = "../sd-core" }
werk-shared = { path = "../werk-shared" }
werk-tui = { path = "../werk-tui" }  # TUI as library
ftui = "0.2"
clap = { version = "4", features = ["derive"] }
# ... rest of CLI deps
```

```rust
// werk-cli/src/main.rs
fn main() {
    if std::env::args().len() <= 1 && std::io::stdin().is_terminal() {
        // No args + interactive terminal → TUI
        werk_tui::run();
    } else {
        // Has args or piped input → CLI
        let args = Cli::parse();
        // ... existing dispatch
    }
}
```

---

## Implementation Phases

### Phase 0: Prepare the Foundation (Cuts & Consolidation)

**Goal:** Clean the codebase before building on it.

**Tasks:**

0.1. Remove TOON format
- Delete `TOON` variant from `Output` enum
- Remove `--toon` flag from clap
- Remove `toon-format` from Cargo.toml
- Remove `is_toon()`, `print_toon()`, and related methods
- Update tests that reference TOON

0.2. Create `werk-shared` crate
- Extract `workspace.rs` (workspace discovery)
- Extract `prefix.rs` (prefix resolution, without dialoguer — pure logic only)
- Extract `error.rs` (shared error types)
- Extract `truncate()` function (one copy)
- Extract `relative_time()` function (new)
- Extract config loading
- Both werk-cli and werk-tui depend on werk-shared

0.3. Consolidate duplicates
- Merge `ContextResult` from `context.rs` and `run.rs` into werk-shared
- Remove duplicate `truncate` implementations

0.4. Simplify CLI `show`
- Default output shows 5 dynamics: Phase, Magnitude, Urgency, Neglect, Movement
- Remove verbose dynamics wall
- Use relative time for timestamps
- `--json` still outputs everything

0.5. Verify
- All existing tests pass
- `cargo clippy` clean
- No functional regressions

### Phase 1: Scaffold the TUI

**Goal:** A working TUI that displays tensions. No editing yet.

**Tasks:**

1.1. Create `werk-tui` crate
- Add to workspace
- Depend on `sd-core`, `werk-shared`, `ftui`
- Define `WerkApp` struct implementing `Model`
- Define `Msg` enum with `From<ftui::Event>`

1.2. Implement Dashboard view (read-only)
- Load tensions from store via DynamicsEngine
- Build `TensionRow` list with computed dynamics
- Sort by urgency tier (URGENT / ACTIVE / NEGLECTED)
- Render with List widget, Badge for phases, ProgressBar for urgency
- Status bar with counts

1.3. Implement navigation
- `j`/`k`/`↑`/`↓` to navigate tension list
- `q` to quit
- `?` for help overlay

1.4. Wire the entry point
- Modify `werk-cli/src/main.rs` to detect no-args and launch TUI
- `werk` → TUI, `werk <subcommand>` → CLI

1.5. Verify
- TUI launches and shows tensions
- Navigation works
- Quitting restores terminal (RAII via TerminalSession)
- CLI commands still work unchanged

### Phase 2: Detail View & Read-Only Navigation

**Goal:** Navigate the full tension forest from the TUI.

**Tasks:**

2.1. Implement Detail view
- Enter on selected tension opens detail
- Show: desired, actual, status, created (relative time), horizon, parent
- Show 5 key dynamics with ProgressBar for magnitude/urgency
- Show last 10 mutations with relative timestamps
- Show children as navigable list
- `Esc` returns to dashboard

2.2. Implement Tree view
- `t` or `2` switches to tree view
- Use ftui Tree widget with forest data
- Phase badges, movement arrows, horizon annotations
- Navigate with j/k, Enter to open detail, Esc to dashboard

2.3. Implement verbose dynamics toggle
- `v` in detail view toggles full dynamics display
- Shows all 13 dynamics when verbose

2.4. Implement filter cycling
- `f` cycles through: Active → All → Resolved → Released → Active
- `R` quick-toggle resolved/released visibility
- Filter state shown in status bar

2.5. Verify
- Full navigation flow: dashboard → detail → tree → dashboard
- All views render correctly at various terminal sizes
- Dynamics display correctly for tensions with/without horizons

### Phase 3: Inline Editing

**Goal:** Full CRUD from within the TUI. Never leave the application.

**Tasks:**

3.1. Implement TextInput overlay system
- Input overlay renders at bottom of current view
- Captures all keyboard input until Enter/Esc
- Does not switch views — overlays on current view

3.2. Implement `r` (update reality)
- Opens TextInput pre-filled with current actual value
- Enter commits: calls `engine.store().update_actual()`
- Triggers dynamics recomputation
- Shows Toast on success

3.3. Implement `d` (update desire)
- Same pattern as reality

3.4. Implement `a` (add tension)
- Multi-step: desired → actual → (optional: parent picker, horizon)
- Tab advances to optional fields, Enter at any point creates with defaults
- If in detail view, auto-sets parent to current tension
- New tension appears in list, dynamics recompute

3.5. Implement `n` (add note)
- Single TextInput, commits as mutation

3.6. Implement `h` (set horizon)
- TextInput accepting horizon formats: "2026", "2026-03", "2026-03-15", "2026-03-15T14:00"
- Validation feedback in real-time

3.7. Implement `R` (resolve) and `X` (release)
- Confirmation prompt (inline, not modal)
- Status change + dynamics recompute + toast

3.8. Implement `Del` (delete)
- Confirmation with tension desired text shown
- Auto-reparent children (same as CLI behavior)

3.9. Implement `m` (move/reparent)
- Opens a tension picker (filtered list of potential parents)
- Select new parent, Enter to confirm

3.10. Verify
- All CRUD operations work from TUI
- Mutations are persisted (verify by opening CLI in another terminal)
- Dynamics recompute after every change
- Toast notifications appear for significant events

### Phase 4: Live Dynamics & Events

**Goal:** The engine runs persistently, detects transitions, and notifies the user.

**Tasks:**

4.1. Wire EventBus to TUI Messages
- Subscribe to engine's EventBus
- Map each sd-core Event type to a Msg::DynamicsEvent
- Handle in update() by showing appropriate Toast

4.2. Implement Toast notifications
- Use ftui Toast / NotificationQueue widgets
- Auto-dismiss after 5 seconds
- Color-coded by severity (info, warning, alert)
- Map event types to toast messages per the table above

4.3. Implement live dynamics update
- After any mutation, recompute dynamics for affected tension
- If tension has siblings, recompute for siblings too (conflict detection)
- Engine tracks PreviousDynamics, emits transition events
- Sparkline widget showing urgency trend over time (in detail view)

4.4. Implement periodic refresh
- Tick every 60 seconds to update relative timestamps
- Re-evaluate urgency (time-dependent) on each tick
- If urgency crosses threshold between ticks, emit event/toast

4.5. Verify
- Transition events fire correctly (create tension, mutate until oscillation triggers)
- Toasts appear and auto-dismiss
- Urgency bars update over time
- No performance issues with periodic recomputation

### Phase 5: Agent Integration

**Goal:** Run AI agents from within the TUI with inline results.

**Tasks:**

5.1. Implement Agent view
- `g` from detail view opens agent view for that tension
- Split layout: scrolling agent output (top), pinned tension context (bottom)
- Use LogViewer for agent output

5.2. Implement agent prompt input
- TextInput at bottom of agent view
- Enter sends prompt to agent subprocess (same mechanism as `werk run`)
- Agent output streams into LogViewer

5.3. Implement structured response handling
- Parse YAML responses (reuse `agent_response.rs` from werk-shared)
- Display suggested mutations as a selectable list
- Apply all / Review each / Cancel flow using keyboard

5.4. Implement mutation application from agent
- Same logic as CLI `run.rs` `apply_single_mutation`
- After applying, recompute dynamics, show toast, update detail view

5.5. Use `Cmd::task()` for agent execution
- Agent subprocess runs in background via `Cmd::task()`
- Result returns as `Msg::AgentResponseReceived`
- UI remains responsive during agent execution
- Spinner widget while waiting

5.6. Verify
- Agent launches from TUI
- Response displays in LogViewer
- Structured mutations can be reviewed and applied
- Dynamics update after agent-suggested changes

### Phase 6: Auto-Init & Polish

**Goal:** Remove friction, polish the experience.

**Tasks:**

6.1. Auto-init on first launch
- Detect no workspace
- Show welcome screen
- Offer local vs global init
- Create workspace and proceed to empty dashboard

6.2. Command palette
- `/` or `:` opens CommandPalette
- Fuzzy search over all actions
- Execute selected action

6.3. Search
- Text search over tension desired/actual fields
- Filter dashboard list to matches
- Enter to navigate to result

6.4. Empty states
- Dashboard with no tensions: welcoming message + "press `a` to begin"
- Detail with no children: clean, no empty headers
- Tree with no tensions: same as dashboard empty state

6.5. Responsive layout
- Handle small terminals gracefully (minimum 40 cols)
- Truncate fields appropriately
- Hide less-important columns when space is tight

6.6. Help overlay
- `?` shows full keybinding reference
- Context-sensitive (shows keys relevant to current view)
- Use ftui Help / HelpRegistry widgets

6.7. Verify
- Fresh install → first launch → auto-init → add first tension → full workflow
- Works in terminals from 40 cols to 200+ cols
- Help overlay accurate for every view

---

## Testing Strategy

### Unit Tests (werk-shared)
- Workspace discovery
- Prefix resolution
- Relative time formatting
- Truncation

### Unit Tests (werk-tui)
- Model state transitions: verify that update() produces correct state for each Msg
- View data preparation: verify TensionRow construction, urgency tier sorting
- Input handling: verify key → Msg mapping

### Snapshot Tests (ftui built-in)
- Dashboard with 0, 1, 5, 20 tensions
- Detail view for tension with/without horizon, with/without children
- Tree view with nested hierarchy
- Agent view with pending suggestions
- Input overlay rendering
- Toast notification rendering

### Integration Tests
- Full lifecycle: launch → add tension → update reality → resolve → verify state
- Agent integration: mock agent subprocess, verify structured response handling
- Workspace discovery: local vs global, auto-init

### Existing Tests
- All sd-core tests remain unchanged (703 tests)
- CLI integration tests adapted for simplified show output
- CLI tests for removed TOON paths deleted

---

## Success Criteria

The rebuild is complete when:

1. `werk` (no args) launches a TUI that shows all active tensions sorted by urgency
2. You can navigate to any tension and see its dynamics without leaving the TUI
3. You can create, update, resolve, and release tensions entirely from within the TUI
4. Dynamics recompute live after every change with transition notifications
5. You can launch an agent session from the TUI and apply its suggestions
6. The CLI still works for scripting: `werk add`, `werk tree`, `werk show`, etc.
7. First launch auto-initializes without requiring `werk init`
8. The application feels fast — 60fps rendering, no perceptible lag on any operation
9. The codebase is smaller than today despite having more functionality

---

## Dependency Summary

### Added
| Crate | Version | Purpose |
|-------|---------|---------|
| `ftui` | 0.2 | TUI framework |

### Removed
| Crate | Reason |
|-------|--------|
| `toon-format` | Dead format, no consumers |
| `dialoguer` | Replaced by ftui input widgets |
| `owo-colors` | Replaced by ftui styling |

### Kept
| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing (still needed for CLI mode) |
| `serde` / `serde_json` | Serialization (agent context, JSON output) |
| `serde_yaml` | Agent response parsing |
| `chrono` | Time handling |
| `which` | Agent command resolution |
| `regex` | Pattern matching |
| `toml` | Config parsing |

---

## Open Questions

1. **Inline mode vs fullscreen?** FrankenTUI supports both. Inline keeps scrollback and works alongside other terminal output. Fullscreen gives more space. Recommendation: default to fullscreen (`App::fullscreen()`), with a `--inline` flag for users who want it pinned in a portion of the terminal.

2. **Single binary vs two binaries?** Plan assumes single binary (Option A). If the ftui dependency meaningfully increases binary size or compile time, reconsider Option B.

3. **Agent streaming?** Current agent integration waits for full response. FrankenTUI's async `Cmd::task()` could enable streaming agent output character-by-character. Worth exploring in Phase 5 but not blocking.

4. **WebAssembly build?** FrankenTUI compiles to WASM with WebGPU rendering. A browser-based werk is technically possible. Not in scope for this plan but architecturally enabled.

---

## File Locations

```
designs/
├── f-tui-rebuild.md              (this file)
├── INDEX.md                       (update with this design)
├── a-agent-command-resolution.md  (completed in latest commit)
├── b-interactive-config.md        (superseded by TUI)
├── c-id-collision-disambiguation.md (completed in latest commit)
├── d-werk-run-inline-prompt.md    (completed in latest commit)
└── e-one-shot-with-structured-suggestions.md (completed in latest commit)
```

Designs A, C, D, E are implemented. Design B (interactive config) is superseded by the TUI's command palette and inline editing — no separate interactive config flow needed when the whole application is interactive.
