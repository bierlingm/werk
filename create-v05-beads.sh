#!/bin/bash
# werk v0.5 — Comprehensive bead creation script
# Creates the full task structure with dependencies and detailed comments.
# Run from: /Users/moritzbierling/werk/desk/werk/
set -euo pipefail
cd "$(dirname "$0")"

BR="br --quiet"

# ============================================================================
# EPICS (Top-level containers for each phase)
# ============================================================================

echo "Creating epics..."

E0=$($BR create "Phase 0: Foundation" -t epic -p P0 \
  -d "Structural cleanup and prerequisites. Remove dead code, extract view state sub-structs, add relative horizon parsing, add filesystem watching. This phase touches no user-visible behavior — it prepares the codebase for the rebuild. Every subsequent phase depends on this being clean." \
  -l "foundation,v05" --silent)

E1=$($BR create "Phase 1: Core TUI Rebuild" -t epic -p P0 \
  -d "The heart of v0.5. Rebuild the TUI around three principles: density without clutter, cursor-not-viewport, and two-and-a-half views (Dashboard/Detail/Tree). Split pane at 120+ cols, tier-grouped dashboard, cursor-based Detail navigation, collapsed secondary views. This phase delivers the new interaction model." \
  -l "tui,v05" --silent)

E2=$($BR create "Phase 2: Projection Engine (sd-core)" -t epic -p P0 \
  -d "Implement the structural projection engine in sd-core. Fritz's central insight is that structure determines the path of least resistance. This engine extrapolates observed mutation patterns forward, classifies per-tension trajectories (Resolving/Stalling/Drifting/Oscillating), projects gap magnitude at future time points, detects urgency collisions, and enhances lever scoring with trajectory awareness. All pure functions, no instrument dependencies. See calm-wandering-crab.md for full design." \
  -l "sd-core,projection,v05" --silent)

E3=$($BR create "Phase 3: Power Features" -t epic -p P1 \
  -d "Features that make werk genuinely compelling beyond a task list: snooze with auto-resurface, composite auto-resolution (parent resolves when all children resolve), recurring tensions, undo, what-if counterfactual preview, guided morning review ritual, behavioral pattern insights. Each feature amplifies what makes werk unique — its structural dynamics model." \
  -l "tui,features,v05" --silent)

E4=$($BR create "Phase 4: TUI Projection Integration" -t epic -p P1 \
  -d "Wire the sd-core projection engine into the TUI. Trajectory indicators on every dashboard row, full projection section in Detail view with gap progression bars, cached FieldProjection on 5-minute cycle, trajectory overlay via command palette. This makes the path of least resistance visible in the daily instrument." \
  -l "tui,projection,v05" --silent)

E5=$($BR create "Phase 5: CLI Expansion" -t epic -p P1 \
  -d "Expand the CLI into a comprehensive scripting and agent API surface. Rich filtering (--urgent, --neglected, --phase), health/insights/diff analytics, trajectory command, batch mutation application, system-wide agent context. The CLI is not for daily human use — it's the pipe-friendly, JSON-everywhere interface that agents and scripts consume." \
  -l "cli,v05" --silent)

E6=$($BR create "Phase 6: Hooks and Agent Integration" -t epic -p P2 \
  -d "Three-tier agent integration: (1) CLI-as-API works with any agent that can run shell commands, (2) structured I/O protocol via werk run with system-wide and decomposition modes, (3) pre/post mutation hooks that trigger external processes with JSON event payloads. Includes CLAUDE.md template and Claude Code hook examples. werk doesn't try to be an agent — it's the state layer agents read from and write to." \
  -l "hooks,agents,v05" --silent)

E7=$($BR create "Phase 7: Polish" -t epic -p P3 \
  -d "Ongoing refinement: tree expand/collapse with persisted state, inline mode for always-on dashboard, MCP server if demand warrants, keyboard macro recording. These are nice-to-haves that improve the experience but aren't essential for v0.5 launch." \
  -l "polish,v05" --silent)

echo "Epics: E0=$E0 E1=$E1 E2=$E2 E3=$E3 E4=$E4 E5=$E5 E6=$E6 E7=$E7"

# ============================================================================
# PHASE 0: FOUNDATION
# ============================================================================

echo "Creating Phase 0 tasks..."

P0_1=$($BR create "Extract view state into sub-structs" -t task -p P0 \
  -d "The WerkApp struct has 40+ fields acting as a god object. Extract into focused sub-structs: DashboardState, DetailState, TreeState, AgentState, SearchState, ReviewState, ReflectState. Every field access changes from self.detail_tension to self.detail.tension. This is a large mechanical refactor — no functional change, but it makes every subsequent modification cleaner because view-specific state is encapsulated. Without this, adding projection state, review state, etc. would make the god object even worse." \
  -l "refactor,foundation" --parent "$E0" --silent)

P0_2=$($BR create "Remove dead code: show_resolved, ToggleResolved, verbose toggle" -t task -p P0 \
  -d "Three pieces of dead/redundant code to remove: (1) show_resolved: bool — redundant with Filter enum, causes confusing interaction where show_resolved=true AND filter=Active shows resolved items. (2) ToggleResolved message — unreachable code. KeyCode::Char('R') always has shift=true, so the non-shift branch (ToggleResolved) never fires. (3) verbose: bool and ToggleVerbose — the verbose toggle hides useful dynamics behind a toggle most users never find. In v0.5, Detail always shows full dynamics. Dashboard never shows them (it has phase/movement/urgency already)." \
  -l "cleanup,foundation" --parent "$E0" --silent)

P0_3=$($BR create "Add relative horizon parsing (+2w, eom, etc.)" -t task -p P0 \
  -d "Extend horizon.rs parse_horizon() to accept relative date formats: +Nd (days), +Nw (weeks), +Nm (months), +Ny (years). Also named shortcuts: eow/friday (end of week), eom (end of month), eoq (end of quarter), eoy (end of year). Users think 'I need this in 2 weeks' not 'I need this by 2026-03-27'. This is additive — all existing absolute formats (YYYY-MM-DD, YYYY-MM, YYYY) continue to work. Used by both TUI and CLI." \
  -l "horizon,foundation" --parent "$E0" --silent)

