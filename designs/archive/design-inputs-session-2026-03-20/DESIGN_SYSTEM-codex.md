# Werk Operative Instrument TUI Design System

This document is grounded in the code actually present in this repository:

- `werk-tui/` for the current terminal application
- `sd-core/`, `werk-shared/`, and `werk-cli/` for the domain model, projections, watch system, agent mutations, and event history
- `ftui 0.2.1` from Cargo registry source, which `werk-tui` depends on directly; this workspace does not contain a local `ftui/` checkout

The brief refers to `werk/`; in this workspace that role is split across `sd-core/`, `werk-shared/`, and `werk-cli/`.

## 1. Instrument Thesis

`werk-tui` should not read like a dashboard, a kanban board, or a text shell with ornament. It should read like an operative instrument: a computational surface for holding structural tension between reality and desire.

The terminal is a strong medium for this because it already favors:

- ruled surfaces
- alignment over decoration
- compression over narration
- repeated use over first-use theater

The system therefore treats every visible element as structural. Lines are not decoration. Badges are not labels. Alerts are not notifications. Every mark states a relation in the field.

## 2. Framework Contract

This system uses `ftui` affordances exclusively. Product UI is composed from the framework’s layout and widget surface; no raw `Frame` painting, no bespoke ASCII chrome, no alternate visual toolkit.

### 2.1 Allowed layout primitives

- `Flex`
- `Grid`
- `ResponsiveLayout`
- `Responsive<T>`
- `Visibility`
- `Columns`
- `Group`
- `Align`
- `Padding`
- `layout::pane` split-tree model for saved multi-pane operator workspaces

### 2.2 Allowed surface widgets

- `Panel`
- `Block`
- `Rule`
- `Badge`
- `Paragraph`
- `List`
- `VirtualizedList`
- `Table`
- `Tree`
- `Sparkline`
- `ProgressBar`
- `MiniBar`
- `Paginator`
- `HistoryPanel`
- `JsonView`
- `Pretty`
- `LogViewer`
- `StatusLine`
- `Spinner`

### 2.3 Allowed overlays, input, and system widgets

- `Modal`
- `Toast`
- `NotificationQueue`
- `CommandPalette`
- `TextInput`
- `TextArea`
- `ValidationErrorDisplay`
- `FocusManager`
- `FocusGraph`
- `FocusIndicator`
- `HelpRegistry`

### 2.4 Diagnostic-only affordances

These are framework affordances but not part of the operator-facing visual language: `InspectorOverlay`, `ConstraintOverlay`, `LayoutDebugger`, `DebugOverlay`, `ErrorBoundary`, drag helpers, and `FilePicker`. They remain implementation and debug tools.

## 3. Operating Laws

### 3.1 Direction is non-negotiable

Reality is always ground: bottom and left.

Desire is always sky: top and right.

This law must survive every breakpoint, modal, and focused mode. The user should never need a legend to know where the system is pulling and where it is reporting from.

### 3.2 Disclosure is additive

Each deeper layer repeats the shallow layer’s grammar and adds structure. It never switches metaphors.

- field scan: many tensions, minimal words, structural exceptions visible immediately
- focused gaze: one tension, local field context, pressure and drift legible
- full analysis: diagnosis, history, projections, competing signals, and review surfaces

### 3.3 Structure outranks prose

The operator should detect conflict, neglect, drift, oscillation, and progress from lines, badges, bars, dots, and panel placement before reading explanatory text.

### 3.4 Signals persist until structurally cleared

Structural alerts stay in the field until the condition changes. They are not ephemeral toasts.

### 3.5 Density is earned, not decorative

Every dense area must compress semantics, not merely more text.

## 4. Spatial Doctrine

### 4.1 Axes

| Axis | Semantic meaning | UI consequence |
| --- | --- | --- |
| Vertical | bottom = confronted reality, top = intended form | every tension surface stacks `desired` above `actual` |
| Horizontal | left = basis, history, constraint, senior context; right = projection, choice, intended motion | details move rightward as they become more intentional |

### 4.2 Quadrants

| Quadrant | Meaning |
| --- | --- |
| top-left | organizing principle under current conditions |
| top-right | clean intended state, phase, horizon, projection |
| bottom-left | actual condition, age, neglect, unresolved friction |
| bottom-right | next move, resolution path, reviewable mutation |

