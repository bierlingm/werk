# TUI Ideas v0.5 — Mined from Dicklesworthstone's Ecosystem

**Date:** 2026-03-13
**Source:** beads_viewer, FrankenTUI, beads_viewer_rust, NTM, FrankenTerm, OpenTUI Rust, charmed_rust, source2prompt_tui, process_triage, rich_rust

---

## Architecture

### Immutable Snapshot Data Flow

beads_viewer's core trick: a `BackgroundWorker` on a separate thread builds an immutable `DataSnapshot` containing all pre-computed list items, tree roots, board columns, and graph layouts. When done, it atomically swaps the snapshot into the UI model. The UI thread only ever reads — never mutates — the current snapshot.

No locks. No jank. Bubble Tea guarantees `update()` and `view()` never run concurrently, and snapshots are immutable once created.

For werk: `DynamicsEngine` produces a `DynamicsSnapshot` on a background thread. The TUI reads from it lock-free. When a mutation happens, a new snapshot is built and swapped in.

### Two-Phase Computation

beads_viewer splits expensive work into two phases:

- **Phase 1 (instant):** Basic data — issue titles, statuses, simple counts. UI renders immediately.
- **Phase 2 (async, 500ms timeout):** Expensive graph metrics — PageRank, betweenness centrality, HITS, critical path. UI updates seamlessly when ready.

For werk: Phase 1 shows block text, status, horizon. Phase 2 computes dynamics scores, tension analysis, neighborhood relationships. The user never waits for computation — they see useful content immediately and it gets richer as background work completes.

### Dataset Tier System

beads_viewer automatically scales computation based on dataset size: <1K, 1-5K, 5-20K, 20K+ items. Smaller datasets get full analysis. Larger datasets get approximations or skip expensive metrics entirely.

For werk: Skip expensive dynamics like full neighborhood conflict detection when the forest has 500+ tensions. Compute incrementally when change ratio <20%.

### Passive-First Architecture (FrankenTerm)

The monitoring loop has zero side effects. Action loops are strictly separated with policy gates. Event-driven waits (pattern match, idle detection) instead of `sleep()` polling.

For werk: The TUI's observation of state (reading tensions, computing dynamics) is strictly separated from mutation (creating, updating, resolving). No mutation happens without an explicit user action passing through a policy gate.

---

## Navigation

### Multi-View Modal Switching

beads_viewer uses single-keystroke view changes: `l` list, `b` board, `g` graph, `t` tree, `i` insights, `h` history. Each view is a separate model struct handling its own input and rendering. The parent model dispatches messages based on current focus.

26+ UI contexts organized into priority tiers:
1. **Overlays** (highest): label picker, help, tutorial, confirmation modals
2. **Views**: graph, board, tree, insights, history
3. **Detail states**: split view, time-travel comparison
4. **Filter/default**: search mode, base list

The priority-ordered `CurrentContext()` function evaluates model state from overlays down to default, returning the most specific active context. Everything downstream — keybindings, help sidebar, status bar — keys off this single context value.

### Context-Sensitive Shortcuts Sidebar

beads_viewer has a toggleable 34-character-wide sidebar showing only the keybindings relevant to the current view. Categories: Navigation, Views, Graph, Insights, History, Board, Filters, Actions (40+ total shortcuts). It only shows what matters right now.

For werk: A `?` toggle that pins a narrow sidebar with keybindings for the current view. Dashboard shows navigation + CRUD keys. Detail shows editing + dynamics keys. Agent view shows apply/review/cancel keys. Never the full list.

### Command Palette with Bayesian Scoring

FrankenTUI's built-in `CommandPalette` widget uses Bayesian scoring for fuzzy matching, not just substring or Levenshtein distance. It learns which commands you use most frequently and surfaces them higher. Live preview pane shows what the command will do before you execute it.

NTM uses this as the primary interaction mode — the fuzzy-searchable command palette with live preview is the main way you interact.

### Vim-Style with Per-View Specializations

`j`/`k` everywhere. `h`/`l` for pane switching. `g`/`G` for top/bottom. `/` for fuzzy search, `n`/`N` for cycling matches. But each view adds its own meaning: arrows expand/collapse tree nodes, numbers jump swimlanes on board view, `n`/`N` navigate search matches.

---

## Layout

### Adaptive Width Thresholds

beads_viewer adjusts layout at specific width breakpoints:
- `<100 cols`: Single pane only
- `100-140 cols`: Split view (list + detail side-by-side)
- `140-180+ cols`: Ultra-wide mode with extra columns and sparklines

Each list item is a mini-dashboard with responsive columns. Left side always shows: selection indicator, type icon, priority badge, status, title. Right side conditionally appears: age (>80 cols), comment count (>100), sparkline (>120), assignee (>100), labels (>140).

For werk: Narrow terminals get the tension list with just phase badge + title. Medium adds horizon and urgency bar. Wide adds sparklines for dynamics trends, neighbor count, and inline magnitude bars.

### FrankenTUI's Layout Primitives

**Flex** (1D constraint solver):
```rust
Flex::vertical()
    .constraints([
        Constraint::Fixed(3),      // header
        Constraint::Min(10),       // content
        Constraint::Fixed(1),      // footer
    ])
    .gap(1)
    .margin(Sides::all(1))
    .split(area);
```

**Grid** (2D named areas):
```rust
Grid::new()
    .rows([Constraint::Fixed(3), Constraint::Min(10), Constraint::Fixed(1)])
    .columns([Constraint::Percentage(30.0), Constraint::Min(20)])
    .area("sidebar", GridArea::span(0, 0, 2, 1))
    .area("content", GridArea::cell(0, 1));
```

**Constraint types:** `Fixed`, `Percentage`, `Min`, `Max`, `Ratio`, `Fill`, `FitContent`, `FitContentBounded`, `FitMin`.

**Pane manager:** Full split-tree with drag-to-resize, inertial throw, hysteresis, pressure snap. Serializable snapshots for saving/restoring layouts.

**Breakpoints:** `Xs`/`Sm`/`Md`/`Lg`/`Xl` — responsive layouts that reorganize based on terminal width.

### Inline Mode

FrankenTUI's key differentiator: `ScreenMode::Inline { ui_height: N }` keeps a stable N-row UI region while normal terminal scrollback continues above. Not alt-screen takeover — coexistence.

For werk as a "leave it open" daily instrument: inline mode means you can still run shell commands above the UI. The tension dashboard stays pinned at the bottom. When you need full detail, switch to alt-screen mode with a keystroke.

---

## Visual Design

### Semantic Design Tokens

beads_viewer never uses raw hex colors. Everything goes through semantic tokens:

**Spacing constants:** XS=1, S=2, M=3, L=4, XL=6 — consistent rhythm.

**Semantic colors:**
- Status: green (open), blue (in progress), red (blocked), gray (closed)
- Priority: P0 critical through P4 backlog, each with distinct color
- Type: Bug, Feature, Task, Epic, Chore — each with emoji + color
- Age: green (<7 days), yellow (7-30 days), red (>30 days)

**Pre-computed styles:** Theme struct holds all pre-built style objects. No style computation during rendering — allocate once, reuse every frame.

For werk: Phase colors (germination green, assimilation blue, completion amber, momentum purple). Movement indicators (advancing →, oscillating ↔, stagnant ○). Urgency gradient from cool to hot. All pre-computed in a WerkTheme struct.

### Adaptive Color Degradation

Detects terminal capabilities (TrueColor, ANSI256, basic 16-color) and adjusts automatically. WCAG AA compliance target (~4.6:1 contrast ratio). Light/dark detection with env var override.

FrankenTUI's `ColorProfile` detection from `TERM`, `COLORTERM`, `NO_COLOR` with automatic downgrade chain: RGB → ANSI16 → Mono. Built-in `contrast_ratio()`, `meets_wcag_aa()`, `best_text_color()` utilities.

### Unicode Visual Elements

beads_viewer uses these extensively:
- **Sparklines:** `▂▃▄▅▆▇█` for inline data trends (urgency over time, activity frequency)
- **Progress bars:** `█` filled, `░` empty
- **Tree connectors:** `│`, `├──`, `└──` with `▾`/`▸`/`●` expand indicators
- **Heatmap colors:** 8-color gradient (dark blue → navy → light blue → gold → coral → hot pink)
- **Mini bars:** Normalized importance bars inside metric panels
- **Rank badges:** Percentile-based coloring
- **Confidence styling:** 80%+ green, 50%+ yellow, lower gray

For werk: Sparklines showing urgency trend over last N mutations. Mini bars for magnitude in list view. Heatmap-style coloring for dynamics dashboard showing which tensions are "hot."

### Alpha Blending (OpenTUI Rust)

Porter-Duff "over" compositing for semi-transparent overlays in terminal. Scissor-based clipping for nested viewports. This enables frosted-glass effects for modals and overlays.

For werk: Modal confirmations (resolve, delete) could use semi-transparent overlay over the current view, keeping context visible but dimmed.

---

## Information Display

### Rich List Item Rendering

Each beads_viewer list row is a responsive mini-dashboard:

**Left (always shown):** selection indicator → repo badge → type icon → priority badge → priority direction → triage indicators → status badge → search score → issue ID → diff badge → title (fills remaining width)

