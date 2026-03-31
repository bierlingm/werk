# The Reimagined Instrument — An Imaginal Sketch

**Date:** 2026-03-31
**Status:** Imaginal. Not a spec. A vision of what the terminal instrument becomes when every ftui capability is composed in service of the practice.

---

## Prologue: What Changes

The current TUI is 7,884 lines of hand-rolled rendering inside an ftui Program shell. It draws lines. It manages cursors with index arithmetic. It switches between input modes by setting an enum. It works — and it taught us what the instrument needs to be.

This sketch starts from the other end: what does the practitioner experience when the instrument fully inhabits its own technology? Not "what widgets do we use" but "what does it feel like to step into the structure, navigate it, act on it, and leave?" The widgets are named because they are real and available. The composition is imagined because it has not been built.

The sacred core is absolute. Desired above actual. Signal by exception. Gesture as unit of change. Locality. Standard of measurement. Structure determines behavior. These are not honored by mention — they are honored by how the instrument behaves when you open it.

---

## I. The Spatial Model

### The Pane System as Inhabited Space

The screen is not a layout. It is a space the practitioner steps into.

**PaneLayout** — ftui's multi-pane system with drag-to-resize, inertial physics, snap points, and magnetic fields — becomes the skeleton of this space. Not as a tiling window manager. As a breathing structure with three zones that relate to each other the way desire, action, and reality relate:

```
┌─────────────────────────────────────────────────────────┐
│                    DESIRE ANCHOR                        │  ← pane boundary (drag-resizable)
├─────────────────────────────────────────────────────────┤
│                                                         │
│                                                         │
│                    THE FIELD                             │
│                                                         │
│                                                         │
├─────────────────────────────────────────────────────────┤
│                    REALITY ANCHOR                        │  ← pane boundary (drag-resizable)
└─────────────────────────────────────────────────────────┘
```

Three horizontal panes. The vertical axis is the one spatial law made literal: desire at top, reality at bottom, the field of action between them. PaneLayout's snap points ensure the middle field always claims at least 60% of the terminal height. Magnetic fields resist collapsing the anchors below their minimum (1 line for desire, 1 line for reality). The practitioner can drag the boundary between desire and field to reveal more of a long desire statement, or between field and reality to show the full reality text. The pane proportions persist across sessions via **Workspace** snapshots.

The desire anchor is not merely text. It is a **Paragraph** widget with styled wrapping, showing the desire statement with deadline on the left and age on the right. When the desire is short, the pane collapses to one line and the field expands. When the desire is long, the pane holds what's needed. The one spatial law is not just vertical ordering — it is the desire pressing down from above, the reality pressing up from below, and the field of action occupying the space between.

The reality anchor mirrors this at the bottom. Same Paragraph, same adaptive sizing. Between them: the field.

### The Field

The field is the middle pane — the primary interaction surface. It contains the operating envelope and everything around it. Here is where the theory of closure lives.

The field uses **Flex** layout internally:

```
┌─ THE FIELD ─────────────────────────────────────────────┐
│  route zone        (Flex: FitContent, compresses first) │
│  ┄┄ console crown ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  │
│  console zone      (Flex: Min(5), sticky action center) │
│  ┄┄ console footer ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  │
│  accumulated zone  (Flex: FitContent, gravity to bottom) │
└─────────────────────────────────────────────────────────┘
```

The route zone uses `FitContent` — it takes exactly what it needs and no more. Under height pressure, it compresses first (the focal length principle: edges blur before center). The console zone has a `Min(5)` constraint — the helm, the command well, and the crown never disappear. The accumulated zone uses `FitContent` with bottom gravity — resolved items settle toward reality.

The Flex solver handles this automatically. No hand-rolled compression arithmetic. The constraint hierarchy IS the compression hierarchy: route's FitContent yields before console's Min, which yields before the desire/reality anchors. ftui's 8 constraint types express the entire compression logic of the instrument in declarative form.

### Focus Graph: Spatial Navigation Made Structural

The current TUI navigates with index arithmetic — `cursor += 1`, bounds checking, mode-dependent behavior. The reimagined instrument uses ftui's **Focus system**: a directed graph where focus moves spatially (Up/Down/Left/Right) through semantically meaningful nodes.

The focus graph mirrors the spatial law:

```
[desire anchor]
      ↕
[route item 1] ↔ [route item 2] ↔ ... (horizontal: siblings at same depth)
      ↕
[console: overdue 1]
      ↕
[console: next step]  → [next step's children peek]
      ↕
[console: held tray]
      ↕
[console: input point]
      ↕
[accumulated 1]
      ↕
[reality anchor]
```

Pitch (Up/Down) is focus movement along the vertical axis of the graph. Roll (Left/Right entering a child) is replacing the current focus graph with the child's focus graph — a structural transition, not a cursor increment. Zoom (Enter) activates a **focus trap** on the selected node, narrowing the navigable graph to the focused element and its immediate affordances.

