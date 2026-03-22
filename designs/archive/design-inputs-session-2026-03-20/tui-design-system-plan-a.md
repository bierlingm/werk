# Werk TUI Design System Plan Alternate

Status: V1 alternate synthesis for Phase 1 plan-space
Date: 2026-03-19
Owner: Codex
Output intent: a mechanically executable design-system plan that can be decomposed into swarm-safe beads without reopening source ambiguity

Certainty markers used in this document:
- `Locked`: decision is binding for downstream work unless a later ADR explicitly supersedes it.
- `Provisional`: decision is the best current synthesis, but should be revalidated by a bounded prototype before broad rollout.
- `Open`: unresolved on purpose; implementers must not guess.

This document synthesizes:
- Five competing design-system visions from `werk-design-inputs/`
- Three post-mortem reflections from `werk-design-inputs/`
- The living codebase in [`/Users/moritzbierling/werk/desk/werk/werk-tui/src`](/Users/moritzbierling/werk/desk/werk/werk-tui/src)
- The core domain model in [`/Users/moritzbierling/werk/desk/werk/sd-core/src`](/Users/moritzbierling/werk/desk/werk/sd-core/src)
- Shared agent/workspace types in [`/Users/moritzbierling/werk/desk/werk/werk-shared/src`](/Users/moritzbierling/werk/desk/werk/werk-shared/src)
- Prior internal design context in [`/Users/moritzbierling/werk/desk/werk/designs`](/Users/moritzbierling/werk/desk/werk/designs)
- The actual `ftui` framework surface shipped to this repo via `ftui 0.2.1`

## 1. Mission

The purpose of this plan is not to produce another aspirational design essay. It is to freeze the expensive decisions early enough that later bead execution does not fork the product into multiple incompatible TUIs.

The plan therefore does five things at once:
- chooses a product stance
- locks the high-cost visual and spatial laws
- maps every meaningful werk concept to a canonical rendering contract
- names the actual `ftui` widgets and composites that will carry those concepts
- decomposes the result into atomic work units that can be executed without rereading the entire planning corpus

The plan is intentionally stricter than the source documents. Where the source set was poetic, this plan becomes binding. Where the source set was vague, this plan either decides or preserves the ambiguity as an explicit open question.

## 2. Scope

In scope:
- the daily interactive TUI in [`/Users/moritzbierling/werk/desk/werk/werk-tui/src`](/Users/moritzbierling/werk/desk/werk/werk-tui/src)
- the rendering and interaction contract for tensions, dynamics, horizons, projections, watch insights, and agent review
- the widget binding contract between werk concepts and `ftui`
- the responsive doctrine at 80, 120, and 160+ columns
- the component taxonomy, state slices, testing strategy, and migration path
- bead decomposition for implementation

Out of scope except where it touches the TUI:
- broad CLI redesign
- MCP architecture
- hook system design beyond the surfaces that appear inside the TUI
- sd-core algorithm changes except where a TUI surface depends on a new derived field or fixture
- final theme variants beyond the semantics contract

## 3. Product Stance

Status: `Locked`

The synthesis decision is:

`werk-tui` is a daily field instrument with on-demand depth.

It is not primarily:
- a dashboard
- a tree explorer
- a command console
- a reporting surface
- a permanent analytics cockpit

It is also not a minimal todo list. The domain already computes rich structure. The correct synthesis is:
- repeated daily operation is the design center
- deep structural analysis is available without polluting the first screen
- watch and agent systems are first-class advisory systems, not primary always-on screen owners
- topology and comparative analysis are secondary surfaces, not the default working plane

This product stance resolves the biggest cross-document ambiguity. It also means any later design that optimizes the first paint for “maximum information density” at the cost of immediate confrontation with the field should be treated as a regression.

## 4. Ground Truth From the Current Codebase

The plan is grounded in the current state of the repo rather than the names in the user brief.

### 4.1 Actual relevant directories

- TUI implementation: [`/Users/moritzbierling/werk/desk/werk/werk-tui/src`](/Users/moritzbierling/werk/desk/werk/werk-tui/src)
- Core domain: [`/Users/moritzbierling/werk/desk/werk/sd-core/src`](/Users/moritzbierling/werk/desk/werk/sd-core/src)
- Shared types: [`/Users/moritzbierling/werk/desk/werk/werk-shared/src`](/Users/moritzbierling/werk/desk/werk/werk-shared/src)
- Design context: [`/Users/moritzbierling/werk/desk/werk/designs`](/Users/moritzbierling/werk/desk/werk/designs)

### 4.2 Actual current UI shape

The current app already contains the seed of the winning system:
- field-first descended navigation
- strong glyph family in [`/Users/moritzbierling/werk/desk/werk/werk-tui/src/glyphs.rs`](/Users/moritzbierling/werk/desk/werk/werk-tui/src/glyphs.rs)
- a restrained six-color palette in [`/Users/moritzbierling/werk/desk/werk/werk-tui/src/theme.rs`](/Users/moritzbierling/werk/desk/werk/werk-tui/src/theme.rs)
- a gaze concept
- alert indexing
- watch insight review
- agent mutation review
- a vim-leaning interaction model

The current app also contains the main architectural debt:
- large amounts of manual paragraph composition in [`/Users/moritzbierling/werk/desk/werk/werk-tui/src/render.rs`](/Users/moritzbierling/werk/desk/werk/werk-tui/src/render.rs)
- bespoke variable-height list logic in [`/Users/moritzbierling/werk/desk/werk/werk-tui/src/vlist.rs`](/Users/moritzbierling/werk/desk/werk/werk-tui/src/vlist.rs)
- implicit semantics spread across layout arithmetic instead of codified component contracts
- a design language that exists in practice, but not yet as a formal system

### 4.3 Actual `ftui` surface available

The repo does not vendor `ftui`, but `Cargo.lock` and the local cargo registry confirm `ftui 0.2.1`.

Important verified `ftui` capabilities:
- layout: `Flex`, `Grid`, `Responsive`, `ResponsiveLayout`, `Visibility`
- widgets: `Panel`, `Block`, `Rule`, `Badge`, `Paragraph`, `List`, `VirtualizedList`, `Table`, `Tree`, `Sparkline`, `ProgressBar`, `HistoryPanel`, `StatusLine`, `Modal`, `Toast`, `NotificationQueue`, `CommandPalette`, `TextInput`, `TextArea`
- layout breakpoints: default `60/90/120/160`, overridable

Implication:
- the design system can and should be built from `ftui` composition
- a small number of werk-specific composite widgets are still justified
- raw ad hoc view rendering should shrink dramatically

## 5. Synthesis Map

### 5.1 Competing design-system sources

| Source | Strongest Ideas | Gaps / Hand-Waving | Unique Contributions | Contradicts Others On |
|--------|-----------------|--------------------|----------------------|------------------------|
| `DESIGN_SYSTEM-claude.md` | Most implementation-detailed widget mapping, explicit state machine and input routing, strong progressive heights model, concrete mutation review surfaces, serious polish/accessibility pass | Assumes some widget behaviors before proving them, especially heterogeneous expansion and nested interaction surfaces; leans richer than the product stance can always afford | Rect-slicing/dispatch framing, panel-based editing flows, explicit quality and edge-case checklist | More willing than others to treat panel-heavy surfaces and richer chrome as default structure |
| `DESIGN_SYSTEM-codex.md` | Strongest framework contract, strongest operating laws, clearest separation of primary vs diagnostic surfaces, additive disclosure doctrine, direct migration guidance away from bespoke rendering | Less expansive about emotional tone and premium feel than the more aesthetic docs; leaves some experiential details intentionally austere | Allowed affordance taxonomy, structural signal hierarchy, explicit “keep debug separate” discipline | More restrictive than prior docs on what can be primary; rejects tables/trees as everyday default surfaces |
| `DESIGN_SYSTEM-gemini.md` | Cleanest compression of mirror philosophy, spatial law, three-depth model, and graceful degradation; good at keeping the system legible and non-baroque | Most inferred from existing renderer rather than verified against actual `ftui`; less complete on edge-state handling and mode architecture | The clearest short-form articulation of “Instrument as Mirror” and “Structural Degradation” | More minimal than Claude; less opinionated about review/workflow surfaces than prior docs |
| `DESIGN_SYSTEM.md` | Broadest full-surface coverage including help, mutation review, insight review, design tokens, anti-patterns, and phased migration | Mixes conceptual design, implementation notes, and view-level detail in a way that leaves some core invariants softer than they should be | Most complete prior surface inventory; useful bridge between today’s renderer and a formal system | Keeps some view assumptions and width-capped habits that can conflict with a stricter ftui-first component contract |
| `k-operative-design-system.md` | Strong non-negotiables, explicit ftui-only rule, clear field/focus/act/trace architecture, strong component taxonomy, good responsive doctrine language | Less detailed on state transitions and advanced flows; some abstractions need operational mapping before coding | Act/trace framing, alert strip framing, strong component nouns, high-quality design-system posture | More formal multi-surface IA than the docs that prefer a tighter field/gaze/analysis triad |

### 5.2 Source-weighted synthesis judgment

What the competitors did best:
- Claude did best at implementation texture.
- Codex did best at laws, contracts, and migration realism.
- Gemini did best at preserving conceptual clarity under constraint.
- The prior comprehensive doc did best at full-surface completeness.
- K-operative did best at naming a reusable design-system structure rather than a one-off UI.

What no single source did alone:
- lock the product stance tightly enough
- attribute framework uncertainty precisely enough
- separate sacred core from reusable components and extras
- translate every good idea into execution-grade beads

## 6. Reflection Synthesis

### 6.1 Convergent questions flagged by 2+ agents

These are real churn magnets because multiple independent reflections surfaced them.

1. What is the primary product loop?
- Daily field steering or periodic deep analysis?
- Is the default experience breadth-first scanning or depth-first study?
- Are watch and agent functions central or episodic?

2. What exactly is fixed about spatial direction?
- Everyone agreed on `desired` above `actual`.
- Not everyone agreed how absolute the horizontal law should be.
- Narrow widths force the question: can left/right semantics disappear without semantic loss?

3. How much analysis belongs on screen by default?
- The domain justifies a lot.
- Repeated use punishes analytical sprawl.
- This is the most important density decision after product stance.

4. Can `ftui` handle the gaze model cleanly?
- Variable-height inline rows remain the highest-risk framework question.
- A bespoke list already exists, which implies the framework fit is not yet proven in practice.

5. Which signals deserve persistence?
- Conflict and neglect were universally treated as primary.
- Projections, compensating strategies, and some higher-order analytics were not.

6. How should complex workflows enter the instrument?
- Inline?
- modal?
- palette?
- dedicated review surface?

7. How much of the premium aesthetic survives real terminals?
- Unicode glyph fidelity
- color degradation
- border consistency
- keyboard variance

### 6.2 Decision hierarchies extracted from the reflections

Claude’s implied order:
1. Catalogue and test actual `ftui` widgets
2. Lock terminal capability assumptions
3. Build rect/layout/rendering primitives
4. Lock navigation and event routing
5. Only then build progressive disclosure and interaction features

Codex’s implied order:
1. Lock product stance
2. Lock directional invariants
3. Lock token semantics
4. Lock information architecture
5. Lock modes and state architecture
6. Lock migration path

Gemini’s implied order:
1. Lock the spatial metaphor
2. Lock glyphs and the three-depth model
3. Verify framework fit for those shapes
4. Harden with snapshots and performance discipline

Synthesis decision:
- use Codex’s order for conceptual decisions
- use Claude’s order for implementation risk reduction
- use Gemini’s discipline as the compression filter

### 6.3 Framework gaps identified by the reflections and codebase review

Confirmed or likely gaps:
- `VirtualizedList` must be proven with heterogeneous row heights before becoming the only field implementation
- gaze expansion ergonomics are not yet proven in actual app code
- focus trapping and input ownership for agent review flows need an explicit modal stack contract
- terminal fallback behavior for glyphs and borders is not yet codified
- there is no local widget gallery or storybook for `ftui`
- current app still relies on manual `Paragraph` assembly for several complex surfaces

### 6.4 Hardening recommendations converged across sources

These recommendations are elevated from “nice to have” to planning inputs:
- create a canonical token specification before broad implementation
- create a component contract for each primary surface
- create scenario fixtures for golden states
- add width-based invariant tests
- document what `ftui` can do, what needs a proof of concept, and what requires adaptation
- use domain-specific composite widgets to prevent view code from collapsing back into ad hoc text composition
- treat the next implementation pass as a structured prototype with strong snapshots, not as the final irreversible build