**Right (conditional on width):** age → comment count → sparkline (>120 cols) → assignee (>100) → labels (>140)

Uses `lipgloss.Width()` for correct emoji/CJK character width handling. Unicode-safe string truncation via `go-runewidth`.

For werk tension rows: `▸ [C] → Fix auth middleware  Mar 14  █████████░ 92%` — selection indicator, phase badge, movement arrow, title, horizon, urgency bar. On wider terminals, add magnitude mini-bar, note count, and dynamics sparkline.

### Swimlane Grouping

beads_viewer's board view supports 3 swimlane modes (status / priority / type), cycleable with a single keystroke. Cards within swimlanes are expandable inline, showing full description and dependency chains.

For werk: Group tensions by phase (germination / assimilation / completion / momentum), by urgency tier, or by parent. Cycle with a keystroke.

### Markdown Rendering

beads_viewer uses Glamour with a custom theme matching the app's color scheme. Covers headings, code blocks with syntax highlighting, tables, links, task lists. Background set to `nil` (not explicit dark) to avoid color-slot remapping in terminals like Solarized Dark.

FrankenTUI has markdown rendering in `ftui-extras`.

For werk: Render tension notes as markdown in the detail view. Richer note content without leaving the terminal.

### Heatmap with Drill-Down

beads_viewer's insights view has a 5x5 heatmap grid (critical-path depth vs priority) with 8-color gradient. Navigate to a cell and drill down to see the individual items in that cell.

For werk: A dynamics heatmap — urgency on one axis, magnitude on the other. Each cell shows how many tensions fall in that quadrant. Drill down to see them. Instantly shows "what's urgent AND high-magnitude" vs "what's drifting and low-energy."

---

## Interactivity

### Staged Workflows (process_triage)

Scan → review → confirm → execute. Users see evidence before any destructive action.

**Shadow mode:** Risk-free observation without actions, for calibration. The system records what it would do without doing it.

For werk: Before resolving a tension with children, show what will happen to the children. Before deleting, show the full subtree that will be affected. Shadow mode for exploring "what if I resolved this?" without committing.

### Spring Physics Animations (charmed_rust)

The `harmonica` crate provides spring-physics for smooth motion — cursor transitions, panel resizes, scroll momentum. Not instantaneous snaps but organic feeling movement.

For werk: Smooth cursor movement between tensions. Panel resize with momentum. Toast notifications that slide in and fade out rather than appearing/disappearing instantly.

### Interactive Tutorial Overlay

beads_viewer has a built-in tutorial with 10,000+ lines of content, organized across 6 major sections. Context-sensitive — tutorial pages filter to match the current view. Progress tracking marks viewed pages.

For werk: A guided first-run experience. "Press `a` to create your first tension." Then "Now press `r` to update its reality." Walk through the core loop: create → observe dynamics → update → observe change. Track completion so it doesn't repeat.

### Pane Drag-to-Resize with Physics

FrankenTUI's pane system supports drag-to-resize with inertial throw (momentum after release), hysteresis (dead zone to prevent jitter), and pressure snap (snaps to constraints when close). Serializable snapshots for saving/restoring custom layouts.

### Macro Recording and Replay

FrankenTUI has `MacroRecorder`, `MacroPlayer`, and `EventRecorder` for recording and replaying input sequences. Record a common workflow once, replay it with a keystroke.

For werk: Record "morning review" — cycle through all urgent tensions, check each one. Replay it tomorrow.

### Inspector Widget

FrankenTUI has a built-in `Inspector` overlay for debugging widget rendering — shows widget boundaries, hit regions, focus graph. Toggle with a dev shortcut.

---

## Performance

### Windowed/Virtualized Rendering

beads_viewer's tree view only renders nodes within the viewport (`visibleRange()`), achieving O(1) render time regardless of tree size. Board view limits visible cards. These patterns prevent performance degradation with large datasets.

FrankenTUI has a `Virtualized` widget for this built in.

For werk: If the tension forest grows to hundreds of items, only render what's visible. The tree view especially benefits — deep hierarchies with collapsed subtrees should cost nothing to render.

### Diff-Based Rendering with Strategy Selection

FrankenTUI computes `BufferDiff::compute(&prev, &next)` and uses a Bayesian strategy selector to pick the cheapest rendering approach: full redraw vs dirty-row vs span-based. Decisions are logged as JSONL for deterministic auditing.

### Debounced File Watching with Deduplication

beads_viewer watches data files with fsnotify (200ms debounce) and skips reprocessing unchanged data via SHA-256 content hashing. A watchdog monitors for hangs and auto-recovers (max 3 attempts).

For werk: Watch the `.werk/` directory. When another process (CLI, agent) modifies tensions, the TUI picks up changes automatically. Debounce rapid edits, skip reprocessing if content hash matches.

### Idle-Time Garbage Collection

beads_viewer triggers GC when no processing is happening. Not during rendering, not during computation — only during idle.

---

## Agent Integration

### Robot Mode Alongside Interactive Mode

Multiple Dicklesworthstone projects provide `--robot-*` flags that output structured JSON on stdout while keeping diagnostics on stderr. The same data model powers both the interactive TUI and the machine API.

For werk: `werk` launches the TUI. `werk --robot show 01KK` outputs JSON. Same binary, same data, two interfaces. An AI agent can query werk state without opening the TUI.

### FrankenTerm's Delta Extraction

4KB overlap matching to extract only new terminal content instead of re-reading full scrollback. For efficiently piping agent output into the TUI's LogViewer without re-rendering everything.

---

## Widget Catalog Worth Using

From FrankenTUI's built-in widgets:

| Widget | Use in werk |
|--------|-------------|
| `List` + `ListItem` | Tension list with highlight, hover, filtering, mouse hit regions |
| `Tree` | Forest view with expand/collapse |
| `Block` | Borders, titles, padding (rounded/double/plain) |
| `Paragraph` | Text rendering for desire/actual/notes |
| `Progress` | Urgency bars, magnitude bars |
| `Sparkline` | Inline dynamics trends |
| `Badge` | Phase badges, status badges |
| `Toast` / `NotificationQueue` | Dynamics transition notifications |
| `Modal` (dialog/stack/animation) | Confirmations with focus integration |
| `CommandPalette` | Fuzzy action search with Bayesian scoring |
| `Input` / `Textarea` | Inline editing of desire, actual, notes |
| `Tabs` | View switching indicator |
| `Scrollbar` | Scroll position indicators |
| `StatusLine` | Bottom bar with counts and context |
| `LogViewer` | Agent output streaming |
| `Spinner` | Loading states during computation |
| `Rule` | Section dividers |
| `Columns` | Multi-column layout |
| `Virtualized` | Large list performance |
| `Focus` (manager/graph/spatial) | Full focus management system |
| `ErrorBoundary` | Error containment |
| `HistoryPanel` | Undo history visualization |

From `ftui-extras` (feature-gated):

| Extra | Use in werk |
|-------|-------------|
| Markdown rendering | Note display |
| Charts | Dynamics visualization |
| Help system (spotlight, tooltip, tour) | Guided first-run |
| Forms with validation | Multi-step tension creation |
| Clipboard | Copy tension context |
| Export | Export tension data |

---

## Ideas That Don't Fit Neatly

### Animated Gradient Banner (NTM)
Shimmering title with pulsing selection highlights and color-coded agent cards. Not just functional — delightful. werk's header could subtly pulse or shift based on the overall "temperature" of the tension forest.

### Quick-Select by Type (source2prompt_tui)
Number keys 1-9 instantly filter to different categories. For werk: `1` = germination only, `2` = assimilation only, `3` = completion only, `4` = momentum only. Instant phase filtering.

### Real-Time Token Estimation (source2prompt_tui)
Visual context-window usage bars showing how much of the LLM context is consumed. For werk's agent view: show how much context the tension + its history + its neighborhood consumes. Help users understand what the agent "sees."

### Confidence-Based Visual Styling (beads_viewer)
80%+ confidence = green, 50%+ = yellow, lower = gray. For werk dynamics: when a dynamic is strongly expressed (high magnitude), render it in full color. When marginal, render it muted. The visual weight matches the signal strength.

### Nerd Font Icons with ASCII Fallbacks (NTM)
Detect whether Nerd Font glyphs are available. Use them if present, fall back to ASCII otherwise. For werk: `` for germination, `` for completion, `` for momentum — but `[G]`, `[C]`, `[M]` if Nerd Fonts aren't installed.

### Three-Tier Command Taxonomy (NTM)
Commands organized as: Lifecycle (create/delete), Orchestration (update/move/resolve), Navigation (view/filter/search). Clean conceptual grouping that makes the command palette intuitive.

### Scroll Indicators
"more above" / "more below" hints when content overflows the viewport. Subtle but prevents the "is there more?" uncertainty.

### Tree State Persistence
beads_viewer persists expand/collapse state to `.beads/tree-state.json` across sessions. For werk: remember which subtrees the user had expanded in the tree view. Don't reset on restart.

### Time-Travel Comparison
beads_viewer's `t`/`T` keys let you compare the current state against previous git revisions. See what changed over time. For werk: compare current dynamics snapshot against yesterday's. See which tensions moved, which stagnated, which are new.

---
---

# TUI v0.5 — Deep Assessment & Redesign Proposal