### 4.3 Canonical screen anatomy

On wide terminals the instrument uses a two-plane or three-plane composition:

- left/main plane: field scan, roots, descendants, sibling structure
- right/upper plane: focused gaze for the selected tension
- right/lower plane: full analysis, history, projections, watch review, or mutation review
- bottom lever: `StatusLine` for mode, ancestry, and key hints

The screen should feel like a ruled desk:

- main field carries breadth
- right side carries depth
- bottom line carries control
- overlays appear only when the user is explicitly writing, confirming, or jumping

Command is mostly on demand, via `CommandPalette`, rather than a permanent top bar. This keeps the “sky” quiet.

## 5. Premium Terminal Aesthetic

The visual tone is restrained phosphor, not nostalgia cosplay.

### 5.1 Surface character

- default panels use `BorderType::Square`
- the active structural locus may use `BorderType::Heavy`
- temporary overlays use `BorderType::Rounded`
- `BorderType::Double` is reserved for explicit confirmation or root-charter surfaces and should be rare

### 5.2 Color doctrine

Use a narrow theme built with `ThemeBuilder` and `Style`, close to the existing `werk-tui` palette:

- base ink: neutral foreground
- dim ink: secondary metadata
- time: cyan
- tension/pressure: amber
- collision/fault: red
- motion/resolution: green

No rainbow tagging. Color is reserved for structural state transitions and priority.

### 5.3 Alignment doctrine

- labels align hard against columns
- dot strips have fixed width
- badges have fixed internal padding
- rule titles are sparse and centered or right-aligned only when the hierarchy calls for it

The visual quality comes from recurrence and edge discipline.

## 6. Visual Grammar

### 6.1 Rules

Rules are semantic operators, not separators.

| Rule type | Meaning | `ftui` contract |
| --- | --- | --- |
| heavy | primary structural boundary, active locus, parent/child threshold, irreversible step | `Rule::new().border_type(BorderType::Heavy)` and `Panel::border_type(BorderType::Heavy)` only for the current field locus |
| light | ordinary sectional division, descriptive strata inside a gaze or analysis panel | `Rule::new().border_type(BorderType::Square)` |
| dotted | implied relation, unpositioned children, temporal uncertainty, postponement, soft partition, drift | `Rule::new().border_type(BorderType::Custom(BorderSet { horizontal: '┄', ..BorderSet::SQUARE }))` |

Usage:

- heavy rules are rare
- light rules are common
- dotted rules are diagnostic

### 6.2 Glyphs for lifecycle and status

These should remain stable across the entire instrument.

| Meaning | Glyph | Primary use |
| --- | --- | --- |
| germination | `◇` | new structure forming |
| assimilation | `◆` | new reality being taken in |
| completion | `◈` | converging on finish |
| momentum | `◉` | self-sustaining motion |
| resolved | `✦` | tension structurally closed |
| released | `·` | tension intentionally let go |

Contract:

- field scan: glyph appears inside a `Badge`
- focused gaze: glyph appears in the header badge cluster
- history: lifecycle transitions are marked with the same glyphs in `HistoryPanel`

### 6.3 Temporal dots

Temporal indicators use the existing six-cell grammar from `werk-tui/src/glyphs.rs`, rendered as fixed-width text in a `Paragraph` or compact `Badge`.

| Mark | Meaning |
| --- | --- |
| `◦` | current position in the horizon window |
| `●` | explicit horizon boundary |
| `◌` | remaining or absent temporal slots |
| `◎` | no horizon; staleness lens instead of commitment window |

Rules:

- dots are always fixed-width and right-aligned
- dots appear without prose in scan mode
- prose horizon labels appear only in gaze and analysis layers
- overdue states change style before they gain more text

### 6.4 Badges

Badges are compressed declarations. They do not explain; they state.

Badge classes:

- lifecycle: phase glyph and label
- state: `ACTIVE`, `RESOLVED`, `RELEASED`
- tendency: `ADV`, `OSC`, `STAG`
- orientation: `CRT`, `PROB`, `REACT`
- urgency: `NOW`, `SOON`, `LATE`
- drift: `TIGHT`, `POSTP`, `LOOSE`, `R-POST`, `OSC`
- neglect: `P→C`, `C-NEG`
- conflict: `CONFLICT`
- assimilation: `DEEP`, `SHALLOW`, `NONE`
- watch/review: `WATCH`, `REVIEW`, `AGENT`