## 7. Best-of-All-Worlds Synthesis

Status: `Locked`

The winning hybrid keeps the best of each plan and discards each plan’s failure mode.

### 7.1 What is retained

From Claude:
- exacting widget mapping
- explicit input-routing seriousness
- strong review/workflow surfaces
- practical polish checklist

From Codex:
- framework contract over bespoke heroics
- additive disclosure instead of mode-replacing churn
- persistent structural signals
- a hard boundary between operator surfaces and diagnostics

From Gemini:
- the mirror thesis
- minimal first paint
- structural degradation instead of decorative collapse
- a compact, memorable core grammar

From the prior comprehensive doc:
- comprehensive coverage of real surfaces already present in the app
- leverage of the existing glyph and color language
- migration awareness

From K-operative:
- strong system nouns
- component system posture
- explicit non-negotiables
- responsiveness as doctrine, not patchwork

### 7.2 What is discarded

- any design that makes the first screen a dashboard
- any design that treats the tree as the primary working surface
- any plan that assumes a rich multi-pane workspace before locking the field instrument
- any attempt to represent every domain nuance as a permanent badge
- any renderer architecture that leaves semantics embedded in handcrafted strings

### 7.3 Final synthesis statement

The alternate design system is:

- a field-first instrument
- with three additive depth layers
- governed by a fixed vertical law
- using a restrained, stable glyph and color vocabulary
- implemented via `ftui` primitives and werk-specific composites
- with persistent local signals and advisory queues
- responsive by collapsing analysis before collapsing meaning
- explicitly testable through golden states and breakpoint invariants

## 8. Architectural Decision Records

### ADR-01: Spatial Law

Status: `Locked`

Decision:
- The vertical axis always encodes `actual/reality` below `desired/desire`.
- This invariant is absolute across every width, mode, and surface.
- The horizontal axis is canonical but collapsible: left means basis, history, grounded context, or already-known condition; right means intention, implication, action, or projection.
- When width is insufficient, horizontal semantics may collapse into vertical stacking; they may not invert.

Alternatives considered:
- Claude: strong vertical law, more flexible panel arrangements
- Codex: vertical law plus strong left=basis/right=intent doctrine
- Gemini: reality as ground, desire as sky, less emphasis on horizontal permanence
- Prior comprehensive doc: directional semantics present but less formally locked
- K-operative: axes and dimensional reading explicitly framed

Rationale:
- Every source agreed on the vertical law.
- The current app already renders desire above reality in gaze and descended contexts.
- Horizontal semantics are valuable for analysis and action grouping, but cannot justify layout breakage on narrow terminals.

Consequences:
- every component spec must preserve desire-above-reality ordering
- analysis layouts can move sections vertically but never swap the relation
- narrow layouts collapse width before violating axis meaning
- snapshot tests must assert this invariant at 80, 120, and 160+

### ADR-02: Depth Model

Status: `Locked`

Decision:
- There are exactly three depth layers:
- `Field`: single-line scan surface
- `Gaze`: inline 3-5 line expansion in context
- `Analysis`: dedicated full panel or full-screen analytical surface
- Disclosure is additive in semantics, not necessarily simultaneous in rendering.
- `Gaze` never replaces the field row; it expands beneath it.
- `Analysis` may replace the field on narrow widths, or coexist on wide widths.

Alternatives considered:
- Claude: rich progressive heights and possible quick/full gaze variants
- Codex: explicit additive disclosure contract
- Gemini: line → gaze → study as a clean triad
- Prior comprehensive doc: tension line, gaze card, full gaze
- Older internal docs: some used dashboard/detail/tree view taxonomy

Rationale:
- Three layers were the strongest consensus across the source set.
- The current app already has line, gaze, and full-gaze behavior.
- Four or more depth layers would overfit the current implementation and increase user memory load.

Consequences:
- every domain concept must declare visibility by depth
- view code must never invent a fourth quasi-depth without a new ADR
- keybindings must map cleanly to three depth transitions
- responsive rules collapse from analysis outward, never from field inward

### ADR-03: Glyph System

Status: `Locked`

Decision:
- Lifecycle phases are rendered with single-character Unicode glyphs:
- `◇` germination
- `◆` assimilation
- `◈` completion
- `◉` momentum
- Empty / unavailable / filler uses `·`
- Activity trail uses six single-cell dots:
- `●` mutated at this time bucket
- `○` no significant mutation in this time bucket
- Tendency glyphs are:
- `→` advancing
- `↔` oscillating
- `·` stagnant / no directional evidence
- Terminal status is expressed primarily through style and explicit badges, not alternate lifecycle glyphs.

Alternatives considered:
- Existing app: tendency uses `→ ↔ ○`
- Claude: lifecycle + status encoding pushes toward more glyph overload
- Codex: lifecycle glyphs, terminal state glyphs, temporal dots, badge classes as a full system
- Gemini: stable lifecycle glyphs and temporal trail
- Prior docs: the same lifecycle family plus auxiliary dots and rule weights

Rationale:
- The existing phase glyph family is already strong and repeatedly endorsed.
- Using `·` for stagnation prevents confusion between tendency and trail dots.
- Keeping status out of the leftmost glyph reduces semantic overload and preserves fast phase recognition.

Consequences:
- `glyphs.rs` becomes a formal token table rather than a convenience file
- tendency must appear in a predictable secondary slot, not ad hoc
- ASCII fallback mapping must exist before implementation closes
- future additions must use badges or labels, not new primary glyph families

### ADR-04: Color Semantics

Status: `Locked`

Decision:
- The design system keeps a restrained six-color semantic palette:
- `default`: primary content
- `dim`: chrome, labels, inactive or resolved context
- `cyan`: selection, focus, agent/advisory accent, gaze emphasis
- `amber`: caution, neglect, stagnation, horizon tightening, overdue-but-recoverable concern
- `red`: conflict, breach, destructive warning, severe review state
- `green`: healthy motion, positive resolution, confirmed progress
- No new semantic hue may be introduced without an ADR.
- Monochrome and low-color degradation must map these semantics to style flags and contrast rather than hue alone.

Alternatives considered:
- Claude: richer premium-aesthetic emphasis
- Codex: tightly disciplined color doctrine
- Gemini: semantic declarations with restraint
- Prior comprehensive doc and current `theme.rs`: already close to this palette

Rationale:
- The current app already uses this exact six-color family successfully.
- A larger palette would create attractive but expensive ambiguity.
- The instrument should feel precise, not decorative.

Consequences:
- styles must be semantic, not screen-specific
- component contracts must reference semantic roles, not raw RGBA values
- theming is possible later, but only if it preserves the semantic role table
- width and glyph degradation tests must also test color degradation

### ADR-05: Widget Binding Contract

Status: `Locked`

Decision:
- Every rendered element must map either:
- directly to an `ftui` widget type
- or to a named werk composite widget built only from `ftui` layout primitives and widgets
- View modules may orchestrate layout and choose composites, but may not become bespoke string-painting engines.
- `Paragraph` remains allowed as a primitive widget, but only inside a named component contract.
- Current ad hoc width arithmetic in top-level render code is migration debt, not a precedent.

Alternatives considered:
- Claude: detailed rendering dispatch with more top-level composition
- Codex: strongest insistence on framework contract and diagnostic separation
- Gemini: inferred direct widget mapping from current renderer
- K-operative: exclusive ftui affordance rule
- Current app: manual `Paragraph` composition in multiple large surfaces

Rationale:
- This is the single most important architectural discipline for reducing rework.
- Without it, every surface will re-encode semantics in custom text layout.
- `ftui` is broad enough to support the desired system with a thin layer of composites.

Consequences:
- introduce a `components/` or equivalent module layer before broad redesign
- every component spec must name its input model, widget tree, and breakpoint behavior
- rendering tests should target components first, views second
- migration can proceed incrementally without semantic drift

### ADR-06: Responsive Doctrine

Status: `Locked`

Decision:
- App-level target widths are:
- `80-119`: compact
- `120-159`: standard
- `160+`: expanded
- `<80` is emergency fallback only and not a stop-ship design target
- Collapse order is fixed:
- analysis density collapses first
- side-by-side coexistence collapses second
- auxiliary labels collapse third
- glyphs, ordering, and primary signals collapse last
- The field remains the center of gravity at every width.

Alternatives considered:
- Claude: more aggressive width-based adaptation tiers
- Codex: “what collapses first” spelled out clearly
- Gemini: structural degradation as explicit doctrine
- Prior v0.5 plan: split pane activated at 120+
- K-operative: narrow/medium/wide doctrine

Rationale:
- 120 is a design target, but making it the mandatory split-pane threshold biases the product back toward dashboard behavior.
- 160+ is the first width where persistent coexistence of field and analysis becomes comfortably sustainable.
- A doctrine is better than ad hoc hiding rules.

Consequences:
- the app should override `ftui` breakpoints rather than rely on defaults blindly
- compact layouts must still express phase, selection, trail, and primary signal state
- expanded layouts may pin analysis, but cannot demote the field to a sidebar
- visual snapshots must exist for compact, standard, and expanded tiers

### ADR-07: Alert Architecture

Status: `Locked`

Decision:
- Alerts are mixed but hierarchical:
- local structural alerts live inline with the relevant tension or descended context
- field-level aggregate alert counts live in the lever/status surface
- action review lives in dedicated review cards or modals
- transient acknowledgements may use toast/notification surfaces, but structural alerts never rely on them
- Number-key shortcuts may target visible actionable alerts in context.

Alternatives considered:
- Claude: badges and numbers, alert mapping to direct actions
- Codex: structural signal hierarchy, local/context/action outcome layers
- Prior comprehensive doc: alert section plus insight review
- K-operative: alert strip
- `werk-watch.md`: silent advisory queue, not push notification

Rationale:
- Structural signals should persist until structurally cleared.
- Pure overlays or toasts would violate the mirror metaphor.
- A mixed hierarchy preserves seriousness without flooding the field.

Consequences:
- alert specifications must distinguish persistent from ephemeral states
- toasts are reserved for acknowledgements, undo windows, and recoverable action results
- watch insights remain advisory queue items, surfaced in lever and dedicated review
- the field must show conflict/neglect/overdue status without opening analysis

### ADR-08: Interaction Model

Status: `Locked`

Decision:
- The instrument is vim-first with universal aliases, not emacs-first and not palette-only.
- Canonical movement:
- `j/k` or arrows move selection
- `l` / `Enter` descends or activates the selected structural target
- `h` / `Backspace` ascends or backs out
- `Space` toggles gaze
- `Tab` opens or cycles analysis-context surfaces
- `Esc` dismisses overlays and returns toward field context
- `:` opens the command palette for infrequent or global actions
- direct-key acts remain primary for common structural gestures

Alternatives considered:
- Claude: navigation state machine plus rich mode routing
- Codex: direct-key operations first, palette second
- Gemini: gesture-oriented acts
- Prior docs: vim-like movement with limited overlays
- v0.5 master plan: more dashboard-style view cycling

Rationale:
- The current app already trains a viable muscle-memory pattern.
- The instrument metaphor rewards repeatable direct gestures.
- The command palette is valuable as a discoverability and expert layer, not as the main steering surface.

Consequences:
- the help surface and status line must expose the canonical vocabulary
- palette commands must mirror direct actions, not invent parallel semantics
- mode routing must be explicit and testable
- any future alternative key scheme would require a distinct ADR

## 9. Binding Constraints

Status: `Locked`

These constraints are binary and testable.