**Date:** 2026-03-13
**Source:** Full code review of werk-tui v0.4.0 (app.rs, input.rs, msg.rs, types.rs, update.rs, all views/*, all overlays/*)

---

## Current State Diagnosis

The v0.4 TUI has **8 views** (Welcome, Dashboard, Detail, TreeView, Neighborhood, Timeline, Focus, DynamicsSummary) plus an Agent view, **5 input modes** (Normal, TextInput, Confirm, MovePicker, Reflect), and **21 command palette actions**. The `WerkApp` struct has **40+ fields** acting as a god object. The `update.rs` file exceeds **2200 lines**.

Key structural problems:
- **Navigation maze:** 4 of 8 views are read-only dead ends (Neighborhood, Timeline, Focus, DynamicsSummary) — you look, then press Esc. They're informational overlays masquerading as full views.
- **Redundant state:** `show_resolved: bool` coexists with `Filter` enum. `ToggleResolved` message is unreachable code.
- **Inconsistent j/k:** Means "move cursor" in Dashboard/Tree/Neighborhood, "scroll paragraph" in Detail, "cycle tensions" in Focus, "move mutation cursor" in Agent.
- **Verbose toggle:** Hides useful dynamics behind `v` — most users never see them.
- **3-step creation flow:** Adding a tension requires 3 sequential prompts (desired, horizon, actual) — tedious for quick capture.
- **Confirm dialogs for reversible actions:** Resolve/Release force a y/n dialog for an easily-undoable status change.

---

## 30 Ideas

1. Collapse 8 views into 3 core views (Dashboard, Detail, Tree) with inline panels instead of separate screens
2. Replace the urgency ticker with a persistent sidebar showing top-priority tensions
3. Unify the hint bars — every view repeats a slightly different hint bar; make one adaptive bar
4. Kill the Neighborhood view — it's a worse version of the Tree view with a selected node
5. Kill the Timeline view — it's a static read-only Paragraph with no interaction
6. Kill the DynamicsSummary view — its content belongs in a header/sidebar, not a full screen
7. Kill the Focus view — it's a stripped-down Detail view with less information
8. Merge the Lever overlay into the detail view as an inline "next action" section
9. Replace the 3-step tension creation flow (desired, horizon, actual) with a single-line quick-add
10. Add inline editing — press `r` and edit in-place on the dashboard row, not via overlay
11. Make the Tree view the primary view instead of the flat dashboard
12. Add Tab to cycle between Dashboard and Tree instead of `1`/`2`/`t` (three keys for two views)
13. Remove the `!`/`@`/`#` ticker jump shortcuts — obscure and undiscoverable
14. Standardize j/k behavior — currently means "move cursor" in some views and "scroll content" in others
15. Remove the `ToggleResolved` message — it overlaps with `CycleFilter` and creates confusion
16. Replace the verbose toggle (`v`) with always showing dynamics in detail and never in dashboard
17. Consolidate the WerkApp struct — it has 40+ fields acting as a god object
18. Extract view-specific state into sub-structs (DetailState, AgentState, SearchState, etc.)
19. Remove the `show_resolved` field — it's redundant with the Filter enum
20. Replace the Reflect mode with a note that accepts multi-line input via the existing TextInput
21. Make search persistent — currently clears on Enter/Esc, losing context
22. Add sort cycling (by urgency, by phase, by name, by horizon) to the dashboard
23. Show children count inline on dashboard rows
24. Remove the separate agent view — show agent responses in the detail view's history section
25. Replace the confirm dialog for resolve/release with a single-key undo instead (resolve immediately, `u` to undo within 5s)
26. Make horizon input smarter — accept "2w", "3m", "+5d" relative formats
27. Add visual grouping in dashboard by tier (section headers: "Urgent", "Active", "Neglected")
28. Remove the command palette or make it the primary interaction method (not both palette AND keybindings)
29. Show the "actual" state on dashboard rows (currently only "desired" is visible)
30. Add a quick-toggle to show/hide resolved tensions without cycling through all 4 filter states

---

## Critical Evaluation

### REJECTED

**3. Unify hint bars** — The hints are already per-view, which is correct behavior. The variation is the feature, not the bug. Different views legitimately have different actions.

**10. Inline editing on dashboard** — The dashboard is a list view. Inline editing would require complex cursor management within table cells and would conflict with j/k navigation. The overlay approach is appropriate for a TUI.

**11. Tree as primary view** — Trees are harder to scan than flat lists. The dashboard-first approach is correct for quick triage. Tree is a secondary exploration tool.

**13. Remove ticker jump shortcuts** — They're undiscoverable but genuinely useful for power users. The real fix is documenting them better in the help overlay (which already exists). Keep them.

**17. Consolidate WerkApp struct** — Pure refactor with no user-facing benefit as a standalone idea. Folded into #18.

**20. Replace Reflect with multi-line TextInput** — The Reflect mode uses TextArea which has real multi-line editing. A TextInput is single-line. The feature itself has value for journaling.

**22. Add sort cycling** — The current sort (by tier then urgency) is the correct default. No compelling use case for alphabetical or phase-based sorting in a tension management tool.

**23. Show children count on dashboard** — Adds visual noise. The tree view already shows hierarchy. The dashboard should stay clean.

**28. Remove command palette or keybindings** — Both are needed. Keybindings for speed, palette for discoverability. They complement each other.

**29. Show actual state on dashboard** — The dashboard is already dense. The detail view is the right place for the actual state. Adding it would require either a second row per item or severe truncation.

### KEPT (renumbered)

1. **Collapse secondary views into panels** (from #1, #4, #5, #6, #7, #8)
2. **Replace urgency ticker with a smarter status line** (from #2)
3. **Tab to cycle primary views** (from #12)
4. **Single-line quick-add for tensions** (from #9)
5. **Standardize j/k behavior** (from #14)
6. **Remove ToggleResolved / show_resolved redundancy** (from #15, #19)
7. **Always show dynamics in detail, never verbose toggle** (from #16)
8. **Extract view state into sub-structs** (from #18)
9. **Persistent search with Esc to clear** (from #21)
10. **Inline agent responses in detail history** (from #24)
11. **Undo instead of confirm for resolve/release** (from #25)
12. **Relative horizon input** (from #26)
13. **Visual tier grouping in dashboard** (from #27)
14. **Quick-toggle resolved visibility** (from #30)

---

## Detailed Proposals

### 1. Collapse Secondary Views into Panels

**What:** Eliminate `View::Neighborhood`, `View::Timeline`, `View::Focus`, `View::DynamicsSummary` as separate full-screen views. Instead:
- **Neighborhood** becomes a section in Detail view (it already has parent, breadcrumbs, and children — just show siblings too)
- **Timeline** becomes an optional bottom panel in Dashboard (toggle with `T`)
- **Focus** is removed entirely — Detail already shows everything Focus shows, plus more
- **DynamicsSummary** becomes a collapsible header section in Dashboard
- **Lever** becomes an inline section in Detail (below dynamics, above history)

**Concrete changes:**
```rust
// Remove from View enum:
// View::Neighborhood, View::Timeline, View::Focus, View::DynamicsSummary

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    Welcome,
    Dashboard,
    Detail,
    TreeView,
    Agent(String),
}

// Dashboard gains an optional bottom panel:
pub enum DashboardPanel {
    None,
    Timeline,
    Health,
}

// Detail view gains a siblings section between parent and children:
fn build_siblings_lines(&self) -> Vec<Line> {
    // Show siblings of current tension (same parent)
    // This replaces the entire Neighborhood view
}

// Detail view gains inline lever section:
fn build_lever_section(&self) -> Vec<Line> {
    if let Some(ref lever) = self.lever {
        if lever.tension_id == self.detail_tension
            .as_ref().map(|t| &t.id).unwrap_or(&String::new())
        {
            return vec![
                Line::from_spans([
                    Span::styled("Next     ", Style::new().fg(CLR_MID_GRAY)),
                    Span::styled(
                        format!("{} -- {}", lever.action.label(), lever.reasoning),
                        Style::new().fg(CLR_CYAN),
                    ),
                ]),
            ];
        }
    }
    vec![]
}
```

**Why it's good:** The current 8 views create a navigation maze. Users must remember `N`, `T`, `F`, `D` shortcuts, each of which dumps them into a screen with minimal interaction and no way to do anything except look and press Esc. The Neighborhood, Timeline, Focus, and DynamicsSummary views are all "read then go back" — they're informational overlays masquerading as full views. Collapsing them into the 3 real views (Dashboard, Detail, Tree) reduces cognitive load and eliminates 4 dead-end screens.

**Downsides:** The Focus view's "cycle through tensions with j/k" is a unique interaction that doesn't map cleanly to Detail. However, you could add j/k to cycle tensions in Detail view when at the top (scroll=0), preserving this. The Timeline as a panel will have less vertical space, but it's already a simple bar chart that works fine in 5-10 rows.

**Confidence: 85%**

---

### 2. Replace Urgency Ticker with a Smarter Status Line

**What:** The urgency ticker currently takes a full row at the top showing the top 3 urgent tensions. Replace it with a more informative combined status line that merges the ticker's urgency info with the title bar's counts.

```rust
// Current: two rows
// Row 1: [1] 95% Write novel  |  [2] 80% Ship feature  |  3 active
// Row 2: werk  |  5 active  2 urgent  1 neglected  3 resolved  0 released

// Proposed: one row
// werk  5 active  2^ urgent  1! neglected     > Write novel 95%  > Ship feature 80%

pub(crate) fn render_status_bar(&self, area: &Rect, frame: &mut Frame<'_>) {
    let mut items = StatusLine::new().separator("  ");

    // Left: summary counts
    items = items.left(StatusItem::text(" werk"));
    items = items.left(StatusItem::text(&format!("{} active", self.total_active)));
    if self.total_urgent > 0 {
        items = items.left(StatusItem::text(&format!("{}^", self.total_urgent)));
        // .style(Style::new().fg(CLR_RED_SOFT))
    }
    if self.total_neglected > 0 {
        items = items.left(StatusItem::text(&format!("{}!", self.total_neglected)));
        // .style(Style::new().fg(CLR_YELLOW_SOFT))
    }

    // Right: top 2 urgent tensions (brief)
    let urgent = self.top_urgent(2);
    for t in urgent {
        let pct = (t.urgency.unwrap_or(0.0) * 100.0) as u32;
        items = items.right(StatusItem::text(
            &format!("{}% {}", pct, truncate(&t.desired, 15))
        ));
    }

    items.style(Style::new().fg(CLR_LIGHT_GRAY)).render(*area, frame);
}
```

**Why it's good:** Saves one row of vertical space (precious in a TUI). The current ticker is visually noisy and duplicates the "active count" that's already in the title bar. Merging them creates a cleaner, denser information display.

**Downsides:** Loses the `!`/`@`/`#` jump shortcuts' visual target. But those shortcuts still work via urgency sorting — you don't need to see the ticker to jump.

**Confidence: 75%**

---

### 3. Tab to Cycle Primary Views

**What:** Replace `1`/`2`/`t` with `Tab` to cycle forward through Dashboard -> Tree -> (back to Dashboard). `Shift+Tab` to cycle backward. Remove `1`, `2`, `t` as view-switching keys.

```rust
// In normal_key_to_msg:
KeyCode::Tab => {
    match self.active_view {
        View::Dashboard => Msg::SwitchTree,
        View::TreeView => Msg::SwitchDashboard,
        _ => Msg::SwitchDashboard,
    }
}
KeyCode::BackTab => {  // Shift+Tab
    match self.active_view {
        View::Dashboard => Msg::SwitchTree,
        View::TreeView => Msg::SwitchDashboard,
        _ => Msg::SwitchDashboard,
    }
}
```

**Why it's good:** `Tab` is universally understood as "switch pane/tab". The current scheme uses three different keys (`1`, `2`, `t`) for two views, which is confusing. `t` and `2` do the same thing. `1` is only useful if you remember the numbering scheme. Tab is muscle memory.

**Downsides:** `Tab` is sometimes used for autocompletion in input modes. Need to ensure it's only active in Normal mode (which is already the case since input modes handle their own keys).

**Confidence: 90%**

---

### 4. Single-Line Quick-Add for Tensions

**What:** Replace the 3-step creation flow (desired -> horizon -> actual) with a single prompt that accepts a concise format. The current flow forces the user through 3 sequential prompts, which is tedious for quick capture.

```rust
// Current: 3 prompts
// "New tension - desired state:" -> "Horizon (e.g. 2026-06):" -> "Actual (current reality):"

// Proposed: 1 prompt with optional syntax
// "New tension: <desired> [horizon] [| actual]"
// Examples:
//   "Write the novel"                    -> desired only
//   "Write the novel 2026-06"            -> desired + horizon
//   "Write the novel 2026-06 | outline"  -> desired + horizon + actual
//   "Write the novel | have an outline"  -> desired + actual (no horizon)

fn parse_quick_add(input: &str) -> (String, Option<String>, Option<String>) {
    let parts: Vec<&str> = input.splitn(2, '|').collect();
    let before_pipe = parts[0].trim();
    let actual = parts.get(1).map(|s| s.trim().to_string());

    // Try to extract a horizon from the end of the desired portion
    let words: Vec<&str> = before_pipe.rsplitn(2, ' ').collect();
    if words.len() == 2 {
        if let Ok(_) = parse_horizon(words[0]) {
            let desired = words[1].to_string();
            let horizon = Some(words[0].to_string());
            return (desired, horizon, actual);
        }
    }

    (before_pipe.to_string(), None, actual)
}
```

**Why it's good:** The dominant use case for "add tension" is quick capture — you want to get the thought down fast. The current 3-prompt flow interrupts your flow. A single line lets you capture at the speed of thought. Power users who want horizons and actuals can use the pipe syntax; casual use just types the desired state.

**Downsides:** The syntax is less discoverable. New users won't know about `|` or trailing horizon dates. Mitigation: show the format in the prompt itself: `"Add: desired [horizon] [| actual]"`. Also, the horizon date detection could misparse (e.g., a desired state ending in a number like "Write chapter 12"). Mitigation: only parse known date formats (YYYY-MM, YYYY-MM-DD, YYYY).

**Confidence: 80%**

---

### 5. Standardize j/k Behavior

**What:** Currently `j`/`k` means different things in different views:
- Dashboard: move cursor in list
- Detail: scroll content vertically
- Tree: move cursor in list
- Focus: cycle to next/previous tension
- Neighborhood: move cursor in list
- Agent: move cursor in mutation list

Standardize to: **j/k always moves a cursor/selection**. Scrolling happens automatically to keep the cursor visible (like every modern list widget does). In Detail view, sections become navigable items rather than raw scroll targets.

```rust
// Detail view becomes section-navigable:
pub enum DetailSection {
    Info,
    Dynamics,
    Lever,
    History(usize),   // index into mutations
    Children(usize),  // index into children
}

// j/k in Detail moves between sections/items:
View::Detail => {
    // Move cursor between: Info, Dynamics, each mutation, each child
    self.detail_cursor = match msg {
        Msg::MoveDown => self.detail_cursor
            .saturating_add(1).min(self.detail_item_count() - 1),
        Msg::MoveUp => self.detail_cursor.saturating_sub(1),
        _ => self.detail_cursor,
    };
    // Auto-scroll to keep cursor in view
    self.ensure_detail_cursor_visible();
}
```

**Why it's good:** Consistent mental model across all views. Users learn one behavior and apply it everywhere. The current scroll-based j/k in Detail view is particularly bad because there's no visual indicator of "where you are" — you're just moving a virtual viewport over text. Section-based navigation gives you a cursor that you can see and act on (e.g., pressing `Enter` on a child item opens it).

**Downsides:** Implementing section-based navigation in Detail is more complex than raw scroll. The current paragraph-based rendering would need to become item-based.

**Confidence: 85%**

---

### 6. Remove ToggleResolved / show_resolved Redundancy

**What:** The codebase has both `show_resolved: bool` and `filter: Filter`. The `ToggleResolved` message and `show_resolved` field are leftovers from an earlier design before `Filter` was added. Remove them.

```rust
// Remove from WerkApp:
// pub(crate) show_resolved: bool,

// Remove from Msg:
// ToggleResolved,

// In visible_tensions(), simplify:
pub(crate) fn visible_tensions(&self) -> Vec<&TensionRow> {
    self.tensions.iter()
        .filter(|t| match self.filter {
            Filter::Active => t.status != "Resolved" && t.status != "Released",
            Filter::All => true,
            Filter::Resolved => t.status == "Resolved",
            Filter::Released => t.status == "Released",
        })
        .filter(|t| { /* search filter */ })
        .collect()
}