Contract:

- `Badge::new(...).with_padding(1, 1)`
- phase and status badges use higher contrast
- analytic badges may be muted until selected
- no badge exceeds one short token plus, at most, one number

### 6.5 Alerts as structural signals

Structural alerts live in dedicated signal rails, not in toast space.

Three levels exist:

- local signal: badge cluster attached to a row or gaze
- context signal rail: a compact `List` or `Group` at the lower-left of the current panel
- field signal board: a dedicated panel for collisions, neglect, urgency collisions, or review backlog

`NotificationQueue` and `Toast` are reserved for operator action outcomes such as “mutation applied” or “horizon invalid”, not for the domain itself.

## 7. Information Architecture

### 7.1 Layer 1: Field Scan

Purpose: rapid sensing across the field.

Primary widgets:

- `Panel`
- `VirtualizedList`
- `Badge`
- `Paragraph`
- `MiniBar`
- `Rule`
- `StatusLine`

What must be visible without opening anything:

- which tensions are advancing, stuck, oscillating, or colliding
- which horizons are compressing or overdue
- which tensions are neglected
- which roots or parents are structurally weak
- where the current selection sits in the tree

### 7.2 Layer 2: Focused Gaze

Purpose: make one tension legible as a live structure.

Primary widgets:

- `Panel`
- `Group`
- `Columns`
- `Paragraph`
- `Badge`
- `Rule`
- `Sparkline`
- `MiniBar`
- `List`

The gaze repeats the scan grammar but adds:

- full desired statement
- actual condition
- local child field
- signal rail
- recent structural motion
- available next moves

### 7.3 Layer 3: Full Analysis

Purpose: determine what is happening, why, and what move is warranted.

Primary widgets:

- `Panel`
- `Table`
- `HistoryPanel`
- `Sparkline`
- `ProgressBar`
- `Tree`
- `Paginator` on narrow terminals

The analysis layer adds:

- computed dynamics matrix
- event and mutation history
- projection and urgency collisions
- sibling competition and neglect inspection
- watch insights and agent suggestions

### 7.4 Additive disclosure contract

- scan shows symbols first
- gaze adds local explanation
- analysis adds comparison, history, and projection
- no deeper layer invents a new legend

## 8. Canonical Rendering Strategies for Structural Dynamics

| Dynamic | Scan rendering | Gaze rendering | Analysis rendering | `ftui` contract |
| --- | --- | --- | --- | --- |
| phase | phase `Badge` with glyph | header badge cluster | first row of dynamics `Table` | `Badge`, `Table` |
| tendency | terse badge `ADV` / `OSC` / `STAG` | badge plus short `Sparkline` | trend row with evidence | `Badge`, `Sparkline`, `Table` |
| structural tension magnitude | 5-cell `MiniBar` | 12-cell `MiniBar` with numeric label | `ProgressBar` with threshold coloring | `MiniBar`, `ProgressBar` |
| conflict | red local badge | dedicated conflict section below the main light rule | competitor table listing implicated siblings | `Badge`, `List`, `Table`, `Panel` |
| neglect | amber local badge | lower-left signal rail item | parent/child neglect table | `Badge`, `List`, `Table` |
| oscillation | `OSC` badge | sawtooth `Sparkline` and reversal count | oscillation row with window and magnitude | `Badge`, `Sparkline`, `Table` |
| resolution | green or amber badge | progress section comparing current and required velocity | `ProgressBar` plus trend row | `Badge`, `ProgressBar`, `Table` |
| horizon drift | dotted horizon badge | dotted section with drift token | drift row and projection table | `Badge`, `Rule`, `Table` |
| urgency | style change on temporal dots and horizon badge | urgency badge plus bar | urgency row and field-level collision table | `Paragraph`, `Badge`, `MiniBar`, `Table` |
| orientation | muted badge | badge near phase and tendency | dynamics row | `Badge`, `Table` |
| compensating strategy | no prose in scan; only amber signal if present | compact badge in signal rail | named row with evidence | `Badge`, `Table`, `List` |
| assimilation depth | phase-adjacent dot or badge | `DEEP` / `SHALLOW` badge | dedicated row | `Badge`, `Table` |
| status | terminal glyph badge | terminal glyph badge in header | event history markers | `Badge`, `HistoryPanel` |
| history | none beyond sparkline hint | last structural event line | full mutation/event stream | `Paragraph`, `Sparkline`, `HistoryPanel` |
| projection trajectory | hidden in scan unless risky | small trajectory badge | projection table and sparkline | `Badge`, `Table`, `Sparkline` |