1. Every visual element maps to a named `ftui` widget or named composite widget built only from `ftui`.
2. The vertical axis always encodes `actual` below `desired`.
3. No surface may place `actual` above `desired` even in compact fallback.
4. The first interactive surface the user sees is always the field, never a report, tree, or full analysis pane.
5. The field row remains one logical row per tension at depth 0.
6. The gaze expands beneath the selected row and never replaces it.
7. The analysis layer may replace the field on compact widths but may not redefine the field’s semantics.
8. Lifecycle phase is always represented by one single-cell glyph at the left edge of the tension stripe.
9. Activity history uses exactly six time buckets by default.
10. Phase glyphs remain single-character Unicode; no multi-character phase tokens.
11. Tendency glyphs remain single-character Unicode; no text labels at depth 0.
12. Status is not encoded by swapping lifecycle glyphs.
13. The semantic color system is capped at six roles unless a new ADR is accepted.
14. Structural alerts persist in view until structurally cleared; they are never toast-only.
15. The lever/status surface is always present.
16. Search, help, edit, review, and palette surfaces must have explicit focus ownership.
17. Watch insights are advisory-only and cannot auto-apply mutations.
18. Agent proposals are always reviewable before application.
19. The design target supports 80 columns minimum and 120 columns comfortably.
20. Persistent side-by-side field + analysis coexistence is reserved for 160+ columns.
21. `Tree` is a secondary exploration surface, not the primary working surface.
22. `Table` is allowed in analysis and review surfaces, not as the default root field rendering.
23. Every domain concept named in this plan must declare a canonical default depth and widget mapping.
24. No direct terminal painting in top-level view functions for semantic content after migration completes.
25. Every breakpoint tier must preserve selection visibility and primary structural signal visibility.
26. No overlay may open without a deterministic dismissal path.
27. Every action that changes state must surface either persistent structural change or ephemeral confirmation.
28. Resolved and released states remain representable, but active tensions dominate the default field.
29. Debug and diagnostic surfaces must remain separable from the operator’s primary loop.
30. Any deviation from these constraints requires an ADR, not a convenience exception.

## 10. Stop-Ship Criteria

The design-system work is not complete until all of the following are true:

- [ ] Every werk domain concept has a canonical rendering specification.
- [ ] Every rendering specification maps to specific `ftui` widget(s) or named composites with configuration intent.
- [ ] Glyph, dot, rule, and badge tables are complete and unambiguous.
- [ ] Color/style semantics are complete and expressed as semantic names rather than raw values.
- [ ] Responsive behavior is specified for compact, standard, and expanded widths.
- [ ] At least one integrated full-screen ASCII mockup demonstrates the system at design target width.
- [ ] A developer who has not read the prior design docs can implement components from the bead set alone.

## 11. Stability Rings

### Ring 1: Sacred Core

Changes here invalidate downstream work.

- product stance
- spatial law
- depth model
- glyph and token system
- widget binding contract
- alert hierarchy
- interaction vocabulary
- responsive collapse order

### Ring 2: Reusable Components

Changes here are costly but mostly localizable.

- lever/status surface
- signal rail
- tension stripe
- gaze card
- analysis sections
- review cards
- input surfaces
- search/help overlays
- tree and history surfaces

### Ring 3: Feature-Gated Extras

Changes here should not destabilize the core.

- time-travel replay
- field resonance overlays
- advanced projection panels
- alternate themes
- command-palette enrichment
- persistent pinning behaviors
- experimental panes

## 12. Canonical Token System

### 12.1 Breakpoint contract

Status: `Locked`

App-level tiers:

| Tier | Width | Intent |
|------|-------|--------|
| Compact | `80-119` | single dominant field, overlays replace |
| Standard | `120-159` | more breathing room, richer rows and rails, no required persistent analysis |
| Expanded | `160+` | optional pinned analysis or dual-plane coexistence |

Implementation note:
- use `ftui_layout::Breakpoints::new_with_xl(80, 120, 160, 200)` or an equivalent explicit app breakpoint model
- do not inherit `60/90/120/160` blindly because the design target begins at 80

### 12.2 Rule semantics

Status: `Locked`

| Token | Meaning | Primary use |
|-------|---------|-------------|
| Heavy rule | plane boundary or analysis section start | analysis pane boundaries, descended header boundary |
| Light rule | local grouping | gaze internals, section subdivision |
| Dotted rule | provisional or contextual expansion | gaze envelope, inline expansion edges |

Binding rules:
- no more than three rule weights
- decorative rules are forbidden
- rule meaning must stay stable across widths

### 12.3 Phase glyph table

Status: `Locked`

| Glyph | Phase | Meaning |
|-------|-------|---------|
| `◇` | Germination | open, forming, not yet solid |
| `◆` | Assimilation | active digestion, consolidating work |
| `◈` | Completion | nearing closure, textured finish |
| `◉` | Momentum | fully engaged, radiating activity |

### 12.4 Tendency glyph table

Status: `Locked`

| Glyph | Tendency | Meaning |
|-------|----------|---------|
| `→` | Advancing | movement aligned with closure |
| `↔` | Oscillating | reversing or unstable progress |
| `·` | Stagnant / inconclusive | insufficient or flat movement |

### 12.5 Activity trail table

Status: `Locked`

| Glyph | Meaning |
|-------|---------|
| `●` | significant mutation occurred in the bucket |
| `○` | no significant mutation occurred in the bucket |

Rules:
- six buckets by default
- newest bucket is rightmost
- trail survives all supported widths
- bucket interval is stable per view model, not recomputed ad hoc in rendering

### 12.6 Status badge classes

Status: `Locked`

Badge classes allowed in the system:

| Class | Semantic role | Typical style |
|-------|----------------|---------------|
| `status-active` | currently active tension | default or cyan-accented when selected |
| `status-resolved` | closed by attainment | dim + green accent |
| `status-released` | consciously abandoned/released | dim + amber or dim only |
| `signal-conflict` | conflict present | red |
| `signal-neglect` | neglect present | amber |
| `signal-overdue` | horizon breached or critical urgency | amber or red depending severity |
| `signal-agent` | agent-derived advisory | cyan |
| `signal-watch` | watch backlog item | cyan |
| `signal-projection` | trajectory risk or forecast note | amber or cyan depending framing |

Rules:
- badges are for declarations and structural signals, not for all metadata
- badges are secondary at depth 0 and primary only in review or analysis surfaces

### 12.7 Color semantics

Status: `Locked`

| Semantic name | Meaning | Existing seed |
|---------------|---------|---------------|
| `fg_default` | primary text and active content | `CLR_DEFAULT` |
| `fg_dim` | chrome, labels, resolved context | `CLR_DIM` |
| `fg_cyan` | focus, agent, selected emphasis, advisory | `CLR_CYAN` |
| `fg_amber` | caution, neglect, tightening urgency | `CLR_AMBER` |
| `fg_red` | conflict, breach, destructive risk | `CLR_RED` |
| `fg_green` | healthy progression and positive confirmation | `CLR_GREEN` |

Non-hue semantics:
- selection may use background contrast
- bold is reserved for active attention, not generic emphasis
- reverse/highlight styles are interaction affordances, not semantic categories

### 12.8 ASCII fallback table

Status: `Provisional`

| Unicode | ASCII fallback |
|---------|----------------|
| `◇` | `o` |
| `◆` | `*` |
| `◈` | `#` |
| `◉` | `@` |
| `→` | `>` |
| `↔` | `~` |
| `·` | `.` |
| `●` | `*` |
| `○` | `o` |

Reason for provisional status:
- terminal capability testing has not yet been run across the actual terminal matrix

## 13. Canonical Werk Concept Rendering Specification

This section is the contract that turns domain vocabulary into stable rendering behavior.

### 13.1 Tension

Status: `Locked`

Default depth:
- field

Primary rendering:
- one `WerkTensionStripe` row in the field

Mandatory visible elements:
- phase glyph
- name / desired label text
- activity trail
- selection state

Widgets:
- composite: `WerkTensionStripe`
- inside composition: `Paragraph`, optional `Badge`, optional `Rule`

Rules:
- name text owns the most horizontal space
- internal IDs never appear at field depth by default

### 13.2 Desired state

Status: `Locked`

Default depth:
- gaze and analysis

Primary rendering:
- top value plane, always above actual

Widgets:
- `Paragraph` inside `WerkGazeCard` or `WerkAnalysisPane`

Rules:
- always precedes actual
- never collapsed away before actual at the same depth

### 13.3 Actual state

Status: `Locked`

Default depth:
- gaze and analysis

Primary rendering:
- bottom value plane, always below desired

Widgets:
- `Paragraph` inside `WerkGazeCard` or `WerkAnalysisPane`

Rules:
- phrased as current condition, not desired outcome
- always spatially lower than desired

### 13.4 Parent-child structure

Status: `Locked`

Default depth:
- field and descended field

Primary rendering:
- descent context, breadcrumbs, child lists

Widgets:
- `StatusLine` for breadcrumb lever
- `Rule` plus `Paragraph` for descended header
- `List` or `VirtualizedList` for child field
- `Tree` for secondary topology surface

Rules:
- everyday operation is one level at a time
- the full tree exists, but is not forced into every screen

### 13.5 Horizon

Status: `Locked`

Default depth:
- field when urgent or overdue
- gaze when set
- analysis always

Primary rendering:
- compact badge or short text in field only if structurally relevant
- explicit line item in gaze/analysis

Widgets:
- `Badge` for urgent/overdue horizon state
- `Paragraph` in analysis metadata rows

Rules:
- neutral future horizons do not crowd field rows
- breached or tightening horizons may promote into local signals

### 13.6 Status

Status: `Locked`

Default depth:
- field via style and optional badge when not active
- analysis as explicit metadata

Primary rendering:
- active: default styling
- resolved/released: dimmed styling and explicit badge in deeper layers

Widgets:
- `Badge`
- component style variants

Rules:
- active remains dominant in the default filter
- terminal state does not hijack the phase glyph

### 13.7 Sibling order / position

Status: `Locked`

Default depth:
- field

Primary rendering:
- row order in field and descended lists

Widgets:
- `List` or `VirtualizedList`
- reordering overlay or modal when editing order

Rules:
- ordering is structural and must remain stable across refresh
- reorder mode must expose clear movement intent and confirmation

### 13.8 Phase

Status: `Locked`

Default depth:
- field, gaze, analysis

Primary rendering:
- leftmost phase glyph in field
- optional textual label in analysis

Widgets:
- composite glyph slot
- `Badge` or `Paragraph` label in analysis

Rules:
- glyph is canonical; text label is secondary

### 13.9 Tendency

Status: `Locked`

Default depth:
- field when space permits
- gaze and analysis

Primary rendering:
- secondary directional token near right-side signal cluster
- explicit analysis row label

Widgets:
- glyph inside stripe
- `Badge` or `Paragraph`

Rules:
- tendency does not replace the trail; it interprets motion at a coarser level

### 13.10 Conflict

Status: `Locked`

Default depth:
- field when present
- gaze and analysis

Primary rendering:
- red signal badge or marker in the row
- named relationship in gaze/analysis

Widgets:
- `Badge`
- `Paragraph`
- optional `List` of conflicting tensions in analysis

Rules:
- conflict is persistent and primary
- conflict must be visible without opening a diagnostic overlay

### 13.11 Oscillation

Status: `Locked`

Default depth:
- implied by trail at field depth
- explicit in gaze and analysis

Primary rendering:
- read from alternating trail pattern plus tendency glyph at field depth
- explicit label in deeper surfaces

Widgets:
- trail token group
- `Badge` / `Paragraph`

Rules:
- avoid redundant duplication in field; one explicit signal plus the trail is enough

### 13.12 Resolution trend

Status: `Provisional`

Default depth:
- analysis

Primary rendering:
- analysis row or progress/forecast subsection

Widgets:
- `Paragraph`
- optional `ProgressBar`

Rules:
- does not belong in field unless tied to a user-facing forecast feature

### 13.13 Orientation

Status: `Locked`

Default depth:
- analysis

Primary rendering:
- textual analysis row

Widgets:
- `Paragraph`
- optional small `Badge`

Rules:
- orientation is interpretive; it remains secondary and on-demand

### 13.14 Compensating strategy

Status: `Locked`

Default depth:
- analysis only

Primary rendering:
- textual analysis row, promoted only when present

Widgets:
- `Paragraph`
- warning-style `Badge` when present

Rules:
- never permanent field chrome
- only visible when detected

### 13.15 Assimilation depth

Status: `Locked`

Default depth:
- analysis

Primary rendering:
- textual analysis row

Widgets:
- `Paragraph`

Rules:
- not a primary signal in the operator loop

### 13.16 Neglect

Status: `Locked`

Default depth:
- field when present
- gaze and analysis

Primary rendering:
- amber badge or signal marker
- supporting text in gaze/analysis

Widgets:
- `Badge`
- `Paragraph`

Rules:
- neglect is a primary structural signal
- the field must surface it without opening analysis

### 13.17 Horizon drift

Status: `Locked`

Default depth:
- gaze when meaningful
- analysis always when present

Primary rendering:
- cautionary analysis row
- optional badge in gaze if it materially changes next action

Widgets:
- `Badge`
- `Paragraph`

