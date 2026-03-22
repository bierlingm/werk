# Werk Operative Instrument Design System

**Version:** 2.0
**Target:** `werk-tui`
**Framework Contract:** `ftui 0.2` only
**Premise:** The instrument should not decorate structural dynamics. It should render them as an operative field.

## 1. Position

This system reimagines `werk` as a premium operator instrument: severe, legible, exact, and calm under density. It is not a dashboard, not a notes app, and not a terminal website. It is a computational instrument for holding the gap between reality and desire.

The aesthetic reference is a deliberate fusion:

- Old terminals: phosphor restraint, ruled surfaces, hard alignment, symbolic compression.
- Updated terminals: responsive grids, overlays, command surfaces, notification systems, progressive disclosure.
- Operator tools: information hierarchy that assumes taste, focus, and repeated use.

The core move is directional. Every major screen in `werk` must make one thing unmistakable:

- `reality` is the ground, evidence, present condition, and lower plane.
- `desire` is the vector, aim, organizing image, and upper plane.
- `tensions` occupy the field between them.
- `dynamics` describe motion through that field.
- `alerts` identify where the structure is lying, drifting, colliding, or starving.

The interface is therefore a directional instrument, not a collection of views.

## 2. Non-Negotiables

### 2.1 Exclusive ftui Affordance Rule

This design system uses only existing `ftui` affordances:

- layout: `Grid`, `Flex`, `Responsive`, `Columns`
- structure: `Panel`, `Block`, `Group`, `Rule`
- navigation: `Tree`, `List`, `Table`, `VirtualizedList`, `Paginator`
- input: `TextInput`, `TextArea`, `CommandPalette`
- signaling: `Badge`, `StatusLine`, `Toast`, `NotificationQueue`, `HelpRegistry`, `Modal`
- telemetry and compression: `Sparkline`, `ProgressBar`, `HistoryPanel`

No bespoke painterly layer. No ad hoc ASCII scene engine. No custom visual system that bypasses the framework just to be expressive. If a concept cannot be expressed through `ftui` composition, it must be redesigned until it can.

### 2.2 Instrument Rules

- The content is the chrome.
- Borders mean containment, not decoration.
- Density must feel earned, never cramped.
- Color is for state transition, warning, or agency, never ambience.
- Every line should answer one of four questions: where am I, what matters, what is changing, what can I do now.

## 3. Spatial Model

The entire TUI is built on one spatial law:

**bottom/left = actuality, top/right = intentionality**

This law governs every view, even when rendered in a single column.

### 3.1 Axes

- Vertical axis: `reality -> desire`
- Horizontal axis: `local detail -> structural context`
- Depth axis: `ambient field -> focused panel -> modal commitment`
- Time axis: `older dots left -> newer dots right`

### 3.2 Dimensional Reading

Every operator-facing screen should reveal three dimensions at once:

- `position`: where the current tension sits in the structure
- `pressure`: how much force exists in the gap
- `direction`: whether motion is advancing, oscillating, stagnant, drifting, or colliding

The user should never need to ask which way is “forward.” Forward is always toward clearer and *realized* desire, more honest reality, and better structural arrangement.

## 4. Information Architecture

The instrument has four layers, always in this order:

1. `field`
2. `focus`
3. `act`
4. `trace`

### 4.1 Field

The field is the main operating surface. It shows the current tension landscape and the active direction of travel. This is the default mode and should be leave-open all day.

### 4.2 Focus

Focus is a contained intensification of one part of the field: gaze, detail, inline expansion, split-pane inspection, or edit panel.

### 4.3 Act

Act is where mutations happen: add, revise, move, resolve, release, invoke agent, apply suggestion, reorder structure.

### 4.4 Trace

Trace is the system’s memory made legible: mutation history, watch insights, event transitions, alerts, pulse, horizon pressure, and session residue.

## 5. Visual Grammar

`werk` should speak through a small number of recurring visual primitives.

### 5.1 Rules

- heavy rule: desire plane, commitment, fixed point
- light rule: reality plane, mutable condition
- dotted rule: ambiguity, unpositioned structure, not-yet-committed ordering
- vertical rule: trunk, descent, continuity through a parent tension

### 5.2 Glyphs

Status shape carries lifecycle:

- `◇` germination
- `◆` assimilation
- `◈` completion
- `◉` momentum
- `✦` resolved
- `·` released

Direction and force are rendered adjacent to, not instead of, the lifecycle glyph:

- `↑` increasing pressure
- `→` advancing
- `↔` oscillating
- `!` neglected or breached
- `⚠` alert requires action

### 5.3 Dots

Dots are the system’s smallest high-value unit. They appear in three roles only:

- temporal dots: time bucket history or pressure window
- presence dots: counts, unread insights, pending reviews
- confidence dots: low-word summaries in badges or lists

Dots are never ornamental filler.

### 5.4 Badges

Badges are compressed semantic declarations:

- phase
- tendency
- urgency tier
- conflict state
- watch insight type
- agent proposal count