P0_4=$($BR create "Add notify crate for filesystem watching" -t task -p P0 \
  -d "Add the notify crate to werk-tui dependencies. Implement a file watcher that monitors .werk/sd.db for external modifications (by CLI, agents, or scripts). When a change is detected, emit Msg::ExternalChange which triggers reload_data(). Debounce with 200ms window to coalesce rapid writes. This eliminates stale data — the TUI always reflects truth. Essential for agent integration where agents modify tensions in the background. Use Cmd::task_named for async watcher, re-arm after each detection." \
  -l "integration,foundation" --parent "$E0" --silent)

# ============================================================================
# PHASE 1: CORE TUI REBUILD
# ============================================================================

echo "Creating Phase 1 tasks..."

P1_1=$($BR create "Implement split pane layout (Grid, auto-detect width)" -t task -p P0 \
  -d "At terminal width >= 120 cols, show Dashboard on left (40%) and Detail on right (60%) using ftui Grid layout. On every MoveUp/MoveDown in Dashboard, also call load_detail() for the newly selected tension. This eliminates the Enter→Detail→Esc→Dashboard loop that is the single most frequent interaction in the current TUI. On narrow terminals (<120), keep current single-pane behavior. The detail load must be fast enough for every cursor move — it's just SQLite read + dynamics computation, which is already cached. Set DashboardState.split_mode based on frame width in view()." \
  -l "tui,layout,phase1" --parent "$E1" --silent)

P1_2=$($BR create "Implement tier-grouped dashboard with section headers" -t task -p P0 \
  -d "Add non-selectable section headers to the dashboard list that group tensions by UrgencyTier: URGENT (red), ACTIVE (white), NEGLECTED (yellow). Headers appear as rows with bold text and tier-appropriate color. Cursor navigation (j/k) must skip header rows — they are visual separators only. This provides structural signal beyond color alone (important for accessibility) and gives immediate sense of proportion ('I have 5 urgent, 3 neglected'). Section headers take 1 row each, so 3 tiers = 3 extra rows — acceptable tradeoff for clarity." \
  -l "tui,dashboard,phase1" --parent "$E1" --silent)

P1_3=$($BR create "Implement cursor-based Detail navigation (sections, not scroll)" -t task -p P0 \
  -d "Replace the current raw-scroll j/k behavior in Detail view with cursor-based section navigation. Define DetailCursor that tracks which section/item is focused: Info, Dynamics, Forecast, Trajectory, Lever, Siblings(index), History(index), Children(index). j/k moves between sections and items within sections. The viewport auto-scrolls to keep the cursor visible. Enter on a child item opens it (push to nav stack). Enter on a sibling navigates to it. This gives consistent j/k behavior across all views (always 'move cursor') and provides a visual indicator of position, unlike the current invisible viewport scroll." \
  -l "tui,detail,phase1" --parent "$E1" --silent)

P1_4=$($BR create "Wire resolution forecasting into Detail dynamics section" -t task -p P0 \
  -d "sd-core already computes Resolution.velocity, Resolution.required_velocity, and Resolution.is_sufficient but the TUI doesn't surface any of it. Add a 'Forecast' line to the Detail dynamics section. Three states: 'On track — resolving ~Mar 27 (1d before horizon)' (green), 'Behind — 60% of required pace' (red), 'Insufficient data — update reality to calibrate' (gray). Extrapolate completion date from magnitude / velocity. This turns dynamics data into actionable foresight — it answers the question every user has: 'will I make it?'" \
  -l "tui,detail,dynamics,phase1" --parent "$E1" --silent)

P1_5=$($BR create "Add Tab for Dashboard ↔ Tree cycling" -t task -p P0 \
  -d "Replace the confusing 1/2/t key mapping (three keys for two views) with Tab to cycle Dashboard→Tree→Dashboard and Shift+Tab to cycle backward. Tab is universally understood as 'switch pane/tab'. Only active in Normal input mode — TextInput and other modes handle their own keys. Remove the old 1/2/t mappings from normal_key_to_msg(). Update help overlays and hint bars accordingly." \
  -l "tui,navigation,phase1" --parent "$E1" --silent)

P1_6=$($BR create "Collapse secondary views into panels and inline sections" -t task -p P0 \
  -d "Eliminate View::Neighborhood, View::Timeline, View::Focus, View::DynamicsSummary as separate full-screen views. Neighborhood → absorbed into Detail as 'Siblings' section (show tensions with same parent). Timeline → toggleable bottom panel in Dashboard (T key, 5-10 rows). Focus → removed entirely (Detail shows everything Focus showed, plus more). DynamicsSummary → modal overlay (D key) or command palette ':health'. Agent view → inline section in Detail with sub-mode for mutation toggles. This eliminates 4 dead-end screens where you can only look and press Esc. Update View enum to: Welcome | Dashboard | Detail | TreeView. Remove all N/T/F/D view-switching keybindings and replace with panel toggles and overlays." \
  -l "tui,views,phase1" --parent "$E1" --silent)

P1_7=$($BR create "Implement single-line quick-add" -t task -p P0 \
  -d "Replace the 3-step tension creation flow (desired→horizon→actual) with a single prompt: 'Add: desired [horizon] [| actual]'. Parse with: split on '|' for actual, then try to extract trailing date format from desired portion (only match YYYY-MM-DD, YYYY-MM, YYYY, +Nw, +Nm, etc. — not arbitrary numbers). Examples: 'Write the novel' (desire only), 'Write the novel 2026-06 | have an outline' (all three), 'Write the novel | have an outline' (desire + actual). Show the format hint in the prompt itself for discoverability. The old multi-step flow is tedious for quick capture." \
  -l "tui,input,phase1" --parent "$E1" --silent)