// In normal_key_to_msg, remove:
// KeyCode::Char('R') => Msg::ToggleResolved,
// (note: Shift+R already maps to StartResolve when shift==true)
```

**Why it's good:** Dead code removal. The `show_resolved` bool interacts with `Filter::Active` in a confusing way: when `show_resolved` is true AND filter is Active, resolved items show. But `Filter::All` also shows resolved items. There's no reason for both mechanisms. The current key mapping also has a collision: `R` maps to `ToggleResolved` when shift is false, and `StartResolve` when shift is true. Since `R` is always uppercase (always has shift), `ToggleResolved` is **unreachable code**.

**Downsides:** None. This is pure cleanup.

**Confidence: 95%**

---

### 7. Remove Verbose Toggle — Always Show Full Dynamics in Detail

**What:** Remove the `verbose: bool` flag and the `v` keybinding for toggling verbose dynamics. In Detail view, always show all dynamics (phase, movement, magnitude, urgency, conflict, neglect, oscillation, resolution, orientation, compensating strategy, assimilation depth, horizon drift). In Dashboard view, never show them (the dashboard already shows phase, movement, urgency).

```rust
// Remove from WerkApp:
// pub(crate) verbose: bool,

// Remove from Msg:
// ToggleVerbose,

// In detail.rs, build_dynamics_lines:
fn build_dynamics_lines(&self) -> Vec<Line> {
    // Always show all dynamics -- no suppress_verbose check
    // Remove the "Verbose Dynamics" header -- it's all just "Dynamics"
}
```

**Why it's good:** The verbose toggle is a half-measure. If the dynamics information is useful, show it. If it's not useful, remove it from the codebase. Having it behind a toggle means most users never see it (defaults off), and those who do toggle it see a jarring UI shift. The Detail view is the right place for full information density — that's why the user navigated there.

**Downsides:** For tensions with all dynamics populated, the Dynamics section becomes longer (up to 12 lines instead of 5). On very short terminals this could push History and Children off-screen. Mitigation: the section-based navigation from idea #5 handles this gracefully with scrolling.

**Confidence: 80%**

---

### 8. Extract View State into Sub-Structs

**What:** The `WerkApp` struct has 40+ fields, many of which are view-specific state that's irrelevant to other views. Extract into focused sub-structs.

```rust
pub struct WerkApp {
    pub(crate) engine: DynamicsEngine,
    pub(crate) tensions: Vec<TensionRow>,
    pub(crate) filter: Filter,
    pub(crate) active_view: View,
    pub(crate) input_mode: InputMode,
    pub(crate) toasts: Vec<Toast>,
    pub(crate) previous_urgencies: HashMap<String, f64>,
    pub(crate) lever: Option<LeverResult>,