Rules:
- drift is secondary to direct horizon breach or urgency

### 13.18 Urgency

Status: `Locked`

Default depth:
- field if high
- gaze and analysis

Primary rendering:
- badge or compact token in field when high
- explicit line in gaze/analysis

Widgets:
- `Badge`
- `ProgressBar` or compact bar in analysis
- `Paragraph`

Rules:
- urgency is not a scalar spectacle; the field only needs thresholds, not raw metrics

### 13.19 Event history

Status: `Locked`

Default depth:
- analysis

Primary rendering:
- concise chronological history section

Widgets:
- `HistoryPanel` if interaction fits
- otherwise `List` within a `Panel`

Rules:
- newest event first
- terse phrasing
- history supports note mutations, watch notes, and agent actions

### 13.20 Projection trajectory

Status: `Provisional`

Default depth:
- analysis

Primary rendering:
- forecast/trajectory subsection

Widgets:
- `Paragraph`
- `ProgressBar`
- optional `Badge`

Rules:
- projections are advisory, not primary field furniture
- if the projective model is feature-gated, the surface must disappear cleanly

### 13.21 Watch insight

Status: `Locked`

Default depth:
- lever/status when backlog exists
- dedicated review surface when opened

Primary rendering:
- count in lever
- review card with observation, suggested mutation, and accept/dismiss actions

Widgets:
- `StatusLine`
- `Panel`
- `List`
- `Modal` or dedicated review plane

Rules:
- watch is silent until return
- watch never auto-executes

### 13.22 Agent mutation proposal

Status: `Locked`

Default depth:
- dedicated review surface

Primary rendering:
- proposal cards grouped by mutation

Widgets:
- `Panel`
- `List`
- `Badge`
- `Modal` or analysis-adjacent review surface

Rules:
- each mutation must be reviewable independently
- agent response text is secondary to actionable deltas

### 13.23 Search result

Status: `Locked`

Default depth:
- overlay or dedicated search plane

Primary rendering:
- path-aware list row with match highlighting

Widgets:
- `TextInput`
- `List`
- optional `Panel`

Rules:
- search is cross-cutting and may break the one-level-at-a-time navigation temporarily
- selecting a result navigates the field context to that location

### 13.24 Filter state

Status: `Locked`

Default depth:
- lever/status only

Primary rendering:
- compact status indicator

Widgets:
- `StatusLine`
- optional `Badge`

Rules:
- filter state should not dominate the field visually

### 13.25 Empty state

Status: `Locked`

Default depth:
- full field

Primary rendering:
- sparse instructional state with one obvious first act

Widgets:
- centered `Paragraph` within a `Panel` or plain field shell

Rules:
- do not fill empty state with analytics or chrome
- first action must be obvious

## 14. Canonical Surface and Component Contract

This section defines the reusable surfaces implementers should build.

### 14.1 `WerkAppShell`

Purpose:
- global frame for field, overlays, lever, and optional pinned analysis

Composition:
- `ResponsiveLayout`
- main content plane
- optional secondary plane
- persistent `StatusLine` at bottom
- top-level modal/overlay stack

Rules:
- shell owns breakpoint classification
- shell owns overlay stacking order
- shell does not own domain semantics

### 14.2 `WerkFieldShell`

Purpose:
- the primary operator surface containing the visible tension rows for the current context

Composition:
- optional descended header
- `List` or `VirtualizedList` of `WerkTensionStripe`
- optional inline `WerkGazeCard`
- optional local `WerkSignalRail`

Rules:
- selection always visible
- current context always legible
- field owns the user’s everyday loop

### 14.3 `WerkTensionStripe`

Purpose:
- canonical depth-0 representation of a tension

Composition:
- left glyph slot
- title/content span
- optional local signal badges
- optional tendency token
- right activity trail

Widgets:
- composite over `Paragraph` and optional `Badge`

Rules:
- one logical row
- title dominates width budget
- trail is right-aligned and stable
- no IDs

### 14.4 `WerkGazeCard`

Purpose:
- inline depth-1 expansion directly beneath a selected stripe

Composition:
- dotted `Rule`
- desired plane
- actual plane
- one-line structural summary cluster
- optional local signals
- closing dotted `Rule`

Preferred widgets:
- `Panel` only if border weight remains quiet
- otherwise `Paragraph` + `Rule` composition
- `Badge`
- optional `ProgressBar`

Rules:
- target height is 3-5 lines in compact and standard tiers
- gaze is for quick recognition, not exhaustive diagnosis
- child count, gap, and the most important signal are favored over verbose prose

### 14.5 `WerkAnalysisPane`

Purpose:
- depth-2 dedicated study surface

Composition:
- header
- value planes
- signal summary
- analysis rows
- history section
- children/siblings subsections as appropriate
- review subsections when active

Widgets:
- outer `Panel`
- interior `Rule`
- `Paragraph`
- `Badge`
- `Table` for structured rows when useful
- `HistoryPanel` or `List`

Rules:
- analysis is explicit and legible, not decorative
- sections may scroll
- pinned coexistence is allowed only in expanded tier

### 14.6 `WerkDescendedHeader`

Purpose:
- show the parent context when the field is focused on a child set

Composition:
- parent title
- heavy rule
- optional summary line

Widgets:
- `Paragraph`
- `Rule`

Rules:
- parent is context, not the active child list
- keep it compact; do not replicate the full analysis header

### 14.7 `WerkSignalRail`

Purpose:
- persistent list of actionable structural signals for the current context

Composition:
- compact heading or none
- one row per signal with badge, short text, optional hotkey

Widgets:
- `List`
- `Badge`
- optional `Panel`

Rules:
- rail may be inline below the field or adjacent at wider tiers
- only primary persistent signals belong here

### 14.8 `WerkLeverBar`

Purpose:
- persistent grip line showing context, counts, and queued advisory state

Composition:
- left: breadcrumb or field label
- center: filter or mode summary when relevant
- right: counts, queue markers, quick hints only when useful

Widgets:
- `StatusLine`

Rules:
- always visible
- quiet by default
- may temporarily prioritize watch backlog message over breadcrumb

### 14.9 `WerkSearchSurface`

Purpose:
- cross-cutting search across the forest

Composition:
- `TextInput`
- results list
- path preview

Widgets:
- `Panel`
- `TextInput`
- `List`

Rules:
- search temporarily suspends local field context without losing it
- escape returns to the previous context cleanly

### 14.10 `WerkCommandSurface`

Purpose:
- global discoverability and infrequent command entry

Composition:
- `CommandPalette`

Rules:
- categories should mirror actual product vocabulary
- palette does not replace direct gestures for frequent acts

### 14.11 `WerkActEditor`

Purpose:
- create and edit desire, reality, notes, and structural metadata

Composition:
- small inline or modal input stack depending act complexity

Widgets:
- `TextInput`
- `TextArea`
- `Panel`
- `Modal` or inline panel

Rules:
- simple acts stay inline when feasible
- long-form reflection uses `TextArea`
- all act surfaces need deterministic confirm/cancel semantics

### 14.12 `WerkConfirmDialog`

Purpose:
- resolve, release, delete, or otherwise confirm irreversible acts

Widgets:
- `Dialog` or `Modal`

Rules:
- confirmation wording must name the affected tension and action
- destructive confirmation cannot be visually identical to benign acknowledgement

### 14.13 `WerkMoveSurface`

Purpose:
- reparenting and reordering

Widgets:
- `Modal`
- `List` or `Tree`
- `StatusLine`

Rules:
- source and destination context must be explicit
- cancel always restores previous structural state untouched

### 14.14 `WerkAgentReviewSurface`

Purpose:
- review proposed mutations from the agent

Widgets:
- `Panel`
- `List`
- `Badge`
- `Modal` or dedicated plane

Rules:
- each mutation shows type, target, preview, and acceptance state
- batch apply is allowed only after per-item reviewability exists

### 14.15 `WerkInsightReviewSurface`

Purpose:
- review watch/daimon observations

Widgets:
- `Panel`
- `List`
- `Badge`

Rules:
- observational text first
- suggested mutation second
- accept and dismiss both explicit

### 14.16 `WerkHelpSurface`

Purpose:
- reveal the vocabulary without dragging the user into documentation mode

Widgets:
- `Panel`
- `Table`
- `Paragraph`

Rules:
- organize by context and act frequency
- help language must match the locked interaction vocabulary exactly

### 14.17 `WerkTreeSurface`

Purpose:
- secondary topology exploration and move destination aid

Widgets:
- `Tree`
- `StatusLine`

Rules:
- tree is not the default opening surface
- expand/collapse state may persist
- opening a node returns to field or analysis context, not to a separate conceptual app

### 14.18 `WerkTraceSurface`

Purpose:
- dedicated historical trace when analysis needs more space

Widgets:
- `HistoryPanel`
- `List`
- `Panel`

Rules:
- this is a depth-2 extension, never field chrome

## 15. Layout and Responsive Doctrine

### 15.1 Compact tier: 80-119 columns

Intent:
- preserve the instrument at minimum supported width

Layout:
- one dominant field plane
- lever at bottom
- local gaze expansion inline
- analysis replaces field when opened
- modals and overlays use most of the screen

What is visible by default:
- phase glyph
- title
- trail
- selection
- only the most important local signal

What collapses:
- long badges become terse
- secondary metadata disappears
- no permanent side-by-side analysis

### 15.2 Standard tier: 120-159 columns

Intent:
- target everyday desktop terminal width without forcing dashboard behavior

Layout:
- field remains dominant
- richer stripes and more comfortable gaze
- local signal rail can coexist beneath or beside the field depending height
- analysis typically opens as a replacement plane, not a permanent second plane

What improves:
- slightly richer stripe content
- more legible gaze summaries
- room for explicit local alert text

What still does not become mandatory:
- persistent split field + analysis

### 15.3 Expanded tier: 160+ columns

Intent:
- support coexistence without changing the product stance

Layout:
- field remains primary plane
- optional pinned analysis plane
- signal rail may become adjacent instead of below
- review surfaces may use side-by-side context and proposal panes

Rules:
- the field must still visually dominate
- analysis cannot become the left pane with the field demoted to a skinny sidebar

### 15.4 Collapse order

Status: `Locked`

1. analysis prose collapses to terse labels
2. secondary sections collapse out of analysis
3. pinned coexistence collapses back to replacement
4. local rail moves below main field
5. row metadata trims
6. only then do optional badges collapse
7. phase glyph, title, selection, trail, and primary structural alert survive

### 15.5 Height doctrine

Status: `Locked`

- the field owns vertical priority
- the lever always consumes one line
- overlays may take most of the screen
- gaze expansion must not trap the cursor out of view
- analysis scrolls internally before the shell starts inventing secondary nested scrolling behavior

## 16. Integrated Mockups

### 16.1 Standard tier mockup, 120 columns

```text
◇ Write the novel                                             →  ○○●●○●   [NEGLECT]
┄──────────────────────────────────────────────────────────────────────────────────────┄
desire   A completed novel I can publish with conviction
actual   42,000 words. Third act unresolved. Research continues to sprawl.
signal   2 stagnant children   conflict with "Learn to rest"   horizon 5d
┄──────────────────────────────────────────────────────────────────────────────────────┄
◆ Fix relationship with brother                               ·  ○●○○○●
◇ Build the company                                           →  ●●●○●●   [OVERDUE]
◈ Get the apartment sorted                                    ·  ○○○○○○

The Field                                              active 4   alerts 2   insights 1
```

### 16.2 Expanded tier mockup, 160+ columns

```text
◇ Write the novel                                     →  ○○●●○●   [NEGLECT]      ┌ analysis ───────────────────────────────┐
┄──────────────────────────────────────────────────────────────────────────────┄   │ desire   A completed novel I can       │
desire   A completed novel I can publish with conviction                         │          publish with conviction        │
actual   42,000 words. Third act unresolved. Research continues to sprawl.      │ actual   42,000 words. Third act        │
signal   2 stagnant children   conflict with "Learn to rest"   horizon 5d       │          unresolved.                    │
┄──────────────────────────────────────────────────────────────────────────────┄   │                                        │
◆ Fix relationship with brother                         ·  ○●○○○●                 │ phase    germination                    │
◇ Build the company                                     →  ●●●○●●   [OVERDUE]    │ tendency advancing                      │
◈ Get the apartment sorted                              ·  ○○○○○○                 │ neglect  2 stagnant children            │
                                                                                  │ conflict Learn to rest                  │
Write the novel › children                                   active 4  alerts 2 │ urgency  high                           │
                                                                                  │                                        │
                                                                                  │ history                                │
                                                                                  │ 2d ago reality updated                 │
                                                                                  │ 5d ago child stalled                   │
                                                                                  └────────────────────────────────────────┘
```