## 9. Canonical Surface Compositions

### 9.1 Tension stripe

The field row is a two-line `VirtualizedList` item. This is the minimum unit that fully obeys the axis law.

Composition:

- line 1: desired statement, phase/status badge, horizon badge
- line 2: actual statement or temporal/reality excerpt, magnitude bar, local signal badges

Widget contract:

- outer item: `Group`
- inner alignment: `Columns`
- desired: `Paragraph`
- actual: `Paragraph`
- phase/status/tendency/drift: `Badge`
- magnitude: `MiniBar`
- time: fixed-width `Paragraph`

Rules:

- desired always sits on the upper row
- actual always sits on the lower row
- left edge carries current-state evidence
- right edge carries horizon and phase
- selected rows may grow to three or four lines; `VirtualizedList` handles this without changing the base grammar

### 9.2 Field plane

Composition contract:

`Panel(Group([header rule, VirtualizedList, optional dotted rule, signal rail, bottom status fragment]))`

Details:

- title is the parent desire or root charter
- `BorderType::Heavy` only when this plane is the active structural locus
- unpositioned children are separated from positioned children by a dotted `Rule`
- field-local alerts sit below the list, not above it

### 9.3 Focused gaze plane

Composition contract:

`Panel(Group([desired paragraph, light rule, badge cluster + mini bars, child preview list, light rule, actual paragraph, signal rail, recent motion sparkline]))`

This order is deliberate:

1. desired is sky
2. structural state sits in the middle
3. actual is ground
4. signals gather in the lower-left
5. motion and next move sit lower-right

### 9.4 Full analysis plane

Composition contract:

`Panel(Group([dynamics table, light rule, projection table, light rule, history panel]))`

Optional adjuncts:

- `Tree` for structural neighborhood
- `Table` for urgency collisions
- `Pretty` or `JsonView` for raw mutation/debug inspection in expert mode

### 9.5 Root field

When multiple roots exist, the instrument should make that structurally obvious.

Contract:

- root overview uses `Tree`
- each root node shows a badge cluster and temporal state
- “no senior organizing principle” is a field-level signal board item, not a toast

## 10. Complete Widget Mapping by Werk Concept