P1_8=$($BR create "Implement persistent search" -t task -p P0 \
  -d "Change search behavior: / opens search input overlay, typing filters live (same as now), Enter dismisses the overlay but KEEPS the filter active (user can browse filtered results with j/k), Esc in filtered list clears the search. Show active search in status bar: 'werk | 3/12 matching \"novel\"'. Current search is 'search and jump' — evaporates after Enter. Persistent search lets you work within a subset ('show me everything about the novel'). The Esc overloading (clear search vs go back) needs care: when search query is active, first Esc clears search; if no search, Esc does normal back navigation." \
  -l "tui,search,phase1" --parent "$E1" --silent)

P1_9=$($BR create "Implement merged status bar (replace ticker + title)" -t task -p P0 \
  -d "Replace the current two-row header (urgency ticker + title bar) with a single merged status bar. Left side: 'werk  N active  N▲ urgent  N⚠ neglected' (only show non-zero counts). Right side: top 2 urgent tensions with percentage, e.g. '95% Ship feature  80% Fix auth'. This saves one row of vertical space while preserving all information. The !/@/# jump shortcuts still work via urgency sorting." \
  -l "tui,layout,phase1" --parent "$E1" --silent)

# ============================================================================
# PHASE 2: PROJECTION ENGINE (sd-core)
# ============================================================================

echo "Creating Phase 2 tasks..."

P2_1=$($BR create "Implement mutation pattern extraction (extract_mutation_pattern)" -t task -p P0 \
  -d "Create sd-core/src/projection.rs. Implement MutationPattern struct and extract_mutation_pattern() function. Extract engagement patterns from mutation history within a configurable time window. Compute: mean_interval_seconds (average time between mutations), frequency_per_day (count / window_days), frequency_trend (compare first-half vs second-half mutation counts — positive = accelerating), gap_samples (at each 'actual' mutation, compute compute_gap_magnitude(desired, new_value), collect up to 10 most recent), gap_trend (linear slope of gap_samples — negative = gap closing), is_projectable (>= 2 gap samples). This is the foundation for all projection — pure function, no Store dependency. Prerequisite: change compute_gap_magnitude visibility from fn to pub(crate) in dynamics.rs." \
  -l "sd-core,projection,phase2" --parent "$E2" --silent)

P2_2=$($BR create "Implement projection primitives (project_gap_at, estimate_time_to_resolution)" -t task -p P0 \
  -d "Add to projection.rs: project_gap_at(pattern, current_gap, seconds_forward) → f64 — linear extrapolation from gap_trend, clamped to [0.0, 1.0]. project_frequency_at(pattern, seconds_forward) → f64 — extrapolates frequency_trend, clamped >= 0. estimate_time_to_resolution(pattern, current_gap) → Option<i64> — returns None if gap not closing (gap_trend >= 0), otherwise current_gap / abs(gap_trend). No synthetic mutation generation — project analytically from observed patterns. The gap metric depends on text comparison (Levenshtein+Jaccard) so generating synthetic text that yields a target gap magnitude is an inversion problem with no clean solution." \
  -l "sd-core,projection,phase2" --parent "$E2" --silent)

P2_3=$($BR create "Implement per-tension projection (project_tension → TensionProjection)" -t task -p P0 \
  -d "Add TensionProjection struct and project_tension() function. Combines pattern extraction + extrapolation into a structural trajectory per tension. Classify trajectory as Resolving (gap closing + engaged), Stalling (low/zero engagement or engagement declining), Drifting (engaged but gap not closing), Oscillating (gap_samples show alternating up/down reversals). Compute: projected_gap at 1w/1m/3m horizons, will_resolve (can gap close before tension horizon?), projected_urgency (call compute_urgency with future 'now' — trivially correct since it only depends on timestamps), oscillation_risk (high gap_sample variance + high frequency), neglect_risk (projected frequency approaches zero). Return Vec<TensionProjection> — one per standard horizon." \
  -l "sd-core,projection,phase2" --parent "$E2" --silent)

P2_4=$($BR create "Implement field-level projection (project_field → FieldProjection)" -t task -p P0 \
  -d "Add FieldProjection struct with TrajectoryBuckets (resolving/stalling/drifting/oscillating counts per horizon) and UrgencyCollision detection. project_field() iterates all active tensions, calls project_tension() for each, aggregates into trajectory buckets. Urgency collision detection: for each tension with a horizon, sample urgency at weekly intervals (now → now+3m). At each week, collect tensions with urgency > 0.7. If 2+ tensions collide in the same week window, record as UrgencyCollision with tension_ids, window bounds, and peak combined urgency. This reveals upcoming crunch periods where multiple deadlines converge." \
  -l "sd-core,projection,phase2" --parent "$E2" --silent)

P2_5=$($BR create "Enhance lever scoring with trajectory awareness" -t task -p P1 \
  -d "Add trajectory_urgency: f64 component to LeverBreakdown: 1.0 if trajectory is Stalling or Oscillating AND tension has approaching horizon (urgency > 0.5), 0.5 if trajectory is Drifting, 0.0 if Resolving. Add to weighted_score() with ~0.10 weight, rebalance existing weights to sum to 1.0. This makes the lever recommend action on tensions that are structurally headed for trouble, not just those that are currently in trouble. It shifts the lever from reactive to predictive." \
  -l "sd-core,projection,lever,phase2" --parent "$E2" --silent)

# ============================================================================
# PHASE 3: POWER FEATURES
# ============================================================================

echo "Creating Phase 3 tasks..."

P3_1=$($BR create "Implement snooze with auto-resurface" -t task -p P1 \
  -d "Press 'z' on any tension to snooze it. Prompt for wake date (accepts same formats as horizon: +3d, monday, eom, absolute). Store as a mutation with field='snoozed_until', value=date string. In visible_tensions(), filter out snoozed tensions where snooze date > today (unless show_snoozed toggle is active). On every Tick (60s), check for expired snoozes and emit toast: '\"Write chapter 3\" has resurfaced'. Show 'N snoozed' in status bar when count > 0. 'Z' (shift) toggles show_snoozed. No schema changes needed — snooze is just a special mutation. This solves the biggest source of dashboard noise: tensions you're aware of but can't act on right now." \
  -l "tui,feature,phase3" --parent "$E3" --silent)