    // View-specific state
    pub(crate) dashboard: DashboardState,
    pub(crate) detail: DetailState,
    pub(crate) tree: TreeState,
    pub(crate) agent: AgentState,
    pub(crate) search: SearchState,
    pub(crate) palette: CommandPalette,
    pub(crate) reflect: ReflectState,
}

pub struct DashboardState {
    pub table_state: RefCell<TableState>,
}

pub struct DetailState {
    pub tension: Option<Tension>,
    pub scroll: u16,
    pub mutations: Vec<MutationDisplay>,
    pub children: Vec<TensionRow>,
    pub dynamics: Option<DetailDynamics>,
    pub parent: Option<Tension>,
    pub ancestors: Vec<(String, String)>,
    pub nav_stack: Vec<String>,
}

pub struct AgentState {
    pub output: Vec<String>,
    pub scroll: u16,
    pub mutations: Vec<AgentMutation>,
    pub mutation_selected: Vec<bool>,
    pub mutation_cursor: usize,
    pub running: bool,
    pub response_text: Option<String>,
}

pub struct SearchState {
    pub query: Option<String>,
    pub buffer: String,
    pub cursor: usize,
    pub active: bool,
    pub input_widget: TextInput,
}

pub struct ReflectState {
    pub textarea: Option<TextArea>,
    pub tension_id: Option<String>,
}
```

**Why it's good:** The current god-object makes it impossible to reason about what state is relevant when. Every method has access to every field, so there's no encapsulation. Extracting sub-structs makes it clear that `agent_mutation_cursor` is only relevant in `View::Agent`, and `detail_scroll` is only relevant in `View::Detail`. This also reduces the cognitive load when modifying any single view — you only need to understand its state struct.

**Downsides:** Large mechanical refactor that touches almost every file. All field accesses change from `self.detail_tension` to `self.detail.tension`. No functional change, but a lot of diff churn.

**Confidence: 90%**

---

### 9. Persistent Search with Esc to Dismiss Overlay, Not Clear Results

**What:** Currently, search works as: `/` opens search -> type -> results filter live -> Enter opens first result AND clears search -> Esc clears search. The problem: you can never keep a search active while browsing. Proposed behavior:

- `/` opens the search input overlay
- Typing filters live (same as now)
- `Enter` dismisses the input overlay but **keeps the filter active** — you can now navigate the filtered list with j/k
- `Esc` in the filtered list clears the search and restores the full list
- The active search is shown in the status bar: `werk | 3/12 matching "novel"`

```rust
// SearchState gains a committed flag:
pub struct SearchState {
    pub query: Option<String>,    // committed search (persists after Enter)
    pub active: bool,             // input overlay is visible
    pub input_widget: TextInput,
}

// In update, Enter in search mode:
KeyCode::Enter => {
    self.search.active = false;   // close overlay
    // BUT keep self.search.query set -- the filter persists
    // User browses filtered results with j/k
}

// Esc when search.query is Some but search.active is false:
KeyCode::Escape => {
    if self.search.query.is_some() {
        // Clear the search filter
        self.search.query = None;
    } else {
        // Normal back behavior
        // ...
    }
}
```

**Why it's good:** The current search is "search and jump" — you search to find one thing, then the search evaporates. This is fine for small lists but poor for larger workloads. Persistent search lets you work within a subset ("show me everything about the novel project") while maintaining full navigation.

**Downsides:** Adds a new Esc behavior. Users might press Esc expecting "go back to dashboard" and instead get "clear search". Mitigation: show a clear indicator that a search filter is active. Could use a different key (e.g., `/` again) to clear the search.

**Confidence: 75%**

---

### 10. Inline Agent Responses in Detail View History

**What:** Instead of switching to a separate `View::Agent` screen, show agent responses as a special history entry in the Detail view. The agent mutation toggles become an inline section.

```rust
// In detail view, after History section, add Agent section if present:
fn build_agent_section(&self) -> Vec<Line> {
    if self.agent.response_text.is_none() || self.agent.mutations.is_empty() {
        return vec![];
    }

    let mut lines = vec![];

    // Show response text (wrapped)
    if let Some(ref text) = self.agent.response_text {
        for line in textwrap::wrap(text, width - 4) {
            lines.push(Line::from_spans([
                Span::styled(&*line, Style::new().fg(CLR_LIGHT_GRAY)),
            ]));
        }
    }

    // Show suggested mutations as a checklist
    for (i, mutation) in self.agent.mutations.iter().enumerate() {
        let check = if self.agent.mutation_selected[i] { "[x]" } else { "[ ]" };
        lines.push(Line::from_spans([
            Span::styled(format!("  {} ", check), Style::new().fg(CLR_CYAN)),
            Span::styled(mutation.summary(), Style::new().fg(CLR_LIGHT_GRAY)),
        ]));
    }

    lines
}
```

**Why it's good:** The Agent view is a context switch. You're looking at a tension's details, you ask the agent for help, and suddenly you're in a completely different screen with different keybindings. Then you apply changes and go back. Keeping everything in the Detail view maintains context. The agent response becomes part of the tension's story.

**Downsides:** The Detail view becomes more complex. Long agent responses could push other sections off-screen. The mutation toggle interaction needs to coexist with Detail's existing keybindings. The Agent view's separate keybinding space (`a` for apply, `1-9` for toggle) would conflict with Detail's keybindings. Might need a "sub-mode" within Detail, which partially defeats the purpose.

**Confidence: 65%**

---

### 11. Undo Instead of Confirm for Resolve/Release

**What:** Remove the "Are you sure? (y/n)" confirmation dialogs for Resolve and Release. Instead, perform the action immediately and show a toast with an undo option: `"Resolved 'Write novel' -- press u to undo (5s)"`. After 5 seconds, the undo expires.

```rust
pub struct UndoAction {
    pub description: String,
    pub undo_fn: Box<dyn FnOnce(&mut WerkApp)>,
    pub expires_at: Instant,
}

// In WerkApp:
pub(crate) pending_undo: Option<UndoAction>,

// On resolve:
Msg::StartResolve => {
    if let Some(tension) = self.selected_tension() {
        let id = tension.id.clone();
        let desired = tension.desired.clone();

        // Perform immediately
        self.engine.store()
            .update_status(&id, TensionStatus::Resolved).ok();
        self.reload_data();

        // Set up undo
        self.pending_undo = Some(UndoAction {
            description: format!("Resolved '{}'", truncate(&desired, 30)),
            undo_fn: Box::new(move |app| {
                app.engine.store()
                    .update_status(&id, TensionStatus::Active).ok();
                app.reload_data();
            }),
            expires_at: Instant::now() + Duration::from_secs(5),
        });

        self.push_toast(/* ... */);
    }
}