### 16.3 Watch insight review mockup

```text
┌ watch insight ───────────────────────────────────────────────────────────────────┐
│ neglect detected on "Write the novel"                                           │
│                                                                                  │
│ Two of three children have not moved in two weeks. The writing thread is still  │
│ alive, but supporting work has become a place to hide from the draft itself.    │
│                                                                                  │
│ suggested mutation                                                               │
│ [ ] add note: "Watch: neglect detected — 2/3 children stagnant for 14d"         │
│                                                                                  │
│ Enter accept   d dismiss   @ follow up   Esc back                               │
└──────────────────────────────────────────────────────────────────────────────────┘
```

## 17. Interaction Model and Modes

### 17.1 Primary structural navigation

Status: `Locked`

| Key | Meaning | Notes |
|-----|---------|-------|
| `j` / `↓` | move to next visible sibling | skips non-selectable separators |
| `k` / `↑` | move to previous visible sibling | same |
| `l` / `Enter` | descend or activate selected structural target | child list or selected child from analysis |
| `h` / `Backspace` | ascend or back out | deterministic return |
| `Space` | toggle gaze | row-local |
| `Tab` | open or cycle analysis context | analysis, then back |
| `Esc` | dismiss overlay / step back toward field | never ambiguous |
| `/` | search | cross-cutting |
| `:` | command palette | infrequent/global |

### 17.2 Acts

Frequent direct acts:
- `a`: add tension in current context
- `e`: edit value planes
- `n`: add note
- `r`: resolve
- `x`: release
- `m`: move / reparent
- `u`: undo recent action
- `f`: cycle filter
- `i`: open insight review when pending
- `@`: open agent review or agent invocation surface depending state

Rules:
- common acts stay direct
- the palette mirrors these actions but does not replace them

### 17.3 Mode stack

Status: `Locked`

Priority order:
1. modal dialogs
2. review surfaces
3. command palette
4. search
5. text input / text area editing
6. analysis interaction
7. field navigation

Rules:
- higher modes own input
- every mode must display its exit path
- mode changes preserve field selection and context unless the act itself changes structure

## 18. State and View-Model Contract

This is not full code architecture, but enough to prevent implementation drift.

### 18.1 Canonical state slices

Core state:
- current navigation context
- visible sibling list / child list
- selected tension id
- filter
- breakpoint tier
- pinned analysis state
- active overlays / modals
- review queues
- transient confirmations

Derived view models:
- field rows
- gaze model for selected row
- analysis model for selected tension
- signal rail items
- lever/status items
- search result rows
- review card rows

Widget states:
- list state or virtualized list state
- tree state
- text input state
- text area state
- palette state
- modal stack state

Persistence candidates:
- last filter
- tree expansion state
- optional pinned analysis preference
- review queue cursor position

### 18.2 Invalidation rules

Status: `Locked`

- structural mutation invalidates field, gaze, analysis, and signal rail for affected context
- note mutation invalidates history/trace and possibly trail
- horizon mutation invalidates urgency, breach state, and forecast
- review-queue changes invalidate lever counts
- breakpoint transition invalidates layout selection, not semantic ordering

## 19. Migration Strategy From the Current Renderer

Status: `Locked`

The migration path must be incremental. A big-bang rewrite would reintroduce hidden assumptions.

### 19.1 What stays initially

- current navigation semantics
- current glyph family
- current theme semantics
- current watch insight flow concept
- current agent review concept

### 19.2 What changes first

- formal token tables
- component contracts
- introduction of named composite widgets
- replacement of top-level hand-built field and gaze rendering with component composition

### 19.3 What changes later

- `vlist.rs` replacement or encapsulation
- full analysis pane redesign
- review surfaces migration
- expanded-tier pinned analysis

### 19.4 Migration rule

No new surface should be added using the old ad hoc rendering style once the composite layer exists.

## 20. Validation Matrix

### 20.1 Golden-state fixtures required

Status: `Locked`

1. New germinating root with no children
2. Healthy advancing parent with assimilating child
3. Neglected subtree
4. Oscillating sibling pair
5. Urgent overdue tension with repeated postponement
6. Multiple roots lacking a clear organizing principle
7. Resolved tension visible under non-default filter
8. Agent mutation review with mixed actions
9. Watch backlog with multiple insight types
10. Deeply nested descended field

### 20.2 Snapshot requirements

- compact field
- standard field
- expanded field + pinned analysis
- gaze open
- analysis open
- search overlay
- help surface
- agent review surface
- insight review surface
- monochrome or degraded fallback

### 20.3 Manual validation requirements

- terminal resize while gaze is open
- unicode fallback behavior
- low-color behavior
- keyboard-only full session
- alert hotkey action path
- external mutation reload path

## 21. Open Questions Preserved Deliberately

These are not omissions. They are bounded unknowns.

### 21.1 Needs user/product input

- Should pinned analysis at `160+` be opt-in, sticky, or automatic on first use?
- Should released tensions remain visually present in `All` with distinct semantics, or collapse behind a secondary filter?
- Should watch insights have a dedicated persistent queue surface beyond lever count plus review mode?

### 21.2 Needs prototyping

- Can `VirtualizedList` cleanly replace the bespoke variable-height list while preserving gaze ergonomics?
- Is `HistoryPanel` the right primitive for mutation/event history, or does a custom trace list produce clearer semantics?
- Does a borderless gaze composition read better than a lightly panelized gaze under real terminal conditions?

### 21.3 Needs `ftui` verification or extension

- proof of focus choreography for nested review + edit flows
- proof of variable-height row performance in real `werk` datasets
- proof of border and glyph consistency across terminal classes

### 21.4 Feature-gated future questions

- how to surface time-travel replay without breaking the sacred depth model
- whether field resonance deserves an always-available analysis subsection
- whether theme variants add real value or just design churn

## 22. Bead Decomposition

Bead conventions:
- `Ring` follows the stability-ring model in this document.
- `Priority` means execution urgency, not abstract importance.
- `Blocked by` and `Blocks` list direct material dependencies, not every possible relationship.
- `Acceptance criteria` must be satisfiable without rereading the prose sections above.

### Bead 1: App Breakpoint Model
- **Description**: Introduce a single app-level breakpoint contract for the TUI: `Compact` at `80-119`, `Standard` at `120-159`, and `Expanded` at `160+`. Implementers must centralize width classification so view code stops hardcoding width thresholds independently, and so `ftui` breakpoints can be overridden to match this doctrine.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: []
- **Blocks**: [12, 19, 58, 59, 60, 72, 77]
- **Acceptance criteria**: A single source of truth exists for breakpoint classification; no new component reads raw width thresholds directly; tests can request each tier deterministically.
- **Effort**: S

### Bead 2: Semantic Style Token Module
- **Description**: Formalize the six-color semantic palette and non-hue interaction styles into a dedicated token module, replacing ad hoc style construction. The module must expose semantic roles such as `fg_default`, `fg_dim`, `fg_cyan`, `fg_amber`, `fg_red`, and `fg_green`, plus selection and emphasis helpers.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: []
- **Blocks**: [3, 5, 9, 11, 21, 24, 27, 31, 72]
- **Acceptance criteria**: New component code can request style roles by semantic name; raw RGBA or widget-specific style literals are no longer the required entry point for new surfaces.
- **Effort**: S

### Bead 3: Glyph and Dot Token Table
- **Description**: Convert the current glyph family into a binding token table: phase glyphs `◇ ◆ ◈ ◉`, tendency glyphs `→ ↔ ·`, and six-bucket activity trail dots `● ○`. This bead must also freeze naming and documentation so later components do not reinterpret glyph meaning.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: [2]
- **Blocks**: [4, 5, 21, 22, 23, 31, 73]
- **Acceptance criteria**: Every glyph has a semantic name, one source of truth, and at least one unit test proving the mapping.
- **Effort**: S

### Bead 4: ASCII and Degradation Mapping
- **Description**: Provide the official fallback mapping for unsupported glyph or border environments, including ASCII replacements for phase, tendency, and trail tokens. The point is not beauty; it is preserving semantic legibility under degraded terminals without inventing per-surface fallbacks later.
- **Ring**: 1
- **Priority**: P1
- **Blocked by**: [3]
- **Blocks**: [20, 72, 73, 76, 77]
- **Acceptance criteria**: There is a tested fallback table and a documented strategy for when Unicode or decorative borders are unavailable.
- **Effort**: S

### Bead 5: Canonical Domain Visibility Matrix
- **Description**: Encode the design-system rule that every werk concept declares where it appears by default: field, gaze, analysis, review-only, or hidden unless present. This view-model matrix must cover phase, tendency, conflict, neglect, urgency, horizon, drift, history, watch insights, and agent proposals.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: [2, 3]
- **Blocks**: [6, 10, 11, 13, 14, 15, 16, 17]
- **Acceptance criteria**: A single data-driven matrix exists; new components can query visibility rules instead of open-coding them.
- **Effort**: M

### Bead 6: Golden Fixture Dataset
- **Description**: Create a fixture set covering the canonical states named in this plan: fresh germinating root, healthy advancing parent, neglected subtree, oscillating pair, overdue horizon, multiple roots, watch backlog, and mixed agent proposal set. The dataset must be usable by component tests, view snapshots, and manual demos.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: [5]
- **Blocks**: [7, 10, 13, 14, 15, 16, 17, 77, 78]
- **Acceptance criteria**: Fixtures load deterministically and expose enough data to render field, gaze, analysis, review, and alert states without mocking from scratch each time.
- **Effort**: M

### Bead 7: Snapshot Harness for Components and Views
- **Description**: Set up a rendering snapshot harness that can render components and full screens at compact, standard, and expanded widths using the golden fixtures. This bead is foundational because the plan’s stop-ship criteria depend on proving invariants, not asserting them rhetorically.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: [1, 6]
- **Blocks**: [19, 21, 27, 28, 31, 39, 41, 46, 48, 50, 58, 59, 60, 72, 73, 77]
- **Acceptance criteria**: Tests can render at least one field screen and one overlay at each target width, and store stable snapshot output.
- **Effort**: M

### Bead 8: Focus and Modal Ownership Contract
- **Description**: Formalize a focus stack for field navigation, editing, search, palette, review surfaces, and modals. The goal is to prevent later ambiguity about who owns input when overlays nest or when agent/watch review surfaces coexist with editing.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: [1]
- **Blocks**: [15, 39, 40, 41, 42, 43, 44, 45, 47, 49, 55, 57]
- **Acceptance criteria**: There is a documented and testable mode stack with deterministic dismissal and return semantics.
- **Effort**: M

### Bead 9: Composite Component Scaffolding
- **Description**: Introduce a dedicated component layer for named composites such as `WerkTensionStripe`, `WerkGazeCard`, `WerkAnalysisPane`, `WerkSignalRail`, and `WerkLeverBar`. This bead exists to stop new design work from going directly back into monolithic view render functions.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: [2, 5]
- **Blocks**: [21, 24, 27, 28, 31, 36, 37]
- **Acceptance criteria**: The codebase has a clear home for domain-specific composites and new UI work lands there by default.
- **Effort**: M

### Bead 10: Field Row View Model
- **Description**: Create the canonical row model for depth-0 field rendering. It must include phase glyph, title, selection state, trail, primary local signal set, optional tendency glyph, and compact horizon or urgency markers when relevant.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: [5, 6]
- **Blocks**: [21, 22, 24, 25, 53, 54]
- **Acceptance criteria**: A field row can be constructed from fixture data without consulting view code; row tests cover active, neglected, conflicting, and overdue cases.
- **Effort**: M

### Bead 11: Structural Signal Severity Model
- **Description**: Define a normalized local-signal model for the TUI so conflict, neglect, overdue horizon, watch backlog, and other operator-facing signals can be ranked, styled, and collapsed consistently. This should decide what is primary versus secondary at field depth.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: [2, 5]
- **Blocks**: [21, 24, 25, 30, 36, 53, 61]
- **Acceptance criteria**: The code has one severity ordering and one “promote to field/gaze/analysis” rule instead of per-surface exceptions.
- **Effort**: M

