# TUI Stream View Rebuild — Big Picture

**Selected shape:** D (Adaptive Hybrid + Configuration Layer)
**Tension:** #15 TUI rebuilt around the operating envelope as primary interaction surface
**Documents:** [Shaping](./tui-shaping.md) · [Breadboard](./tui-breadboard.md) · [Foundation](./werk-conceptual-foundation.md)

---

## Frame

### Problem

- Current TUI is a flat field chart with gaze cards — none of the conceptual architecture (envelope centering, frontier computation, epochs, zoom levels) is implemented
- No distinction between route (future theory) and console (action-relevant frontier) — all children render the same way
- No epoch awareness — resolved/released steps and notes don't accumulate as compression pressure; no "fresh epoch" signal
- Navigation is a flat list cursor — no zone-aware pitch, no zoom levels, no peek
- Visual treatment is hardcoded — no configuration, no adaptive chrome
- The instrument doesn't breathe with its content

### Outcome

- The deck is the primary interaction surface, centered on the console (frontier of action)
- Children are classified into zones: route (future), console (overdue, next, held, accumulated), and the user sees the structural distinction
- Epoch mechanics govern what's in the console — reality/desire updates close epochs, fresh epochs start clean
- Three zoom levels (orient/normal/focus) work as focal length adjustments
- Visual treatment adapts to content (chrome appears under load) and is configurable
- The user can see, navigate, and act on their structure in a way that aligns with the conceptual foundation

---

## Shape

### Fit Check (R × D)

| Req | Requirement | Status | D |
|-----|-------------|--------|---|
| R0 | Deck is primary interaction surface, console center, lands here on open | Core goal | ✅ |
| R1 | 4-zone layout: desire, route, console, reality | Core goal | ✅ |
| R2 | Console contains epoch action-relevant items (overdue, next, held, accumulated) | Core goal | ✅ |
| R3 | Console aggregates child signals — locality, one level | Must-have | ✅ |
| R4 | Pitch/roll/zoom navigation (j/k, h/l, Enter/Shift+Enter) | Must-have | ✅ |
| R5 | Left=intent, right=trace, column layout | Must-have | ✅ |
| R6 | Signal by exception — silence default, deviations pop | Must-have | ✅ |
| R7 | Epoch mechanics — desire/reality/resolve/release close epochs | Must-have | ✅ |
| R8 | Experiential states: NORMAL, INPUT, FOCUSED, PATHWAY | Must-have | ✅ |
| R9 | Intelligent compression — design for comfort, degrade gracefully | Must-have | ✅ |

### Parts

| Part | Mechanism | Flag |
|------|-----------|:----:|
| **C1** | Separators appear contextually — console boundaries visible when 2+ content zones | |
| **C2** | Monochrome + one accent (cyan default) for cursor, OVERDUE, console boundaries | |
| **C3** | Route compresses first, bookends (first/last) persist longest for trajectory shape | |
| **C4** | Hybrid peek — Space for inline children+reality, Enter for full focus | |
| **C5** | Time-amplified signals — text annotation intensity grows with duration | |
| **C6** | Ordinals at orient zoom, hidden at normal | |
| **D7** | Configuration layer — 10 settings via `deck.*` config keys, sacred architecture invariant | |

### Vocabulary

| Term | Meaning |
|------|---------|
| **Deck** | The full view — flight deck working surface |
| **Console** | Action zone at center — frontier of action as interaction surface |
| **Route** | Remaining theory — positioned steps in order of operations, above the console |
| **Log** | One tension's epoch sequence — linear history |
| **Logbook** | Composite lattice of all logs — the queryable whole |
| **Epoch** | Period of action within a stable desire-reality frame — closed by desire/reality/resolve/release |

### Spatial Layout (Normal Zoom)

```
  ← #N parent breadcrumb...                             [dim]   ← screen boundary

  [deadline]  desire text                                  [age] ← top anchor
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  [deadline]  route step text                    [#ID] [→N] [age] ← route zone
  [deadline]  route step text                    [#ID] [→N] [age]
  [deadline]  route step text                    [#ID] [→N] [age]

  ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄   ← console boundary (adaptive)
  [deadline]  ▸ overdue step text         OVERDUE  [#ID] [→N]    ← console zone
  [deadline]  ▸ next step text            ← here   [#ID] [→N] [age]
              · N held
              + ___
              ✓ N resolved · ~ N released · ※ N note
  ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄   ← console boundary (adaptive)

              reality text                                 [age] ← bottom anchor
  ──────────────────────────────────────────────────────────────
  ↓ N prior epochs                                       [dim]   ← screen boundary
```

### Zoom Principle

**Zoom is focal length — center sharpens or blurs relative to edges.**

| Zoom | Center | Edges | Trigger |
|------|--------|-------|---------|
| Orient | Compresses — console/route lose detail | Gain weight — parent, siblings, grandchild signals | Shift+Enter |
| Normal | Balanced — route as list, console compact | Desire/reality anchors, boundary signals subtle | Default / Esc from focus |
| Focus | Expands — one element's full detail in console | Compress — everything else becomes indicators | Enter on element |

### Navigation

| Axis | Keys | Movement |
|------|------|----------|
| Pitch | j/↓, k/↑ | Through order of operations. Cursor starts at console, radiates outward. |
| Roll | l/→ | Descend into selected step (new deck) |
| Roll | h/← | Ascend to parent (previous deck) |
| Zoom | Enter | Focus on selected element |
| Zoom | Shift+Enter | Toggle orient |
| Peek | Space | Inline children + reality below selected step |
| Quick edit | ! | Edit desire from console position |
| Quick edit | ? | Edit reality from console position |

### Experiential States