Badges are used to compress nouns. Rules and dots carry relationships and time.

## 6. Tone and Material

The premium feel comes from restraint plus precision.

### 6.1 Surface Behavior

- Default surface is mostly open text with sparse framing.
- `Panel` is used only when content must become a contained work surface.
- `Modal` is for consequence, not convenience.
- `Toast` is for acknowledgement, not storytelling.

### 6.2 Typography

- Monospace only.
- Tabular alignment everywhere.
- Hard left edges and disciplined right-edge metadata.
- Titles do not shout. Weight comes from placement and spacing, not decorative styling.

### 6.3 Color

The palette should remain severe and mostly achromatic, with signal colors appearing as interventions.

- `ink.primary`: focused truth
- `ink.secondary`: structural context
- `ink.tertiary`: scaffolding
- `signal.advance`: cool active motion
- `signal.warn`: amber pressure
- `signal.alert`: red contradiction or breach
- `signal.agent`: cyan synthetic actuation
- `signal.resolution`: green earned movement

The current six-color palette is directionally correct. The system should formalize it, not expand it casually.

## 7. Core Views

## 7.1 The Field View

The field is the canonical daily view. It is not a list of records. It is a tension field.

At root, the operator sees a structured forest with immediate directional signals. When descended, the operator enters a parent tension’s operative channel: desire at the top, reality at the bottom, positioned tensions on the trunk between them.

### Field composition

- top context strip: breadcrumb, filter, structural counts
- main field: tree or virtualized list of tensions
- right or inline focus area at larger widths
- bottom lever: mode, pending alerts, watch count, agent state, key prompt

### Field behavior

- narrow: single column, no split, focus overlays inline
- medium: single column plus richer row metadata
- wide: split-pane with `Tree`/`VirtualizedList` on left and live focus panel on right
- ultra-wide: add secondary trace column for alerts/watch/history only if signal density warrants it

## 7.2 Descended Structural Channel

This is the signature `werk` layout and should feel like entering a live section through a structure.

Top:

- parent desire
- heavy rule

Middle:

- positioned children on a vertical trunk
- unpositioned children below a dotted boundary
- selected row opens a `Panel` or side focus depending on width

Bottom:

- light rule
- parent reality
- alert strip

The vertical trunk is not decoration. It is the directional backbone from current ground toward desired state.

## 7.3 Focus Panel

The focus panel is the operator’s workbench for a selected tension.

Use `Panel` with internal sections:

- header: title, phase badge, tendency badge, urgency badge
- definition: desire and reality
- dynamics: magnitude, tendency, conflict, neglect, drift, resolution
- structure: parent, children, siblings, ordering
- trace: recent mutations, watch insights, agent proposals

The panel should never feel like a form dump. Each section exists to inform the next act.

## 7.4 Command Surface

`CommandPalette` is not an extra feature. It is the high-bandwidth operator surface.

It should expose:

- navigation commands
- mutation acts
- structure redesign acts
- watch and insight review acts
- agent acts
- layout and filter acts

Every command should preview consequence in plain structural terms.

## 7.5 Watch / Pulse Surface

This is the ambient intelligence layer, rendered with `Table`, `Sparkline`, `Badge`, and `NotificationQueue`.

It should answer:

- what crossed a threshold
- what is drifting without attention
- where conflict is emerging
- where resolution is gaining force

This is not a generic observability dashboard. It is a structural listening surface.

## 8. Component System

## 8.1 Tension Row

The tension row is the atomic unit. It must render dense information without losing calm.

Row grammar:

`[cursor] [phase] [title] [badges...] [magnitude/progress] [dots] [right-edge temporal metadata]`

Possible composition with `ftui`:

- `Tree` row for hierarchical contexts
- `List`/`VirtualizedList` row for flattened or filtered contexts
- `Badge` for phase, tendency, urgency, conflict
- `Sparkline` or `ProgressBar` for magnitude and recent motion

Rules:

- title gets the most width
- right edge is reserved for time, urgency, horizon, or count
- selection changes weight first, then color
- resolved/released rows recede sharply

## 8.2 Desire / Reality Planes

These are not just text fields. They are anchoring planes in the instrument.

- `desire` uses a heavy upper separation and stronger text presence
- `reality` uses lighter separation and more grounded metadata
- both can expand into `TextArea` surfaces during edit mode

When shown together, the operator should feel the gap between them without explanatory prose.

## 8.3 Alert Strip

Alerts live below reality in descended view and at the right edge or footer elsewhere.

Each alert includes:

- severity glyph
- terse statement of structural condition
- direct action hint

Implementation:

- `Badge` for severity/type
- `Table` or stacked `List` rows for multiple alerts
- `Toast` for threshold crossings that deserve transient acknowledgement
- `NotificationQueue` for queued review items from watch or agent

Alerts are derived state. They should appear and disappear cleanly as structure changes.

## 8.4 Lever

The lever is the bottom `StatusLine`. It is the operator’s grip.

Left:

- path
- view
- filter

Center:

- immediate prompt, command echo, or contextual hint

Right:

- mode
- insight count
- review count
- watch status
- clock or heartbeat if useful

The lever should feel terse and trustworthy, not chatty.

## 8.5 Agent Review Card

Agent output should appear as reviewable structure, not as a blob of assistant prose.

Use `Panel` with:

- observation paragraph
- proposed structural acts as selectable rows
- consequence hints
- accept/reject shortcuts

Accepted acts should emit concise `Toast`s and land in trace/history immediately.

## 8.6 History / Trace

`HistoryPanel` is the native place for mutation memory, watch events, and agent-applied changes.

Structure trace by event class:

- state mutation
- structural transition
- dynamic threshold crossing
- watch insight
- agent action

The trace surface should teach the operator what kind of system they are inhabiting.

## 9. Responsive Doctrine

Responsiveness is not about squeezing the same dashboard into less space. It is about preserving the directional law under constraint.

### 9.1 Narrow

- single operational column
- field only
- focus appears inline or as modal
- badges collapse aggressively
- sparklines shorten or disappear before text does

### 9.2 Medium

- richer row metadata
- inline focus or optional split
- alert strip remains visible

### 9.3 Wide

- split field and focus with `Columns` or `Grid`
- live updates in focus panel as cursor moves
- watch and history can occupy subordinate zones

### 9.4 Degradation

At lower terminal capabilities:

- preserve hierarchy, rules, badges, and status text first
- simplify borders
- remove decorative emphasis
- reduce sparklines before removing alerts

The operator must never lose structural meaning because the terminal is worse.

## 10. Structural Dynamics Rendering

Each dynamic needs one canonical rendering strategy.

| Dynamic | Primary expression | Secondary expression |
|---|---|---|
| phase | lifecycle glyph + phase badge | row ordering/grouping |
| tendency | tendency badge | row accent/color |
| magnitude | `ProgressBar` | width-weighted row emphasis |
| conflict | alert badge + red marker | conflict section in focus panel |
| neglect | alert row + amber badge | watch queue item |
| oscillation | alternating dots/sparkline pattern | tendency badge |
| resolution | green badge or forward marker | trace event |
| orientation | group/header badge | focus metadata |
| compensating strategy | explicit warning section | trace event |
| assimilation depth | depth badge | focus metric row |
| horizon drift | drift badge | timeline/history entry |
| urgency | right-edge badge and/or pressure bar | row ordering |
| temporal pressure | combined urgency/magnitude emphasis | watch surface ranking |

No dynamic should require prose to become visible.

## 11. Modes

The system has a few modes and should admit them clearly.

- `normal`: move, inspect, invoke
- `insert`: type directly into a structural surface
- `review`: inspect alerts, watch items, agent proposals
- `command`: palette-first operation
- `reorder`: redesign structure spatially

Mode must be visible in the lever and reinforced by cursor/input treatment.

## 12. Interaction Principles

- Prefer inline, local acts before full-screen context switches.
- Descend to work inside structure; ascend to recover orientation.
- Review before consequence when the structure could be damaged.
- Fast paths should exist for repeated operator behavior.
- Every destructive or high-consequence act should be explainable in one line.

The instrument should feel closer to piloting or editing than to filling forms.

## 13. ftui Mapping

This is the binding contract from concept to framework.

| Werk concept | ftui primitive |
|---|---|
| primary field | `Tree` or `VirtualizedList` |
| row compression | `Badge`, `Sparkline`, `ProgressBar` |
| focus workbench | `Panel` |
| split layout | `Grid`, `Columns`, `Responsive` |
| bottom lever | `StatusLine` |
| quick operation surface | `CommandPalette` |
| editing | `TextInput`, `TextArea` |
| structural alerts | `Badge`, `List`, `Toast`, `NotificationQueue` |
| trace/history | `HistoryPanel`, `Table` |
| confirmation | `Modal` |
| help legend | `HelpRegistry` |

This table should guide implementation decisions. If a feature proposal does not map cleanly onto this table, it is probably the wrong feature.

## 14. Implementation Direction

The current code still carries too much manual paragraph composition. The redesign should move toward compositional `ftui` surfaces in this sequence:

1. formalize tokens and badges
2. replace ad hoc field rows with a canonical row renderer
3. make descended view use real structural panels and rules
4. install split-pane field/focus layout at wide widths
5. move alerts and watch into `NotificationQueue`/trace surfaces
6. route all high-bandwidth action through `CommandPalette`
7. standardize help, review, and modal consequence patterns

## 15. Standard of Quality

The operator should feel all of this immediately:

- the tool knows what is foreground and background
- the tool knows which direction reality and desire run
- the tool makes tension, drift, collision, and progress legible without verbosity
- the tool rewards repeated use
- the tool feels serious enough to hold a life structure without becoming solemn or heavy

This is the bar:

**a terminal instrument whose lines, badges, glyphs, dots, alerts, and panels make structural dynamics feel native to computation**