### Bead 12: Lever and Status View Model
- **Description**: Formalize the bottom-line `StatusLine` model used by the lever. It must support breadcrumb context, filter state, alert counts, and watch/agent queue counts, while preserving the rule that watch backlog may temporarily replace the breadcrumb emphasis without destroying context.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: [1, 5]
- **Blocks**: [27, 52, 53, 54, 58, 59, 60, 69]
- **Acceptance criteria**: One typed model exists for lever content and can render root field, descended field, and queue-backlog states.
- **Effort**: M

### Bead 13: Gaze View Model
- **Description**: Define the data model for the inline gaze card. It must include desired, actual, child summary, primary signals, and any compact gap or horizon summary needed for 3-5 line rendering. It must not leak full analysis concerns into the gaze.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: [5, 6]
- **Blocks**: [28, 29, 30, 56]
- **Acceptance criteria**: Given a selected tension, the app can build a complete gaze model without querying view-specific helpers or recalculating layout logic.
- **Effort**: M

### Bead 14: Analysis View Model
- **Description**: Define the depth-2 analysis model with clearly separated subsections: value planes, signal summary, interpretive dynamics, history, siblings, children, and optional feature-gated forecast data. This bead prevents analysis surfaces from becoming a generic dump of domain structs.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: [5, 6]
- **Blocks**: [31, 32, 33, 34, 35, 57, 60]
- **Acceptance criteria**: Analysis rendering can be driven entirely from this model; the model omits data that belongs only in debug surfaces.
- **Effort**: M

### Bead 15: Review Queue View Models
- **Description**: Create typed view models for watch insight review and agent mutation review, including action state, preview text, queue position, and acceptance/dismissal affordances. This bead explicitly separates advisory queue semantics from structural field semantics.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: [5, 6, 8]
- **Blocks**: [46, 47, 48, 49, 69, 70]
- **Acceptance criteria**: Review surfaces render from typed models and no longer need to parse raw YAML or mutation lists directly inside the UI layer.
- **Effort**: M

### Bead 16: Search Result View Model
- **Description**: Define the canonical search result row: matched title, path or breadcrumb, reason for match, and target context needed to jump back into the field. The result model must support search as a cross-cutting overlay without losing the user’s current field state.
- **Ring**: 1
- **Priority**: P1
- **Blocked by**: [5, 6]
- **Blocks**: [39, 63]
- **Acceptance criteria**: Search results can be rendered and activated without bespoke path formatting in the overlay renderer.
- **Effort**: S

### Bead 17: Tree Surface View Model
- **Description**: Create a derived tree model for the secondary topology surface and move/reparent workflows. The model must expose stable labels, expansion state keys, and a clean mapping from field context to tree cursor context without making tree rendering the primary navigation model.
- **Ring**: 1
- **Priority**: P2
- **Blocked by**: [5, 6]
- **Blocks**: [45, 50]
- **Acceptance criteria**: Tree rendering and move workflows consume a typed topology model instead of querying store data ad hoc.
- **Effort**: M

### Bead 18: Renderer Debt Audit and Deletion Map
- **Description**: Produce an explicit audit of manual rendering debt in the current code, especially inside `render.rs`, identifying what becomes a composite, what becomes a pure view orchestrator, and what can be deleted once migrated. This bead is necessary to prevent partial migration from freezing obsolete code paths indefinitely.
- **Ring**: 1
- **Priority**: P1
- **Blocked by**: []
- **Blocks**: [53, 54, 56, 57, 58, 59, 60, 80]
- **Acceptance criteria**: There is a tracked list of rendering regions to replace and a rule for when old code is removed rather than left parallel.
- **Effort**: S

### Bead 19: Responsive Invariant Test Suite
- **Description**: Write automated tests that verify the sacred layout laws across compact, standard, and expanded widths: `desired` above `actual`, field visible first, phase and trail preserved, and structural signals surviving collapse. This bead exists because responsive bugs are expensive and subtle.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: [1, 7]
- **Blocks**: [58, 59, 60, 72, 73, 77]
- **Acceptance criteria**: Failing layout regressions can be caught by tests rather than manual resize inspection alone.
- **Effort**: M

### Bead 20: Terminal Capability Probe and Manual Matrix
- **Description**: Add a lightweight terminal-capability check and a manual validation matrix for color depth, Unicode support, and border degradation. The probe should not become a fancy configuration system; it should simply make fallback choices explicit and testable.
- **Ring**: 1
- **Priority**: P1
- **Blocked by**: [4]
- **Blocks**: [72, 73, 76]
- **Acceptance criteria**: The app can choose or report degraded token behavior, and the repo contains a concrete terminal validation checklist.
- **Effort**: S

### Bead 21: `WerkTensionStripe` Component
- **Description**: Implement the canonical depth-0 row composite using the field row view model. It must render the phase glyph at the far left, title with maximum width budget, optional primary signal cluster, optional tendency token, and a stable six-bucket right-aligned trail.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [3, 9, 10, 11]
- **Blocks**: [22, 23, 24, 25, 53, 54, 58, 59]
- **Acceptance criteria**: Snapshot tests prove that active, neglected, conflicting, overdue, and resolved examples render consistently across target widths.
- **Effort**: M

### Bead 22: Activity Trail Renderer
- **Description**: Implement the reusable six-bucket activity trail renderer used by field rows and any compact historical hint surfaces. The renderer must accept semantic bucket data, not already-formatted strings, so trail meaning stays centralized and width-aware.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [3, 10, 21]
- **Blocks**: [53, 54, 77]
- **Acceptance criteria**: The trail renderer produces the correct newest-right ordering, survives compact widths, and obeys Unicode and ASCII fallback rules.
- **Effort**: S

### Bead 23: Phase Glyph Slot Component
- **Description**: Extract the leftmost lifecycle slot into a small reusable component that maps phase semantics to glyph and style. This allows field rows, analysis headers, and other surfaces to share one implementation of phase identity.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [3, 21]
- **Blocks**: [31, 37, 53]
- **Acceptance criteria**: There is one implementation for phase-slot rendering and tests prove all phase mappings.
- **Effort**: S

### Bead 24: Signal Badge Cluster
- **Description**: Build the component that renders the row-local signal cluster for field and gaze surfaces. It must prioritize conflict, neglect, overdue horizon, and other primary alerts according to the signal severity model, trimming secondary badges before primary ones.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [10, 11, 21]
- **Blocks**: [25, 28, 53, 54, 58, 59]
- **Acceptance criteria**: When width is tight, the cluster collapses deterministically and still preserves the highest-priority signal.
- **Effort**: M

### Bead 25: Horizon and Urgency Token Component
- **Description**: Implement the compact horizon/urgency indicator used when a tension is near or past horizon. This component must follow the design rule that neutral horizons stay out of the field, while urgent or overdue horizons promote into visible local signal territory.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [10, 11, 24]
- **Blocks**: [28, 31, 53, 54]
- **Acceptance criteria**: Field rows show nothing for non-urgent horizons, but urgent and overdue states render clearly and testably.
- **Effort**: S

### Bead 26: Descended Header Component
- **Description**: Implement the compact parent-context header used when viewing a child set. It must show the parent title and a clear structural boundary without turning descended context into a second full analysis header.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [9, 10]
- **Blocks**: [54, 59]
- **Acceptance criteria**: Descended contexts render with clear parent identity and a strong but restrained boundary, and snapshots prove that the header remains compact across widths.
- **Effort**: S

### Bead 27: Lever / Status Line Component
- **Description**: Implement the persistent `StatusLine`-based lever using the typed view model. It must support root-field, descended-field, filter, and queue-count states, while staying visually quiet by default.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [7, 9, 12]
- **Blocks**: [52, 53, 54, 58, 59, 60, 69]
- **Acceptance criteria**: The lever is always present, uses semantic content regions consistently, and has snapshot coverage for breadcrumb and backlog modes.
- **Effort**: S

### Bead 28: Gaze Card Skeleton
- **Description**: Implement the structural shell of the inline gaze card beneath a selected row. The skeleton must own the 3-5 line budget, dotted boundaries, spacing rules, and insertion mechanics, but not yet the final contents of every summary token.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [7, 9, 13, 24, 25]
- **Blocks**: [29, 30, 56, 58, 59]
- **Acceptance criteria**: A selected row can expand inline with stable height targets, field selection remains visible, and the card never replaces the row itself.
- **Effort**: M

### Bead 29: Desired / Actual Plane Component
- **Description**: Implement the paired value-plane renderer used by gaze and analysis surfaces. The component must guarantee that desired is always displayed above actual, with shared spacing, truncation, and wrapping rules across both depths.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [13, 28]
- **Blocks**: [31, 56, 57, 77]
- **Acceptance criteria**: Desired/actual ordering is invariant across all supported widths and depths, and snapshots explicitly verify the vertical law.
- **Effort**: S

### Bead 30: Gaze Summary Line Component
- **Description**: Implement the one-line structural summary inside the gaze card: child count summary, primary local signal summary, and compact horizon/gap context where appropriate. The summary must privilege recognition over completeness.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [11, 13, 28]
- **Blocks**: [56]
- **Acceptance criteria**: The summary line can render healthy, neglected, conflicting, and overdue states without exceeding the gaze’s target density budget.
- **Effort**: S

### Bead 31: Analysis Pane Skeleton
- **Description**: Implement the depth-2 analysis shell as a named composite using `Panel`, `Rule`, and interior sections. This bead sets the section order, header behavior, scroll strategy, and feature-gate insertion points without yet finishing all subsection content.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [7, 9, 14, 23, 25, 29]
- **Blocks**: [32, 33, 34, 35, 57, 60, 79]
- **Acceptance criteria**: An analysis plane can open for a selected tension, render the value planes and section boundaries, and scroll without breaking selection context.
- **Effort**: M

### Bead 32: Analysis Metadata Grid
- **Description**: Implement the structured analysis subsection for phase, tendency, neglect, urgency, drift, orientation, and other interpretive dynamics that belong at depth 2. The grid may use `Table` or a disciplined line list, but it must not become a raw dump of every domain field.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [14, 31]
- **Blocks**: [57, 60]
- **Acceptance criteria**: Analysis metadata is grouped, styled semantically, and omits debug-only material; tests prove primary versus secondary signal ordering.
- **Effort**: M

### Bead 33: History / Trace Section
- **Description**: Implement the history subsection used by the analysis surface, including notes, mutations, and derived events in concise chronological form. This bead must choose whether `HistoryPanel` or a custom list presentation produces the clearer result in the actual app context.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [14, 31]
- **Blocks**: [51, 57, 79]
- **Acceptance criteria**: Analysis history renders with terse, readable entries and supports at least the fixture cases for note addition, update, resolve, and watch-generated note.
- **Effort**: M

### Bead 34: Children Section Component
- **Description**: Implement the analysis subsection that shows immediate children from the currently selected tension. This is not a full tree; it is a navigable structural sublist that supports jumping deeper into the field or analysis context.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [14, 31]
- **Blocks**: [57, 60]
- **Acceptance criteria**: Children render as a compact, navigable sublist with stable selection behavior and no duplicate field-level chrome.
- **Effort**: S

### Bead 35: Siblings Section Component
- **Description**: Implement the analysis subsection that shows siblings for contextual comparison without turning the analysis pane into a dashboard. This should enable quick lateral navigation while preserving the one-level-at-a-time philosophy.
- **Ring**: 2
- **Priority**: P2
- **Blocked by**: [14, 31]
- **Blocks**: [57, 60]
- **Acceptance criteria**: Siblings are visible, navigable, and visually subordinate to the primary selected tension.
- **Effort**: S

### Bead 36: Signal Rail Component
- **Description**: Implement the persistent local signal rail for contexts where more than one structural alert needs explicit textual presence. The rail must render primary signals with short text and optional hotkey markers, and degrade below or beside the field depending width.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [9, 11]
- **Blocks**: [54, 58, 59, 60, 61]
- **Acceptance criteria**: Conflict, neglect, overdue, and watch backlog examples can render in a stable rail without duplicating the entire analysis surface.
- **Effort**: M

### Bead 37: Empty State Component
- **Description**: Implement the empty field state for root and descended contexts. It must present one obvious next act, avoid analytics entirely, and respect the instrument’s restrained tone.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [9, 23]
- **Blocks**: [53, 54, 77]
- **Acceptance criteria**: Empty root and empty descended states render clearly and do not introduce dead-end screens or extra chrome.
- **Effort**: S