P3_2=$($BR create "Implement composite auto-resolution + horizon inheritance" -t task -p P1 \
  -d "Two linked behaviors that make hierarchy load-bearing: (1) When a tension resolves, check if all siblings under the same parent are now resolved/released. If so, auto-resolve the parent with a toast: 'All children done — \"Launch product\" auto-resolved'. Recurse upward (parent's resolution might cascade further). (2) When creating a child tension, if the child has no horizon but the parent does, inherit the parent's horizon. This aligns with Fritz: a parent's gap closes when all sub-gaps close. The horizon inheritance creates natural deadline propagation. Both are small additions to existing handlers — resolve handler and create-child handler." \
  -l "tui,feature,hierarchy,phase3" --parent "$E3" --silent)

P3_3=$($BR create "Implement recurring tensions" -t task -p P1 \
  -d "Store recurrence as a mutation: field='recurrence', value='+1w' or '+2w' or '+1m' etc. Set with 'Y' key in Detail view (prompts for interval) or ':recur' in command palette. When resolving a tension that has recurrence set: (1) resolve the current tension normally, (2) create a new tension with same desired, empty actual, same parent, horizon = today + interval, (3) copy the recurrence mutation to the new tension, (4) toast: 'Recurring: new \"Weekly review\" created'. A '~' indicator marks recurring tensions in dashboard. Many real tensions are cyclical — weekly planning, monthly reviews. Without this, users re-create manually each time or keep them permanently active (defeats resolution)." \
  -l "tui,feature,phase3" --parent "$E3" --silent)

P3_4=$($BR create "Implement undo with 5-second window" -t task -p P1 \
  -d "Add UndoAction struct: description String, undo_fn Box<dyn FnOnce(&mut WerkApp)>, expires_at Instant. Add pending_undo: Option<UndoAction> to WerkApp. On resolve/release: perform immediately (no confirm dialog), set up undo closure that reverts status to Active, show toast: 'Resolved \"Write novel\" — press u to undo (5s)'. On 'u' keypress: if pending_undo exists and not expired, execute undo_fn, toast: 'Undone'. On Tick: expire pending_undo if past deadline. Keep confirm dialog ONLY for Delete (truly destructive). This replaces flow-breaking y/n confirms with the Gmail/Slack pattern: act immediately, undo if wrong." \
  -l "tui,feature,undo,phase3" --parent "$E3" --silent)

P3_5=$($BR create "Implement what-if counterfactual preview" -t task -p P1 \
  -d "Before resolving/releasing/deleting, show a preview pane of cascading effects. When user presses R: instead of confirming, show what-if overlay for 2 seconds. Compute by: use store's begin_transaction(), apply the hypothetical change, recompute dynamics, collect diffs (orphaned children, auto-resolved parents, urgency redistribution, lever shift, events that would fire), then rollback_transaction(). Display results in an overlay. Press R again to confirm, Esc to cancel. This exploits werk's unique advantage: it has a full computational dynamics model, so you can literally run the simulation forward before committing. No other tool does this." \
  -l "tui,feature,whatif,phase3" --parent "$E3" --silent)

P3_6=$($BR create "Implement guided morning review ritual" -t task -p P1 \
  -d "Ctrl+R or ':review' starts a structured walkthrough. Build a review queue in priority order: (1) Past-horizon tensions (using Forest::tensions_past_horizon) — prompt: 'This was due N days ago. Done, still active, or release?'. (2) Urgent tensions — prompt: 'Current reality: [actual]. Has anything changed?' → auto-opens reality update. (3) Neglected tensions — prompt: 'You haven't touched this in N days. Still committed?'. (4) Stagnant tensions (movement=Stagnant, phase!=Germination) — prompt: 'No movement. What's blocking progress?'. (5) Summary: 'Reviewed 12 tensions. 3 updated, 1 resolved, 2 snoozed. Lever: [next action]'. New InputMode::Review(ReviewState) with its own keybindings: Enter to act, s to skip, Esc to exit early. This transforms werk from a passive display into an active practice instrument — the confrontation with current reality that Fritz describes as essential." \
  -l "tui,feature,review,phase3" --parent "$E3" --silent)

P3_7=$($BR create "Implement behavioral pattern insights" -t task -p P1 \
  -d "Compute and display behavioral patterns from mutation history. Available via ':insights' in command palette or as a section in health overlay. Patterns to detect: (1) Attention distribution: which tensions get the most/least updates. (2) Oscillation tendency: count tensions with detected oscillation in the last 30 days. (3) Resolution velocity by phase: 'You resolve Completion-phase tensions 4x faster than Assimilation-phase'. (4) Temporal patterns: day-of-week activity distribution. (5) Horizon drift: 'You've postponed X 3 times — repeated postponement'. (6) Neglect patterns: 'Parent getting attention while children stagnant'. Use store.mutations_between(start, end) for time-windowed analysis. Include urgency collision warnings from the projection engine. Display as a modal overlay with severity-colored lines." \
  -l "tui,feature,insights,phase3" --parent "$E3" --silent)

# ============================================================================
# PHASE 4: TUI PROJECTION INTEGRATION
# ============================================================================

echo "Creating Phase 4 tasks..."

P4_1=$($BR create "Add trajectory indicator to dashboard rows" -t task -p P1 \
  -d "Add a single-character trajectory indicator after the movement char in each dashboard row. Format: [Phase] Movement Trajectory Desired... Indicators: '↓' Resolving (green), '—' Stalling (dim gray), '~' Drifting (yellow), '⇌' Oscillating (red). Add trajectory: Option<Trajectory> field to TensionRow. In reload_data(), after computing dynamics: check if last_projection_time is None or > 5 minutes ago; if so, call project_field() and cache in field_projection; then populate trajectory on each TensionRow from the cached projection. On narrow terminals (<40 cols) the trajectory column is hidden." \
  -l "tui,projection,dashboard,phase4" --parent "$E4" --silent)