**Focus groups** organize the console: overdue items form one group, held items another. Within a group, Tab cycles. Between groups, Up/Down traverses. The focus system handles all of this — no hand-rolled zone-aware cursor logic.

**Focus history** with `back()` makes roll-left (ascend to parent) a single call. The navigation stack is the focus stack. Breadcrumbs are computed from focus history, not maintained as separate state.

### Screen Boundaries as Signal Space

The edges of the three-pane layout are not dead chrome. They are peripheral vision.

Above the desire anchor: a one-line **StatusLine** showing the parent breadcrumb. Dim. The `h/left` hint. At root level: empty — the absence of a parent is itself structural information (you are at a root tension, a coherence generator).

Below the reality anchor: a one-line StatusLine showing the log indicator and session info. `N prior epochs` with a down-arrow hint. The session gesture count. The help key.

Left edge of the field: a gutter. In normal zoom, empty. In orient zoom, ordinals appear. If a route item has a signal (sequencing pressure, containment violation), a small glyph appears in the gutter — peripheral, not focused. The gutter is 2 characters wide and uses **Responsive** visibility: hidden below 60 columns, visible above.

Right edge of the field: the trace column. IDs, children indicators, ages. Right-aligned via **Columns** helper. Content adapts to width — at narrow terminals, IDs drop first, then ages, leaving only the children indicator. **Responsive** breakpoints determine this: `>120 cols` shows everything, `80-120` drops ages, `<80` shows only IDs.

---

## II. The Interaction Model

### Gestures as Undoable Transactions

The current TUI has no undo. Every gesture is permanent on commit. The reimagined instrument uses ftui's **Undo** system with transactions.

Every operative gesture — resolve, release, update reality, update desire, add child, reposition, note — is wrapped in an undo transaction. The transaction groups all mutations under one gesture_id. `Ctrl+Z` undoes the last gesture. `Ctrl+Shift+Z` redoes. The undo stack is displayed nowhere by default (signal by exception — the affordance exists silently). But when undo is invoked, a **Toast** appears briefly: "Undone: resolved #42" with an action button to redo.

Undo changes the vocabulary of interaction. The practitioner can try a restructuring — reorder three steps, release one, add two new ones — and if the result does not feel right, undo the entire gesture. This makes gestures explorative as well as committal. The theory of closure becomes something you can draft and revise within a single session.

The **Undo** system's snapshot store means the full state before a gesture is preserved. This interacts with epoch mechanics: if a gesture closes an epoch (reality update), undoing it reopens the epoch. The instrument's structural integrity is maintained through the undo boundary, not despite it.

### The Command Palette as Unified Surface

The current TUI dispatches gestures through single-key bindings in a mode-dependent state machine. This works for practiced users but walls off the instrument's vocabulary from discovery.

The reimagined instrument adds **CommandPalette** as a universal entry point. Press `:` (colon, the command prefix) or `/` (search prefix) or `Ctrl+K` and the palette appears — a **Modal** overlay with Bayesian fuzzy search, ranked results, and keyboard-driven selection.

The palette unifies three functions:

1. **Command dispatch.** Type "resolve" or "res" or even "done" and the palette ranks matching gestures. Bayesian scoring learns from the practitioner's usage patterns — if you always resolve via the palette, "r" starts matching "resolve" more strongly than "release." The palette shows key bindings alongside each result: `resolve [r]`, `release [~]`, so the practitioner learns the direct keys through the palette's own affordance.

2. **Search.** Type a tension name, a short code, a phrase from a desire statement. **FrankenSearch** (the hybrid BM25 + semantic search already in the codebase) feeds results directly into the palette. Select a result and the instrument navigates there — roll into the tension, set focus on it. Search and navigation become one gesture.

3. **Quick navigation.** Type `#42` to jump to tension 42. Type `..` to ascend to parent. Type `/overdue` to filter the current view. The palette is the universal "I know what I want" surface.

The palette appears as a Modal with **backdrop dimming** (the field dims behind it) and **focus trapping** (keyboard input goes only to the palette until dismissed). Animation: slide-down from the top of the field, scale-fade on appearance. Dismissal: Esc, or selecting a result.

The palette's HintRanker uses Bayesian posteriors to learn which commands the practitioner uses most, in which contexts. Over time, the palette becomes a personalized instrument — showing what this particular practitioner reaches for, not a generic list.

### Focus-Trapped Modals Replace Mode Switching

The current TUI has 10 input modes (Normal, Adding, Editing, Annotating, Confirming, Moving, Searching, Help, Pathway, Reordering). Each mode changes key bindings globally. Mode transitions are a source of confusion: "am I in editing mode? Why doesn't `j` work?"

The reimagined instrument replaces mode switching with **Modal** overlays and focus traps:

- **Adding a child:** A Modal slides up from the input point containing a **TextInput** (grapheme-aware, word operations, undo within the input). The field dims behind it. Focus is trapped in the modal. Enter commits, Esc cancels. The practitioner never loses spatial context — the theory of closure is visible behind the dim overlay.

- **Editing desire/reality:** A Modal containing a **TextArea** (multi-line editor with selection, wrapping, scrolling) appears over the anchor zone being edited. The TextArea is pre-filled with the current text. Full editing capabilities: select words, undo within the editor, scroll long text. The Modal has confirm/cancel buttons visible at the bottom.

- **Confirming a destructive gesture** (release, resolve with children): A Modal with `dialog::confirm` preset — a focused question with two buttons. "Release #42 and its 3 children? [Confirm] [Cancel]". The Modal's animation (scale-fade) gives the moment weight.

- **Pathway palettes:** A Modal with a **List** of 3-5 options, each explained in one line. Arrow keys to select, Enter to choose, Esc to dismiss. The pathway context ("child deadline exceeds parent") is shown above the list. This replaces the current PATHWAY mode with a self-contained decision surface that traps focus until resolved.

- **Reordering:** This is where **keyboard drag** transforms the experience. Select a step, press a reorder key (e.g., `m` for move), and the step enters keyboard-drag mode. Arrow keys move it through the order of operations. Drop positions (Before/Inside/After) are shown as visual indicators between steps. The **drag-and-drop** system's typed payload carries the step's identity and structural constraints (it cannot be dropped outside its parent). Press Enter to confirm the new position, Esc to cancel. A custom drag preview shows the step's text following the cursor position.

Every one of these interactions is a focused, self-contained surface that exists on top of the instrument's spatial structure rather than replacing it. The practitioner always knows where they are because the field is always visible behind the interaction.

### Reactive State

The current TUI reloads data after every mutation. The reimagined instrument uses ftui's **Reactive** system:

- `Observable<Tension>` wraps the current tension. When it changes (after a gesture), all dependent computations re-derive automatically.
- `Computed<Frontier>` derives the frontier classification from the observable tension and its children. No manual recomputation calls.
- `Computed<RenderPlan>` derives the layout plan from the frontier, zoom level, and terminal dimensions. When any input changes, the plan updates.
- `TwoWayBinding` connects the TextInput in a modal to the field it will mutate on commit. The practitioner sees their edits reflected in real-time preview (desire text updates as they type, before commit).
- `batch_scope` groups multiple reactive updates (from a single gesture that produces multiple mutations) into one render cycle. No intermediate flicker.

Reactive state eliminates an entire category of bugs: stale views after mutations, inconsistent state between frontier and display, render plans that don't match the data. The framework guarantees consistency.

---

## III. The Signal Model

### Silence as Default State

Open the instrument to a healthy tension — one with an active theory of closure, a recent reality update, no overdue steps, no containment violations. What do you see?

Desire at top. Route items in monochrome. The console showing the next step with a forward glyph. The input point resting. Reality at bottom. No color except the cursor highlight. No badges. No sparklines. No toasts. Silence.

This is the instrument at rest. The absence of signal IS the signal: everything is on track. The practitioner's eye finds nothing demanding attention, which means attention is free to choose where to go. The field is a calm surface.

### Exception Surfaces

When something deviates, signals appear at the point of deviation, using the minimum visual weight needed:

**Time-amplified overdue** — A step past its deadline gains visual weight proportional to how far past. Just overdue: amber text. Days overdue: bold amber. Severely overdue: bright amber, bold, with an `OVERDUE` **Badge** widget that pulses once on first appearance (via Badge's built-in styling) then settles. The Badge is a compact status pill — exactly what ftui's Badge widget is for. The escalation is continuous, not stepped: **AdaptiveColor** computes the amber intensity from the urgency value, so the color itself encodes time.

**Containment violation** — When a child's deadline exceeds its parent's, a small `!` glyph appears in the left gutter next to that child. Dim. Not shouting. The glyph is rendered as a **Badge** with warning style. On focus (cursor lands on that child), the violation becomes a full sentence in the console crown: "deadline Mar 30 exceeds parent deadline Mar 15". If the practitioner presses Enter, the pathway palette modal appears with resolution options.

**Sequencing pressure** — A `~` in the gutter when order conflicts with deadline ordering. Same principle: gutter glyph at rest, full explanation on focus.

**Stale reality** — After desire changes, reality is marked stale until updated. The reality anchor's age annotation shifts from dim to amber. The reality text itself does not change appearance (it is still the truth of what was last articulated). Only the temporal annotation signals that the frame has shifted and reality may no longer be current.

**Epoch freshness** — The console crown shows epoch age. A **Badge** with adaptive color: green if fresh (reality updated recently), dim if normal, amber if stale (long time since last reality grounding). This single indicator tells the practitioner whether their operating frame is current without requiring them to check dates.

**Sparklines for trajectory** — The right edge of the desire anchor can show a tiny **Sparkline** (9-level Unicode blocks, gradient) encoding the recent gap trend. Rising blocks: gap is growing (desire is pulling away from reality). Falling blocks: gap is closing (reality is catching up). Flat: stable. The sparkline is 8 characters wide. It appears only when there are enough data points (at least 4 gap samples), and only when the trend is non-trivial. Otherwise: silence. The sparkline is rendered via ftui's Sparkline widget with AdaptiveColor gradient — green when closing, amber when growing.

### Toast Notifications for Gesture Feedback

The current TUI shows a transient message in the status bar after a gesture. It competes for space and disappears without the practitioner noticing.

The reimagined instrument uses **Toast** notifications:

- After a gesture completes: a Toast slides in from the bottom-right corner. "Resolved #42" with a subtle check icon. Auto-dismisses after 3 seconds. Position: bottom-right, outside the field, in the reality anchor area or below it.
- After undo: "Undone: resolved #42" with an action button: `[Redo]`. The action button is a real Toast affordance — pressing it triggers redo.
- After a structural signal: "Containment: #42 deadline exceeds #15" as a warning Toast, amber, with an action button: `[Resolve]` which opens the pathway palette.
- **NotificationQueue** manages stacking. If multiple gestures happen quickly (batch resolve), toasts stack with the most recent on top. Maximum 3 visible; older ones fade with exit animation.

Toasts are never the primary signal channel. They confirm that a gesture landed. The primary signal is always the structural change itself — the step moving from route to accumulated, the reality text updating, the frontier recomputing. Toasts are the instrument acknowledging "I heard you."

### The Color Philosophy

**AdaptiveColor** is ftui's system for computing colors that work in both dark and light terminals. The reimagined instrument uses it throughout:

- **Monochrome + one accent.** The field is rendered in terminal foreground color (respects the user's terminal theme). One accent color — cyan by default, configurable — marks the cursor, the console boundaries, and focused elements.
- **Exception colors computed from data.** Amber is not a style constant. It is computed from urgency values via AdaptiveColor, producing a continuous spectrum from "barely past deadline" (faint amber) to "severely overdue" (saturated amber). Green for freshness, computed from epoch age. These are the only semantic colors.
- **No color in the default state.** A healthy instrument renders in monochrome + accent. Color appears only as exception signal. This makes color itself a signal — when the practitioner sees amber, it means something, because amber is never decorative.

---

## IV. The Temporal Model

### Time in the Field

The left column of the field is the temporal spine. Deadlines appear as text annotations, positioned alongside their steps. The spine uses **Columns** layout with a fixed left width accommodating the longest deadline visible.

But time is not just text labels. Time manifests through several visual channels simultaneously:

**Vertical position = order of operations (taxis).** The route items are arranged in the order the practitioner committed to. This is structural time — the sequence of intentions.

**Deadline annotations = calendar constraint (chronos).** The left column shows when. Month + day for near deadlines, month only for distant ones, year + month for next-year horizons. **Responsive** to terminal width: at narrow terminals, deadlines abbreviate.

**The console = readiness (kairos).** What is action-relevant right now, independent of calendar or sequence. The frontier surfaces kairos — the right moment to act. The console crown's readout summarizes the kairos situation: how many are overdue, what is next, how fresh is the epoch.

These three orientations coexist. The practitioner sees all three without switching views. This is one of the reimagined instrument's key advances over the current TUI, where time is annotated but not structurally expressed.

### The Survey View Reimagined

The survey view — the Napoleonic field scan — is currently a flat list grouped by time bands. The reimagined survey exploits **VirtualizedList** and **Tree** to become a genuine temporal map.

**VirtualizedList** with Fenwick tree indexing handles the full field of tensions — potentially hundreds of steps across all roots — at O(log n). Variable-height items (some steps have signals, badges, sparklines; most are single-line) are handled natively. Overscan ensures smooth scrolling.

The survey's primary axis is time. But instead of flat bands (Overdue / This Week / This Month / Later), the survey uses a **Tree** widget where the top-level nodes are temporal frames and the children are steps within those frames:

```
▾ Overdue (3)
    · threshold detection implemented              #46  ← #15   5d overdue
    · API contract review                          #31  ← #08   2d overdue
    · weekly reflection                            #72  ← #03   1d overdue
▾ This Week (7)
    · finalize console redesign                    #15  ← #02   due Mar 30
    · ...
▸ Next 2 Weeks (12)
▸ This Month (8)
▸ Later (23)
  No Deadline (5)
```

Tree's lazy expansion means the practitioner opens only the temporal frames they care about. Guide styles (ftui's 5 options) create visual hierarchy between frames. The tree is navigable with pitch (up/down through items) and the frame can be narrowed or widened with `[` and `]` — which collapses or expands tree depth.

Each step in the survey shows its parent tension as a dim annotation on the right (`← #15`), providing structural context within the temporal view. The yaw toggle (Tab) carries selection: if you are looking at #46 in the survey and press Tab, you land in #15's deck with #46 focused.

**Follow mode** on VirtualizedList means the survey can auto-scroll to keep the most urgent item visible, even as the field updates (after a gesture, after time passes and a deadline crosses).

### Temporal Sparklines in Orient Zoom

When the practitioner enters orient zoom (Shift+Enter), the parent tension's structure wraps the current view. In this wider context, each sibling tension can show a **Sparkline** encoding its recent activity pattern: dense blocks for active periods, sparse blocks for quiet ones. This is a temporal signature — the shape of engagement over time. The practitioner sees, at a glance, which siblings are being actively worked and which have gone quiet. No labels, no numbers. Just the shape.

---

## V. The Adaptive Model

### Responsive Layout

The instrument must work in an 80x24 terminal and a 200x60 terminal. These are not the same instrument — they are the same structure rendered at different focal lengths.

**ResponsiveLayout** (ftui's multi-layout selector by terminal size) defines three breakpoint regimes:

- **Compact** (< 80 cols or < 24 rows): Desire and reality collapse to one-line summaries. Route compresses to a count. Console shows only helm + input. Gutter hidden. Right column shows only IDs. The instrument is usable but sparse.

- **Standard** (80-120 cols, 24-40 rows): The full three-pane layout. Route items visible. Console with crown and footer. Gutter visible. Right column with IDs, children, and ages.

- **Expansive** (> 120 cols, > 40 rows): Desire and reality can show multi-line text without scrolling. Route items gain more breathing room. The console shows the full anatomy (crown, warning lane, helm, command well, held tray, footer). A side panel becomes available for peek (Space shows children in a panel to the right of the field, using **Grid** layout to split the field 70/30).

The **Breakpoint** system detects terminal size. The **Resize coalescer** (regime-aware batching) ensures that rapid terminal resizing does not flood the render pipeline — it waits for the resize to settle before committing to a new regime.

### Degradation Under Pressure

ftui's **Bayesian diff** with regime detection and **degradation cascade** handle resource pressure. If rendering takes longer than the frame budget:

1. Sparklines disappear (decorative, not structural).
2. Badges simplify to text (no styling computation).
3. Focus animations disable (instant transitions).
4. Toast animations disable (instant appear/disappear).
5. PaneLayout physics disable (instant resize, no inertia).

The practitioner never sees a slow frame. The instrument degrades gracefully, shedding visual polish while preserving structural information. The **evidence telemetry** system (cross-module metrics fusion) tracks which degradation level is active and whether it is recovering.

### Session Persistence

**Workspace** snapshots save:
- Pane proportions (desire/field/reality split)
- Zoom level
- Focused tension and cursor position
- Survey tree expansion state
- Command palette usage statistics (for Bayesian ranking)

On next launch, the instrument restores exactly where the practitioner left off. The session boundary (closing and reopening the instrument) is a threshold — but it is not a discontinuity. The workspace persists.

**State persistence** (ftui's automatic save/restore) handles this at the framework level. No manual serialization code.

### Dark/Light Terminal Adaptation

**AdaptiveColor** samples the terminal's background luminance (via querying terminal attributes or configuration) and adjusts all computed colors:

- Amber in a dark terminal is warm and visible. Amber in a light terminal shifts to a darker orange that maintains contrast.
- The accent color (cyan by default) adjusts its saturation.
- The monochrome text respects the terminal's native foreground.

The practitioner never configures colors. The instrument adapts to the environment.

---

## VI. The Development Model

### Inspector Overlay

Press a debug key combination (Ctrl+Shift+I) and ftui's **Inspector** appears as an overlay showing the widget tree: every Paragraph, every Flex container, every focus node, every reactive binding. This is the instrument's own X-ray — visible structure of the structure-displaying structure.

The Inspector shows:
- Widget hierarchy with sizes and constraints
- Focus graph with current focus highlighted
- Reactive dependency graph (which Observables feed which Computeds)
- Layout cache hit rates (is the S3-FIFO eviction working?)

The **DebugOverlay** adds constraint visualization — colored borders showing Flex constraints, Min/Max zones, FitContent regions. The practitioner-developer sees exactly why the compression engine made the choices it did.

### Program Simulator for Testing

ftui's **Program simulator** enables deterministic testing of the entire instrument. Inject a sequence of events (key presses, terminal resizes, data changes), run the model through its update cycle, and assert on the resulting view.

This replaces manual testing of TUI flows. A test case:

1. Load a tension with 5 children, 2 overdue.
2. Assert: console crown shows "2 overdue".
3. Inject: Down arrow x3 (focus on first overdue).
4. Assert: overdue Badge visible on focused item.
5. Inject: 'r' (resolve gesture).
6. Assert: Toast appears "Resolved #N". Console crown shows "1 overdue". Frontier recomputed.
7. Inject: Ctrl+Z (undo).
8. Assert: Toast appears "Undone: resolved #N". Console crown shows "2 overdue" again.

Deterministic. Repeatable. No terminal emulator needed. The simulator runs in CI.

### Input Macros for Regression Testing

ftui's **Input macros** (record/replay event sequences) allow recording a real interaction session and replaying it against a test dataset. The practitioner-developer performs a complex flow — navigate to a tension, reorder three steps, update reality, check the survey — and the macro captures every keystroke. Replay it after code changes to verify the flow still works.

Macros compose with the Program simulator: record once interactively, replay deterministically in CI.

### Asciicast for Demos

**Asciicast** session recording captures the terminal output as a castable recording. The practitioner-developer performs a demo flow — "here is how you navigate the theory of closure, resolve a step, and see the epoch close" — and the recording is shareable. Useful for documentation, for teaching, for showing the instrument to potential practitioners.

---

## VII. What Becomes Possible

### Undo for Gestures

This changes the character of the instrument. Currently, every gesture is a one-way door. With undo, gestures become exploratory. "What if I release this tension?" Try it, see the structural result, undo if it does not feel right. The theory of closure becomes a drafting surface, not just a commitment surface. This is structurally significant: the practitioner's relationship to gestures shifts from cautious to experimental. Fritz's insight that "the only satisfying resolution of tension is to create the desired outcome" applies to the instrument itself — the practitioner can try creating the outcome, see if it resolves the tension, and reverse if it does not.

### Canvas Minimap of the Tension Tree

ftui's **Canvas** (braille/half-block drawing, feature-gated) enables a minimap: a compressed 2D rendering of the full tension tree, showing structure (depth, breadth) and signal (overdue nodes as bright dots, stale nodes as dim dots). The minimap lives in orient zoom's expanded edges — a peripheral visualization that shows the whole field in one glance.

The canvas uses half-block characters for 2x vertical resolution. Each tension is a dot. Branches show parent-child relationships as lines. The current tension is highlighted. The minimap is not interactive — it is a map you glance at, not navigate through. It answers: "where am I in the whole structure?"

At expansive terminal sizes (> 160 cols), the minimap could become a persistent side panel — a structural radar always visible in peripheral vision.

### Markdown in Desire/Reality Text

ftui's **Markdown rendering** (feature-gated) means desire and reality statements can contain lightweight formatting: bold for emphasis, lists for structured reality descriptions, code spans for technical references. The TextArea editor supports this input. The Paragraph renderer shows it styled.

This matters because reality statements are often technical and structured. "API endpoint works for GET but not POST. Authentication token expires after 1h instead of 24h. Database migration pending on staging." With markdown rendering, this becomes scannable rather than a wall of text.

### Inline Validation for Input

ftui's **ValidationError** widget and **Validation pipeline** mean that input errors surface as you type, not after commit:

- Creating a child with a deadline beyond the parent's deadline: amber inline validation message appears below the TextInput as you type the deadline. "Deadline exceeds parent #15's deadline (Mar 30)."
- Entering an empty desire: red inline validation. "Desire cannot be empty."
- Setting a deadline in the past: amber message. "Deadline is in the past."

The **ValidationError** widget animates on appearance (slide-down, as specified in ftui), drawing attention without blocking. The practitioner can commit anyway (the pathway palette will offer resolution options) or fix the issue first.

### Stacked Notifications with Action Buttons

Batch gestures — resolve 3 steps, reorder 5, update reality — produce multiple signals. The **NotificationQueue** stacks Toast notifications with priority ordering. The most actionable Toast is on top. Each Toast can carry an action button: `[Undo]`, `[Resolve]`, `[View]`. The action buttons mean that the Toast is not just feedback — it is a continuation point. "Containment violation on #42 [Resolve]" — the practitioner taps the button and lands in the pathway palette without navigating to the item first.

### Session Recording for Practice Review

**Asciicast** combined with session metadata (gesture timestamps, navigation path, dwell times) creates a reviewable record of practice. The practitioner (or their coach, or an AI assistant) can replay a session and study the pattern of engagement: where did attention go? What was deferred? What was revisited? Where was the practitioner stuck?

This is not surveillance. It is the instrument offering its own behavior as material for the practice. The recording exists only locally. It is the practitioner's choice to review it, share it, or delete it.

### Layout Persistence Across Sessions

**Workspace** with migration/versioning means that the instrument's physical layout — how the practitioner has arranged the space — persists and evolves. A practitioner who always expands the desire anchor (because they write long desire statements) will find the instrument configured that way on every launch. A practitioner who collapses the reality anchor (because they keep reality brief) will find it collapsed.

The layout is itself a record of practice style. If the Workspace includes version migration, the layout survives instrument upgrades.

### Virtualized Lists at Scale

A practitioner with 200 active tensions and 500 total steps will never notice. **VirtualizedList** with Fenwick tree indexing renders only what is visible, computes scroll positions at O(log n), and handles variable-height items natively. The survey view, which aggregates steps across all tensions, performs identically whether the field has 10 steps or 10,000.

This means the instrument does not degrade as practice matures. A five-year practitioner with thousands of accumulated tensions and logbook entries navigates with the same responsiveness as a first-day user.

### Help System as Contextual Guide

ftui's **HelpSystem** with registry and **HintRanker** (Bayesian posteriors) replaces the current static help overlay with a contextual guide:

- Press `?` and the help system shows keybindings relevant to the current focus state. On a route item: "Enter focus / l descend / r resolve / m move". On the input point: "a add / n note / ! desire / ? reality".
- The HintRanker learns which bindings the practitioner uses and which they do not. Over time, it surfaces the less-used bindings more prominently — gently teaching the full vocabulary.
- **Tooltips** (feature-gated) appear after dwelling on an element: hover on a Badge and see "3 days overdue — deadline Mar 27". Not intrusive. Only when the practitioner pauses.
- **Guided tours** (feature-gated) for first-time users: a step-by-step walkthrough that highlights each zone, explains each axis of navigation, and invites the practitioner to try each gesture. The tour uses Modal + focus trapping to guide attention through the instrument's space.

---

## VIII. Composition

This section names the widget for each element of the instrument, showing how ftui's modules compose.

### The Desk

| Element | Widget | Layout | Runtime |
|---------|--------|--------|---------|
| Three-pane skeleton | **PaneLayout** (3 horizontal panes, drag-to-resize) | Magnetic snap points for min pane heights | **Workspace** persistence |
| Desire anchor | **Paragraph** (styled wrapping, alignment) | Flex: FitContent | Observable<Tension> |
| Reality anchor | **Paragraph** | Flex: FitContent | Observable<Tension> |
| Route zone | **List** (selectable, markers, highlight) | Flex: FitContent (compresses first) | Computed<Frontier> |
| Console crown | **Rule** + **Badge** spans | Flex: Fixed(1) | Computed<Frontier> |
| Console items | **List** (within console flex region) | Flex: Min(5) | Computed<Frontier> |
| Console footer | **Rule** + **Badge** spans | Flex: Fixed(1) | Computed<Frontier> |
| Accumulated zone | **List** | Flex: FitContent (gravity bottom) | Computed<Frontier> |
| Gutter | **Columns** (2-char fixed) | Responsive: hidden < 60 cols | Signal computation |
| Trace column | **Columns** (right-aligned) | Responsive: degrades at narrow widths | |
| Status bar (top) | **StatusLine** | Fixed(1) at top of PaneLayout | Focus history → breadcrumb |
| Status bar (bottom) | **StatusLine** | Fixed(1) at bottom of PaneLayout | Session state |
| Cursor | Focus system highlight | Focus graph | Undo-aware |

### Interactions

| Interaction | Widget | Behavior |
|-------------|--------|----------|
| Add child | **Modal** + **TextInput** + **ValidationError** | Focus-trapped, dimmed backdrop, inline validation |
| Edit desire/reality | **Modal** + **TextArea** + **ValidationError** | Multi-line, pre-filled, scrollable |
| Resolve/release confirm | **Modal** (dialog::confirm) | Confirm/cancel with animation |
| Pathway palette | **Modal** + **List** | 3-5 options, context header |
| Reorder step | **Drag-and-drop** (keyboard variant) | Typed payload, drop positions, custom preview |
| Command palette | **CommandPalette** | Bayesian fuzzy search, FrankenSearch integration |
| Search | **CommandPalette** | Search mode, results navigate to tensions |
| Undo/redo | **Undo** (transactions) | Gesture-scoped, toast feedback |
| Gesture feedback | **Toast** + **NotificationQueue** | Stacked, action buttons, auto-dismiss |
| Help | **HelpSystem** + **HintRanker** | Contextual, learning, tooltips |

### Survey View

| Element | Widget | Layout |
|---------|--------|--------|
| Temporal tree | **Tree** (lazy expansion, guide styles) | Fill |
| Step items | **VirtualizedList** (within tree nodes) | Variable height, overscan |
| Step annotations | **Badge** (overdue, stale) + dim parent text | Inline |
| Temporal sparklines | **Sparkline** (orient zoom only) | Inline, 8 chars |
| Frame controls | `[`/`]` keys mapping to tree collapse/expand | |

### Ground Mode

| Element | Widget | Layout |
|---------|--------|--------|
| Epoch history | **Tree** (expandable epochs) | Fill |
| Gesture log | **LogViewer** (scrollable, searchable) | Fill |
| Field statistics | **Table** (rows/columns, sortable) | Grid |
| Session timeline | **Sparkline** (engagement density) | Inline |
| Mutation JSON | **JsonView** (structured display) | Modal on demand |

---

## IX. The Feel

Open the instrument. The terminal fills with a calm, monochrome field. Desire at the top: where you are going. Reality at the bottom: where you stand. Between them, the theory of closure — your bridge. The cursor rests at the operating envelope, the frontier of action. Nothing is colored. Nothing demands attention. The structure is at rest.

Press `j` to move down through the theory. Each step is a line, a statement of intent. The cursor highlights where you are. Press `l` to descend into a step — the screen transitions (not jump-cuts, but a smooth replacement of the focus graph) and now you are inside the step's own structure. Its desire at top, its reality at bottom, its own theory of closure between. Press `h` to ascend back. The focus history carries you.

Press `:` and the command palette slides down. Type "res" — "resolve" ranks first. Press Enter. A confirm modal appears: "Resolve 'threshold detection implemented'?" Confirm. The step moves from the route into the accumulated zone. A toast slides in: "Resolved #46." The frontier recomputes. The console crown updates: one fewer step in the theory. Silence returns.

An overdue step glows amber in the route. Not a badge, not a banner — the text itself carries the signal. The amber deepens day by day. When you focus on it, the console crown reads: "3 days overdue." The pathway palette is one keypress away. Or you can resolve it. Or update reality to acknowledge the delay. The instrument offers affordances; it does not insist.

Press `Tab` to yaw into the survey. The temporal tree appears: overdue items at the top (because they are the most action-relevant), upcoming deadlines expanding below. Each step shows its parent tension in dim text on the right — structural context within temporal flow. Navigate to a step, press `Tab` again, and you land back in that step's deck. The selection carries across the transition. Two views of the same structure, transpositionally linked.

Press `Shift+Enter` for orient zoom. The view widens. The parent tension's desire frames the top. Siblings appear as a list with tiny sparklines — temporal signatures of engagement. The current tension's route compresses to a summary. Grandchild counts appear. You see the neighborhood. Press `Shift+Enter` again to return to normal. The focal length shifts back.

Resize the terminal. The instrument adapts without jarring. At narrow widths, route items show less detail. At extreme compression, only the console survives — the helm, the input, the frontier. The instrument breathes with its container.

Close the instrument. Open it tomorrow. The panes are where you left them. The cursor is where you left it. The session is new, but the workspace remembers. You step into the same space, now with one more day of structural time behind you. The epoch age in the console crown has ticked up. If something went overdue overnight, it is amber now. If everything is on track, silence greets you.

This is what it feels like to inhabit a structure that serves the practice.

---

## X. What This Sketch Does Not Address

- **Exact key bindings.** The state machine specification (#16) determines bindings. This sketch assumes the navigation axes (pitch/roll/yaw/zoom) and gesture keys exist but does not specify them.
- **Logbook navigation.** The logbook (cursor below reality) is a future concern. This sketch notes the transition point but does not design the logbook view.
- **Multi-player identity.** Sessions, gestures, and provenance will eventually carry multi-player identity. The spatial model accommodates this (toasts could show author, gesture history could show who) but does not design it.
- **Threshold mechanics.** Thresholds (navigational boundaries with contextual signals) are specified in the foundation but deferred here. The Modal system provides the mechanism; the content and trigger logic are separate design work.
- **Ground mode detail.** Ground mode's full design (Table for statistics, LogViewer for gesture history, JsonView for raw data) is sketched in Section VIII but not elaborated.
- **Performance profiling.** Whether the reactive system, the virtualized lists, and the pane layout actually meet frame budgets in practice requires prototyping. The degradation cascade is the safety net.
- **Web and desktop surfaces.** This sketch is terminal-native. The web surface (Axum server + HTML frontend) and desktop surface (Tauri) are separate design exercises that may share the spatial model but not the widget implementation.

---

## Epilogue: From Lines to Structure

The current TUI draws lines on a terminal. The reimagined instrument composes structural widgets into an inhabited space. The difference is not cosmetic. It is the difference between painting a picture of a room and building a room you can walk through.

Every widget named in this sketch exists in ftui v0.2.1. Every layout primitive is implemented. Every runtime feature is available. The composition is the creative work — deciding how PaneLayout's magnetic fields serve the one spatial law, how the Focus system's directed graph becomes the navigation model, how Reactive's dependency tracking eliminates stale state, how the Undo system transforms gestures from commitments to explorations.

The sacred core is not mentioned in the code. It is embodied in the structure: desire above actual, always. Silence until exception. Gesture as the unit of meaningful change. Locality in signal propagation. The user's own standards as the only basis for computation. The instrument does not assert these principles. It enacts them.