### Bead 38: Filter State Indicator
- **Description**: Implement the small filter-state surface used by the lever or local context. The design rule is that filter state must be visible but not visually dominate the field.
- **Ring**: 2
- **Priority**: P2
- **Blocked by**: [12]
- **Blocks**: [27, 52, 62]
- **Acceptance criteria**: The user can always tell which filter is active, and filter state changes do not rearrange unrelated lever content unpredictably.
- **Effort**: S

### Bead 39: Search Overlay Surface
- **Description**: Implement the cross-cutting search overlay using `TextInput` plus result list, driven by the search result view model. The overlay must absorb input cleanly, preserve the user’s previous field context, and navigate back into the field when a result is activated.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [7, 8, 16]
- **Blocks**: [63, 77]
- **Acceptance criteria**: The user can search, browse path-aware results, select one, and return to the correct field context without losing prior selection state.
- **Effort**: M

### Bead 40: Command Palette Categories and Registration
- **Description**: Register the TUI’s command palette categories and actions using the locked vocabulary of field navigation, structural acts, review flows, and diagnostics. This bead does not replace direct gestures; it creates the discoverability layer for infrequent and global actions.
- **Ring**: 2
- **Priority**: P2
- **Blocked by**: [8]
- **Blocks**: [70, 78, 80]
- **Acceptance criteria**: Palette categories mirror actual product language, and at least the major actions can be executed from the palette without inventing alternative semantics.
- **Effort**: S

### Bead 41: Help Surface
- **Description**: Implement the help overlay showing context-aware keybindings and the locked interaction vocabulary. The help surface must make the muscle-memory model legible without reading like documentation for a different app.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [7, 8]
- **Blocks**: [78, 80]
- **Acceptance criteria**: Help text matches actual keybindings, mode ownership, and act names, and snapshot coverage exists at compact and standard widths.
- **Effort**: S

### Bead 42: Inline Text Input Act Surface
- **Description**: Implement the small inline or panelized text input surface used for quick create and edit acts. It must support the sequence name → desired → actual for creation, and compact edits for simple value changes without dragging the user into a heavyweight modal unnecessarily.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [8, 9]
- **Blocks**: [64, 65]
- **Acceptance criteria**: A user can create or edit simple values with predictable confirm/cancel behavior and without corrupting field selection state.
- **Effort**: M

### Bead 43: Text Area Reflection Surface
- **Description**: Implement the multi-line note or reflection surface using `TextArea` for longer-form annotations. This must preserve the instrument’s act language while accommodating notes that exceed single-line input.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [8, 9]
- **Blocks**: [65, 77]
- **Acceptance criteria**: The user can enter multi-line notes, submit or cancel deterministically, and return to the same field context.
- **Effort**: S

### Bead 44: Confirm Dialog Surface
- **Description**: Implement the confirm dialog used for resolve, release, delete, and comparable irreversible acts. The dialog must name the action and target explicitly and support keyboard-only confirm and cancel paths.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [8]
- **Blocks**: [66, 68]
- **Acceptance criteria**: Destructive and irreversible acts all route through one consistent confirm surface with correct semantic styling.
- **Effort**: S

### Bead 45: Move / Reparent Surface
- **Description**: Implement the move and reparent workflow surface, likely using a modal list or tree-backed destination picker. The user must always know source, destination, and cancel behavior; the flow must never feel like opaque structural teleportation.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [8, 17]
- **Blocks**: [67]
- **Acceptance criteria**: The user can select a destination, confirm the move, or cancel with no structural mutation applied.
- **Effort**: M

### Bead 46: Agent Review Card Component
- **Description**: Implement the reusable card for a single agent mutation proposal, including mutation type, target preview, acceptance toggle, and any error or validation state. This is the atomic unit of trustworthy agent interaction.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [7, 15]
- **Blocks**: [47, 70, 77]
- **Acceptance criteria**: Mixed proposal types render in a uniform structure and each card can expose accept, reject, and preview states.
- **Effort**: M

### Bead 47: Agent Review Surface Integration
- **Description**: Assemble the full agent review workflow from the card component and review queue model, integrating queue position, batch actions, and dismissal. The surface must make review primary and natural-language agent output secondary.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [8, 15, 46]
- **Blocks**: [70, 77]
- **Acceptance criteria**: An agent response with multiple mutations can be reviewed and applied selectively without leaving ambiguous state behind.
- **Effort**: M

### Bead 48: Watch Insight Card Component
- **Description**: Implement the reusable watch/daimon insight card that shows trigger type, observation, and suggested follow-up mutation or note. The card must feel advisory rather than alarmist and preserve the doctrine that watch is silent until return.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [7, 15]
- **Blocks**: [49, 69, 77]
- **Acceptance criteria**: Multiple watch trigger types render clearly and the suggestion path is explicit without auto-applying anything.
- **Effort**: S

### Bead 49: Watch Insight Surface Integration
- **Description**: Assemble the watch insight review workflow using the insight card component, queue position, and accept/dismiss/follow-up actions. This surface must integrate with the lever backlog count and preserve advisory semantics.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [8, 15, 48]
- **Blocks**: [69, 77]
- **Acceptance criteria**: A user can review pending watch items, accept or dismiss them, and see queue count updates reflected in the lever.
- **Effort**: M

### Bead 50: Tree Surface
- **Description**: Implement the secondary tree surface using the typed tree model and the `ftui` `Tree` widget. The surface is for topology exploration and move assistance, not for replacing the field as the daily operator plane.
- **Ring**: 2
- **Priority**: P2
- **Blocked by**: [7, 17]
- **Blocks**: [67, 78]
- **Acceptance criteria**: Expand/collapse and activation work consistently, tree state can persist, and the field remains the default entry point.
- **Effort**: M

### Bead 51: Trace Surface
- **Description**: Implement a secondary trace surface or analysis subsection for deeper historical review when the standard history block is insufficient. This bead is deliberately lower priority because history must exist before it needs dedicated expansion.
- **Ring**: 2
- **Priority**: P3
- **Blocked by**: [33]
- **Blocks**: [79]
- **Acceptance criteria**: Long histories remain readable without polluting the core analysis pane and without violating the sacred depth model.
- **Effort**: S

### Bead 52: Toast and Notification Adapter
- **Description**: Introduce the thin adapter that restricts toast/notification usage to ephemeral acknowledgements, undo windows, and similar action outcomes. This bead exists to prevent transient surfaces from quietly becoming the alert architecture by accident.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [12, 27, 38]
- **Blocks**: [68]
- **Acceptance criteria**: Structural alerts do not require toast rendering to be visible, while action confirmations can use a clearly bounded transient channel.
- **Effort**: S

### Bead 53: Root Field View Integration
- **Description**: Integrate the new field shell, tension stripe, lever, and empty state into the root field view. This bead replaces the top-level hand-built scan surface with the new component system while preserving today’s core navigation semantics.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [10, 18, 21, 22, 24, 27, 37]
- **Blocks**: [55, 56, 58, 59, 62, 64, 65, 66]
- **Acceptance criteria**: The root field can be navigated, renders snapshots cleanly at all target widths, and no longer relies on monolithic custom string composition for its core row rendering.
- **Effort**: M

### Bead 54: Descended Field Integration
- **Description**: Integrate the new field shell, descended header, local signal rail, and child rows into descended contexts. This bead ensures the one-level-at-a-time structure works with the new component system rather than only at the root.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [10, 18, 21, 24, 26, 27, 36]
- **Blocks**: [55, 56, 59, 60, 67]
- **Acceptance criteria**: Descending into a tension produces a coherent child field with parent context, selection, signals, and lever updates all working together.
- **Effort**: M

### Bead 55: Selection and Cursor Preservation
- **Description**: Ensure that selection, scroll position, and context are preserved correctly across moves between field, descended field, overlays, and review surfaces. This bead is separate because interaction quality depends as much on return semantics as on visual rendering.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [8, 53, 54]
- **Blocks**: [56, 57, 63, 64, 65, 67]
- **Acceptance criteria**: Exiting search, help, edit, review, or analysis returns the user to a deterministic field selection and viewport state.
- **Effort**: M

### Bead 56: Gaze Interaction Integration
- **Description**: Wire `Space` and related selection behavior to the new gaze card so the selected row can expand inline in root and descended contexts. The integration must preserve cursor visibility and maintain the field as the dominant frame of reference.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [28, 29, 30, 53, 54, 55]
- **Blocks**: [58, 59, 77, 78]
- **Acceptance criteria**: Gaze opens and closes predictably, follows selection correctly, and never causes the selected row to vanish or the field to lose structural clarity.
- **Effort**: M

### Bead 57: Analysis Interaction Integration
- **Description**: Wire `Tab`, enter/activate behaviors, and back-out semantics to the analysis pane. On compact and standard widths, analysis replaces the field plane; on expanded widths, it can coexist when pinned. This bead implements the locked depth model in actual navigation.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [31, 32, 33, 34, 35, 55]
- **Blocks**: [58, 59, 60, 78]
- **Acceptance criteria**: The user can open analysis from a selected tension, inspect it, navigate to a child or sibling when appropriate, and return to the original field context deterministically.
- **Effort**: M

### Bead 58: Compact Layout Integration
- **Description**: Apply the compact-tier layout doctrine to the field, gaze, lever, and overlays. The bead’s job is to prove that 80-column operation preserves meaning first and collapses analysis and secondary chrome before anything sacred disappears.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [1, 19, 21, 24, 27, 28, 36, 53, 56, 57]
- **Blocks**: [72, 73, 77]
- **Acceptance criteria**: Compact snapshots show a usable field, gaze, and analysis replacement flow with sacred invariants intact.
- **Effort**: M

### Bead 59: Standard Layout Integration
- **Description**: Apply the standard-tier layout doctrine at `120-159` columns, keeping the field dominant while allowing more comfortable gaze density and clearer contextual rails. This bead deliberately avoids forcing permanent split-pane analysis at the standard target width.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [1, 19, 21, 24, 26, 27, 28, 36, 53, 54, 56, 57]
- **Blocks**: [72, 73, 77, 78]
- **Acceptance criteria**: Standard snapshots and manual runs feel materially more breathable than compact, but still unmistakably field-first.
- **Effort**: M

### Bead 60: Expanded Pinned Analysis Integration
- **Description**: Add the optional `160+` expanded-tier behavior where field and analysis can coexist. The field must remain visually primary and the pinning behavior must be deterministic rather than an accidental side effect of width.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [1, 31, 32, 34, 35, 57, 59]
- **Blocks**: [77, 78, 79]
- **Acceptance criteria**: On expanded widths, pinned analysis coexists without demoting the field to a thin sidebar, and users can still operate the app comfortably with pinning disabled.
- **Effort**: M

### Bead 61: Alert Hotkey Mapping
- **Description**: Map visible actionable alerts to deterministic hotkeys in contexts where direct action makes sense, preserving today’s numbered-alert behavior where appropriate. This bead ties the signal rail to actual operator affordances instead of leaving it as static prose.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [11, 36]
- **Blocks**: [78]
- **Acceptance criteria**: At least the highest-priority alert actions can be triggered from the current context and the UI visibly communicates the mapping.
- **Effort**: S

### Bead 62: Filter Cycle Integration
- **Description**: Integrate the locked filter behavior into the new field architecture so active, all, and any later filter states remain clearly visible but visually lightweight. This bead should also settle how resolved and released states are represented when included.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [38, 53]
- **Blocks**: [77, 78]
- **Acceptance criteria**: Filter changes update lever and field rendering coherently and do not create hidden state the user cannot perceive.
- **Effort**: S

### Bead 63: Search Navigation Integration
- **Description**: Connect the search overlay to actual navigation so selecting a result restores the field in the correct context, selection, and depth state. The point is to make search a bridge back into the field, not a second parallel navigation model.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [39, 55]
- **Blocks**: [78]
- **Acceptance criteria**: Search activation lands the user in the correct descended context and restores ordinary field navigation immediately.
- **Effort**: S

### Bead 64: Create Act Integration
- **Description**: Wire the inline text input act surface into actual create flows, including root create and child create. The sequence name → desired → actual should work with skip/cancel semantics and should update field, lever, and trail state correctly.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [42, 53, 55]
- **Blocks**: [68, 77, 78]
- **Acceptance criteria**: Users can create root and child tensions from the current field context and see the new item appear predictably with correct selection placement.
- **Effort**: M