| Werk concept | Primary widget(s) | Configuration contract |
| --- | --- | --- |
| workspace field | `ResponsiveLayout`, `Grid`, `Panel` | default wide layout is two or three planes plus bottom `StatusLine` |
| root set | `Tree`, `Panel`, `Badge` | use `TreeGuides::Unicode` or `TreeGuides::Bold`; roots show signals inline |
| parent/child structure | `Tree` in overview, `VirtualizedList` in working field | overview optimizes topology; working field optimizes active operations |
| selected tension | `Panel`, `FocusIndicator` | active locus gets heavy border or explicit focus border |
| desired statement | `Paragraph` | always top-aligned inside its local surface; wraps in gaze, truncates in scan |
| actual statement | `Paragraph` | always bottom section; muted unless recent, urgent, or conflicting |
| horizon label | `Badge` | short natural-language label, cyan by default, amber/red under pressure |
| temporal indicator | fixed-width `Paragraph` or minimal `Badge` | six-dot grammar, right-aligned, no prose in scan mode |
| status | `Badge` | `✦` and `·` are terminal states, never mixed with active lifecycle glyphs |
| lifecycle phase | `Badge` | glyph plus short token, phase-colored but restrained |
| structural magnitude | `MiniBar`, `ProgressBar` | compact in scan, numeric in analysis |
| tendency | `Badge`, `Sparkline` | badge in scan, sparkline evidence in gaze and analysis |
| orientation | `Badge` | muted unless it shifts or becomes diagnostic |
| conflict | `Badge`, `List`, `Table` | local red badge, then explicit competitor listing |
| neglect | `Badge`, `List`, `Table` | signal rail item in gaze; analysis compares parent/child pattern |
| oscillation | `Badge`, `Sparkline`, `Table` | sparkline is required wherever oscillation is explained |
| resolution | `Badge`, `ProgressBar`, `Table` | render as ratio of actual to required velocity |
| assimilation | `Badge`, `Table` | deep assimilation can quietly lower signal intensity over time |
| compensating strategy | `Badge`, `Table`, `List` | never toast; only appears when analytically meaningful |
| horizon drift | `Badge`, dotted `Rule`, `Table` | dotted treatment is mandatory for postponement or instability |
| urgency | `Badge`, `MiniBar`, `Table` | style escalates before prose does |
| mutation history | `HistoryPanel` | chronological, terse, shares glyph grammar with the field |
| event stream | `HistoryPanel`, `LogViewer` | `LogViewer` is appropriate for live watch/hook traces |
| projection | `Table`, `Sparkline`, `Badge` | one row per horizon, one badge for trajectory |
| sibling competition | `Table`, `Tree` | show implicated siblings together, not as isolated badges |
| search/jump | `CommandPalette` | index tensions and commands in one surface; categories separate “jump” from “act” |
| inline filtering | `TextInput`, `ValidationErrorDisplay` | narrow filter bar above field or inside command palette |
| desire revision | `Modal`, `TextInput` or `TextArea` | desire edit modal prefers upper placement and rounded border |
| reality confrontation | `Modal`, `TextArea` | reality write surface prefers lower-left placement via `ModalPosition::Custom` where space permits |
| horizon editing | `Modal`, `TextInput`, `ValidationErrorDisplay` | parse failures stay local to the form |
| move/reparent | `Modal`, `Tree`, `List` | destination tree on left, confirmation summary on right |
| reorder siblings | `List` or `VirtualizedList`, `FocusManager` | focus border indicates drag/reorder locus; no bespoke drag art |
| agent suggestion review | `Modal`, `Table`, `Paragraph` | top section = proposed mutations, bottom section = rationale |
| watch insight review | `Modal`, `Table`, `Paragraph`, `Badge` | queue state is persistent; reviewed/unreviewed are badges |
| help | `Modal`, `HelpRegistry`, `Paragraph`, `Paginator` | help pages follow the same terminology as live surfaces |
| operator feedback | `NotificationQueue`, `Toast`, `StatusLine` | only action outcomes, never structural alerts |

## 11. Responsive Doctrine

The instrument degrades by removing breadth first, not meaning.

### 11.1 Breakpoint policy

| Breakpoint | Layout | Preserved invariants |
| --- | --- | --- |
| `Xl` | `Grid`: field left, gaze upper-right, analysis lower-right | full breadth and full depth visible at once |
| `Lg` | `Columns`: field left, gaze/analysis stacked right inside one panel | field remains primary; right side still means intent/depth |
| `Md` | `Flex::vertical`: field above, gaze below | vertical law becomes more explicit |
| `Sm` | single field plane; gaze and analysis move to `Modal` surfaces | top/bottom desired/actual split remains inside each modal |
| `Xs` | single active focus, optionally `ScreenMode::Inline`; analysis is paged | no secondary plane, but no semantic loss |

### 11.2 What collapses first

In order:

1. analytic comparison tables
2. secondary history surfaces
3. verbose horizon prose
4. local child previews

What never collapses:

- desired above actual
- left-ground / right-sky orientation
- phase/status glyphs
- temporal dots
- structural alerts

### 11.3 Narrow-screen tactics

- use `Visibility` to hide secondary panes, not core semantics
- use `Modal` for write surfaces and full analysis
- use `Paginator` for analysis sections on `Sm` and `Xs`
- keep the bottom `StatusLine`

## 12. Interaction Patterns

### 12.1 Focus and navigation

Use `FocusManager`, `FocusGraph`, and `FocusIndicator` so the focus model itself reflects structural direction:

- left: basis, ancestors, root overview, history
- right: child creation, projection, mutation review, action surfaces
- up: more intended or abstract strata within a surface
- down: more actual or grounded strata within a surface

Recommended focus styles:

- row focus: underline or subtle style overlay
- panel focus: border focus
- modal focus: border focus only

### 12.2 Selection

- field navigation uses `VirtualizedList`
- structural overview and move mode use `Tree`
- comparative diagnosis uses `Table`

The selected item is not merely highlighted; its depth surfaces become available in the right plane or modal.

### 12.3 Progressive disclosure