P4_2=$($BR create "Add Trajectory section to Detail view" -t task -p P1 \
  -d "New section between Dynamics/Forecast and Next Action in Detail view. Shows: trajectory classification (↓ Resolving), gap progression bars at three horizons (Gap now: ■■■■■■□□□□ 0.62, Gap +1w: ■■■■■□□□□□ 0.55, Gap +1m: ■■■□□□□□□□ 0.34), time-to-resolution estimate ('~6 weeks'), engagement trend ('accelerating'/'steady'/'declining'). Conditional risk flags: '⚠ Oscillation risk — gap reversals detected', '⚠ Neglect risk — engagement declining toward zero'. Add ProjectionSummary struct to DetailState. Populate from cached FieldProjection on load_detail()." \
  -l "tui,projection,detail,phase4" --parent "$E4" --silent)

P4_3=$($BR create "Cache FieldProjection in WerkApp on 5-minute cycle" -t task -p P1 \
  -d "Add field_projection: Option<FieldProjection> and last_projection_time: Option<DateTime<Utc>> to WerkApp. In reload_data(), after computing per-tension dynamics: if last_projection_time is None or > 5 minutes ago, call project_field() with all active tensions and their mutations, store result, update timestamp. The 5-minute cache prevents expensive recomputation on every cursor move (in split pane mode, load_detail() fires on every j/k). Also recompute on Msg::ExternalChange (filesystem watch) since the underlying data changed." \
  -l "tui,projection,cache,phase4" --parent "$E4" --silent)

P4_4=$($BR create "Add trajectory overlay via command palette ':trajectory'" -t task -p P1 \
  -d "Add ':trajectory' action to command palette. When triggered, show a modal overlay with the field-wide structural funnel: trajectory distribution at each horizon (1w/1m/3m) as horizontal bar charts (N resolving, N stalling, N drifting, N oscillating). Below that, urgency collision warnings: 'Week of Mar 24: Ship feature (92%) + Fix auth (85%) — high combined urgency'. This is the full-field view — NOT a separate View (respects three-views-only principle), but a modal overlay like the health summary. Dismiss with Esc or ':trajectory' again." \
  -l "tui,projection,overlay,phase4" --parent "$E4" --silent)

# ============================================================================
# PHASE 5: CLI EXPANSION
# ============================================================================

echo "Creating Phase 5 tasks..."

P5_1=$($BR create "Add werk list with rich filtering" -t task -p P1 \
  -d "New 'list' command that replaces tree for flat-list use cases. Flags: --urgent (tier=Urgent only), --neglected (tier=Neglected), --stagnant (movement=Stagnant), --phase G|A|C|M, --snoozed (show snoozed only), --sort urgency|phase|name|horizon, --json (JSON array output). Default: active tensions sorted by tier then urgency (same as TUI dashboard). Each line: [Phase] Movement Desired  Horizon  Urgency%. With --json: full TensionRow-equivalent objects including dynamics and trajectory." \
  -l "cli,query,phase5" --parent "$E5" --silent)

P5_2=$($BR create "Add werk health and werk insights CLI commands" -t task -p P1 \
  -d "Two new analytics commands. 'werk health': phase distribution (counts + bars), movement ratios, alert summary (urgent count, neglected count, stagnant count), system-wide activity sparkline. Mirrors the TUI health overlay but for terminal/script consumption. With --json: structured health data. 'werk insights [--days N]': behavioral pattern digest — attention distribution, oscillation patterns, resolution velocity by phase, temporal patterns, horizon drift, neglect patterns. Default window: 30 days. With --json: structured insights array." \
  -l "cli,analytics,phase5" --parent "$E5" --silent)

P5_3=$($BR create "Add werk diff command" -t task -p P1 \
  -d "New 'diff' command showing what changed in a time window. 'werk diff' = today's changes. 'werk diff --since yesterday', 'werk diff --since \"3 days ago\"', 'werk diff --since 2026-03-10'. Output: grouped by tension — each tension that had mutations in the window, showing field changes with old→new values. Summary line: 'N tensions updated, N created, N resolved'. With --json: structured change objects. Uses store.mutations_between() for time-windowed queries. Useful for daily standup prep and agent context." \
  -l "cli,analytics,phase5" --parent "$E5" --silent)

P5_4=$($BR create "Add werk trajectory CLI command" -t task -p P1 \
  -d "New 'trajectory' command exposing the projection engine via CLI. 'werk trajectory 01KK': per-tension trajectory (trajectory classification, gap progression, time-to-resolution, risk flags). 'werk trajectory': full-field structural funnel (trajectory distribution at 1w/1m/3m). 'werk trajectory --collisions': upcoming urgency collision windows. All support --json for agent consumption. Human output: trajectory char + label, gap bars, collision warnings. Include projection data in 'werk show --json' and 'werk context' output as a 'projection' object." \
  -l "cli,projection,phase5" --parent "$E5" --silent)

P5_5=$($BR create "Add werk reopen, snooze, recur CLI commands" -t task -p P1 \
  -d "Three new orchestration commands: 'werk reopen 01KK' — reactivate a resolved/released tension (status → Active). Useful when a tension was prematurely resolved. 'werk snooze 01KK +3d' — set snooze date, 'werk snooze 01KK --clear' — remove snooze. 'werk recur 01KK +1w' — set recurrence interval, 'werk recur 01KK --clear' — remove recurrence. All record appropriate mutations and support --json output." \
  -l "cli,orchestration,phase5" --parent "$E5" --silent)

P5_6=$($BR create "Add werk batch apply for bulk mutations" -t task -p P1 \
  -d "New 'batch' command: 'werk batch apply mutations.yaml' — reads a YAML file of mutations and applies them all. 'werk batch apply -' — reads from stdin (pipe from agent). 'werk batch validate mutations.yaml' — validates without applying. Same YAML format as agent structured responses: mutations array with action, tension_id, new_value, reasoning fields. Supported actions: update_actual, update_desired, update_status, create_child, add_note. Reports results per mutation (success/failure). This enables any external process to produce mutations and apply them in bulk — key for CI/CD integration and batch agent workflows." \
  -l "cli,agents,phase5" --parent "$E5" --silent)