// On 'u' keypress:
KeyCode::Char('u') => {
    if let Some(undo) = self.pending_undo.take() {
        if undo.expires_at > Instant::now() {
            (undo.undo_fn)(self);
            self.push_toast(Toast::new(
                format!("Undone: {}", undo.description),
                ToastSeverity::Info,
            ));
        }
    }
    Msg::Noop
}
```

**Why it's good:** Confirm dialogs are flow-breakers. You've already decided to resolve — the `R` keypress is the decision. The confirm dialog doesn't prevent mistakes (you just hit `y` reflexively); what prevents mistakes is the ability to undo. This pattern (action + timed undo) is used by Gmail, Slack, and virtually every modern interface.

**Downsides:** The undo window is time-limited. If you resolve, walk away, and come back after 5 seconds, you can't undo. Mitigation: the status change is still in the mutation history, so a manual re-activation is possible. Also, Delete should probably keep the confirm dialog since it's truly destructive.

**Confidence: 85%**

---

### 12. Relative Horizon Input

**What:** Extend horizon parsing to accept relative date formats in addition to the existing absolute formats.

```rust
// In horizon.rs, extend parse_horizon:
pub fn parse_horizon(input: &str) -> Result<NaiveDate, String> {
    let input = input.trim();

    // Relative formats: +Nd, +Nw, +Nm, +Ny
    if input.starts_with('+') {
        let (num_str, unit) = input[1..].split_at(input.len() - 2);
        let n: i64 = num_str.parse().map_err(|_| "invalid number")?;
        let today = Utc::now().date_naive();
        return match &input[input.len()-1..] {
            "d" => Ok(today + chrono::Duration::days(n)),
            "w" => Ok(today + chrono::Duration::weeks(n)),
            "m" => {
                // Add N months
                Ok(today + chrono::Months::new(n as u32))
            }
            "y" => {
                // Add N years
                Ok(today + chrono::Months::new(n as u32 * 12))
            }
            _ => Err("use +Nd, +Nw, +Nm, or +Ny".to_string()),
        };
    }

    // Named shortcuts
    match input {
        "eow" | "friday" => { /* end of this week */ }
        "eom" => { /* end of this month */ }
        "eoq" => { /* end of this quarter */ }
        "eoy" => { /* end of this year */ }
        _ => {}
    }

    // Existing absolute parsing...
}
```

**Why it's good:** When setting a horizon, you think "I need this done in 2 weeks" not "I need this done by 2026-03-27". The current system forces you to do mental date arithmetic. `+2w` is faster and more natural.

**Downsides:** Minimal. The parsing is additive — all existing formats continue to work. The only risk is namespace collision (e.g., someone types `+2w` meaning something else), but since this is specifically a horizon input context, that's not a concern.

**Confidence: 95%**

---

### 13. Visual Tier Grouping in Dashboard

**What:** Add section headers to the dashboard list that group tensions by tier: Urgent, Active, Neglected. This replaces the current flat sorted list with a visually segmented one.

```rust
// In render_tension_list, insert tier headers:
fn render_tension_list(&self, area: &Rect, frame: &mut Frame<'_>) {
    let visible = self.visible_tensions();
    let mut rows: Vec<Row> = Vec::new();
    let mut current_tier: Option<UrgencyTier> = None;

    for row in &visible {
        if current_tier != Some(row.tier) {
            current_tier = Some(row.tier);
            // Insert section header
            let header_text = match row.tier {
                UrgencyTier::Urgent => "^ URGENT",
                UrgencyTier::Active => "* ACTIVE",
                UrgencyTier::Neglected => "! NEGLECTED",
                UrgencyTier::Resolved => "~ RESOLVED",
            };
            let header_style = match row.tier {
                UrgencyTier::Urgent => Style::new().fg(CLR_RED_SOFT).bold(),
                UrgencyTier::Active => Style::new().fg(CLR_LIGHT_GRAY).bold(),
                UrgencyTier::Neglected => Style::new().fg(CLR_YELLOW_SOFT).bold(),
                UrgencyTier::Resolved => Style::new().fg(CLR_DIM_GRAY).bold(),
            };
            rows.push(Row::new(vec![
                String::new(),
                String::new(),
                header_text.to_string(),
            ]).style(header_style));
        }
        // ... existing row rendering
    }
}
```

**Why it's good:** The current dashboard uses color alone to distinguish tiers. Color is not always sufficient — some terminals have poor color support, and color-blind users lose the signal entirely. Section headers provide a structural signal that complements color. They also give the user an immediate sense of proportion ("I have 5 urgent items and 3 neglected items") without counting.

**Downsides:** Section headers take up vertical space. Each header is one row, so 3 tiers = 3 extra rows. On small terminals this matters. Also, the cursor navigation needs to skip header rows, which adds complexity. Headers should not be selectable.

**Confidence: 70%**

---

### 14. Quick-Toggle Resolved Visibility

**What:** Add a single key (`Space`) that toggles between "show only active" and "show all" without cycling through the full filter chain (Active -> All -> Resolved -> Released).

```rust
// In normal_key_to_msg:
KeyCode::Char(' ') => {
    // Quick toggle: Active <-> All
    self.filter = match self.filter {
        Filter::Active => Filter::All,
        _ => Filter::Active,
    };
    Msg::Noop  // or a new Msg::FilterChanged
}
// Keep 'f' for the full cycle through all 4 filter states
```

**Why it's good:** The most common filter operation is "let me see resolved items" -> look at them -> "hide them again". The current cycle requires pressing `f` up to 3 times to get back to Active after viewing All. A toggle key makes the common case a single keypress.

**Downsides:** Uses up the `Space` key. Space is a valuable key in TUIs — often used for selection/toggle. If you later want multi-select, Space is the obvious key for it. Alternative: use `x` key. Or make `f` smarter: if current filter is not Active, `f` goes directly back to Active instead of cycling forward.

**Confidence: 70%**

---

## Prioritized Implementation Order

| Priority | Idea | Confidence | Effort |
|----------|------|-----------|--------|
| 1 | #6: Remove ToggleResolved/show_resolved | 95% | Tiny |
| 2 | #12: Relative horizon input | 95% | Small |
| 3 | #3: Tab to cycle views | 90% | Small |
| 4 | #8: Extract view state sub-structs | 90% | Medium (mechanical) |
| 5 | #5: Standardize j/k behavior | 85% | Medium |
| 6 | #11: Undo instead of confirm | 85% | Medium |
| 7 | #1: Collapse secondary views | 85% | Large |
| 8 | #4: Single-line quick-add | 80% | Medium |
| 9 | #7: Remove verbose toggle | 80% | Small |
| 10 | #2: Smarter status line | 75% | Small |
| 11 | #9: Persistent search | 75% | Medium |
| 12 | #13: Tier grouping | 70% | Medium |
| 13 | #14: Quick-toggle resolved | 70% | Tiny |
| 14 | #10: Inline agent responses | 65% | Large |

The first 3 items are quick wins with near-certain improvement. Items 4-7 are the architectural backbone of a v0.5 rewrite. Items 8-14 are polish that can be done incrementally.

---
---

# 10 Power Features — New Functionality for v0.5

**Date:** 2026-03-13
**Method:** Generated ~100 candidates across data model, workflow, visualization, AI integration, structural dynamics theory, analytics, and UX. Evaluated each against: (1) amplifies what makes werk unique vs. any task manager, (2) pragmatically implementable, (3) complexity burden justified, (4) meaningfully improves daily experience, (5) genuinely innovative. These 10 survived.

---

## 1. What-If Mode (Counterfactual Preview)

Before you resolve, release, or delete a tension, see exactly what will happen to the structural dynamics of the entire forest. A preview pane shows: which children become orphans or auto-cascade-resolve, how urgency redistributes across siblings, whether the lever recommendation shifts, which dynamics events would fire.

**Why this is brilliant:** No other tool does this. It exploits the fact that werk has a full computational model of structural dynamics — you can literally *run the simulation forward* before committing. This is the difference between a task list and a structural dynamics instrument. The engine already has `compute_full_dynamics_for_tension`, Forest traversal, and cascade counting in the lever. You clone the engine state (or use an in-memory fork), apply the hypothetical change, re-run dynamics, diff the results.

```rust
pub struct WhatIfResult {
    pub orphaned_children: Vec<(String, String)>,  // (id, desired)
    pub auto_resolved: Vec<(String, String)>,       // children that would cascade-resolve
    pub lever_shift: Option<(LeverResult, LeverResult)>,  // (before, after)
    pub urgency_changes: Vec<(String, f64, f64)>,   // (id, old_urgency, new_urgency)
    pub events_that_would_fire: Vec<String>,
}

fn compute_what_if(&mut self, tension_id: &str, action: WhatIfAction) -> WhatIfResult {
    // Fork engine into in-memory clone
    let mut shadow = self.engine.clone_in_memory();
    // Apply hypothetical action
    match action {
        WhatIfAction::Resolve => {
            shadow.store().update_status(tension_id, TensionStatus::Resolved).ok();
        }
        WhatIfAction::Release => {
            shadow.store().update_status(tension_id, TensionStatus::Released).ok();
        }
        WhatIfAction::Delete => {
            shadow.store().delete_tension(tension_id).ok();
        }
    };
    // Recompute dynamics on shadow, diff against current
    // ...
}
```

Trigger: press `R` to resolve, but instead of "are you sure?", show a what-if preview for 2 seconds. Press `R` again to confirm, `Esc` to cancel.

**Effort:** Medium. The hard part is cloning the engine cheaply. If the store is SQLite, you'd need an in-memory copy or use transactions with rollback. sd-core already has `DynamicsEngine::new_in_memory()` and the store supports `begin_transaction()` / `rollback_transaction()`.

---

## 2. Guided Morning Review Ritual

A structured walkthrough mode (`Ctrl+R` or `:review`) that cycles through tensions in a specific order, asking one question per tension and recording the answer as a mutation. The sequence:

1. **Past-horizon tensions** (using `Forest::tensions_past_horizon()`): "This was due [N days ago]. Is it done, still active, or should you release it?"
2. **Urgent tensions** (tier == Urgent): "Current reality: [actual]. Has anything changed?" — auto-opens reality update prompt
3. **Neglected tensions** (tier == Neglected): "You haven't touched this in [N days]. Still committed? Update reality, snooze, or release?"
4. **Stagnant tensions** (movement == Stagnant, phase != Germination): "No movement detected. What's blocking progress?"
5. **Summary:** "Reviewed 12 tensions. 3 updated, 1 resolved, 2 snoozed. Lever: [next action]."

**Why this is brilliant:** The biggest problem with any personal management system is that people stop using it. A structured ritual solves the "I opened it, now what?" problem. It transforms werk from a passive display into an active practice — which is exactly what Robert Fritz's structural dynamics demands. The morning review is the *confrontation with current reality* that Fritz describes as essential.

```rust
pub struct ReviewState {
    pub queue: Vec<ReviewItem>,
    pub current_index: usize,
    pub completed: Vec<ReviewOutcome>,
}