| State | Feel | Available | Entry | Exit |
|-------|------|-----------|-------|------|
| NORMAL | Navigating the deck at rest | Pitch, roll, zoom, peek, initiate gestures | Default / return from other states | Gesture initiation |
| INPUT | Writing — text field active, deck dimmed | Type, confirm, cancel | 'a' (add), 'e' (edit), '!' (desire), '?' (reality), '※' (note) | Enter (confirm) / Esc (cancel) |
| FOCUSED | One element expanded — its detail fills console | Mutations on focused element, zoom out, navigate within | Enter on element | Esc / Shift+Enter |
| PATHWAY | Decision fork — 3-5 options inline | Select, confirm, dismiss | Gesture produces structural signal | Enter (select) / Esc (dismiss) |

---

## Slices

### Dependency Graph

```
V1 (skeleton) → V2 (frontier) → V3 (roll) → V4 (gestures)
                                                    ↓
                                              V5 (epochs) → V6 (compression)
                                                                    ↓
                                                              V7 (focus) → V8 (peek+signals)
                                                                                    ↓
                                                                              V9 (orient)
```

### Slices Grid

|  |  |  |
|:--|:--|:--|
| **V1: DECK SKELETON**<br>⏳ PENDING<br><br>• Desire/reality anchors<br>• Column layout engine<br>• Parent breadcrumb<br>• Log indicator<br>• Cursor highlight<br><br>*Demo: Deck opens with aligned columns, desire top, reality bottom* | **V2: FRONTIER + CONSOLE**<br>⏳ PENDING<br><br>• Frontier computation<br>• Route step lines<br>• Console zones (overdue, next, held, accumulated)<br>• Zone-aware pitch (j/k)<br><br>*Demo: Children classified into zones, cursor pitches through route and console* | **V3: ROLL NAVIGATION**<br>⏳ PENDING<br><br>• Descend into child (l)<br>• Ascend to parent (h)<br>• Breadcrumb updates<br>• Child deck loads as new context<br><br>*Demo: l descends, h ascends, breadcrumb tracks path* |
| **V4: GESTURES + INPUT**<br>⏳ PENDING<br><br>• Create child (a)<br>• Edit desire (!), reality (?)<br>• Add note (※)<br>• Text input with dimmed deck<br>• Quick-edit from console<br><br>*Demo: Press 'a', type step, confirm — child appears. Press '?' — edit reality* | **V5: EPOCH MECHANICS**<br>⏳ PENDING<br><br>• Epoch boundary detection<br>• Reality/desire update closes epoch<br>• Accumulated indicators clear<br>• Fresh epoch = no signals<br>• Reality stale after desire change<br><br>*Demo: Edit reality — indicators vanish. Resolve step — indicator reappears* | **V6: COMPRESSION + CONFIG**<br>⏳ PENDING<br><br>• Compression engine<br>• Route-first compression<br>• Adaptive chrome (C1)<br>• Config reader (deck.*)<br>• 10 configurable settings<br><br>*Demo: Shrink terminal — route compresses. Change deck.chrome — separators appear* |
| **V7: FOCUS ZOOM**<br>⏳ PENDING<br><br>• Enter on element → detail view<br>• Desire + children + reality expanded<br>• Spatial compression (top/bottom)<br>• Esc to return<br>• • &nbsp;<br><br>*Demo: Enter on route step — full detail expands, everything else compresses* | **V8: PEEK + SIGNALS**<br>⏳ PENDING<br><br>• Space for inline peek<br>• Children + reality preview<br>• Child deviance annotations<br>• Time-amplified overdue signals<br>• →N children indicator<br><br>*Demo: Space — children appear inline. Overdue signal intensifies over days* | **V9: ORIENT ZOOM**<br>⏳ PENDING<br><br>• Shift+Enter for orient<br>• Parent desire/reality frame<br>• Siblings line<br>• Grandchild counts (N sub)<br>• Ordinals in left column<br><br>*Demo: Shift+Enter — parent frames the view, siblings listed, grandchild counts visible* |

---

## Implementation Notes

### Key New Code

| Component | What | Replaces |
|-----------|------|----------|
| `frontier.rs` | Classify children into route/overdue/next/held/accumulated based on epoch + deadline + position | Flat sibling sorting in `load_siblings()` |
| `deck.rs` (state) | Zone-aware cursor, zoom level, focused element, peek state, render plan | `InstrumentApp` fields, `VirtualList`, `GazeState` |
| `deck.rs` (render) | Column-based 4-zone rendering with adaptive chrome | `render_field()` element-based rendering |
| `compression.rs` | Constraint hierarchy: available space × zoom × config → render plan | (does not exist) |
| `epoch.rs` | Epoch boundary detection, accumulation tracking, clean close | (does not exist — depends on #36) |
| `config.rs` | `deck.*` settings reader/writer | (does not exist) |

### Dependencies on Other Tensions

| Tension | Dependency | Impact |
|---------|-----------|--------|
| #36 (epoch lifecycle) | V5 needs epoch boundary detection in the data model | V2 stubs epoch computation (everything = current epoch). V5 fills in real logic when #36 is ready. |
| #16 (state machine spec) | V4 needs gesture bindings, V7-V9 need state transitions | Shape D defines the states and their feel. #16 specifies the full transition table. Implementation can proceed with the key bindings defined here. |
| #47 (threshold detection) | Threshold mechanics (#19) are out of scope | No dependency for V1-V9. Thresholds layer on top of the deck later. |

### Epoch Stub Strategy

V2 introduces frontier computation with an epoch stub: `compute_epoch()` returns "everything since tension creation is the current epoch." This means all resolved/released/notes appear in the accumulated indicator. Real epoch boundaries (V5) narrow this to "since last desire/reality change." The frontier computation API doesn't change — only the epoch boundary input changes.