P5_7=$($BR create "Add werk run --system and werk run --decompose" -t task -p P1 \
  -d "Two new modes for the run command. '--system': instead of single-tension context, provide full system state to the agent — all active tensions with dynamics, the lever recommendation, health summary, trajectory data, top urgency/neglect items, urgency collision warnings. Enables agents to reason about the entire tension forest. '--decompose': specialized prompt template that instructs the agent to break a tension into 3-7 sub-tensions returned as create_child mutations. Reuses existing agent infrastructure (spawn process, parse YAML, apply mutations). Also: add --dry-run to show what would be applied without applying." \
  -l "cli,agents,phase5" --parent "$E5" --silent)

P5_8=$($BR create "Add werk context --all and --urgent, include projection data" -t task -p P1 \
  -d "Extend the context command: '--all' outputs context for all active tensions (not just one), '--urgent' outputs context for urgent-tier tensions only. Both provide the same rich context per tension (dynamics, family, mutations) but in bulk. Also: include projection data (trajectory, projected gaps, risks) in both 'werk context' and 'werk show --json' output as a 'projection' object. Include trajectory and urgency collision data in agent prompts (one-shot mode) as additional context lines." \
  -l "cli,agents,context,phase5" --parent "$E5" --silent)

# ============================================================================
# PHASE 6: HOOKS AND AGENT INTEGRATION
# ============================================================================

echo "Creating Phase 6 tasks..."

P6_1=$($BR create "Implement HookRunner with pre/post mutation hooks" -t task -p P2 \
  -d "Create a HookRunner struct that reads hook configuration from .werk/config.toml and executes shell commands in response to events. Hook types: pre_mutation (runs before, can block — exit 0 = allow, exit 1 = block with stderr as reason), post_mutation (fire-and-forget after any state change), post_resolve, post_create, periodic (runs on TUI tick). Execution model: spawn 'sh -c <command>', pipe event JSON to stdin, capture stdout/stderr. Pre-hooks that fail or exit non-zero block the mutation. Post-hooks that fail are logged as warnings but don't block. Timeout: 5 seconds per hook." \
  -l "hooks,phase6" --parent "$E6" --silent)

P6_2=$($BR create "Wire hooks into store mutation path" -t task -p P2 \
  -d "Integrate HookRunner into every mutation function: update_actual, update_desired, update_horizon, update_status, update_parent, create_tension, delete_tension, record_mutation (for notes). Before mutation: build HookEvent JSON (event type, tension_id, tension_desired, field, old_value, new_value, dynamics summary), call pre_mutation hook. If blocked: return WerkError::HookBlocked with stderr message. After mutation: call post_mutation hook (fire-and-forget). For resolve: also call post_resolve hook. For create: also call post_create hook. Both CLI and TUI go through the same code path." \
  -l "hooks,phase6" --parent "$E6" --silent)

P6_3=$($BR create "Add hook configuration schema to config.toml" -t task -p P2 \
  -d "Define the [hooks] section in .werk/config.toml: hooks.pre_mutation, hooks.post_mutation, hooks.post_resolve, hooks.post_create, hooks.periodic. Values are shell commands (strings). Document the event JSON payload format: { event, timestamp, tension_id, tension_desired, field, old_value, new_value, dynamics: { phase, movement, urgency, forecast_on_track } }. For resolve events: add cascade array. For create events: add parent_id. Add 'werk config set hooks.post_mutation \"path/to/script.sh\"' as the configuration method." \
  -l "hooks,config,phase6" --parent "$E6" --silent)

P6_4=$($BR create "Write example hook scripts and CLAUDE.md template" -t task -p P2 \
  -d "Create four example hook scripts in designs/hooks-examples/: (1) slack-notify.sh — post_resolve hook that sends Slack webhook on tension resolution. (2) auto-commit.sh — post_mutation hook that git-adds .werk/sd.db and commits. (3) agent-review.sh — periodic hook that checks for neglected tensions and runs 'werk run' on the first one. (4) pre-validate.sh — pre_mutation hook that blocks deletion of tensions with children. Also create a CLAUDE.md template (designs/claude-md-template.md) documenting all werk commands for Claude Code integration, with reading/modifying/analyzing sections." \
  -l "hooks,docs,phase6" --parent "$E6" --silent)

P6_5=$($BR create "Test with Claude Code hooks integration" -t task -p P2 \
  -d "Create a .claude/settings.json example that demonstrates Claude Code hooks for werk integration: (1) user_prompt_submit hook that prepends 'werk list --urgent --json | head -5' output to give Claude context about urgent tensions. (2) post_tool_use hook triggered on 'git commit' that records a note on the most relevant tension. Test end-to-end: Claude Code session where the agent reads werk state, makes modifications, and the user sees changes reflected in the TUI (via filesystem watch). Document the tested workflow in the CLAUDE.md template." \
  -l "hooks,agents,claude,phase6" --parent "$E6" --silent)

# ============================================================================
# PHASE 7: POLISH
# ============================================================================

echo "Creating Phase 7 tasks..."

P7_1=$($BR create "Tree expand/collapse with persisted state" -t task -p P3 \
  -d "Use ftui Tree widget with h/l (vim-style) for collapse/expand. Persist the set of expanded node IDs to .werk/tree-state.json on app exit and restore on launch. Without persistence, every TUI restart resets the tree — users with large forests have to re-expand their working subtrees every time. HashSet<String> serialized as JSON array." \
  -l "tui,polish,phase7" --parent "$E7" --silent)

P7_2=$($BR create "Inline mode for always-on dashboard" -t task -p P3 \
  -d "ftui supports ScreenMode::Inline { ui_height: N } which keeps an N-row UI region at the bottom of the terminal while normal scrollback continues above. Implement a --inline N flag for the TUI binary that activates inline mode with N rows (default 8). Shows a compact dashboard: status bar + top urgent tensions. Press a key to expand to full alt-screen mode. This makes werk a constant companion rather than a tool you open and close." \
  -l "tui,polish,phase7" --parent "$E7" --silent)