pub struct ReviewItem {
    pub tension_id: String,
    pub category: ReviewCategory,  // PastHorizon, Urgent, Neglected, Stagnant
    pub prompt: String,            // The question to ask
    pub suggested_actions: Vec<ReviewAction>,
}

pub enum ReviewAction {
    UpdateReality,
    Resolve,
    Release,
    Snooze(NaiveDate),
    Skip,
}
```

The review mode is a linear walk through a pre-computed queue. `j`/`k` or `Enter` to act, `s` to skip, `Esc` to exit early. Results are recorded as mutations/notes.

**Effort:** Medium. The queue construction uses existing Forest queries. The walkthrough is a new `InputMode::Review(ReviewState)` with simple keybindings.

---

## 3. Split Pane: List + Detail Side-by-Side

On terminals wider than ~120 columns, automatically show the dashboard/tree on the left (40%) and the selected tension's detail on the right (60%). Selecting a different tension in the list instantly updates the detail pane. No more pressing Enter to navigate into detail and Esc to come back.

**Why this is brilliant:** This is the single highest-impact UX improvement. The current Enter-Detail-Esc-Dashboard loop is the most frequent interaction in the entire TUI, and it's a full context switch every time. Split pane eliminates it entirely. ftui already has `Grid` layout with named areas and responsive breakpoints.

```rust
// In view(), check terminal width:
let use_split = area.width >= 120;

if use_split {
    let grid = Grid::new()
        .columns([Constraint::Percentage(40.0), Constraint::Fill])
        .rows([Constraint::Fixed(1), Constraint::Fill, Constraint::Fixed(1)]);

    let rects = grid.split(area);
    // Left: dashboard list
    self.render_tension_list(&rects[/* left content */], frame);
    // Right: detail of selected tension (auto-loaded on cursor move)
    self.render_detail_body_responsive(&rects[/* right content */], frame);
} else {
    // Narrow: current single-pane behavior
}
```

The key change: on every `MoveUp`/`MoveDown` in Dashboard, also call `load_detail()` for the newly selected tension. This is cheap — the engine caches computed dynamics.

**Effort:** Small-Medium. The layout is straightforward with ftui Grid. The main work is ensuring detail loads are fast enough for every cursor move (they should be — it's just reading from SQLite + computing dynamics).

---

## 4. Resolution Forecasting with Velocity Sufficiency

sd-core already computes `Resolution.velocity`, `Resolution.required_velocity`, and `Resolution.is_sufficient` — but the TUI doesn't surface any of it. Show a prediction: "At current velocity, this tension will resolve in ~14 days" or "You're falling behind — current pace won't meet the March 28 horizon."

Display this in the Detail view's Dynamics section as a new "Forecast" line:

```
Forecast    On track — resolving ~Mar 27 (1d before horizon)
```
or
```
Forecast    Behind — current pace reaches Apr 15 (18d past horizon)
```
or
```
Forecast    Insufficient data — update reality to calibrate
```

**Why this is brilliant:** This turns dynamics data into *actionable foresight*. Currently the TUI shows urgency (how close is the deadline) and magnitude (how big is the gap), but never answers the question every user actually has: "will I make it?" The engine already computes the answer — it's `resolution.is_sufficient`. You're just wiring it to the display.

```rust
fn build_forecast_line(&self, cd: &ComputedDynamics, tension: &Tension) -> Option<Line> {
    let resolution = cd.resolution.as_ref()?;
    let urgency = cd.urgency.as_ref();

    if let (Some(required_vel), Some(is_sufficient)) =
        (resolution.required_velocity, resolution.is_sufficient)
    {
        let ratio = resolution.velocity / required_vel;
        if is_sufficient {
            // Extrapolate completion date from magnitude / velocity
            let magnitude = cd.structural_tension.as_ref()
                .map(|st| st.magnitude).unwrap_or(0.0);
            if resolution.velocity > 0.0 {
                let secs_remaining = (magnitude / resolution.velocity) as i64;
                let forecast_date = Utc::now()
                    + chrono::Duration::seconds(secs_remaining);
                Some(Line::from_spans([
                    Span::styled("Forecast    ", Style::new().fg(CLR_MID_GRAY)),
                    Span::styled(
                        format!("On track -- resolving ~{}",
                            forecast_date.format("%b %d")),
                        Style::new().fg(CLR_GREEN),
                    ),
                ]))
            } else {
                None
            }
        } else {
            Some(Line::from_spans([
                Span::styled("Forecast    ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(
                    format!("Behind -- {:.0}% of required pace", ratio * 100.0),
                    Style::new().fg(CLR_RED_SOFT),
                ),
            ]))
        }
    } else {
        None // No horizon, can't forecast
    }
}
```

**Effort:** Small. The computation exists in sd-core. This is purely a display addition to `build_dynamics_lines()`.

---

## 5. Snooze with Auto-Resurface

Press `z` on any tension to snooze it. Prompted for a wake date (`+3d`, `monday`, `eom`, or an absolute date). Snoozed tensions vanish from the default dashboard view but reappear automatically when the snooze expires. A small indicator in the status bar shows "2 snoozed" so you don't forget they exist. Press `Z` to view all snoozed tensions.

**Why this is brilliant:** The biggest source of dashboard noise is tensions you're aware of but can't act on right now. Without snooze, you have three bad options: delete it (lose it), resolve it (lie), or leave it cluttering the list (cognitive overhead). Snooze is the honest answer: "I acknowledge this, but not now."

```rust
// Store snooze as a mutation with field "snoozed_until"
// This uses the existing mutation system -- no schema changes needed
fn snooze_tension(&mut self, tension_id: &str, until: NaiveDate) {
    let mutation = Mutation::new(
        tension_id.to_string(),
        Utc::now(),
        "snoozed_until".to_string(),
        None,
        until.format("%Y-%m-%d").to_string(),
    );
    self.engine.store().record_mutation(&mutation).ok();
}

// In visible_tensions(), filter snoozed:
.filter(|t| {
    if show_snoozed { return true; }
    let snooze = self.get_snooze_date(&t.id);
    snooze.map(|d| d <= today).unwrap_or(true) // show if expired or no snooze
})

// On Tick (every 60s), check for expired snoozes and emit toast:
// "'Write chapter 3' has resurfaced"
```

**Effort:** Small. Snooze is just a special mutation. Filtering is a check against today's date. No schema changes.

---

## 6. Composite Auto-Resolution with Cascade

When all children of a tension reach Resolved status, the parent automatically resolves too (with a toast: "All children resolved — 'Launch product' auto-resolved"). Children without a horizon inherit the parent's horizon, creating natural deadline propagation through the tree.

**Why this is brilliant:** Currently, parent-child relationships in werk are purely organizational — they don't carry structural meaning. This change makes hierarchy *load-bearing*. A parent tension becomes a composite goal that resolves when its components resolve. This aligns with Fritz's concept of structural tension: the parent's gap closes when all sub-gaps close. The horizon inheritance means creating a child automatically gives it temporal urgency proportional to the parent's urgency.

```rust
// In the status update handler, after resolving a tension:
fn check_parent_auto_resolution(&mut self, tension_id: &str) {
    let tension = self.engine.store()
        .get_tension(tension_id).ok().flatten();
    let parent_id = tension.and_then(|t| t.parent_id.clone());

    if let Some(pid) = parent_id {
        let children = self.engine.store()
            .get_children(&pid).unwrap_or_default();
        let all_resolved = children.iter().all(|c|
            c.status == TensionStatus::Resolved
            || c.status == TensionStatus::Released
        );
        if all_resolved && !children.is_empty() {
            self.engine.store()
                .update_status(&pid, TensionStatus::Resolved).ok();
            let parent = self.engine.store()
                .get_tension(&pid).ok().flatten();
            if let Some(p) = parent {
                self.push_toast(Toast::new(
                    format!("All children done -- '{}' auto-resolved",
                        truncate(&p.desired, 30)),
                    ToastSeverity::Info,
                ));
            }
            // Recurse: this parent's resolution might cascade further up
            self.check_parent_auto_resolution(&pid);
        }
    }
}

// On child creation, inherit parent's horizon if child has none:
fn create_child_with_inheritance(
    &mut self, parent_id: &str, desired: &str, actual: &str,
) {
    let parent = self.engine.store()
        .get_tension(parent_id).ok().flatten();
    let horizon = parent.and_then(|p| p.horizon.clone());
    self.engine.store().create_tension_full(
        desired, actual,
        Some(parent_id.to_string()),
        horizon,
    ).ok();
}
```

**Effort:** Small. Two additions to existing handlers — one in resolve, one in create-child. No new data model.

---

## 7. Agent Auto-Decomposition

When creating a new high-level tension, offer to auto-decompose it: "Break this down? (y/n)". If yes, send the tension to the agent with a decomposition prompt. The agent returns 3-7 sub-tensions, each shown as a checkbox list. Toggle the ones you want, press `a` to create them all as children.

**Why this is brilliant:** The hardest part of using any planning tool is the initial decomposition — turning "launch the product" into actionable sub-goals. Most people either leave tensions too vague (and they stagnate) or don't bother decomposing (and they feel overwhelmed). Auto-decomposition leverages the existing agent infrastructure with a single new prompt template.

```rust
// New prompt template for decomposition:
const DECOMPOSE_PROMPT: &str = r#"
You are helping decompose a structural tension into sub-tensions.

