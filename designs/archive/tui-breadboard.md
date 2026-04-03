# TUI Stream View — Breadboard

**Shape:** D (Adaptive Hybrid + Configuration Layer)
**Source:** designs/tui-shaping.md
**Scope:** Stream view only — one tension's deck in all states and zoom levels.

---

## Places

| # | Place | Description |
|---|-------|-------------|
| P1 | Deck (NORMAL) | Default state. Desire, route, console, reality visible. All zoom levels (normal/orient). Pitch/roll/zoom available. |
| P2 | Deck (FOCUSED) | Focus zoom via Enter. One element's detail fills the console. Mutation gestures available. Shift+Enter or Esc to return. |
| P3 | Deck (INPUT) | Text editing. Creating or editing desire, reality, note, or new child. Deck dims as context. Confirm/cancel to exit. |
| P4 | Deck (PATHWAY) | Decision palette. 3-5 options inline after a gesture produces a structural signal. Select/dismiss to exit. |
| P5 | Child Deck | New deck for a child tension. Entered via roll-right (l/→). Separate tension context. Roll-left (h/←) returns to P1. |
| P6 | Log | Prior epochs. Entered by navigating past reality (j/↓ past bottom). Design deferred — noted as transition target. |

---

## UI Affordances

| # | Place | Zone | Affordance | Control | Wires Out | Returns To |
|---|-------|------|------------|---------|-----------|------------|
| **Screen boundary signals** | | | | | | |
| U1 | P1 | boundary | Parent breadcrumb (`← #N text...`) | display | — | — |
| U2 | P1 | boundary | Log indicator (`↓ N prior epochs`) | display | — | — |
| **Desire zone** | | | | | | |
| U3 | P1 | desire | Desire text | display | — | — |
| U4 | P1 | desire | Desire deadline (left col) | display | — | — |
| U5 | P1 | desire | Desire age (right col) | display | — | — |
| U6 | P1 | desire | Desire rule (━━━) | display | — | — |
| **Route zone** | | | | | | |
| U7 | P1 | route | Route step line (per step: deadline, text, ID, →N, age) | display | — | — |
| U8 | P1 | route | Route compressed indicator (`▲ N remaining · next [date]`) | display | — | — |
| **Console zone** | | | | | | |
| U9 | P1 | console | Console boundary top (┄┄┄, contextual) | display | — | — |
| U10 | P1 | console | Overdue step line (per overdue: deadline, ▸, text, OVERDUE, ID, →N) | display | — | — |
| U11 | P1 | console | Next committed step line (deadline, ▸, text, ID, →N, age) | display | — | — |
| U12 | P1 | console | Held indicator (`· N held`) | display | — | — |
| U13 | P1 | console | Input point (`+ ___`) | display | — | — |
| U14 | P1 | console | Accumulated indicator (`✓ N resolved · ~ N released · ※ N note`) | display | — | — |
| U15 | P1 | console | Console boundary bottom (┄┄┄, contextual) | display | — | — |
| **Reality zone** | | | | | | |
| U16 | P1 | reality | Reality text | display | — | — |
| U17 | P1 | reality | Reality age (right col) | display | — | — |
| U18 | P1 | reality | Reality rule (──) | display | — | — |
| **Cursor** | | | | | | |
| U19 | P1 | — | Cursor highlight (selected line) | display | — | — |
| U20 | P1 | — | Selected step text wrap (full text on selected, truncated on others) | display | — | — |
| **Child signal annotations (R3)** | | | | | | |
| U21 | P1 | — | Deviance annotation on child line (one per child, only when deviant) | display | — | — |
| U22 | P1 | — | Children indicator on step line (→N in right col) | display | — | — |
| **Peek (P1 local state)** | | | | | | |
| U23 | P1 | — | Inline peek (appears below selected step on Space): children list + reality text | display | — | — |
| **Orient additions (P1, orient zoom)** | | | | | | |
| U24 | P1 | orient | Parent desire (full text, replaces breadcrumb) | display | — | — |
| U25 | P1 | orient | Parent reality (full text, below current reality) | display | — | — |
| U26 | P1 | orient | Siblings line (`siblings: #N name ✓ · #N name`) | display | — | — |
| U27 | P1 | orient | Grandchild count per console item (`(N sub)`) | display | — | — |
| U28 | P1 | orient | Ordinals in left column (position number) | display | — | — |
| **Focus view (P2)** | | | | | | |
| U29 | P2 | focus | Focused element heading (deadline + text + ID) | display | — | — |
| U30 | P2 | focus | Focused element desire (full text, with age) | display | — | — |
| U31 | P2 | focus | Focused element children (individual lines with glyphs) | display | — | — |
| U32 | P2 | focus | Focused element reality (full text, with age) | display | — | — |
| U33 | P2 | focus | Top compressed indicator (`▲ N more in route`) | display | — | — |
| U34 | P2 | focus | Bottom compressed indicator (`▼ overdue · next · held · resolved...`) | display | — | — |
| **INPUT state (P3)** | | | | | | |
| U35 | P3 | input | Text input field (with cursor) | type | → N12 | — |
| U36 | P3 | input | Input prompt/label (what you're editing) | display | — | — |
| U37 | P3 | input | Dimmed deck background | display | — | — |
| **PATHWAY state (P4)** | | | | | | |
| U38 | P4 | pathway | Pathway options (3-5 inline, arrow-selectable) | select | → N14 | — |
| U39 | P4 | pathway | Pathway context line (what triggered this) | display | — | — |
| **Status line** | | | | | | |
| U40 | P1 | chrome | Hint line (context-sensitive key hints) | display | — | — |
| U41 | P1 | chrome | Transient message (auto-expiring feedback) | display | — | — |

---

## Code Affordances

| # | Place | Component | Affordance | Control | Wires Out | Returns To |
|---|-------|-----------|------------|---------|-----------|------------|
| **Data loading** | | | | | | |
| N1 | P1 | engine | `load_tension(id)` | call | → S1, → N2, → N3 | — |
| N2 | P1 | engine | `load_children(id)` | call | → S2 | — |
| N3 | P1 | engine | `load_mutations(id)` | call | → S8 | — |
| **Frontier computation** | | | | | | |
| N4 | P1 | frontier | `compute_frontier(children, epoch)` — classifies each child as: route (positioned, not yet action-relevant), overdue, next, held, or accumulated (resolved/released/note since epoch) | call | → S3 | → U7, U8, U10, U11, U12, U14 |
| N5 | P1 | frontier | `compute_epoch(tension, mutations)` — determines current epoch boundary, what's accumulated | call | → S7 | → N4 |
| **Signal computation** | | | | | | |
| N6 | P1 | signals | `compute_child_signals(child, mutations)` — per-child: desire-changed, reality-stale, internal resolution counts. Returns annotation (or none if not deviant). | call | — | → U21 |
| N7 | P1 | signals | `compute_overdue_intensity(step, now)` — days overdue → visual weight (C5: time-amplified) | call | — | → U10 |
| **Layout engine** | | | | | | |
| N8 | P1 | layout | `compute_columns(width, entries)` — calculates left col width (longest deadline), main col start, right col budget | call | → S9 | → U3-U18, U29-U34 |
| N9 | P1 | layout | `compute_compression(height, zoom, frontier, config)` — decides what to show/compress based on available space, zoom level, config. Returns render plan. | call | → S10 | → N10 |
| N10 | P1 | render | `render_deck(frame, plan)` — main render dispatch. Reads render plan and paints zones. | call | → U1-U41 | — |
| **Navigation** | | | | | | |
| N11 | P1 | nav | `handle_pitch(direction)` — moves cursor through route/console toward desire(k) or reality(j). Wraps through zones. | call | → S4 | → U19 |
| N11b | P1 | nav | `handle_roll(direction)` — l: descend into selected → P5. h: ascend to parent → reload P1 with parent context. | call | → N1 | → P5, P1 |
| N11c | P1 | nav | `handle_zoom(level)` — Enter → P2 (FOCUSED). Shift+Enter toggles orient. Esc from P2 → P1. | call | → S5 | → P2 |
| N11d | P1 | nav | `handle_peek()` — Space on step with children: toggle inline children list. | call | → N2 | → U23 |
| **Gesture handlers** | | | | | | |
| N12 | P3 | gestures | `handle_input_submit(text, target)` — commits text to engine. Target: desire, reality, note, new child. May trigger epoch close (R7). | call | → N15, → N16 | — |
| N13 | P1 | gestures | `initiate_gesture(kind)` — starts a gesture: 'a' (add child), 'e' (edit desire), 'r' (resolve), '~' (release), '※' (note), etc. Transitions to appropriate state. | call | — | → P2, P3, P4 |
| N14 | P4 | gestures | `handle_pathway_select(option)` — user picks from palette. Executes chosen mutation. | call | → N15, → N16 | → P1 |
| **Mutation & epoch** | | | | | | |
| N15 | — | engine | `apply_mutation(gesture)` — writes mutation to store. The core write path. | call | → S1, S2, S8 | — |
| N16 | — | engine | `check_epoch_close(tension, mutation)` — after desire/reality/resolve/release mutation, checks if epoch should close. If yes, compresses accumulated into log. | call | → S7, → N4 | — |
| **Config** | | | | | | |
| N17 | — | config | `read_config()` — reads deck.* settings from store/file | call | → S6 | → N9, N10 |
| **Focus-specific** | | | | | | |
| N18 | P2 | focus | `load_focused_detail(element_id)` — loads full desire, reality, children, signals for the focused element | call | → S11 | → U29-U32 |
| N19 | P2 | focus | `compute_focus_compression(frontier, focused_id)` — determines what compresses above/below the focused element, preserving spatial order | call | — | → U33, U34 |
| **Orient-specific** | | | | | | |
| N20 | P1 | orient | `load_orient_context(parent_id)` — loads parent tension, siblings, computes grandchild counts | call | → S12 | → U24-U28 |

---

## Data Stores

| # | Scope | Store | Description |
|---|-------|-------|-------------|
| S1 | P1 | `current_tension` | The tension being viewed (desire, reality, status, horizon, epoch) |
| S2 | P1 | `children` | Children of current tension (full Tension objects) |
| S3 | P1 | `frontier` | Computed frontier: `{ route: Vec, overdue: Vec, next: Option, held: Vec, accumulated: Vec }` |
| S4 | P1 | `cursor` | Cursor position: which zone (route/console) and index within it |
| S5 | P1 | `zoom_level` | Current zoom: Normal, Focus, Orient |
| S6 | — | `config` | Deck config settings (deck.chrome, deck.color, deck.peek, etc.) |
| S7 | P1 | `epoch` | Current epoch: boundary timestamp, accumulated count by type |
| S8 | P1 | `mutations` | Mutation history for current tension + children (for signal computation) |
| S9 | P1 | `columns` | Computed column layout: left_width, main_start, right_start |
| S10 | P1 | `render_plan` | What to show at current zoom/space: which zones visible, what's compressed, chrome level |
| S11 | P2 | `focused_detail` | Full detail of focused element: desire, reality, children, signals |
| S12 | P1 | `orient_context` | Parent tension, siblings, grandchild counts (loaded on orient zoom) |

---

## Key Wiring Flows

### Flow 1: Open the deck (land at console)

```
N1 load_tension → S1 (current tension)
N2 load_children → S2 (children)
N3 load_mutations → S8 (mutations)
N5 compute_epoch(S1, S8) → S7 (epoch)
N4 compute_frontier(S2, S7) → S3 (frontier: route/overdue/next/held/accumulated)
N6 compute_child_signals (per child in S2, using S8) → annotations
N8 compute_columns → S9
N9 compute_compression(height, zoom=Normal, S3, S6) → S10 (render plan)
N10 render_deck(S10) → U1-U22 (the visible deck)
```

Cursor starts at first console item (overdue or next).

### Flow 2: Pitch navigation (j/k)

```
User presses j/k
→ N11 handle_pitch(direction)
→ S4 cursor moves to next/prev selectable element (route step, overdue, next, held indicator, input point, accumulated indicator)
→ U19 cursor highlight repaints
→ U20 selected step wraps, previously selected truncates
```

Cursor wraps through zones: route (k from top of console) ↔ console ↔ reality anchor (j from bottom of console). Past reality → P6 (log, deferred).

### Flow 3: Roll navigation (l to descend)

```
User presses l on a step with children
→ N11b handle_roll(Right)
→ N1 load_tension(selected_child_id) — loads child as new context
→ full reload flow (Flow 1) with child as current tension
→ P5 (Child Deck) — effectively a new P1 instance for the child
```

### Flow 4: Focus zoom (Enter)

```
User presses Enter on a route/console step
→ N11c handle_zoom(Focus)
→ S5 zoom_level = Focus
→ N18 load_focused_detail(element_id) → S11
→ N19 compute_focus_compression(S3, element_id) → compressed indicators
→ N9 compute_compression(height, zoom=Focus, ...) → S10 (new render plan)
→ N10 render_deck → P2 view (U29-U34 + desire/reality anchors)
```

Esc or Shift+Enter returns: S5 → Normal, re-render P1.

### Flow 5: Peek (Space)

```
User presses Space on step with children (→N > 0)
→ N11d handle_peek()
→ N2 load_children(selected_step_id) — if not already cached
→ also load reality text for that step
→ U23 inline children list appears below selected step, reality text beneath children
→ N9 recompute compression (peek expansion takes space)
→ N10 re-render

Next j/k press → U23 closes, cursor moves normally
```

The peek shows a mini descended view: children list + reality. No desire (you can already see the step's desire text on its line). This gives enough context to decide whether to descend (l) or move on.

### Flow 5b: Quick edit from console (special character input)

```
User is at resting cursor position in console (NORMAL state)
User types '!' → transitions to P3 (INPUT) for desire editing
User types '?' → transitions to P3 (INPUT) for reality editing
User types '※' (or configured key) → transitions to P3 (INPUT) for note

The special character is consumed (not inserted into text).
Text input opens with current desire/reality pre-filled, or blank for note.
```

This pattern (cf. Claude Code's `!` for shell commands) allows editing desire/reality without navigating away from the console. The cursor stays at its position; after confirming, you're back at the console. The specific characters are candidates — exact bindings deferred to #16 (state machine), but the pattern of "special char from NORMAL → targeted INPUT" is a shape-level decision.

### Flow 6: Create a child (a)

```
User presses 'a' in P1
→ N13 initiate_gesture(AddChild) → transition to P3
→ U36 "new step:" prompt, U35 text input active, U37 deck dims
User types and presses Enter
→ N12 handle_input_submit(text, AddChild)
→ N15 apply_mutation(create_child)
→ N16 check_epoch_close — no close (child creation doesn't close epoch)
→ N1 reload → full Flow 1
→ transition back to P1
```

### Flow 7: Update reality (triggers epoch close)

```
User focuses on reality (j to bottom), presses 'e'
→ N13 initiate_gesture(EditReality) → transition to P3
→ U35 text input with current reality pre-filled
User edits and presses Enter
→ N12 handle_input_submit(new_text, EditReality)
→ N15 apply_mutation(update_reality)
→ N16 check_epoch_close → YES: reality update closes epoch
  → accumulated facts compressed into log
  → S7 new epoch, S3 frontier recomputed (accumulated now empty)
→ N1 reload → full Flow 1 (fresh epoch — console indicators gone)
→ transition back to P1
```

### Flow 8: Pathway palette (structural decision)

```
User sets a child deadline that exceeds parent deadline
→ N15 apply_mutation(set_deadline) detects containment violation
→ N13 initiate_gesture(Pathway) → transition to P4
→ U38 options: [keep as-is] [clip to parent] [extend parent] [promote to sibling]
→ U39 context: "child deadline exceeds parent"
User selects with j/k, presses Enter
→ N14 handle_pathway_select(chosen_option)
→ N15 apply_mutation(chosen_mutation)
→ transition back to P1
```

### Flow 9: Orient zoom (Shift+Enter)

```
User presses Shift+Enter in P1
→ N11c handle_zoom(Orient)
→ S5 zoom_level = Orient
→ N20 load_orient_context(parent_id) → S12 (parent, siblings, grandchild counts)
→ N9 compute_compression(height, zoom=Orient, ...) → S10
→ N10 render_deck → P1 with orient additions (U24-U28)

Shift+Enter again → S5 Normal, re-render without orient context
```

---

## Existing Code Mapping

| New concept | Existing code | Status |
|-------------|--------------|--------|
| S1 current_tension | `app.parent_tension` | Rename, extend with epoch |
| S2 children | `app.siblings` (Vec<FieldEntry>) | Replace FieldEntry with richer type |
| S4 cursor | `app.vlist.cursor` (usize) | Replace with zone-aware cursor |
| S5 zoom_level | (does not exist) | **New** |
| S7 epoch | (does not exist) | **New** — needs data model support |
| S3 frontier | (does not exist) | **New** — currently all children shown flat |
| N4 compute_frontier | (does not exist) | **New** — core new computation |
| N5 compute_epoch | (does not exist) | **New** — depends on #36 (epoch lifecycle) |
| N6 compute_child_signals | `glyphs::temporal_indicator` (partial) | Extend significantly |
| N8 compute_columns | (does not exist) | **New** — currently ad-hoc width calc |
| N9 compute_compression | (does not exist) | **New** — core new layout logic |
| N10 render_deck | `app.render_field()` | **Rewrite** |
| N11 handle_pitch | `Msg::Up/Down` in `update_normal` | **Rewrite** — zone-aware cursor |
| N11b handle_roll | `Msg::Descend/Ascend` | Adapt |
| N13 initiate_gesture | `Msg::Start*` handlers | Adapt |
| N15 apply_mutation | `engine.create_*`, `engine.update_*` | Extend |
| InputMode | `state::InputMode` enum | Map to P1-P4 |
| GazeState | `app.gaze` | **Replace** with peek (U23) and focus (P2) |
| Alerts | `app.alerts` | Evaluate — may become signals (N6/N7) |

---

## Config Wiring (D7)

| Setting | Affects | Code affordances |
|---------|---------|-----------------|
| `deck.chrome` | U9, U15 (console boundaries) | → N9, N10 |
| `deck.color` | All U styling | → N10 |
| `deck.accent` | Cursor, OVERDUE, boundaries | → N10 |
| `deck.ordinals` | U28 (ordinals in left col) | → N9, N10 |
| `deck.peek` | U23 (inline children) vs focus-only | → N11d |
| `deck.signals` | U21 (annotation intensity) | → N7 |
| `deck.compression` | Route-first vs symmetric | → N9 |
| `deck.wrap` | U20 (step text wrapping) | → N10 |
| `deck.trunk` | Route trunk line (│) | → N10 |
| `deck.gutter` | Left gutter signal marks | → N10 |

---

## Slices

Each slice ends in a demo-able state. Ordered by dependency — each builds on the previous.

### Slice Summary

| # | Slice | Mechanism | Key Affordances | Demo |
|---|-------|-----------|-----------------|------|
| V1 | Deck skeleton with column layout | N8, N10, U1-U6, U16-U19 | Desire/reality anchors, breadcrumb, column alignment, cursor on placeholder | "Deck opens. Desire at top, reality at bottom, columns aligned. Cursor visible." |
| V2 | Frontier computation + console | N1-N5, N11, U7, U10-U14 | Frontier classifies children. Route steps and console zones render. Pitch navigates. | "Children appear in correct zones. Overdue flagged. Cursor pitches through route and console." |
| V3 | Roll navigation + child deck | N11b, U1 (breadcrumb updates) | Descend into child, see its deck. Ascend back. Breadcrumb tracks path. | "Press l on a step, its deck loads. Press h, return to parent. Breadcrumb updates." |
| V4 | Gestures + INPUT state | N12, N13, N15, U35-U37 | Create child, edit desire, edit reality, add note. Text input with dimmed background. Quick-edit via special chars (!/?) from console. | "Press 'a', type a step, confirm. Press '?', edit reality. New child appears in console." |
| V5 | Epoch mechanics | N5, N16, S7 | Reality/desire update closes epoch. Accumulated indicators clear. Fresh epoch signal. | "Edit reality. Console indicators vanish — fresh epoch. Resolve a step, indicator appears." |
| V6 | Compression engine + config | N9, N17, S6, S10 | Adaptive chrome, route-first compression, config-driven rendering. | "Shrink terminal — route compresses to count. Set deck.chrome=structured — separators appear." |
| V7 | Focus zoom | N11c, N18, N19, U29-U34 | Enter on step → detail fills console. Spatial compression above/below. Children shown. | "Press Enter on a route step. Its desire, children, reality expand. Everything else compresses." |
| V8 | Peek + signals | N6, N7, N11d, U21-U23 | Space for inline peek. Child deviance annotations. Time-amplified overdue signals. | "Press Space — children + reality appear inline. Overdue step's signal intensifies over days." |
| V9 | Orient zoom | N11c, N20, U24-U28 | Shift+Enter shows parent context, siblings, grandchild counts. Route compresses. | "Shift+Enter — parent desire/reality frame the view. Siblings listed. Grandchild counts visible." |

### Slice Details

**V1: Deck skeleton with column layout**

| # | Affordance | Control | Wires Out | Returns To |
|---|------------|---------|-----------|------------|
| U1 | Parent breadcrumb | display | — | — |
| U2 | Log indicator | display | — | — |
| U3 | Desire text | display | — | — |
| U4 | Desire deadline (left col) | display | — | — |
| U5 | Desire age (right col) | display | — | — |
| U6 | Desire rule | display | — | — |
| U16 | Reality text | display | — | — |
| U17 | Reality age (right col) | display | — | — |
| U18 | Reality rule | display | — | — |
| U19 | Cursor highlight | display | — | — |
| U40 | Hint line | display | — | — |
| N1 | load_tension | call | → S1 | — |
| N8 | compute_columns | call | → S9 | → all U |
| N10 | render_deck (skeleton) | call | → U1-U6, U16-U19 | — |

*Demo: Deck opens showing desire (with deadline left, age right), reality (with age), breadcrumb, log indicator. Columns aligned. Cursor visible on a placeholder element between desire and reality.*

---

**V2: Frontier computation + console**

| # | Affordance | Control | Wires Out | Returns To |
|---|------------|---------|-----------|------------|
| U7 | Route step line | display | — | — |
| U8 | Route compressed indicator | display | — | — |
| U9 | Console boundary top | display | — | — |
| U10 | Overdue step line | display | — | — |
| U11 | Next committed step line | display | — | — |
| U12 | Held indicator | display | — | — |
| U13 | Input point | display | — | — |
| U14 | Accumulated indicator | display | — | — |
| U15 | Console boundary bottom | display | — | — |
| U20 | Selected step text wrap | display | — | — |
| N2 | load_children | call | → S2 | — |
| N3 | load_mutations | call | → S8 | — |
| N4 | compute_frontier | call | → S3 | → U7-U15 |
| N5 | compute_epoch (stub) | call | → S7 | → N4 |
| N11 | handle_pitch | call | → S4 | → U19 |

*Demo: Children appear in correct zones — route above, overdue/next in console, held and accumulated as indicators. Cursor pitches through route and console with j/k. Selected step text wraps.*

---

**V3: Roll navigation + child deck**

| # | Affordance | Control | Wires Out | Returns To |
|---|------------|---------|-----------|------------|
| N11b | handle_roll | call | → N1 | → P5, P1 |

*Demo: Press l on a step with children — its deck loads as a new context. Desire/reality are that child's. Press h — return to parent. Breadcrumb updates to show path.*

---

**V4: Gestures + INPUT state**

| # | Affordance | Control | Wires Out | Returns To |
|---|------------|---------|-----------|------------|
| U35 | Text input field | type | → N12 | — |
| U36 | Input prompt/label | display | — | — |
| U37 | Dimmed deck background | display | — | — |
| N12 | handle_input_submit | call | → N15 | — |
| N13 | initiate_gesture | call | — | → P3 |
| N15 | apply_mutation | call | → S1, S2 | — |

*Demo: Press 'a' — input overlay appears. Type step name, confirm. Child appears in console. Press '?' — reality editor opens. Press '!' — desire editor opens. Press '※' — note input. All from console home position.*

---

**V5: Epoch mechanics**

| # | Affordance | Control | Wires Out | Returns To |
|---|------------|---------|-----------|------------|
| N5 | compute_epoch (full) | call | → S7 | → N4 |
| N16 | check_epoch_close | call | → S7, → N4 | — |

*Demo: Resolve two steps — accumulated indicator shows `✓ 2 resolved`. Edit reality — epoch closes, indicators vanish (fresh epoch). Resolve another step — indicator reappears as `✓ 1 resolved`. Edit desire — epoch closes again, reality marked stale.*

---

**V6: Compression engine + config**

| # | Affordance | Control | Wires Out | Returns To |
|---|------------|---------|-----------|------------|
| N9 | compute_compression | call | → S10 | → N10 |
| N17 | read_config | call | → S6 | → N9, N10 |

*Demo: Shrink terminal height — route compresses to `▲ 4 remaining · next Mar 24`. Expand — route items reappear. Run `werk config deck.chrome structured` — console boundaries always visible, trunk line appears. Run `deck.chrome quiet` — whitespace only.*

---

**V7: Focus zoom**

| # | Affordance | Control | Wires Out | Returns To |
|---|------------|---------|-----------|------------|
| U29 | Focused element heading | display | — | — |
| U30 | Focused element desire | display | — | — |
| U31 | Focused element children | display | — | — |
| U32 | Focused element reality | display | — | — |
| U33 | Top compressed indicator | display | — | — |
| U34 | Bottom compressed indicator | display | — | — |
| N11c | handle_zoom (focus) | call | → S5 | → P2 |
| N18 | load_focused_detail | call | → S11 | → U29-U32 |
| N19 | compute_focus_compression | call | — | → U33, U34 |

*Demo: Navigate to a route step, press Enter. Its desire, children (individual lines), and reality expand in the console. Route items above compress to `▲ 3 more in route`. Console items below compress to `▼ 1 overdue · ▸ next · 3 held`. Esc returns to normal.*

---

**V8: Peek + signals**

| # | Affordance | Control | Wires Out | Returns To |
|---|------------|---------|-----------|------------|
| U21 | Deviance annotation | display | — | — |
| U22 | Children indicator (→N) | display | — | — |
| U23 | Inline peek (children + reality) | display | — | — |
| N6 | compute_child_signals | call | — | → U21 |
| N7 | compute_overdue_intensity | call | — | → U10 |
| N11d | handle_peek | call | → N2 | → U23 |

*Demo: Press Space on a step with →3 — its three children and reality text appear inline below it. Move cursor — peek closes. A child whose desire changed shows annotation `desire changed 2d`. An overdue step that's been overdue 7 days shows brighter signal than one overdue 1 day.*

---

**V9: Orient zoom**

| # | Affordance | Control | Wires Out | Returns To |
|---|------------|---------|-----------|------------|
| U24 | Parent desire (full) | display | — | — |
| U25 | Parent reality (full) | display | — | — |
| U26 | Siblings line | display | — | — |
| U27 | Grandchild count | display | — | — |
| U28 | Ordinals | display | — | — |
| N11c | handle_zoom (orient) | call | → S5 | — |
| N20 | load_orient_context | call | → S12 | → U24-U28 |

*Demo: Press Shift+Enter — parent desire frames the top, parent reality frames the bottom. Siblings listed. Console items show grandchild counts `(2 sub)`. Route compresses to summary. Ordinals appear in left column. Shift+Enter again returns to normal.*

---

### Dependencies

```
V1 (skeleton) → V2 (frontier) → V3 (roll) → V4 (gestures)
                                                    ↓
                                              V5 (epochs) → V6 (compression)
                                                                    ↓
                                                              V7 (focus) → V8 (peek+signals)
                                                                                    ↓
                                                                              V9 (orient)
```

V1-V4 are the core loop: see, navigate, act. V5 adds epoch awareness. V6 makes it adaptive. V7-V9 add the zoom levels and signal depth.