P7_3=$($BR create "MCP server for werk (optional)" -t task -p P3 \
  -d "Build only if demand warrants. A dedicated MCP server (werk-mcp crate) exposing tools: werk_list, werk_show, werk_add, werk_reality, werk_resolve, werk_context, werk_health, werk_insights, werk_trajectory. And resources: werk://tensions (live list), werk://tension/{id} (detail), werk://health (summary), werk://lever (current recommendation). Lower priority than CLI+hooks because: CLI already serves as an excellent API, MCP requires per-framework setup, and hooks provide event-driven integration that MCP resources don't." \
  -l "mcp,polish,phase7" --parent "$E7" --silent)

# ============================================================================
# DEPENDENCY WIRING
# ============================================================================

echo "Wiring dependencies..."

# Phase 1 depends on Phase 0
for task in $P1_1 $P1_2 $P1_3 $P1_4 $P1_5 $P1_6 $P1_7 $P1_8 $P1_9; do
  $BR dep add "$task" "$P0_1" -t blocks 2>/dev/null  # all phase 1 depends on sub-struct extraction
  $BR dep add "$task" "$P0_2" -t blocks 2>/dev/null  # all phase 1 depends on dead code removal
done

# Phase 2 (projection) has internal ordering
$BR dep add "$P2_2" "$P2_1" -t blocks 2>/dev/null  # primitives depend on pattern extraction
$BR dep add "$P2_3" "$P2_2" -t blocks 2>/dev/null  # per-tension depends on primitives
$BR dep add "$P2_4" "$P2_3" -t blocks 2>/dev/null  # field-level depends on per-tension
$BR dep add "$P2_5" "$P2_3" -t blocks 2>/dev/null  # lever enhancement depends on per-tension

# Phase 3 depends on Phase 1 (TUI rebuild)
for task in $P3_1 $P3_2 $P3_3 $P3_4 $P3_5 $P3_6 $P3_7; do
  $BR dep add "$task" "$P1_3" -t blocks 2>/dev/null  # all features need cursor-based detail
  $BR dep add "$task" "$P1_6" -t blocks 2>/dev/null  # all features need collapsed views
done
# Specific dependencies within Phase 3
$BR dep add "$P3_1" "$P0_3" -t blocks 2>/dev/null  # snooze needs relative horizon parsing
$BR dep add "$P3_6" "$P3_1" -t blocks 2>/dev/null  # morning review needs snooze (as an action option)
$BR dep add "$P3_7" "$P2_4" -t blocks 2>/dev/null  # insights include urgency collisions from projection
$BR dep add "$P3_4" "$P1_6" -t blocks 2>/dev/null  # undo replaces confirm dialogs from collapsed views

# Phase 4 (TUI projection) depends on Phase 2 (sd-core projection) + Phase 1 (TUI rebuild)
for task in $P4_1 $P4_2 $P4_3 $P4_4; do
  $BR dep add "$task" "$P2_4" -t blocks 2>/dev/null  # all TUI projection needs field-level projection
  $BR dep add "$task" "$P1_1" -t blocks 2>/dev/null  # needs split pane (trajectory in detail pane)
done
$BR dep add "$P4_1" "$P4_3" -t blocks 2>/dev/null  # dashboard indicators need cached projection
$BR dep add "$P4_2" "$P4_3" -t blocks 2>/dev/null  # detail section needs cached projection

# Phase 5 (CLI) can proceed in parallel with Phase 3/4 but needs Phase 0
for task in $P5_1 $P5_2 $P5_3 $P5_4 $P5_5 $P5_6 $P5_7 $P5_8; do
  $BR dep add "$task" "$P0_3" -t blocks 2>/dev/null  # CLI needs relative horizon parsing
done
$BR dep add "$P5_4" "$P2_4" -t blocks 2>/dev/null  # trajectory CLI needs projection engine
$BR dep add "$P5_8" "$P2_3" -t blocks 2>/dev/null  # context projection data needs per-tension projection
$BR dep add "$P5_5" "$P3_1" -t blocks 2>/dev/null  # snooze/recur CLI needs the feature designed
$BR dep add "$P5_5" "$P3_3" -t blocks 2>/dev/null  # recur CLI needs recurring tensions
$BR dep add "$P5_7" "$P5_6" -t blocks 2>/dev/null  # run --decompose depends on batch apply format

# Phase 6 (hooks) depends on Phase 5 (CLI surface exists)
for task in $P6_1 $P6_2 $P6_3 $P6_4 $P6_5; do
  $BR dep add "$task" "$P5_1" -t blocks 2>/dev/null  # hooks need CLI commands to exist
done
$BR dep add "$P6_2" "$P6_1" -t blocks 2>/dev/null  # wiring needs HookRunner
$BR dep add "$P6_3" "$P6_1" -t blocks 2>/dev/null  # config schema needs HookRunner
$BR dep add "$P6_4" "$P6_2" -t blocks 2>/dev/null  # examples need wiring done
$BR dep add "$P6_5" "$P6_4" -t blocks 2>/dev/null  # Claude test needs examples
$BR dep add "$P6_5" "$P0_4" -t blocks 2>/dev/null  # Claude test needs filesystem watch

# Phase 7 (polish) depends on Phase 1 (TUI exists)
$BR dep add "$P7_1" "$P1_5" -t blocks 2>/dev/null  # tree persist needs Tab cycling
$BR dep add "$P7_2" "$P1_1" -t blocks 2>/dev/null  # inline mode needs split pane logic
$BR dep add "$P7_3" "$P5_1" -t blocks 2>/dev/null  # MCP needs CLI surface

# ============================================================================
# ADD DETAILED COMMENTS WITH CONTEXT
# ============================================================================

echo "Adding context comments..."