The parent tension is:
  Desired: {desired}
  Actual: {actual}
  Horizon: {horizon}

Break this into 3-7 concrete sub-tensions. Each sub-tension should be:
- A specific, actionable gap between a desired state and current reality
- Small enough to make progress on within days, not months
- Together, they should cover the full scope of the parent

Respond in YAML:
---
mutations:
  - action: create_child
    parent_id: "{parent_id}"
    desired: "..."
    actual: "..."
    reasoning: "why this sub-tension matters"
response: |
  Brief explanation of the decomposition strategy.
---
"#;

// Trigger: after creating a tension, if it has no children:
// "Decompose with agent? (y/n)" -> sends to agent -> shows checkbox list
```

This reuses the entire existing agent flow (spawn process, parse YAML response, show mutations as checkboxes, apply selected). The only new code is the prompt template and a trigger point.

**Effort:** Small. The agent infrastructure exists. This is a new prompt and a trigger point.

---

## 8. Behavioral Pattern Insights (Periodic Digest)

Once per week (or on-demand via `:insights`), compute and display behavioral patterns from the mutation history:

- **Attention distribution:** "You updated 'Ship feature' 12 times but 'Write documentation' only once this week"
- **Oscillation tendency:** "3 of your tensions have oscillated in the last 30 days — you tend to advance then retreat on writing-related work"
- **Resolution velocity:** "You resolve Completion-phase tensions 4x faster than Assimilation-phase ones"
- **Temporal patterns:** "You're most active on Mondays and Wednesdays. Thursday-Sunday you rarely update."
- **Horizon drift:** "You've postponed 'Launch blog' 3 times — repeated postponement pattern detected"
- **Neglect detection:** "Parent 'Novel project' is getting attention while its children are stagnant"

**Why this is brilliant:** This turns werk from a tool that shows you *what your tensions are* into one that shows you *who you are as a practitioner*. Fritz's structural dynamics is fundamentally about self-awareness — recognizing your own patterns of oscillation, compensation, and stagnation. This feature makes those patterns visible.

```rust
pub struct InsightDigest {
    pub period: (DateTime<Utc>, DateTime<Utc>),
    pub insights: Vec<Insight>,
}

pub struct Insight {
    pub category: InsightCategory,
    pub severity: InsightSeverity,  // Observation, Warning, Pattern
    pub title: String,
    pub detail: String,
    pub tension_ids: Vec<String>,
}

pub enum InsightCategory {
    AttentionDistribution,
    OscillationPattern,
    VelocityAnalysis,
    TemporalPattern,
    HorizonDrift,
    NeglectPattern,
}

fn compute_insights(
    engine: &mut DynamicsEngine, window_days: i64,
) -> InsightDigest {
    let now = Utc::now();
    let start = now - Duration::days(window_days);
    let mutations = engine.store()
        .mutations_between(start, now).unwrap_or_default();

    // Count mutations per tension -> attention distribution
    let mut per_tension: HashMap<String, usize> = HashMap::new();
    for m in &mutations {
        *per_tension.entry(m.tension_id().to_string())
            .or_insert(0) += 1;
    }

    // Detect imbalances, oscillation patterns, horizon drifts...
    // All the data is already in sd-core -- this is
    // aggregation over existing computations
}
```

Display as a modal overlay or a dedicated section in the Health dashboard. Each insight is one line with a severity indicator, clickable to drill into the relevant tensions.

**Effort:** Medium. The mutation data exists via `store.mutations_between()`. The dynamics are already computed. This is aggregation and formatting over existing data.

---

## 9. Filesystem Watch with Live Reload

Watch the `.werk/sd.db` file for external modifications. When another process (the CLI, an agent, a script) modifies the database, the TUI detects the change and reloads automatically — no restart needed, no stale data.

**Why this is brilliant:** werk has three interfaces: the TUI, the CLI, and agents. Currently, if you use the CLI to resolve a tension while the TUI is open, the TUI shows stale data until the next 60-second tick (and even then, only if `reload_data()` re-reads from the store). Filesystem watching makes the TUI a live dashboard that always reflects truth. This is essential for agent integration — an agent can modify tensions in the background and the user sees the changes appear in real time.

```rust
// Use notify crate (standard Rust file watching):
use notify::{Watcher, RecursiveMode, Event as NotifyEvent};

// In WerkApp initialization:
fn setup_file_watcher(db_path: PathBuf) -> Cmd<Msg> {
    Cmd::task_named("file_watcher", async move {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(
            move |res: Result<NotifyEvent, _>| {
                if let Ok(event) = res {
                    if event.kind.is_modify() {
                        tx.send(()).ok();
                    }
                }
            }
        ).unwrap();
        watcher.watch(&db_path, RecursiveMode::NonRecursive).unwrap();

        // Wait for change
        rx.recv().ok();
        Msg::ExternalChange
    })
}

// In update():
Msg::ExternalChange => {
    self.reload_data();
    self.push_toast(Toast::new(
        "External change detected -- reloaded".into(),
        ToastSeverity::Info,
    ));
    // Re-arm the watcher
    self.setup_file_watcher()
}
```

Add `notify` to Cargo.toml dependencies. Debounce with a 200ms window to coalesce rapid writes.

**Effort:** Small. The `notify` crate is mature and does the heavy lifting. The reload logic already exists (`reload_data()`).

---

## 10. Recurring Tensions

When resolving a tension, optionally mark it as recurring: "Recreate in +1w / +2w / +1m / custom?" If yes, resolving creates a new identical tension (same desired, same parent) with a fresh horizon offset from today. The resolved original stays in history. A small `~` indicator marks recurring tensions in the dashboard.

**Why this is brilliant:** Many real-world structural tensions are cyclical — weekly planning, monthly reviews, quarterly goals, regular maintenance. Currently, you either re-create them manually each time (tedious) or keep them permanently active (defeats the purpose of resolution). Recurring tensions let you experience the satisfaction of resolution while ensuring the next cycle is automatically prepared.

```rust
// Store recurrence as a config mutation on the tension:
// field: "recurrence", value: "+1w" | "+2w" | "+1m" | "none"

fn resolve_with_recurrence(&mut self, tension_id: &str) {
    let tension = self.engine.store()
        .get_tension(tension_id).ok().flatten();
    if let Some(t) = tension {
        // Resolve the current one
        self.engine.store()
            .update_status(tension_id, TensionStatus::Resolved).ok();

        // Check for recurrence
        let recurrence = self.get_recurrence(tension_id);
        if let Some(interval) = recurrence {
            let new_horizon = parse_horizon(&interval).ok();
            self.engine.store().create_tension_full(
                &t.desired,
                &t.actual,  // or reset to empty
                t.parent_id.clone(),
                new_horizon.map(|d| Horizon::from(d)),
            ).ok();
            self.push_toast(Toast::new(
                format!("Recurring: new '{}' created",
                    truncate(&t.desired, 25)),
                ToastSeverity::Info,
            ));
        }
    }
}

// Set recurrence: new command in palette and keybinding
// `:recur +2w` or press `Y` in detail view -> prompt for interval
```

**Effort:** Small. Uses existing `create_tension_full` and mutation recording. The recurrence interval is stored as a mutation/note. No schema changes.

---

## Summary Table

| # | Feature | Unique to werk? | Effort | Impact |
|---|---------|-----------------|--------|--------|
| 1 | What-If counterfactual preview | Yes — requires dynamics engine | Medium | Transformative |
| 2 | Guided morning review ritual | Yes — driven by dynamics tiers | Medium | Habit-forming |
| 3 | Split pane (list + detail) | No — but eliminates #1 friction | Small-Med | Massive UX win |
| 4 | Resolution forecasting | Yes — uses existing velocity data | Small | High insight |
| 5 | Snooze with auto-resurface | No — but essential missing primitive | Small | High QoL |
| 6 | Composite auto-resolution | Yes — structural cascade | Small | Meaningful hierarchy |
| 7 | Agent auto-decomposition | Yes — leverages agent + structure | Small | Powerful AI use |
| 8 | Behavioral pattern insights | Yes — structural dynamics mirror | Medium | Self-awareness |
| 9 | Filesystem watch + live reload | No — but enables multi-tool workflow | Small | Reliability |
| 10 | Recurring tensions | No — but solves real workflow gap | Small | High utility |

**Recommended implementation order** (by effort-to-impact ratio):
1. #4 Resolution forecasting (just wiring existing sd-core data to display)
2. #5 Snooze (mutation + filter)
3. #6 Composite auto-resolution (two small handler additions)
4. #9 Filesystem watch (add `notify` crate)
5. #10 Recurring tensions (mutation + create-on-resolve)
6. #3 Split pane (Grid layout)
7. #7 Agent auto-decomposition (new prompt template)
8. #2 Guided morning review (new InputMode + queue)
9. #8 Behavioral insights (aggregation over existing data)
10. #1 What-If mode (engine forking + diff rendering)
