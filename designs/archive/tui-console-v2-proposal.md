# TUI Console V2 Proposal

**Status:** Proposal only. This is not ground-truth shaping yet.
**Scope:** Console zone refinement inside the existing deck view.
**Anchors:** `designs/werk-conceptual-foundation.md`, `designs/tui-shaping.md`, `designs/tui-breadboard.md`, `designs/tui-big-picture.md`

## What I Read And Ran

- `designs/werk-conceptual-foundation.md`
- `designs/tui-shaping.md`
- `designs/tui-breadboard.md`
- `designs/tui-big-picture.md`
- `cargo run --bin werk -- tree`
- `cargo run --bin werk -- show 15`
- `./target/release/werk`
- `werk-tui/src/deck.rs`
- `werk-tui/src/update.rs`
- `werk-tui/src/app.rs`

## Current-State Diagnosis

The current console is already structurally correct, but it does not yet feel like a real instrument console.

What is good now:

- The deck already respects the sacred spatial law.
- Frontier classification is real.
- Compression is real.
- Epoch mechanics are real.
- Focus and peek are real.
- The input point is present at the frontier.

What is missing:

1. The console is not an explicit component. It is assembled inline inside the middle render pass, mixed with route and accumulated placement logic.
2. The header is one centered sentence in a rule, not a readable telemetry surface.
3. The action surface is one line of text, not a center console.
4. Overdue, next, held, and accumulated are visually too similar.
5. Hierarchy is only present after focus/peek, not as a native property of the console.
6. Dynamic extent exists informally through leftover space, but not as a deliberate console signal.
7. Empty or held-only states are under-expressed. This matters for `#15`, whose current frontier is uncommitted with 3 held children.

Relevant code evidence:

- The console body is rendered ad hoc in [`werk-tui/src/deck.rs`](/Users/moritzbierling/werk/desk/werk/werk-tui/src/deck.rs#L749).
- The current header is a single rule/readout line in [`werk-tui/src/deck.rs`](/Users/moritzbierling/werk/desk/werk/werk-tui/src/deck.rs#L937).
- The current input surface is a single row in [`werk-tui/src/deck.rs`](/Users/moritzbierling/werk/desk/werk/werk-tui/src/deck.rs#L1035).
- Navigation treats the console as a flat selectable list in [`werk-tui/src/deck.rs`](/Users/moritzbierling/werk/desk/werk/werk-tui/src/deck.rs#L1360).
- Enter and Space both overload the same focus detail mechanism in [`werk-tui/src/update.rs`](/Users/moritzbierling/werk/desk/werk/werk-tui/src/update.rs#L183) and [`werk-tui/src/update.rs`](/Users/moritzbierling/werk/desk/werk/werk-tui/src/update.rs#L260).
- App state has no explicit console plan object yet in [`werk-tui/src/app.rs`](/Users/moritzbierling/werk/desk/werk/werk-tui/src/app.rs#L65).

## 30 Candidate Ideas

1. Turn the console into a bounded chassis with a top rail, body, helm, and footer.
2. Replace the single readout string with modular telemetry chips.
3. Give the next committed step a dedicated helm row at the center.
4. Split the console into an action lane and a command well.
5. Give overdue items their own warning lane above the helm.
6. Render held items as a tray or side-pocket rather than ordinary rows.
7. Render accumulated items as a settling dock near reality.
8. Make console height explicitly load-driven.
9. Make the empty console a meaningful idle state.
10. Promote the input line into a two-row command well.
11. Add a stronger cursor reticle or halo inside the console.
12. Use the left and right gutters as instrument rails for badges and markers.
13. Add a hierarchy dock for the selected item inside the console.
14. Split focus and peek into true console-local states instead of one overloaded focus state.
15. Move the log indicator into the console footer.
16. Add a mini closure gauge to the console crown.
17. Add severity-based chrome skins: idle, active, pressure.
18. Collapse compressed route/held/trace into chips instead of summary rows.
19. Make commands contextual to the current cursor target.
20. Add a tiny console mode label like `NOW`, `EMPTY`, or `PRESSURE`.
21. Use slight left/right asymmetry so overdue feels left-weighted and held feels right-weighted.
22. Add explicit per-zone labels like `OVERDUE`, `NEXT`, `HELD`, `TRACE`.
23. Box only the command well, leaving the rest of the console open.
24. Add a right-side hierarchy breadcrumb for the selected child.
25. Literalize the steering-wheel metaphor with surrounding directional key hints.
26. Use a two-column console layout on very wide terminals.
27. Show a recent-gesture tape inside the console.
28. Keep a sticky action row visible even when route compresses away.
29. Let direct typing from the helm open an armed input state.
30. Introduce explicit `ConsolePlan` / `ConsoleSection` / `ConsoleTelemetry` code objects.

## Critical Evaluation

| # | Verdict | Why |
|---|---------|-----|
| 1 | **Keep** | The console needs an actual chassis to feel like a center console rather than a list with a rule. |
| 2 | **Keep** | Readouts should scan like instruments, not prose. |
| 3 | **Keep** | A center console needs a center of gravity; the next step should be that anchor. |
| 4 | **Keep** | Separating action from commands improves hierarchy and clarity. |
| 5 | **Keep** | Overdue is action-relevant pressure and deserves its own lane. |
| 6 | **Keep** | Held items are conceptually different from route and should look different. |
| 7 | **Keep** | Accumulated items already gravitate toward reality; making that explicit is strong. |
| 8 | **Keep** | R2.6 explicitly wants extent to be signal. |
| 9 | **Keep** | R2.8 explicitly wants the empty console to be meaningful. |
| 10 | **Keep** | The current single input line is too weak for the conceptual center. |
| 11 | Reject | A stronger reticle is useful but secondary; the shell and helm solve more of the problem with less noise. |
| 12 | Reject | Instrument rails risk becoming decorative chrome without enough structural value. |
| 13 | **Keep** | The user explicitly asked for hierarchy, and selected-item hierarchy belongs in the console. |
| 14 | **Keep** | Current state overloading is already visible in the code and will limit refinement. |
| 15 | **Keep** | The console footer should own trace/log readouts; the current bottom bar feels detached. |
| 16 | Reject | Closure is useful, but a literal gauge is weaker than compact telemetry chips unless carefully justified. |
| 17 | **Keep** | Chrome should intensify by pressure and context, not remain static. |
| 18 | **Keep** | Summary rows are functional but low-polish; chips are cleaner under compression. |
| 19 | **Keep** | A real console should expose relevant controls at the point of action. |
| 20 | Reject | Mode labels can help, but the telemetry and shell state already communicate this more elegantly. |
| 21 | **Keep** | Mild asymmetry gives spatial semantics without violating the vertical law. |
| 22 | Reject | Explicit labels would make the console feel dashboardy and verbose. |
| 23 | Reject | Boxing only the command well understates the console; the whole console should feel composed. |
| 24 | Reject | Selected-child hierarchy belongs in a dock or peek, not as a permanent breadcrumb. |
| 25 | Reject | Too literal and gimmicky. The user wants polish, not novelty for its own sake. |
| 26 | Reject for now | It is plausible later, but it is not the highest-value first refinement. |
| 27 | Reject as a separate feature | Keep the signal, but fold it into footer telemetry rather than a new tape widget. |
| 28 | **Keep** | When compression hits, the action center must remain visible. |
| 29 | **Keep** | Direct typing from the helm is a real console behavior and improves fluency. |
| 30 | **Keep** | Without explicit console plan objects, the UI will stay ad hoc and hard to polish. |

## Final Design

The final design I would pursue is:

**A load-responsive center console with a bounded chassis, a modular telemetry crown, a dedicated helm row for the next actionable step, a command well directly beneath it, a held tray below that, and a trace/log footer settling toward reality.**

This is not a new fifth zone. It is a real composition of the existing console zone.

### Console Anatomy

1. **Chassis**
   The console is rendered as an explicit component inside the middle zone.
   It has a crown, body, helm, tray, and footer.

2. **Telemetry Crown**
   A one-line or two-line chip rail.
   Primary chips:
   - closure
   - epoch age
   - overdue pressure
   - next deadline
   - held count
   - last act

3. **Warning Lane**
   If overdue exists, it appears first, inside the console shell, with amber emphasis.
   This is pressure at the frontier.

4. **Helm Row**
   The next committed step is the visual center.
   If there is no next step, the helm becomes a purposeful prompt:
   `no committed next step`

5. **Command Well**
   The current `▸ ___` line becomes a two-row control surface:
   - row 1: prompt / target / direct-typing surface
   - row 2: contextual command chips

6. **Held Tray**
   Held items sit below the helm, slightly right-weighted and slightly indented.
   They feel available, not committed.

7. **Trace Footer**
   Accumulated items, prior events, and recent act settle into the footer nearest reality.

8. **Hierarchy Dock**
   Space on the selected item opens a compact child dock in the body or tray area.
   Enter still opens full focus.

9. **Dynamic Extent**
   Console height expands from load, not from arbitrary leftover space alone.

### The Feel

- Quiet state: open, sparse, inviting.
- Active state: composed and balanced.
- Pressure state: denser crown, stronger border, visible warning lane.
- Held-only state: the console acknowledges that the theory exists but is not yet committed.

## Why This Fits `#15`

`cargo run --bin werk -- show 15` shows the current frontier is:

- no committed route
- no overdue
- no next step
- 3 held children

That is exactly the kind of state where the current console under-speaks. The improved console should say, visually and immediately:

- the epoch is live
- there is no committed next bridge
- there are 3 held candidates available
- the command surface is ready to either add or commit structure

## Mockups

### A. Held-only / uncommitted state like `#15`

```text
        ╭─ closure 0/3 ─ epoch 27m ─ held 3 ─ last act 27m ─────────────────────╮
        │                                                                        │
        │  no committed next step                                                │
        │  ▸ choose a held step or create the next bridge                        │
        │                                                                        │
        │    · survey view designed and implemented                         #18   │
        │    · threshold mechanics implemented                              #19   │
        │    · pathway palettes in TUI                                      #58   │
        │                                                                        │
        │  [a add]  [n note]  [! desire]  [? reality]  [type to act]             │
        │                                                                        │
        │  ↓ 4 prior events                                             fresh log │
        ╰────────────────────────────────────────────────────────────────────────╯
```

### B. Active state with overdue + next + held + trace

```text
        ╭─ closure 5/11 ─ epoch 3d ─ ⚠ 2 overdue ─ next Mar 30 ─ held 2 ───────╮
Mar 21  │ ! fix parser edge case                                          #23 2d │
Mar 24  │ ! answer user on migration path                                 #31 1d │
Mar 30  │ ▸ refine console shell and helm                                 #15 →3 │
        │   [Enter focus] [Space peek] [r resolve] [e edit] [type to act]       │
        │                                                                        │
        │     · orient zoom layout study                                  #18    │
        │     · threshold signal polish                                   #19    │
        │                                                                        │
        │  ✓ 2 resolved  ~ 1 released  ↓ 9 prior events  last act 17m            │
        ╰────────────────────────────────────────────────────────────────────────╯
```

### C. Empty console

```text
        ╭─ fresh epoch ─ no pressure ─ no held steps ────────────────────────────╮
        │                                                                        │
        │  nothing action-relevant in the current epoch                          │
        │  ▸ type to add the next bridge                                         │
        │                                                                        │
        │  [a add]  [n note]  [! desire]  [? reality]                            │
        │                                                                        │
        │  ↓ 12 prior events                                                     │
        ╰────────────────────────────────────────────────────────────────────────╯
```

## Detailed Plan For Each Kept Idea

### 1. Bounded Chassis

**What**

Render the console as its own shell inside the middle zone, with explicit top and bottom edges and a small interior padding budget.

**Why**

This is the single biggest shift from "good structure, weak feel" to "real console". It creates center gravity and visual ownership.

**Downsides**

- Costs 2 lines of chrome if done carelessly.
- Can feel heavy on small terminals unless chrome degrades.

**Confidence**

95%

**Code sketch**

```rust
struct ConsolePlan {
    rect: Rect,
    skin: ConsoleSkin,
    crown: CrownPlan,
    warning: Option<WarningLane>,
    helm: HelmPlan,
    held: HeldTrayPlan,
    footer: FooterPlan,
}

enum ConsoleSkin {
    Idle,
    Active,
    Pressure,
}
```

Use `Block` when the terminal is tall enough; fall back to open rails when compressed.

### 2. Telemetry Crown

**What**

Replace the current centered rule sentence with composable telemetry chips.

**Why**

Telemetry should be scannable by chunk, not parsed as prose. This also lets compression move chips in and out by priority.

**Downsides**

- Needs a tight priority order.
- Can get busy if every fact becomes a chip.

**Confidence**

93%

**Recommended chip order**

1. overdue pressure
2. next deadline
3. closure
4. epoch age
5. held count
6. last act

**Code sketch**

```rust
struct CrownPlan {
    left: Vec<Chip>,
    center: Vec<Chip>,
    right: Vec<Chip>,
}

struct Chip {
    text: String,
    kind: ChipKind,
}

enum ChipKind {
    Neutral,
    Accent,
    Warning,
    Quiet,
}
```

### 3. Helm Row

**What**

The next step gets a dedicated hero row at the center of the console. If there is no next step, the row becomes a purposeful structural prompt.

**Why**

The console needs a wheel. The next committed step is the wheel.

**Downsides**

- If the row is too theatrical, the TUI becomes gimmicky.
- Must not overshadow overdue.

**Confidence**

96%

**Behavior**

- `next exists`: render it as the helm target.
- `no next, held exists`: render `no committed next step`.
- `nothing exists`: render `type to add the next bridge`.

### 4. Action Lane + Command Well

**What**

Split the current one-line input point into:

- a prompt/target row
- a controls row

**Why**

The command surface becomes legible and feels like controls rather than placeholder text.

**Downsides**

- Costs one extra line in the compact case.
- Needs a graceful one-line fallback under compression.

**Confidence**

94%

**Code sketch**

```rust
struct HelmPlan {
    target: HelmTarget,
    prompt: String,
    commands: Vec<CommandChip>,
    typing: Option<TypingState>,
}

enum HelmTarget {
    Next(usize),
    InputOnly,
    HeldChoice,
}
```

### 5. Warning Lane

**What**

Overdue items become a distinct lane above the helm, styled in amber, with hard cap + summary when crowded.

**Why**

Pressure should surface in place, by exception, and with stronger geometry than ordinary action.

**Downsides**

- Too much amber will look alarmist.
- Must keep only 1-2 visible overdue items before summarizing.

**Confidence**

90%

**Recommendation**

- show up to 2 overdue rows
- remainder collapses into crown chip: `+3 overdue`

### 6. Held Tray

**What**

Held items render below the helm, slightly indented and slightly right-weighted. They read as reserve options, not active course.

**Why**

This gives the console visible hierarchy and better conceptual fidelity.

**Downsides**

- Too much asymmetry will look messy.
- Needs a stable compression rule.

**Confidence**

89%

**Recommendation**

- show up to 2 held items
- then a chip or summary row for the rest

### 7. Trace Footer

**What**

Accrued facts, prior events, and recent act move into a dedicated footer at the bottom of the console shell.

**Why**

This finally makes the console feel like a complete operating envelope from pressure to action to settling trace.

**Downsides**

- Some information currently in the bottom bar would move.
- Must avoid duplicating the global bottom chrome.

**Confidence**

88%

**Footer contents**

- accumulated counts
- prior event count
- last act age

### 8. Dynamic Extent

**What**

The console gets an explicit target height based on load, not just whatever middle space remains after route/reality.

**Why**

R2.6 demands that console extent itself be signal.

**Downsides**

- Without hysteresis, it can visually jump after small mutations.
- Adds layout complexity.

**Confidence**

92%

**Code sketch**

```rust
fn compute_console_load(frontier: &Frontier, selected_has_children: bool) -> u8 {
    let overdue = frontier.overdue.len().min(3) as u8 * 3;
    let next = if frontier.next.is_some() { 3 } else { 0 };
    let held = frontier.held.len().min(3) as u8;
    let trace = frontier.accumulated.len().min(2) as u8;
    let hierarchy = if selected_has_children { 2 } else { 0 };
    overdue + next + held + trace + hierarchy
}

fn target_console_height(load: u8) -> u16 {
    match load {
        0..=1 => 5,
        2..=4 => 7,
        5..=8 => 9,
        _ => 11,
    }
}
```

Use hysteresis: do not shrink until load drops by at least one band.

### 9. Meaningful Empty State

**What**

When the console is empty, render an intentional idle console instead of a thin placeholder.

**Why**

R2.8 explicitly requires this. It also improves the emotional feel of the instrument.

**Downsides**

- Empty-state prose can become self-important.

**Confidence**

90%

**Recommendation**

Keep it minimal:

- `nothing action-relevant in the current epoch`
- `type to add the next bridge`
- show the four primary commands

### 10. Hierarchy Dock

**What**

Space opens a compact hierarchy dock for the selected item inside the console. This is not full focus; it is a local peek.

**Why**

The user explicitly asked for hierarchy. The console should show just enough child structure to support action at the frontier.

**Downsides**

- Can clutter the body if it is allowed everywhere.
- Must degrade aggressively under height pressure.

**Confidence**

84%

**Recommendation**

- show up to 3 children
- optionally one-line reality stub
- if too small, fall back to chip: `→3 children`

### 11. Severity-Based Chrome

**What**

The shell changes subtly by state:

- idle: quiet edges
- active: normal edges
- pressure: stronger crown/footer and amber warning presence

**Why**

Chrome should respond to structural pressure, not remain static.

**Downsides**

- Easy to overdo.

**Confidence**

87%

**Recommendation**

Do not change colors much. Change geometry more than color.

### 12. Compression Into Chips

**What**

When content compresses, route/held/trace collapse into crown or footer chips before they collapse into anonymous summary rows.

**Why**

This feels significantly more polished than floating summary lines.

**Downsides**

- Some users may miss the explicit rows.

**Confidence**

89%

**Compression priority**

1. compress trace rows into footer chips
2. compress held rows into tray summary chip
3. compress route rows into crown chip
4. preserve helm row as long as possible

### 13. Contextual Commands

**What**

The command well shows commands for the current target, not a fixed static string.

**Why**

That is how a real console behaves. Controls are relevant to the current surface.

**Downsides**

- Requires more command planning logic.
- Discoverability must remain stable.

**Confidence**

86%

**Example**

```text
On next step:    [Enter focus] [Space peek] [e edit] [r resolve]
On held item:    [Enter focus] [Space peek] [e edit] [m move]
On empty helm:   [a add] [n note] [! desire] [? reality]
```

### 14. Sticky Action Center

**What**

No matter how aggressively the route compresses, the helm row and command well remain visible.

**Why**

The center of action must not disappear under compression.

**Downsides**

- Other content will disappear sooner.

**Confidence**

93%

**Rule**

Never compress away:

- helm target row
- one command row
- one footer telemetry row

### 15. Armed Typing

**What**

Typing a printable character while the helm is active opens an input state immediately at the console.

**Why**

This is the most natural "real console" behavior.

**Downsides**

- Must avoid conflicts with single-key commands.
- Needs careful key-routing.

**Confidence**

85%

**Recommendation**

- reserve existing one-key commands
- any other printable char opens `InputMode::HelmTyping`
- seed the buffer with the typed char

### 16. Explicit Console Plan Objects

**What**

Introduce explicit plan types so the console is computed first, then rendered.

**Why**

This is required for polish. The current render path is doing too much layout, classification, and conditional presentation in one place.

**Downsides**

- It is a real refactor.

**Confidence**

94%

**Code sketch**

```rust
struct ConsolePlan {
    skin: ConsoleSkin,
    extent: ConsoleExtent,
    crown: CrownPlan,
    warning: Vec<RowPlan>,
    helm: HelmPlan,
    held: TrayPlan,
    dock: Option<HierarchyDockPlan>,
    footer: FooterPlan,
}

fn compute_console_plan(
    frontier: &Frontier,
    cursor: DeckCursor,
    state: &ConsoleState,
    width: u16,
    height: u16,
) -> ConsolePlan
```

## Recommended Code Shape

### New Types

Add to [`werk-tui/src/deck.rs`](/Users/moritzbierling/werk/desk/werk/werk-tui/src/deck.rs):

```rust
pub struct ConsoleState {
    pub peek: Option<ConsolePeek>,
    pub typing: Option<HelmTypingState>,
}

pub struct ConsolePlan { /* see above */ }
pub struct ConsoleMetrics { /* widths, target heights, chip budgets */ }
```

Add to [`werk-tui/src/app.rs`](/Users/moritzbierling/werk/desk/werk/werk-tui/src/app.rs):

```rust
pub console_state: crate::deck::ConsoleState,
```

### New Render Split

Refactor `render_deck()` so the middle zone becomes:

```rust
let console_rect = self.compute_console_rect(...);
let route_rect = ...;
let path_rect = ...;

self.render_route(...);
self.render_console(console_rect, ...);
self.render_path(...);
```

This is better than continuing to interleave everything in one long pass.

### New Event Split

In [`werk-tui/src/update.rs`](/Users/moritzbierling/werk/desk/werk/werk-tui/src/update.rs), stop overloading `ZoomLevel::Focus` for both Enter and Space behavior.

Recommended:

```rust
enum ConsolePeekMode {
    Inline,
    Focus,
}
```

Space:

- open/close hierarchy dock

Enter:

- open/close full focus

Typing:

- open helm typing state

## Implementation Order

1. Extract a `ConsolePlan` and `render_console()` without changing behavior.
2. Replace the current header + input row with crown + helm + footer scaffolding.
3. Add dynamic extent and sticky action-center rules.
4. Add held tray and trace footer.
5. Add hierarchy dock and armed typing.

## First-Pass Recommendation

If I were building this, I would implement **one coherent change**, not a pile of small embellishments:

**Build the console as a real, bounded, load-responsive component whose central visual object is the helm row, whose telemetry lives in a modular crown, whose commands live in a command well, and whose trace settles into a footer.**

That is the direction most likely to make the component feel intentional, elegant, and instrument-grade rather than merely "more featureful".

## Second Pass: More Ruthless Exploration

This second pass applied a stricter filter.

The question was not "could this be cool?"

The question was:

- does it materially increase center-of-gravity?
- does it improve scanability?
- does it preserve the sacred architecture?
- does it compress gracefully?
- does it avoid turning the console into dashboard theater?

### Second-Pass 30 Ideas

1. Give the console a bounded chassis with crown, body, and footer.
2. Replace the prose header with modular telemetry chips.
3. Make the next committed step a dedicated helm row at the center.
4. Create a distinct overdue warning queue above the helm.
5. Render held items as a tray below the helm instead of ordinary rows.
6. Settle accumulated items into a trace footer near reality.
7. Make console height banded by load, with hysteresis.
8. Design the empty console as a meaningful idle state.
9. Design the held-only console as an explicit `no committed bridge` state.
10. Promote the input line into a two-row command well.
11. Make command chips contextual to the current cursor target.
12. Let direct typing open helm input immediately from normal mode.
13. Put hierarchy peek in a dock inside the console rather than inline anywhere.
14. Split Space peek from Enter focus in the state model.
15. Make chrome severity-sensitive: idle, active, pressure.
16. Compress into chips before degrading into summary sentences.
17. Guarantee a sticky center so helm and commands never disappear.
18. Fold prior-events and log readouts into the console footer.
19. Use mild left/right asymmetry so overdue feels left-weighted and held feels right-weighted.
20. Show a selected-item reality stub in the dock.
21. Add a closure micro-gauge to the crown.
22. Add a route memory strip when route is fully compressed.
23. Put transient gesture feedback inside the console instead of only the bottom bar.
24. Add a stronger cursor reticle around the helm target.
25. Use a two-column console layout on very wide terminals.
26. Add side rails for micro-telemetry.
27. Add a title plate or notch marking the console.
28. Add a footer quick-action row.
29. Refactor the console into explicit `ConsolePlan` / `ConsoleState` / `ConsoleMetrics` objects.
30. Split the render path into separate route / console / path passes.

### Second-Pass Evaluation

| # | Verdict | Reason |
|---|---------|--------|
| 1 | **Keep** | Highest-leverage structural improvement. |
| 2 | **Keep** | Instruments should scan in chunks, not sentences. |
| 3 | **Keep** | The console needs a steering wheel; the next step is it. |
| 4 | **Keep** | Overdue pressure deserves a distinct lane. |
| 5 | **Keep** | Held is not route and should not look like route. |
| 6 | **Keep** | Trace belongs at the console floor nearest reality. |
| 7 | **Keep** | Extent as signal is core, not decorative. |
| 8 | **Keep** | Empty console meaning is explicitly required. |
| 9 | Reject | Correct need, but better treated as a variant of the idle-state system. |
| 10 | **Keep** | One-line input is too weak for the center surface. |
| 11 | **Keep** | Controls should reflect what the cursor is on. |
| 12 | Reject | Strong phase-2 idea, but too much key-routing complexity for first-pass excellence. |
| 13 | **Keep** | This is the cleanest way to make hierarchy native to the console. |
| 14 | **Keep** | Current Enter/Space overloading already constrains the design. |
| 15 | Reject | Nice polish, but lower leverage than shell, helm, and layout. |
| 16 | **Keep** | Compression should stay elegant before it turns generic. |
| 17 | **Keep** | The action center must never disappear. |
| 18 | Reject | Correct, but really part of the trace footer rather than a separate idea. |
| 19 | Reject | Too easy to drift into ornament. |
| 20 | Reject | Useful only as part of the hierarchy dock, not standalone. |
| 21 | Reject | Too dashboardy; chips communicate the same fact more cleanly. |
| 22 | Reject | Crown chips already cover route memory more elegantly. |
| 23 | Reject | Good later, but not first-order. |
| 24 | Reject | Once the helm is real, this becomes redundant. |
| 25 | Reject | Worth revisiting later, not now. |
| 26 | Reject | Decorative risk too high. |
| 27 | Reject | Too literal; risks gimmick. |
| 28 | Reject | Duplicates the command well. |
| 29 | **Keep** | Required to make the whole thing robust and polishable. |
| 30 | Reject | Implied by 29 rather than standing alone as a distinct improvement. |

### Passed Ideas From Second Pass

#### 1. Bounded Chassis

**Plan**

Render the console as a real shell inside the middle zone with a crown, body, and footer, using `Block` when there is enough height and falling back to open rails when compressed.

**Why**

This creates visual ownership immediately. It is the difference between a list with a separator and a real center console.

**Downsides**

- costs vertical lines
- can feel heavy if over-boxed

**Confidence**

96%

#### 2. Telemetry Crown

**Plan**

Replace the current centered readout sentence with chips ordered by priority:

1. overdue
2. next deadline
3. closure
4. epoch age
5. held count
6. last act

**Why**

Chips scan faster and degrade better than prose.

**Downsides**

- can get noisy if too many survive

**Confidence**

94%

#### 3. Helm Row

**Plan**

Reserve the visual center of the console for one hero row. If `next` exists, render it there. If not, render a purposeful structural prompt such as `no committed next step`.

**Why**

The user wants a steering wheel. This is it.

**Downsides**

- can become theatrical if overstyled

**Confidence**

97%

#### 4. Overdue Warning Queue

**Plan**

Render overdue steps as a short amber queue above the helm, capped at two visible rows. Collapse the rest into a crown chip such as `+3 overdue`.

**Why**

Overdue is exception pressure and deserves stronger geometry than ordinary action.

**Downsides**

- too much amber becomes alarmist

**Confidence**

91%

#### 5. Held Tray

**Plan**

Render held steps below the helm as a distinct tray. Show up to two, then collapse the rest into a held summary chip.

**Why**

Held is reserve possibility, not committed route. The eye should know that instantly.

**Downsides**

- easy to make visually fussy

**Confidence**

89%

#### 6. Trace Footer

**Plan**

Move accumulated counts and trace readouts into a footer at the bottom of the console shell, nearest reality.

**Why**

This completes the console from pressure, to action, to settled trace.

**Downsides**

- must not duplicate bottom-bar chrome

**Confidence**

89%

#### 7. Dynamic Extent With Hysteresis

**Plan**

Compute a load score from overdue, next, held, trace, and dock state, then choose a height band with hysteresis so the console does not pulse after small changes.

**Why**

Extent is itself a signal and should be deliberate.

**Downsides**

- adds layout complexity

**Confidence**

93%

```rust
enum ConsoleBand {
    Idle,
    Light,
    Loaded,
    Pressure,
}

fn compute_console_load(frontier: &Frontier, has_dock: bool) -> u8 {
    let overdue = frontier.overdue.len().min(3) as u8 * 3;
    let next = if frontier.next.is_some() { 3 } else { 0 };
    let held = frontier.held.len().min(3) as u8;
    let trace = frontier.accumulated.len().min(2) as u8;
    let dock = if has_dock { 2 } else { 0 };
    overdue + next + held + trace + dock
}
```

#### 8. Meaningful Idle State

**Plan**

Make empty and uncommitted states first-class render templates instead of just "less content". A held-only frontier should say `no committed next step`; a truly empty frontier should say `nothing action-relevant in the current epoch`.

**Why**

The current console under-speaks exactly these states.

**Downsides**

- copy can become self-important if over-written

**Confidence**

91%

#### 10. Two-Row Command Well

**Plan**

Replace the current one-line input surface with two rows:

- a prompt / typing row
- a controls row

Under compression, collapse back to one line.

**Why**

This makes the action surface feel like controls, not a placeholder.

**Downsides**

- costs an extra line in medium-height layouts

**Confidence**

95%

#### 11. Contextual Command Chips

**Plan**

Make the command row depend on the cursor target.

**Why**

A real console exposes relevant controls at the point of action.

**Downsides**

- discoverability can suffer if controls move too much

**Confidence**

88%

```rust
fn command_chips(target: CursorTarget) -> Vec<CommandChip> {
    match target {
        CursorTarget::Next(_) => chips(&["Enter focus", "Space peek", "e edit", "r resolve"]),
        CursorTarget::HeldItem(_) => chips(&["Enter focus", "Space peek", "e edit", "m move"]),
        CursorTarget::AccumulatedItem(_) => chips(&["Enter focus", "o reopen", "y yank"]),
        _ => chips(&["a add", "n note", "! desire", "? reality"]),
    }
}
```

#### 13. Hierarchy Dock

**Plan**

When Space is pressed on a child, open a compact dock inside the console showing up to three children and, if room allows, a one-line reality context.

**Why**

This makes hierarchy native to the console without exploding into full focus.

**Downsides**

- can clutter the console if height budgeting is weak

**Confidence**

85%

#### 14. Split Peek From Focus In State Model

**Plan**

Stop using the same focus machinery for both Enter and Space. Enter remains full focus zoom; Space opens a lighter dock-local peek.

**Why**

The semantics are different, and the current overloading already limits the design.

**Downsides**

- moderate refactor cost

**Confidence**

93%

```rust
enum ConsoleLocalState {
    Rest,
    PeekDock { sibling_index: usize },
    HelmInput,
}

enum ZoomLevel {
    Normal,
    Focus,
    Orient,
}
```

#### 16. Chip-First Compression

**Plan**

When space gets tight, collapse route, held, and trace into crown/footer chips before falling back to generic summary lines.

**Why**

This keeps compression elegant.

**Downsides**

- some users may prefer explicit summary rows

**Confidence**

91%

#### 17. Sticky Action Center

**Plan**

Make the helm row, one command row, and one footer row non-negotiable. Everything else compresses around them.

**Why**

The center of action must survive pressure.

**Downsides**

- route and held detail disappear sooner

**Confidence**

95%

#### 29. Explicit `ConsolePlan` / `ConsoleState` / `ConsoleMetrics`

**Plan**

Pull the console out of the monolithic middle render pass and compute it explicitly before rendering.

**Why**

Without this, every UI improvement stays fragile and ad hoc.

**Downsides**

- real refactor cost

**Confidence**

97%

```rust
pub struct ConsolePlan {
    pub rect: Rect,
    pub crown: CrownPlan,
    pub warning_rows: Vec<RowPlan>,
    pub helm: HelmPlan,
    pub held: TrayPlan,
    pub dock: Option<DockPlan>,
    pub footer: FooterPlan,
}

pub struct ConsoleState {
    pub local: ConsoleLocalState,
    pub band: ConsoleBand,
}

pub struct ConsoleMetrics {
    pub inner_width: u16,
    pub crown_height: u16,
    pub footer_height: u16,
    pub minimum_center_height: u16,
}
```

## Final Synthesis Across Both Passes

Both passes converged very strongly.

They disagree on some ornaments and some phase-2 ideas, but they agree on the core architecture almost perfectly.

### What Survived Both Passes

1. The console must become an explicit component, not ad hoc middle-zone rows.
2. The console needs a true visual chassis.
3. The next committed step should be the visual center.
4. The header should become modular telemetry, not a prose sentence.
5. Overdue, held, and trace should have distinct geometry.
6. The command surface should be more than one line.
7. Hierarchy should be present locally in the console.
8. Console extent should be deliberate and signal-bearing.
9. Empty and uncommitted states need purposeful renderings.
10. Compression should preserve polish by moving into chips before generic summaries.
11. The action center must remain visible under pressure.
12. The code needs explicit `ConsolePlan` / `ConsoleState` style abstractions.

### What The Second Pass Cut Back

The second pass was especially useful because it killed ideas that would make the console more theatrical than excellent.

These are not the right next moves:

- literal steering-wheel metaphors
- gauges and sparklines
- side rails
- extra title plates
- extra footer action rows
- reticle-heavy selection chrome
- wide-layout cleverness before the core console is solved

### The Combined Final Direction

The best synthesis is:

**A bounded, load-responsive center console with a telemetry crown, a warning queue for overdue pressure, a helm row for the current next bridge, a two-row contextual command well, a held tray below it, a compact hierarchy dock on Space, and a trace footer settling toward reality.**

That should be backed by:

- explicit `ConsolePlan` / `ConsoleState` / `ConsoleMetrics`
- a clean separation between dock peek and full focus
- chip-first compression
- sticky preservation of helm + commands + footer
- deliberate height bands with hysteresis

### Practical Implementation Order After Synthesis

1. Extract `ConsolePlan` and a dedicated `render_console()`.
2. Replace the current header and input line with crown + helm + footer scaffolding.
3. Add warning queue, held tray, and trace footer.
4. Add banded extent, chip-first compression, and sticky action-center guarantees.
5. Split Space peek from Enter focus and add the hierarchy dock.
6. Add contextual command chips.
7. Revisit phase-2 ideas only after the above feels right in the terminal.

### Final Call

If only one sentence survives from the entire document, it should be this:

**Do not keep embellishing the current console rows. Replace them with a real console component whose center is the helm row and whose entire composition is built to preserve action legibility under compression.**