# Epic-level context comments
$BR comments add "$E0" "CONTEXT: Phase 0 is the 'boring but essential' phase. None of these changes are visible to users, but without them the codebase becomes unmaintainable as we add projection state, review state, and all the new features. The sub-struct extraction (P0_1) is the most important — it turns a 40-field god object into focused state containers. Every subsequent PR is smaller and easier to review."

$BR comments add "$E1" "CONTEXT: The v0.4 TUI has 8 views, 5 input modes, and 21 command palette actions. Users navigate a maze of dead-end screens (Neighborhood, Timeline, Focus, DynamicsSummary) where you can only look and press Esc. The rebuild reduces to 3 views (Dashboard/Detail/Tree) with everything else as panels or overlays. The split pane at 120+ cols eliminates the #1 friction: the Enter→Detail→Esc→Dashboard loop."

$BR comments add "$E1" "DESIGN RATIONALE: Three principles guide every decision: (1) Density without clutter — show maximum useful information per pixel, (2) Cursor not viewport — user always has a position, screen follows cursor, (3) Two-and-a-half views — Dashboard list, Detail depth, Tree structure, nothing more. If it's not one of these three, it's a panel, overlay, or inline section."

$BR comments add "$E2" "CONTEXT: Fritz's central insight is that structure determines the path of least resistance, and energy moves along that path. The current system only shows dynamics at the present moment. The projection engine extrapolates observed engagement patterns forward, classifies trajectories, and makes the path of least resistance visible. This is the theoretical heart of werk — it's what makes it a structural dynamics instrument rather than a task list."

$BR comments add "$E2" "DESIGN DECISION: No synthetic mutation generation. The gap metric depends on text comparison (Levenshtein+Jaccard), so generating synthetic text that yields a target gap magnitude is an inversion problem with no clean solution. Instead, project dynamics analytically from observed patterns. The one exception is compute_urgency(), which depends only on timestamps and can be called directly with a future 'now'."

$BR comments add "$E3" "CONTEXT: These features amplify what makes werk unique vs. any task manager. Snooze acknowledges tensions honestly ('not now' instead of ignoring or lying). Auto-resolution makes hierarchy structural rather than decorative. What-if preview exploits the dynamics engine for foresight. Morning review transforms the tool from passive dashboard to active practice. Each feature was selected from ~100 candidates and evaluated for: uniqueness to werk, pragmatic implementability, complexity burden, daily impact, and innovation."

$BR comments add "$E4" "CONTEXT: Phase 4 is where the projection engine becomes visible. Dashboard gets a single-character trajectory indicator per row. Detail gets gap progression bars. The field-wide structural funnel overlay shows where the entire system is headed. All reads from a cached FieldProjection that recomputes every 5 minutes — never blocks the UI."

$BR comments add "$E5" "CONTEXT: The CLI is NOT for daily human use — that's the TUI. The CLI is the scripting surface for: (1) quick one-off commands, (2) piping with Unix tools, (3) agent consumption/mutation, (4) automation scripts. Three principles: --json everywhere, composable primitives, semantic exit codes. Every command that produces output also accepts --json and returns structured data."

$BR comments add "$E6" "CONTEXT: werk doesn't try to be an agent. werk is the state layer that agents read from and write to. Three-tier integration: (1) CLI-as-API works with any agent that can run shell commands, (2) structured I/O via werk run, (3) pre/post mutation hooks that trigger external processes. The hook system is the most powerful tier — it enables event-driven integration without polling."

# Task-level context on key decisions
$BR comments add "$P1_3" "DESIGN NOTE: The current Detail view uses raw paragraph scroll — j/k moves an invisible viewport over text. This is the biggest UX problem in the TUI. Users have no sense of 'where am I' in the detail. Section-based navigation with a visible cursor position solves this completely. It also enables Enter on a child/sibling to navigate into it, which is impossible with viewport scroll."

$BR comments add "$P1_6" "MIGRATION NOTE: The Agent view (View::Agent) is the trickiest to collapse. Its keybindings (a for apply, 1-9 for toggle) conflict with Detail's keybindings. Solution: agent responses appear as a collapsible section in Detail. When the agent section is focused (cursor on it), Detail enters a sub-mode where a=apply and 1-9=toggle work. Esc exits the sub-mode, not the view."

$BR comments add "$P2_1" "IMPLEMENTATION NOTE: compute_gap_magnitude in dynamics.rs (line ~758) needs to become pub(crate) so projection.rs can call it. This is the 60% normalized Levenshtein + 40% token-level Jaccard metric. No behavioral change — just visibility."

$BR comments add "$P3_5" "IMPLEMENTATION NOTE: The what-if preview uses Store's transaction support: begin_transaction() → apply hypothetical change → recompute dynamics → diff against pre-change state → rollback_transaction(). This is safe because SQLite transactions are truly atomic. The preview computation should complete in <100ms for a typical tension forest."

$BR comments add "$P3_6" "DESIGN NOTE: The morning review is the single most habit-forming feature. Without it, users open werk and think 'I see a list, now what?' The structured walkthrough answers that question every morning. The sequence (past-horizon → urgent → neglected → stagnant) mirrors Fritz's practice: confront current reality, acknowledge what's overdue, decide what to keep/release."

$BR comments add "$P5_7" "DESIGN NOTE: --system mode is critical for agents that need to reason about the entire tension forest, not just one tension at a time. An agent with system-wide context can say 'these three tensions conflict' or 'you should resolve X before Y because of cascade effects' — reasoning that's impossible with single-tension context."

$BR comments add "$P6_1" "DESIGN NOTE: Pre-hooks are powerful but dangerous. A broken pre-hook blocks all mutations. Mitigations: 5-second timeout, clear error message with stderr content, 'werk config set hooks.pre_mutation \"\"' to disable. Post-hooks are fire-and-forget — failures are logged but don't block. This asymmetry is intentional: pre-hooks are policy gates, post-hooks are notifications."

echo ""
echo "=== DONE ==="
echo "Created 8 epics + 42 tasks with full dependency structure."
echo "Run 'br stats' to verify, 'br ready' to see unblocked work."