### Bead 65: Edit and Note Integration
- **Description**: Wire the inline text input and text area surfaces into edit flows for desired, actual, and notes. This bead must preserve focus ownership and return semantics and ensure that history, trail, and analysis invalidate correctly after edits.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [42, 43, 53, 55]
- **Blocks**: [68, 77, 78]
- **Acceptance criteria**: Desire, reality, and note edits work from the field or analysis context and update the appropriate derived surfaces without requiring a full app restart.
- **Effort**: M

### Bead 66: Resolve / Release Integration
- **Description**: Wire resolve and release acts through the confirm dialog and into the new rendering architecture. This bead must also settle post-action selection behavior and how resolved or released items remain visible under different filters.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [44, 53]
- **Blocks**: [68, 77, 78]
- **Acceptance criteria**: Resolve and release behave distinctly, require explicit confirmation, and produce consistent field and analysis updates afterward.
- **Effort**: M

### Bead 67: Move / Reorder Integration
- **Description**: Integrate move, reparent, and reorder flows using the move surface and descended/root field contexts. This must preserve structural clarity, avoid silent reorder side effects, and update relevant derived views and breadcrumbs correctly.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [45, 50, 54, 55]
- **Blocks**: [68, 78]
- **Acceptance criteria**: Reorder and reparent operations can be completed or canceled safely and leave field, tree, and analysis state coherent.
- **Effort**: M

### Bead 68: Undo and Action Feedback Integration
- **Description**: Integrate transient acknowledgements and the undo window for recent actions using the bounded notification adapter. The design rule is that ephemeral feedback must support action trust without becoming the primary channel for structural truth.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [44, 52, 64, 65, 66, 67]
- **Blocks**: [77, 78]
- **Acceptance criteria**: Recent create, edit, resolve, release, and move operations provide recoverable feedback, and structural state remains visible even if notifications are dismissed.
- **Effort**: M

### Bead 69: Watch Lever Count and Review Entry Integration
- **Description**: Integrate pending watch insight counts into the lever and wire the `i` entry path into the watch review surface. This bead closes the loop between the silent daimon queue and the operator’s daily return to the field.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [27, 48, 49]
- **Blocks**: [77, 78]
- **Acceptance criteria**: Pending watch items appear in the lever, open the correct review surface, and clear or decrement correctly when reviewed.
- **Effort**: S

### Bead 70: Agent Entry and Review Integration
- **Description**: Integrate the agent entry path and the review surface into the new component architecture, ensuring that proposals are always reviewable and that the command surface, field, and review queue agree on the same semantics.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [40, 46, 47]
- **Blocks**: [77, 78]
- **Acceptance criteria**: Agent invocation leads into the review flow cleanly and no mutation is applied without passing through review state.
- **Effort**: M

### Bead 71: External Change Reload Integration
- **Description**: Harden the app against external mutations from CLI commands or watch processes by making reload behavior explicit for field, analysis, and queue state. The goal is graceful coherence, not hiding that the world changed under the user.
- **Ring**: 2
- **Priority**: P2
- **Blocked by**: [53, 54, 55]
- **Blocks**: [76, 78]
- **Acceptance criteria**: External updates refresh the relevant views without corrupting selection or leaving stale queue counts behind.
- **Effort**: M

### Bead 72: Monochrome and Low-Color Support
- **Description**: Implement semantic degradation for environments with limited color support. The system must preserve emphasis through dim/bold/reverse and layout hierarchy so that conflict, neglect, selection, and advisories remain distinguishable without relying on full color.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [4, 19, 20, 58, 59]
- **Blocks**: [76, 77]
- **Acceptance criteria**: Snapshot or test runs in degraded style mode retain primary semantic distinctions and do not collapse into illegible flat text.
- **Effort**: S

### Bead 73: Unicode Fallback Support
- **Description**: Implement the runtime or configuration path that swaps Unicode tokens and border weights for the approved ASCII fallbacks when required. This bead should preserve semantics rather than attempting a perfect visual imitation.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [4, 19, 20, 58, 59]
- **Blocks**: [76, 77]
- **Acceptance criteria**: The app can render a coherent field and key overlays using fallback tokens, and snapshots prove the fallback path is real rather than theoretical.
- **Effort**: S

### Bead 74: Large Dataset Performance Budget
- **Description**: Establish and test performance expectations for loading and operating on larger tension fields, including startup, field scrolling, gaze toggling, and analysis opening. This bead ensures the instrument stays “ready to hand” rather than turning into a slow reporting tool.
- **Ring**: 2
- **Priority**: P2
- **Blocked by**: [6, 53, 56, 57]
- **Blocks**: [75, 76]
- **Acceptance criteria**: Baseline timings exist for representative dataset sizes and regressions become measurable.
- **Effort**: M

### Bead 75: Variable-Height List Proof of Concept and Decision
- **Description**: Run the decisive proof of concept for whether `ftui` `VirtualizedList` can replace or encapsulate the current bespoke variable-height list behavior without sacrificing gaze ergonomics, scroll stability, and performance. This bead can end either in adoption or in a formal wrapper decision, but not in continued ambiguity.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [28, 53, 56, 74]
- **Blocks**: [80]
- **Acceptance criteria**: The repo contains a documented decision with measurements and behavior notes; future work no longer treats field-list architecture as unresolved.
- **Effort**: M

### Bead 76: Cross-Terminal Manual Validation Pack
- **Description**: Create the manual validation pack for terminals and environments named in the reflections: color depth, Unicode behavior, resize behavior, keyboard behavior, and external update handling. This bead turns “test later” into a repeatable validation activity.
- **Ring**: 2
- **Priority**: P2
- **Blocked by**: [20, 71, 72, 73, 74]
- **Blocks**: [80]
- **Acceptance criteria**: There is a concrete checklist or script-backed process for validating the design system on representative terminals.
- **Effort**: S

### Bead 77: Golden Snapshot Completion
- **Description**: Fill out the full golden snapshot suite for the new design system: compact field, standard field, expanded field with pinned analysis, gaze open, search, help, review surfaces, and degraded render paths. This bead is the final rendering proof that the system is stable and legible.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [7, 19, 37, 39, 46, 47, 48, 49, 58, 59, 60, 64, 65, 66, 69, 70, 72, 73]
- **Blocks**: [80]
- **Acceptance criteria**: All major surfaces and tiers have golden coverage tied to the canonical fixtures, and failures clearly identify semantic regressions.
- **Effort**: M

### Bead 78: End-to-End Session Walkthrough
- **Description**: Create a scripted end-to-end walkthrough covering the primary operator loop: open field, navigate, open gaze, descend, edit, review insight, review agent suggestion, search, and return. This bead validates that the design system behaves like a coherent instrument rather than a pile of correct components.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: [40, 41, 50, 56, 57, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71]
- **Blocks**: [80]
- **Acceptance criteria**: A developer can execute the walkthrough without undocumented detours, and the session confirms the locked interaction vocabulary in practice.
- **Effort**: M

### Bead 79: Feature-Gated Analysis Extensions
- **Description**: Add clearly bounded extension hooks or placeholder sections for projection trajectory, time-travel replay, and field resonance so future work can plug into the analysis layer without reopening sacred-core questions. This bead does not fully implement those features; it reserves stable insertion points.
- **Ring**: 3
- **Priority**: P3
- **Blocked by**: [31, 33, 51, 60]
- **Blocks**: []
- **Acceptance criteria**: The analysis plane has documented, optional extension seams and no feature-gated idea requires reworking the field, gaze, or lever contracts.
- **Effort**: M

### Bead 80: Design System Handbook and Closeout Checklist
- **Description**: Produce the implementation-facing handbook that summarizes tokens, composites, mode ownership, responsive tiers, review semantics, migration decisions, and validation commands. This is the bead that makes the design system operable by fresh contributors without re-reading every source doc and reflection.
- **Ring**: 2
- **Priority**: P0
- **Blocked by**: [18, 40, 41, 75, 76, 77, 78]
- **Blocks**: []
- **Acceptance criteria**: A new developer can identify component boundaries, target widths, token meanings, and validation steps from one concise handbook and the design system stop-ship criteria are all traceable.
- **Effort**: M

## 23. Critical Path

The longest practical dependency chain is:

1. Bead 1: App Breakpoint Model
2. Bead 5: Canonical Domain Visibility Matrix
3. Bead 6: Golden Fixture Dataset
4. Bead 7: Snapshot Harness for Components and Views
5. Bead 9: Composite Component Scaffolding
6. Bead 10: Field Row View Model
7. Bead 21: `WerkTensionStripe` Component
8. Bead 53: Root Field View Integration
9. Bead 55: Selection and Cursor Preservation
10. Bead 56: Gaze Interaction Integration
11. Bead 57: Analysis Interaction Integration
12. Bead 58: Compact Layout Integration
13. Bead 59: Standard Layout Integration
14. Bead 77: Golden Snapshot Completion
15. Bead 80: Design System Handbook and Closeout Checklist

Why this is the critical path:
- it moves from sacred-core semantics
- to typed view models
- to canonical components
- to the primary operator loop
- to responsive proof
- to closeout validation

## 24. Quick Wins

These beads have high leverage with relatively few blockers:

- Bead 2: Semantic Style Token Module
- Bead 3: Glyph and Dot Token Table
- Bead 18: Renderer Debt Audit and Deletion Map
- Bead 23: Phase Glyph Slot Component
- Bead 27: Lever / Status Line Component
- Bead 37: Empty State Component
- Bead 38: Filter State Indicator
- Bead 41: Help Surface

Why they matter:
- they reduce ambiguity immediately
- they do not require the entire integration stack to exist
- they improve codebase legibility before the largest migrations begin

## 25. Foundational Beads

These beads have the most downstream dependents:

- Bead 1: App Breakpoint Model
- Bead 5: Canonical Domain Visibility Matrix
- Bead 6: Golden Fixture Dataset
- Bead 7: Snapshot Harness for Components and Views
- Bead 8: Focus and Modal Ownership Contract
- Bead 9: Composite Component Scaffolding
- Bead 10: Field Row View Model
- Bead 14: Analysis View Model

If these are wrong:
- nearly every later component or integration bead will churn

## 26. Implementation Phases

### Phase 1: Foundation

Recommended beads:
- 1 through 20

What is true when Phase 1 ends:
- the sacred-core decisions exist as typed contracts, not only prose
- target widths are fixed and testable
- token semantics are centralized
- canonical view models exist for field, gaze, analysis, and review queues
- fixture data and snapshot infrastructure are ready
- modal/focus ownership is explicit
- framework uncertainty has been reduced to named proof points

### Phase 2: Components

Recommended beads:
- 21 through 52

What is true when Phase 2 ends:
- the canonical composite widgets exist
- field, gaze, analysis, lever, rail, search, help, act surfaces, and review cards can all render in isolation
- transient notifications are constrained to their proper role
- the app has reusable building blocks instead of monolithic render logic

### Phase 3: Integration

Recommended beads:
- 53 through 71

What is true when Phase 3 ends:
- root and descended field views are running on the new component architecture
- gaze and analysis obey the locked depth model
- create, edit, resolve, move, watch review, and agent review all work end-to-end
- layout tiers behave differently by doctrine rather than accident
- external state changes can be handled without losing coherence

### Phase 4: Polish

Recommended beads:
- 72 through 78

What is true when Phase 4 ends:
- degraded terminals remain legible
- Unicode fallback is real
- large datasets have known performance characteristics
- the list architecture ambiguity is resolved
- manual validation and golden snapshots cover the real interaction surface

### Phase 5: Hardening and Extensions

Recommended beads:
- 79 through 80

What is true when Phase 5 ends:
- analysis has stable seams for future feature-gated extensions
- new contributors can implement and validate against a concise handbook
- the design system is portable knowledge, not oral tradition

## 27. Final Guidance for Execution

If execution pressure forces sequencing tradeoffs, preserve this order of seriousness:

1. Never trade away sacred-core laws to ship a richer-looking screen sooner.
2. Prefer componentization over heroic view-level rewrites.
3. Prefer typed view models over ad hoc formatting.
4. Prefer snapshots and fixtures over subjective confidence.
5. Prefer a smaller correct field instrument over a larger but ambiguous analytics surface.

If a future conversation proposes a git diff that violates:
- field-first opening
- desire-above-reality
- three-depth model
- six-color semantic discipline
- persistent structural signal hierarchy
- `ftui`-backed component contract

then that proposal is not a refinement of this plan. It is a fork.