- hoverless terminal interaction means selection is the first reveal
- “open” does not switch metaphors; it promotes the selected structure into gaze
- “analyze” adds tables, history, and projections without hiding the gaze grammar

### 12.4 Command surfaces

`CommandPalette` is the primary command surface for:

- jump to tension
- confront reality
- revise desire
- set or revise horizon
- resolve or release
- move or reparent
- review agent or watch suggestions

Commands and tensions should live in one indexed space, separated by category rather than by entirely different UIs.

### 12.5 Input surfaces

Use the smallest writing surface that fits the operation:

- `TextInput` for short desire edits, horizon changes, filters
- `TextArea` for actual state, notes, agent prompts, and review comments
- `ValidationErrorDisplay` inline with the form

Placement:

- desire-writing surfaces prefer upper or upper-right placement
- reality-writing surfaces prefer lower-left placement where terminal size allows
- confirmation modals are centered and visually calmer than conflict alerts

### 12.6 Review modes

Agent and watch reviews are not plain paragraphs. They are structured review surfaces:

- mutation list in `Table`
- prose rationale in `Paragraph`
- accept/reject affordance in `StatusLine` and command palette
- history consequences accessible immediately in adjacent analysis

### 12.7 Help

Help should be built from `HelpRegistry` and rendered as a modal reference sheet. The help surface uses the same badge vocabulary and axis language as the instrument itself.

## 13. Structural Signal Hierarchy

### 13.1 Local

Signals directly attached to a tension:

- overdue
- drifting
- oscillating
- conflicting
- neglected

Rendering:

- badge cluster on the tension stripe
- repeated in the lower-left signal rail of the gaze plane

### 13.2 Context

Signals about the current field slice:

- multiple roots
- missing senior organizing principle
- urgency collisions
- clustered neglect
- review backlog

Rendering:

- `Panel` or `Group` below the field list
- compact `List` entries with badge-led markers

### 13.3 Action outcome

Signals about what the operator just did:

- mutation applied
- mutation rejected
- parse failed
- watch insight marked reviewed

Rendering:

- `NotificationQueue`
- `Toast`
- brief `StatusLine` confirmation

## 14. Mode Architecture

### 14.1 Primary modes

- field mode
- gaze mode
- analysis mode
- search/jump mode
- write reality mode
- revise desire mode
- move/reorder mode
- agent review mode
- watch review mode
- help mode

### 14.2 Mode presentation

- field and gaze are co-present when space allows
- analysis is a companion plane on wide screens and a modal or paged sheet on narrow screens
- search/jump is always a `CommandPalette`
- write and review modes are always `Modal`-based

The mode model should be visible in the bottom `StatusLine`, never hidden in implied behavior.

## 15. Recommended Implementation Direction for `werk-tui`

### 15.1 Replace custom painting with widget composition

Current manual rendering in `werk-tui/src/render.rs` should be re-expressed as:

- `ResponsiveLayout` for macro layout
- `Panel` + `Group` + `Columns` for composed surfaces
- `VirtualizedList` in place of the bespoke `vlist.rs`
- `Rule` for all separators, including dotted custom rules
- `Badge` for all compressed state tokens

### 15.2 Promote the current grammar into first-class tokens

The existing glyph and rule choices in `werk-tui/src/glyphs.rs` are directionally correct. They should become design-system tokens with stable meaning across scan, gaze, analysis, history, and review.

### 15.3 Introduce real operator surfaces

- use `CommandPalette` instead of plain search paragraphs
- use `Modal` + `Table` for agent mutation review
- use `Modal` + `Table` for pending watch insights
- use `HistoryPanel` for mutation history instead of plain wrapped text
- use `Sparkline` and `ProgressBar` where dynamics currently appear only as prose

### 15.4 Keep debug affordances separate

`JsonView`, `Pretty`, `LogViewer`, and debug overlays are valuable but should live behind expert or debug modes. They are not the public visual grammar.

## 16. Final Character

The instrument should feel:

- exact, not ornamental
- calm, not casual
- premium, not luxurious
- compressed, not cryptic
- serious, not solemn

The operator should be able to glance at the field and know:

- what is wanted
- what is true
- where motion exists
- where structure is slipping
- where collisions are forming
- what move is now warranted

If those facts are not visible from lines, badges, glyphs, dots, alerts, and panel position, the design has become too verbal.
