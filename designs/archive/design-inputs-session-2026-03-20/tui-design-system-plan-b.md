# Werk Operative Instrument: TUI Design System Synthesis Plan v1

**Generated:** 2026-03-19
**Methodology:** Agentic Coding Flywheel — Phase 1 (Plan Space)
**Sources:** 5 competing design visions + 3 post-mortem reflections + living codebase
**Status:** V1 — comprehensive but iterative. Decisions marked [UNCERTAIN] need prototyping or user input.

---

# PART I: SYNTHESIS MAP

## 1. Source Analysis

### Source 1: Claude Design (DESIGN_SYSTEM-claude.md)

| Dimension | Assessment |
|-----------|------------|
| **Strongest Ideas** | Implementation-level Rust code for every concept. Virtual list with dynamic heights. Complete element dispatch system. Responsive width thresholds (80/104/140). Accessibility with color-blind alternatives. Caching strategy for computed dynamics. Error handling with graceful degradation. |
| **Gaps / Hand-Waving** | Assumes Panel can contain TextInput without proof. Layout constraint resolution algorithm assumed. No mention of CommandPalette, Tree, or Badge widgets. VirtualizedList from ftui not used — reimplements scrolling. No split-pane layout for wide terminals. |
| **Unique Contributions** | Most complete rendering pipeline (element assembly → height calc → rect assignment → widget render → style application). Pre-computation caching strategy. Terminal capability detection (COLORTERM sniffing). Minimum terminal size handling (40x5). Word-wrap implementation. |
| **Contradicts Others On** | Uses Paragraph for everything rather than Badge/List/Table. Three breakpoints (80/104/140) vs Codex's five-tier system. No CommandPalette — uses custom search. Gaze card as `Panel` with `Paragraph` content vs Codex's `Group` composition. |

### Source 2: Codex Design (DESIGN_SYSTEM-codex.md)

| Dimension | Assessment |
|-----------|------------|
| **Strongest Ideas** | Grounded in actual repo structure (notes sd-core, werk-shared, werk-cli split). Most comprehensive ftui widget catalog with explicit "allowed" lists. Three-tiered signal hierarchy (local/context/action outcome). Five responsive breakpoints (Xs/Sm/Md/Lg/Xl). Canonical rendering strategies table mapping every dynamic to scan/gaze/analysis widgets. Two-line tension stripe (desired above actual). |
| **Gaps / Hand-Waving** | No implementation code — stays at specification level. Widget compositions described but not demonstrated. "The operator should feel..." without concrete acceptance criteria. Pane layout model mentioned but deferred. |
| **Unique Contributions** | Signal hierarchy (local badge → context rail → field board). Explicit "diagnostic-only affordances" list. Quadrant semantics (top-left = organizing principle, bottom-right = next move). `VirtualizedList` as primary field widget. Two-line tension stripe with desired above actual per row. Badge classes enumerated (lifecycle, state, tendency, orientation, urgency, drift, neglect, conflict, assimilation, watch/review). |
| **Contradicts Others On** | Two-line tension stripe vs everyone else's one-line. `BorderType::Square` as default vs Prior doc's `Rounded` for containers. `Tree` for root overview vs flat list. Heavy rule = "rare" vs Prior doc's heavy rule for every descended header. |

### Source 3: Gemini Design (DESIGN_SYSTEM-gemini.md)

| Dimension | Assessment |
|-----------|------------|
| **Strongest Ideas** | "Mirror not dashboard" philosophy — strongest articulation of instrument identity. "Acts not edits" interaction language. Three Depths named precisely (Line → Gaze → Study). Emphasis on gesture as spatial descent. Most concise — avoids over-specification. Tendency arrows (→ ↔ ○) as inline glyphs. |
| **Gaps / Hand-Waving** | Most underspecified of all five. Widget mapping is thin (8 rows vs 30+ in others). No responsive doctrine detail. No implementation code. No alert system design. No agent integration beyond "Agent Card" mention. Framework inference acknowledged as limitation. |
| **Unique Contributions** | "Study" naming for Depth 2 (vs "Analysis" or "Full Gaze"). Dynamics Grid as responsive `Grid` with named areas. Trail Dots (●○) as binary active/quiet per time bucket. Emphasis on "50ms to Field" startup budget. Suggestion of domain-specific ftui widgets (StructuralTensionLine, DynamicsGrid). |
| **Contradicts Others On** | `List` widget as primary field (not `VirtualizedList` or `Paragraph`). Trail Dots (●○) vs six-dot temporal system (◌◦●◎). "Study" depth uses `Grid` with named areas vs Panel sections. `BorderType::Thick` for agent vs Prior doc's `Rounded` for overlays. |

### Source 4: Prior Comprehensive Design (DESIGN_SYSTEM.md)

| Dimension | Assessment |
|-----------|------------|
| **Strongest Ideas** | Most detailed implementation vision with concrete ftui code. Anti-patterns section (11 things to NOT do). Trunk-as-Panel-border innovation (no manual trunk segments). Flex constraint-based layout eliminates char-width math. Design tokens section with concrete constants. Migration strategy in 6 phases. Alert bar as fixed region above lever. Selection as left-edge accent (Panel::borders(LEFT)). |
| **Gaps / Hand-Waving** | Written as if ftui API is fully known, but some assumptions untested. No split-pane wide layout. Assumes `Flex::horizontal().split()` with `FitContent` exists (it does). Some widget compositions (Group::vertical) may not match actual API. |
| **Unique Contributions** | Anti-patterns list (no char math, no manual trunk, no manual cursor, no clearing rects). Trunk as `Panel::borders(LEFT)` wrapping positioned section. Selection as `Panel::borders(LEFT)` + `Heavy` + cyan. Toast for transient messages (lever stays stable). Section headers as `Rule::new().title()`. Dotted separator as `Rule` with custom `BorderSet`. Alert bar as `Badge` widgets in fixed row above lever. Empty state design ("nothing here yet" with ◇). Descended empty state ("no children yet, press a to decompose"). Filter state (resolved/released as dim). Reorder grab handle (≡ glyph). Sparkline for activity history replacing dot trail. MiniBar for magnitude. Content max-width 104 centered. |
| **Contradicts Others On** | `Flex::horizontal()` for tension line layout vs everyone else's Paragraph-based approach. `Badge` for horizon labels vs inline Span. Alert bar above lever vs below reality (Claude) or context rail (Codex). Tab to expand full gaze vs Space for gaze, Tab for full (matches actual codebase). |

### Source 5: K-operative Design (k-operative-design-system.md)

| Dimension | Assessment |
|-----------|------------|
| **Strongest Ideas** | Four-layer architecture (field/focus/act/trace) — clearest layer separation. CommandPalette as primary high-bandwidth surface. Watch/Pulse surface as structural listening layer. Most nuanced mode system (normal/insert/review/command/reorder). Broadest view vocabulary (field, descended channel, focus panel, command surface, watch surface). Interaction principles as design laws. |
| **Gaps / Hand-Waving** | Least implementation-specific. No code examples. Widget mapping is minimal (13 rows). No responsive breakpoints specified. "The lever should feel terse" without concrete specification. Color palette uses semantic names (ink.primary) without color values. |
| **Unique Contributions** | Four-layer architecture (field/focus/act/trace). "Trace" as explicit layer for system memory. Watch/Pulse surface concept. "The content is the chrome" principle. CommandPalette as unifying command + navigation surface. Focus panel with five internal sections (header, definition, dynamics, structure, trace). Mode architecture explicitly separating normal/insert/review/command/reorder. |
| **Contradicts Others On** | Four layers vs three depths (everyone else). CommandPalette-first vs direct-key-first interaction. "Ultra-wide" adds third column vs two-pane max (Prior doc). `ProgressBar` for magnitude vs `MiniBar` (Prior doc). |

---

## 2. Reflection Convergence Analysis

### 2.1 Convergent Questions (flagged by 2+ agents)

| Question | Claude | Codex | Gemini | Decision Impact |
|----------|--------|-------|--------|----------------|
| **ftui widget composition capabilities** | "Panel + TextInput uncertain" | "VirtualizedList variable-height unproven" | "List variable-height support unclear" | BLOCKS all widget binding decisions |
| **Responsive behavior under constraint** | "Surprisingly complex" | "Needs direct testing" | "Width thresholds need concrete breakpoints" | BLOCKS layout architecture |
| **What is primary vs secondary signal** | "Edge case proliferation" | "Which dynamics are first-class?" | "Domain thresholds need tuning" | BLOCKS information architecture |
| **Product stance: daily steering vs periodic review** | Implicit: daily | Explicit question | Implicit: daily | BLOCKS mode and layout decisions |
| **Watch/agent role: core loop vs episodic** | Agent session as mode | "Not settled" | Agent as modal transformation | BLOCKS screen real estate allocation |
| **Implementation gap: vision → concrete widgets** | "Significant interpretation needed" | "Biggest difficulty" | "Mapping to concrete ftui types was challenging" | BLOCKS all implementation work |

### 2.2 Decision Hierarchies (from reflections)

**Claude's Order:**
1. Widget inventory audit
2. Terminal capability detection
3. Basic rendering pipeline
4. Core navigation
5. Basic display (MVP)
6. Progressive disclosure
7. Interaction features
8. Advanced features
9. Polish

**Codex's Order:**
1. Product stance
2. Directional invariants
3. Visual tokens
4. Layer model
5. Interaction/mode model
6. State architecture
7. Component/layout composition
8. Migration sequencing
9. Test strategy

**Gemini's Order:**
1. Layout architecture (base)
2. Expansion mechanism (interaction)
3. Symbol & theme contract (visual)
4. Agent protocol (structural)

**Synthesized Order (this plan):**
1. Directional invariants (spatial law) — foundational, all agree
2. Visual token catalog (glyphs, rules, colors) — all agree this must be locked early
3. Depth/layer model — all agree on three depths
4. Widget binding contract — maps concepts to ftui widgets
5. Layout architecture — responsive breakpoints, constraint system
6. Interaction model — keys, modes, progressive disclosure
7. Component specifications — tension line, gaze card, etc.
8. State architecture — app state shape
9. Integration features — agent, watch, alerts
10. Polish — themes, accessibility, edge cases

### 2.3 Framework Gaps (identified in reflections)

| Gap | Who Flagged | Severity | Mitigation |
|-----|-------------|----------|------------|
| VirtualizedList variable-height rows | Claude, Codex, Gemini | HIGH | Prototype early; fall back to manual vlist if needed |
| Panel + TextInput composition | Claude | MEDIUM | Prototype; ftui Panel accepts any Widget impl |
| Rule with custom BorderSet for dotted lines | Prior doc | LOW | ftui supports Custom(BorderSet) — confirmed in source |
| CommandPalette integration depth | Codex, K-operative | MEDIUM | Start with search/jump; expand to full command surface later |
| Modal backdrop and occlusion | Prior doc | LOW | ftui Modal has BackdropConfig — confirmed |
| Sparkline data format | Prior doc, Gemini | LOW | Sparkline takes &[f64] — confirmed |
| FocusManager for multi-pane navigation | Codex | MEDIUM | Defer split-pane to Ring 3; single-pane focus is simpler |

### 2.4 Hardening Recommendations (synthesized)

1. **Widget gallery prototypes** — Build minimal test programs for: VirtualizedList with variable heights, Panel containing TextInput, Badge in StatusLine, Modal with backdrop, Sparkline with real data, Rule with custom BorderSet
2. **Scenario fixtures** — Create structural scenarios (neglected subtree, oscillating pair, overdue horizon, agent review) and render them deterministically
3. **Snapshot/golden tests** — Render scenarios to buffer, compare against golden files
4. **Responsive invariant tests** — Verify desire-above-actual, phase glyphs visible, alerts survive at every breakpoint
5. **Performance baseline** — Measure dynamics computation for 1/10/100/1000 tensions; measure render time for complex views

---

## 3. Best-of-All-Worlds Synthesis

### What we take from each source:

**From Claude:**
- Complete rendering pipeline architecture (element assembly → height calc → rect assign → widget render)
- Caching strategy for computed dynamics
- Terminal capability detection
- Error handling with graceful degradation
- Word-wrap implementation details

**From Codex:**
- Comprehensive ftui widget catalog and framework contract
- Signal hierarchy (local → context → field)
- Five-tier responsive breakpoints (Xs/Sm/Md/Lg/Xl)
- Canonical rendering strategies table (dynamic × depth × widget)
- Badge class vocabulary
- Quadrant semantics

**From Gemini:**
- "Mirror not dashboard" / "Acts not edits" philosophy
- Concision principle — avoid over-specification
- "50ms to Field" startup budget
- Domain-specific widget suggestion (future consideration)

**From Prior Comprehensive (DESIGN_SYSTEM.md):**
- Anti-patterns (no char math, no manual trunk, no manual cursor)
- Trunk as Panel::borders(LEFT) innovation
- Selection as Panel::borders(LEFT) + Heavy cyan
- Design tokens section
- Alert bar above lever
- Flex constraint-based layout for tension lines
- Migration strategy phasing
- Empty state designs
- Toast for transient messages

**From K-operative:**
- Four-layer architecture (field/focus/act/trace)
- CommandPalette as high-bandwidth unifying surface
- Watch/Pulse as structural listening surface
- "The content is the chrome" principle
- Mode architecture clarity

---

# PART II: ARCHITECTURAL DECISIONS (ADRs)

## ADR-1: Spatial Law

**Decision:** Reality is ground (bottom/left), desire is sky (top/right). This is ABSOLUTE, not contextual.

**Alternatives considered:**
- Claude: Absolute, vertical only (horizontal less emphasized)
- Codex: Absolute with quadrant semantics (top-left = organizing principle, bottom-right = next move)
- Gemini: Absolute ("The single most important concept")
- Prior doc: Absolute with both axes + depth axis + time axis
- K-operative: Absolute ("bottom/left = actuality, top/right = intentionality")

**Rationale:** All five sources agree on the core law. Codex's quadrant semantics add precision without contradiction. Prior doc's four-axis model (vertical, horizontal, depth, time) is the most complete and subsumes the others. We adopt the four-axis model.

**The Four Axes:**
1. **Vertical:** reality (bottom) → desire (top)
2. **Horizontal:** basis/history/constraint (left) → projection/choice/intention (right)
3. **Depth:** ambient field (shallow) → focused panel (medium) → modal commitment (deep)
4. **Time:** older (left) → newer (right) within temporal indicators

**Consequences:**
- Every composed surface must place desired content above actual content
- Headers carry desire; footers carry reality
- Left-aligned content is grounded; right-aligned content is intentional
- Modals represent commitment escalation
- Temporal indicators read left-to-right as past-to-future
- These invariants must survive ALL responsive breakpoints
- These invariants must be tested programmatically

---

## ADR-2: Depth Model

**Decision:** Three depths, additive, named Field / Gaze / Analysis.

**Alternatives considered:**
- Claude: Depth 0/1/2 (Scanning/Focused Gaze/Full Analysis)
- Codex: Layer 1/2/3 (Field Scan/Focused Gaze/Full Analysis)
- Gemini: Depth 0/1/2 (Field/Gaze/Study)
- Prior doc: Field (scanning), Gaze (quick + full), Full Gaze (with dynamics/history)
- K-operative: Four layers (field/focus/act/trace) — different model

**Rationale:** Three additive depths are universally agreed. The Prior doc's distinction between "quick gaze" and "full gaze" maps to sub-depths within Gaze. K-operative's four-layer model is orthogonal — field/focus/act/trace describe *purposes*, while depths describe *information density*. We adopt three depths with K-operative's purpose taxonomy as a cross-cutting concern.

**Depth Specification:**

| Depth | Name | Trigger | What's Added | Widget Surface |
|-------|------|---------|-------------|----------------|
| 0 | **Field** | Default | One line per tension: glyph, desire text, horizon, temporal dots | Paragraph in VirtualizedList or manual vlist |
| 1 | **Gaze** | Space | Inline expansion: children preview, reality text, last event | Panel (Rounded, cyan border) |
| 2 | **Analysis** | Tab (from Gaze) | Dynamics table, mutation history, sparklines, badges | Panel sections with Grid/Columns layout |

**Additive contract:** Each deeper layer REPEATS the shallower layer's visual grammar and ADDS structure. No layer switches metaphors.

**Transition rules:**
- Space on selected → toggles Gaze (Depth 0 ↔ 1)
- Tab on gazed → toggles Analysis (Depth 1 ↔ 2)
- l/Enter → descends into children (new Field view, Depth 0)
- h/Backspace → ascends to parent (previous Field view)
- Gaze and Analysis are in-place expansions; descent is navigation

**Consequences:**
- Gaze card must contain the tension line as its heading (additive)
- Analysis must contain gaze content plus dynamics (additive)
- VirtualizedList (or manual vlist) must handle variable-height items
- Gaze/Analysis expansion changes only the selected item's height

---

## ADR-3: Glyph System

**Decision:** Six lifecycle glyphs + tendency coloring. Glyphs are the primary phase indicator — no separate status badge.

**Glyph Table (LOCKED):**

| Phase/Status | Glyph | Unicode | Meaning |
|-------------|-------|---------|---------|
| Germination | ◇ | U+25C7 | Open, forming, not yet engaged |
| Assimilation | ◆ | U+25C6 | Solid, being worked, substance accumulating |
| Completion | ◈ | U+25C8 | Internal structure visible, nearing closure |
| Momentum | ◉ | U+25C9 | Dense center, energy forward |
| Resolved | ✦ | U+2726 | Crystallized, tension closed by convergence |
| Released | · | U+00B7 | Minimal, tension closed by release |

**Tendency Coloring (applied to glyph):**

| Tendency | Color | Semantic |
|----------|-------|----------|
| Advancing | CLR_CYAN | Forward motion, agency |
| Stagnant | CLR_DEFAULT | No movement |
| Oscillating | CLR_AMBER | Back-and-forth, attention needed |

**Alternatives considered:**
- Gemini proposed tendency arrows (→ ↔ ○) as separate inline glyphs alongside phase glyphs
- K-operative proposed force glyphs (↑ → ↔ ! ⚠) adjacent to lifecycle glyphs
- All five sources agreed on the six lifecycle glyphs exactly

**Rationale:** The six lifecycle glyphs are unanimous across all sources. Tendency is encoded through COLOR on the glyph, not through a separate glyph (Claude, Prior doc approach). This keeps the tension line compact. Additional force/direction glyphs (K-operative's ↑ → ↔) are deferred to badge space in Gaze/Analysis depths where there's room. At Field depth, density demands a single glyph carrying both phase and tendency.

**Consequences:**
- Phase glyph is always the first visual element on a tension line
- Glyph color always indicates tendency (cyan/default/amber)
- Resolved/Released glyphs are always CLR_DIM (tendency is moot for terminal states)
- The glyph system is shared across all depths (Field, Gaze, Analysis, History)
- No separate "status badge" — the glyph IS the status

---

## ADR-4: Color Semantics

**Decision:** Six foreground colors + two background colors. Strict semantic assignment.

**Color Table (LOCKED):**

| Token | Hex | Role | Usage Rules |
|-------|-----|------|-------------|
| `CLR_DEFAULT` | #DCDCDC | Active content | Desire text, active tension content, stagnant tendency |
| `CLR_DIM` | #646464 | Structure/chrome | Borders, rules, labels, resolved/released items, separators |
| `CLR_CYAN` | #50BED2 | Agency/selection | Selected items, gaze borders, operator action, agent accent, advancing tendency |
| `CLR_AMBER` | #C8AA3C | Attention/warning | Oscillation, neglect, staleness, drift, urgency approaching |
| `CLR_RED` | #DC5A5A | Conflict ONLY | Structural conflict between siblings — NOTHING ELSE gets red |
| `CLR_GREEN` | #50BE78 | Advancement | Resolution velocity, gap convergence, momentum, advancing evidence |
| `CLR_BG` | #000000 | Background | Terminal black, no gray, no transparency |
| `CLR_SELECTED_BG` | #23232A | Selection band | Barely perceptible shift for selected line |

**Alternatives considered:**
- Claude: Same six colors but CLR_GREEN for Completion phase (mixing phase and dynamics)
- Codex: Added semantic names (ink.primary/secondary/tertiary, signal.advance/warn/alert/agent/resolution) — useful aliases but same underlying colors
- K-operative: Used semantic naming only (signal.advance, signal.warn) without concrete values
- Gemini: Same palette, less explicit about exclusivity rules

**Rationale:** The Prior doc's six-color palette with strict semantic rules is the strongest. Red-is-conflict-only prevents alert fatigue. Cyan-is-agency gives the operator a consistent "I am here" signal. The Prior doc's concrete hex values match the existing `theme.rs` in werk-tui.

**Color application rules (from Prior doc, adopted):**
1. Cyan is the operator's color — selected, editing, gazing = cyan accented
2. Amber is the system speaking — structural concern, oscillation, neglect
3. Red is RESERVED for conflict — structural conflict between siblings, nothing else
4. Green appears ONLY on evidence of advancement — resolution velocity, gap convergence
5. Default is the working color — active tensions, desire text, majority of screen
6. Dim is structure — borders, rules, labels, resolved items

**Consequences:**
- No rainbow tagging
- Phase does NOT determine color (phase determines glyph shape only)
- Tendency determines glyph color
- Urgency determines temporal indicator color (cyan → amber → red as urgency increases)
- The color vocabulary is shared across all depths and modes
- Theme support: these six colors are the design tokens; dark mode is the only supported mode (terminal black background)

---

## ADR-5: Widget Binding Contract

**Decision:** Every werk domain concept maps to specific ftui widgets. No raw Frame painting. No bespoke ASCII chrome.

**Primary Widget Bindings:**

| Domain Concept | ftui Widget(s) | Configuration |
|----------------|---------------|---------------|
| **Tension line (Field)** | `Paragraph` in `Flex::horizontal()` | 4 constraints: glyph(Fixed 4) + desire(Fill) + horizon(FitContent) + temporal(Fixed 8) |
| **Tension line (selected)** | `Panel` wrapping line | `Borders::LEFT`, `BorderType::Heavy`, cyan border, `CLR_SELECTED_BG` |
| **Gaze card** | `Panel` + internal `Group` | `BorderType::Rounded`, cyan border, internal sections |
| **Analysis sections** | `Columns`/`Grid` inside Panel | Two-column: dynamics left, history right |
| **Desire header (descended)** | `Paragraph` in `Flex::horizontal()` | Bold text + age suffix + horizon + temporal |
| **Reality footer (descended)** | `Paragraph` in `Flex::horizontal()` | Dim text + age suffix |
| **Heavy rule** | `Rule` | `BorderType::Heavy`, dim style |
| **Light rule** | `Rule` | `BorderType::Square`, dim style |
| **Dotted separator** | `Rule` | `Custom(BorderSet)` with dotted chars, dim style |
| **Trunk (positioned section)** | `Panel` | `Borders::LEFT`, `BorderType::Square`, dim, wraps positioned children |
| **Status bar (lever)** | `StatusLine` | Left: breadcrumbs; Right: filter, insight count, help hint |
| **Phase glyph** | `Span` | Colored by tendency |
| **Horizon label** | `Badge` | Dim style, natural width |
| **Temporal indicator** | `Span` sequence | 6 glyphs, colored by urgency |
| **Alert bar** | `Badge` widgets in `Flex` row | Fixed height above lever, amber/red styled |
| **Magnitude bar** | `MiniBar` | 8-width, filled/empty chars |
| **Activity sparkline** | `Sparkline` | 6-12 data points, dim→cyan gradient |
| **Text input (single)** | `TextInput` in `Panel` | Rounded border, cyan, placeholder text |
| **Text input (multi)** | `TextArea` in `Panel` | With soft wrap |
| **Confirm dialog** | `Modal` + `Panel` | Centered, backdrop, bounded size |
| **Search** | `Panel` + `TextInput` + `List` | Full-screen overlay [UNCERTAIN: CommandPalette later] |
| **Help overlay** | `Modal` + `Panel` | Full-screen, Rule section headers |
| **Agent review** | `Panel` + `List` | Checkboxes, heavy rule divider |
| **Insight review** | `Panel` + `List` | Progressive expand (like gaze) |
| **Breadcrumbs** | `StatusItem::Text` | Glyph + truncated name, `›` separator |
| **Spinner (agent)** | `StatusLine` + `Spinner` | Braille animation |
| **Transient message** | `Toast` | BottomCenter, 3s timeout |
| **Empty state** | `Paragraph` in `Panel` | Centered, dim, ◇ glyph, invitation text |

**Alternatives considered:**
- Codex proposed `VirtualizedList` as primary field widget (better for large tension counts)
- Codex proposed `Tree` for root overview (better topology visibility)
- Codex proposed `Table` for dynamics display (better column alignment)
- Claude used `Paragraph` for everything (simpler but less semantic)
- K-operative proposed `ProgressBar` instead of `MiniBar` for magnitude

**Rationale:** The Prior doc's approach of using Flex constraints for layout and Panel for selection/trunk is the most innovative and eliminates the most manual rendering code. We adopt it as the target architecture. However, the current codebase uses manual vlist — migration should be phased. `VirtualizedList` is the target for Field view but may need prototyping to confirm variable-height support.

**Consequences:**
- All rendering code must compose ftui widgets — no direct buffer painting
- The widget binding table is the source of truth for implementation
- Any concept not in this table must be added before it can be rendered
- Widget configurations (border types, styles, constraints) are design tokens

---

## ADR-6: Responsive Doctrine

**Decision:** Five breakpoints matching ftui's built-in Breakpoint system. Content degrades by removing breadth, never meaning.

**Breakpoint Table:**

| Breakpoint | Width | Layout | Behavior |
|-----------|-------|--------|----------|
| **Xs** | < 60 | Single focus | Active tension only; gaze/analysis as modal; phase glyphs + desire text only; temporal dots hidden |
| **Sm** | 60-89 | Single column | Field view; truncated desire; 4-dot temporal; no horizon labels; gaze as modal |
| **Md** | 90-119 | Single column, richer | Full field view; 6-dot temporal; horizon labels; inline gaze; alert bar |
| **Lg** | 120-159 | Optional split | Field left + gaze/analysis right (if split-pane implemented) |
| **Xl** | 160+ | Full split | Field + gaze + analysis visible simultaneously |

**Content max-width:** 104 characters, centered on terminals wider than 104. This is the design target.

**Degradation order (what collapses first):**
1. Analysis comparison tables
2. Secondary history surfaces
3. Verbose horizon prose → abbreviated labels
4. Sparklines and MiniBar visualizations
5. Local child previews in gaze
6. Horizon labels (< 80 columns)
7. Temporal dots (< 60 columns: 6→4→hidden)

**What NEVER collapses:**
- Desired above actual (spatial law)
- Phase glyphs (lifecycle visibility)
- Selection indicator (operator orientation)
- Structural alerts (safety signals)
- Lever/status line (mode awareness)

**Alternatives considered:**
- Claude: Three breakpoints (80/104/140) — too few, misses small terminals
- Codex: Five breakpoints with split-pane progression — adopted
- Gemini: Three width bands (120+/80-120/<80) — adequate but coarse
- Prior doc: No split-pane; max 104 always single column — too conservative for wide terminals

**Rationale:** ftui has native Breakpoint support (Xs/Sm/Md/Lg/Xl) with Responsive widget. Using the framework's breakpoint system rather than custom thresholds reduces implementation burden. Split-pane at Lg+ is aspirational — single-column with inline gaze is the MVP. [UNCERTAIN: Split-pane at Lg/Xl is Ring 3 — may never be implemented if single-column works well enough.]

**Consequences:**
- Responsive layout uses ftui's `Responsive` and `Visibility` widgets
- Content area width = min(terminal_width, 104), centered
- Layout tests must verify at each breakpoint
- Split-pane is optional and deferred to Ring 3

---

## ADR-7: Alert Architecture

**Decision:** Three tiers: local badges on tension lines, alert bar above lever, and toast for action outcomes.

**Alert Tiers:**

| Tier | What | Where | Persistence | Trigger |
|------|------|-------|-------------|---------|
| **Local signal** | Badge on tension line | Right edge of tension line or gaze badge cluster | Until condition clears | Dynamics computation |
| **Alert bar** | Badge row above lever | Fixed 1-row region between content and lever | Until condition clears | Field-level aggregation |
| **Action outcome** | Toast | Bottom-center overlay | 3 seconds | User action result |

**Alert types and visual treatment:**

| Alert | Tier | Style | Glyph |
|-------|------|-------|-------|
| Neglect (3+ weeks) | Alert bar | CLR_AMBER | ⚠ |
| Horizon past | Alert bar | CLR_AMBER | ⚠ |
| Structural conflict | Alert bar | CLR_RED | ⚡ |
| Oscillation detected | Local badge | CLR_AMBER | ↔ |
| Multiple roots | Alert bar | CLR_AMBER | ⚠ |
| Mutation applied | Toast | CLR_CYAN | — |
| Mutation rejected | Toast | CLR_AMBER | — |
| Parse failure | Toast | CLR_RED | — |

**Alert bar implementation:**
```
Screen layout:
  Content area     Constraint::Fill
  Alert bar        Constraint::FitContent (0 or 1 row)
  Lever            Constraint::Fixed(1)
```

Each alert in the bar is a numbered `Badge` (press 1-9 to act on it).

**Alternatives considered:**
- Claude: Alerts as numbered `Paragraph` lines below reality footer — scrolls away
- Codex: Three-tier signal hierarchy (local/context/field board) — most complex, includes dedicated signal board panel
- Prior doc: Alert bar above lever with Badge widgets — cleanest
- K-operative: Alert strip below reality + Toast + NotificationQueue — mixed

**Rationale:** The Prior doc's alert bar is the strongest design: alerts are persistent (not scrollable), numbered for direct action, and positioned between content and lever where they're always visible. Toast is reserved for action outcomes per Codex's signal hierarchy principle.

**Consequences:**
- Alert bar takes 0-1 rows depending on active alerts
- Alerts are computed from dynamics, not stored
- Number keys (1-9) in Normal mode trigger alert actions
- Alert bar uses ftui's `Flex::vertical()` split
- Field-level alerts aggregate: if 5 tensions are neglected, one "5 neglected" badge, not five badges

---

## ADR-8: Interaction Model

**Decision:** Vim-style navigation with modal input. Direct keys for common acts. CommandPalette for discovery and less-common acts.

**Mode Architecture:**

| Mode | Entry | Exit | Primary Keys |
|------|-------|------|-------------|
| **Normal** | Default / Esc from any mode | — | j/k navigate, l/Enter descend, h/Bksp ascend, Space gaze, Tab full gaze |
| **Adding** | `a` from Normal | Esc cancel, Enter advance step | Multi-step: name → desire → reality → horizon |
| **Editing** | `e` from Normal | Esc cancel, Enter submit | Tab cycles fields (desire/reality/horizon) |
| **Annotating** | `n` from Normal | Esc cancel, Enter submit | Single TextArea for note |
| **Confirming** | `r`/`x` from Normal | Esc cancel, y confirm | Resolve or Release confirmation |
| **Moving** | `m` from Normal | Esc cancel, Enter confirm | Search for destination parent |
| **Reordering** | `Shift+J/K` from Normal | Esc cancel, Enter commit | j/k move item, Enter lock position |
| **Searching** | `/` from Normal | Esc cancel, Enter jump | TextInput filter + List results |
| **AgentPrompt** | `@` from Normal | Esc cancel, Enter send | TextArea for agent prompt |
| **ReviewingMutations** | After agent response | Esc cancel, `a` apply | Space toggle, j/k navigate |
| **ReviewingInsights** | `i` from Normal | Esc close | Space expand, j/k navigate, `a` apply, `d` dismiss |
| **Help** | `?` from Normal | Any key | Read-only overlay |

**Key Binding Summary (Normal Mode):**

| Key | Action | Category |
|-----|--------|----------|
| j / ↓ | Move down | Navigation |
| k / ↑ | Move up | Navigation |
| l / Enter | Descend into children | Navigation |
| h / Backspace | Ascend to parent | Navigation |
| g | Jump to top | Navigation |
| G | Jump to bottom | Navigation |
| Space | Toggle gaze (Depth 0↔1) | Disclosure |
| Tab | Toggle analysis (Depth 1↔2) | Disclosure |
| a | Add tension | Act |
| e | Edit selected tension | Act |
| n | Add note to selected | Act |
| r | Resolve selected | Act |
| x | Release selected | Act |
| m | Move/reparent selected | Act |
| u | Undo last mutation | Act |
| Shift+J | Reorder: move selected down | Act |
| Shift+K | Reorder: move selected up | Act |
| @ | Invoke agent on selected | Agent |
| i | Review pending insights | Review |
| / | Search tensions | Navigation |
| f | Cycle filter (Active / All) | View |
| ? | Show help overlay | Help |
| 1-9 | Act on numbered alert | Alert |
| q / Ctrl+C | Quit | System |

**Alternatives considered:**
- K-operative: Five modes (normal/insert/review/command/reorder) with CommandPalette as primary surface
- Gemini: Fewer modes, emphasis on gesture weight ("Acts not edits")
- Claude: Same mode set as current codebase (aligned with this decision)

**Rationale:** The current codebase already implements this model well. Direct keys for frequent acts (add, edit, resolve) with Vim-style navigation. CommandPalette is valuable but deferred — the current search overlay serves the jump function, and direct keys cover common acts. [UNCERTAIN: CommandPalette unification of search + commands is a Ring 3 feature.]

**Consequences:**
- All modes visible in lever/status line
- Mode transitions are explicit and documented
- Each mode has its own input handler
- Esc always returns to Normal mode
- No mode should require more than 2 keypresses to enter

---

# PART III: BINDING CONSTRAINTS

Binary. Testable. Shaping.

## Spatial Constraints

1. **Every visual composition places desired content above actual content.** Zero exceptions. If a widget shows both desire and reality, desire is on top.

2. **The vertical axis ALWAYS encodes reality→desire direction.** Headers are desire. Footers are reality. Children fill the gap.

3. **Left edge carries grounded/current information. Right edge carries intentional/projected information.** Phase glyphs left, temporal indicators right.

4. **These spatial invariants survive every responsive breakpoint.** At Xs width (< 60 cols), desire is still above reality.

## Widget Constraints

5. **Every visual element maps to a named ftui widget type.** No raw `frame.buffer` painting. No `set_cell()` calls outside ftui widget render implementations.

6. **The trunk line is a Panel border, not a manual glyph.** `Panel::borders(Borders::LEFT)` wrapping the positioned children section. No `FieldElement::TrunkSegment`.

7. **Selection is a left-edge accent.** `Panel::borders(Borders::LEFT)` with `BorderType::Heavy` and cyan border. No full-width background span arithmetic.

8. **Rules are ftui Rule widgets, not repeated characters.** `Rule::new()` fills its rect automatically. No `"━".repeat(width)`.

## Information Constraints

9. **Three depth layers: Field (1 line), Gaze (Panel expansion), Analysis (dynamics + history).** No fourth depth. K-operative's "trace" is a purpose category, not a depth.

10. **Phase glyphs are single-character Unicode.** No multi-character sequences for phase indication. ◇◆◈◉✦· only.

11. **Six foreground colors only.** Default, Dim, Cyan, Amber, Red, Green. No additional colors without ADR amendment.

12. **Red is conflict only.** If a UI element is red, it represents structural conflict between siblings. Nothing else.

## Layout Constraints

13. **Content renders at max 104 characters width, centered.** Wider terminals get margins, not wider content.

14. **Minimum viable terminal: 40 columns, 5 rows.** Below this, show "terminal too small" message.

15. **Alert bar is persistent (not scrollable) and positioned between content and lever.** Alerts do not scroll with field content.

16. **Lever (StatusLine) is always the bottom row.** One row, never hidden, shows mode + breadcrumbs + counts.

## Interaction Constraints

17. **Esc always returns to Normal mode.** From any mode, Esc cancels and returns to Normal. No exceptions.

18. **Gaze does not navigate — it reveals.** Space expands in place. Only l/Enter changes the field (descends).

19. **Progressive disclosure is additive.** Each deeper layer contains the shallower layer's content. No metaphor switching.

---

# PART IV: STOP-SHIP CRITERIA

Every one must be checkable before the design system is considered complete:

- [ ] **SC-1: Domain coverage** — Every werk domain concept (tension, phase, tendency, magnitude, conflict, neglect, oscillation, resolution, drift, urgency, orientation, compensating strategy, assimilation, horizon, mutation, event) has a canonical rendering specification at each applicable depth.

- [ ] **SC-2: Widget binding** — Every rendering specification maps to specific ftui widget(s) with configuration parameters. No specification says "render somehow" — it says which widget, which constraints, which styles.

- [ ] **SC-3: Glyph completeness** — Glyph table is complete and unambiguous: one glyph per phase (4), one glyph per terminal state (2), tendency encoded by color (3 colors). No glyph serves double duty.

- [ ] **SC-4: Color/style completeness** — Color/style table is complete with semantic names, hex values, and usage rules matching ftui's Style type. Every styled element references a named style token.

- [ ] **SC-5: Responsive specification** — Responsive behavior specified for Xs (<60), Sm (60-89), Md (90-119), Lg (120-159), Xl (160+) widths. Degradation order documented. "Never collapse" list documented.

- [ ] **SC-6: ASCII mockup** — At least one full-screen ASCII mockup demonstrates the integrated system at Md width (the design target at 104 columns), showing: descended view with desire header, heavy rule, positioned children on trunk, dotted separator, unpositioned children, light rule, reality footer, alert bar, and lever.

- [ ] **SC-7: Implementability** — A developer who has never seen the prior designs can read this plan and implement any single bead without consulting external documents.

---

# PART V: STABILITY RINGS

## Ring 1: Sacred Core (changes here invalidate everything downstream)

- **Spatial law** (ADR-1): Four axes, absolute
- **Depth model** (ADR-2): Three depths, additive
- **Glyph system** (ADR-3): Six glyphs, tendency coloring
- **Color semantics** (ADR-4): Six colors, strict assignment
- **Widget binding contract** (ADR-5): Concept → widget mapping
- **Interaction model** (ADR-8): Vim-style, modal input

## Ring 2: Reusable Components (can evolve independently)

- **Alert architecture** (ADR-7): Three tiers
- **Responsive doctrine** (ADR-6): Five breakpoints, degradation order
- **Badge vocabulary**: Badge classes and their meanings
- **Temporal indicator system**: Six-dot grammar
- **Rule semantics**: Heavy/Light/Dotted meanings
- **Design tokens**: Constants for widths, padding, timing

## Ring 3: Feature-Gated Extras (optional, can be added incrementally)

- **Split-pane layout** at Lg/Xl widths
- **CommandPalette** unification of search + commands
- **Agent session** full visual transformation
- **Watch/Pulse surface** as dedicated panel
- **Theme variants** (light mode, reduced color)
- **Accessibility mode** (color-blind alternatives, screen reader hints)
- **Tree view** for root overview
- **Sparkline** for activity history
- **Performance optimizations** (caching, lazy computation)

---

# PART VI: COMPONENT SPECIFICATIONS

## 6.1 Tension Line (Field Depth — the atomic unit)

**Purpose:** Display one tension in the field. One line. Maximum scan rate.

**Layout (Flex::horizontal with 4 constraints):**

```
  ◆ Build the authentication layer              Mar 20 ◌◌◦◌●◌
  ├──────────────────────────────────────────────────────────────┤
  │glyph│ desire text                      │horizon│ temporal   │
  │Fx(4)│ Fill                             │FitCnt│ Fixed(8)   │
```

**Regions:**
1. **Glyph region** (Fixed 4): 2-char indent + 1-char glyph + 1-char space. Glyph colored by tendency.
2. **Desire region** (Fill): Paragraph with desire text. Truncated to fit. Selected items may word-wrap (height > 1).
3. **Horizon region** (FitContent): Badge with compact horizon label. Hidden if no horizon. Hidden at Sm/Xs width.
4. **Temporal region** (Fixed 8): 6 temporal dots + 2 spacing. Colored by urgency. Reduced to 4 dots at Sm. Hidden at Xs.

**Selection treatment:**
- Unselected: raw Paragraph line, no container
- Selected: `Panel::borders(Borders::LEFT)` + `BorderType::Heavy` + `border_style(CLR_CYAN)` + `style(bg(CLR_SELECTED_BG))`
- The heavy left bar (┃) is the selection indicator

**Resolved/Released treatment:**
- Glyph: ✦ or · in CLR_DIM
- Desire text: CLR_DIM
- Temporal indicator: hidden
- Horizon: hidden
- Entire line recedes visually

**Descended view trunk:**
- Positioned children: wrapped in `Panel::borders(Borders::LEFT)` with `BorderType::Square` (│) in CLR_DIM
- This gives continuous trunk lines for free — no manual trunk segments

**Data contract (FieldEntry):**
```rust
struct FieldEntry {
    id: String,
    desired: String,
    status: TensionStatus,
    phase: CreativeCyclePhase,
    tendency: StructuralTendency,
    has_children: bool,
    position: Option<i32>,
    horizon_label: Option<String>,
    temporal_indicator: String,  // 6 chars
    temporal_urgency: f64,       // 0.0-1.0
}
```

---

## 6.2 Gaze Card (Depth 1 — inline expansion)

**Purpose:** Understand one tension's current state without losing field context.

**Structure (top to bottom = desire to reality):**

```
╭──────────────────────────────────────────────────────────────╮
│ ◆ Build the authentication layer              Mar 20 ◌◌◦◌●◌ │
│ ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ │
│ ◇ Design token storage schema                               │
│ ◆ Implement OAuth2 flow                                     │
│ · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · │
│   ◇ Research session management                             │
│   ◇ Write integration tests                                 │
│ ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ │
│ Using JWT with refresh tokens. Redis for session store.      │
╰──────────────────────────────────────────────────────────────╯
```

**Composition:**
1. **Heading line** — tension line itself, inside the panel (same layout as field line)
2. **Light Rule** — `Rule::new().style(STYLES.dim)` — separates vision from action
3. **Positioned children** — phase glyphs + desire text, with tendency colors
4. **Dotted separator** — `Rule` with custom BorderSet or `· · ·` pattern
5. **Unpositioned children** — indented 2 chars, acknowledged but not committed
6. **Light Rule** — separates action from ground
7. **Reality text** — CLR_DIM. The current actual state. This is the ground.

**Panel configuration:**
- `BorderType::Rounded` (╭╮╰╯)
- Border style: `CLR_CYAN` (the operator is gazing here)
- No background override (terminal bg)

**Empty gaze card** (no children, no reality):
```
╭──────────────────────────────────────────────────────────────╮
│ ◇ Some new tension with nothing yet                          │
╰──────────────────────────────────────────────────────────────╯
```

Emptiness speaks for itself. No "no children" text.

**Children limit:** Max 8 children shown in gaze. If more, show count: `… and 4 more`.

**Data contract (GazeData):**
```rust
struct GazeData {
    id: String,
    actual: String,
    horizon: Option<String>,
    created_at: String,
    children: Vec<ChildPreview>,  // max 8
    last_event: Option<String>,
}
```

---

## 6.3 Analysis View (Depth 2 — full dynamics)

**Purpose:** Total structural immersion. Determine what is happening, why, and what move is warranted.

**Structure (extends gaze card downward):**

```
╭──────────────────────────────────────────────────────────────╮
│ ◆ Build the authentication layer              Mar 20 ◌◌◦◌●◌ │
│ ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ │
│ ◇ Design token storage schema                               │
│ ◆ Implement OAuth2 flow                                     │
│ ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ │
│ Using JWT with refresh tokens.                               │
│ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                              │
│ DYNAMICS                  │ HISTORY                          │
│ phase       assimilation  │ 2h ago    updated reality        │
│ tendency    advancing     │ 1d ago    added child            │
│ magnitude   ████░░░░ .72  │ 3d ago    set horizon            │
│ conflict    —             │ 1w ago    created                │
│ neglect     —             │           ⋮                      │
│ oscillation —             │ 3w ago    initial desire         │
│ drift       stable        │                                  │
│                                                              │
╰──────────────────────────────────────────────────────────────╯
```

**Heavy rule (━) separates operational content from analytical content.** Above: what you act on. Below: what the structure tells you.

**Two-column layout:**
- Left: Dynamics table (label 13w + value)
- Right: History (reverse chronological, relative time + description)

**Dynamics display:**

| Dynamic | Label | Value Format | Color Rule |
|---------|-------|-------------|------------|
| phase | `phase` | Phase name | CLR_DEFAULT |
| tendency | `tendency` | Tendency name | Tendency color |
| magnitude | `magnitude` | MiniBar(8) + decimal | CLR_DEFAULT |
| conflict | `conflict` | Pattern name or `—` | CLR_RED if present, CLR_DIM if absent |
| neglect | `neglect` | Type name or `—` | CLR_AMBER if present, CLR_DIM if absent |
| oscillation | `oscillation` | Reversal count or `—` | CLR_AMBER if present, CLR_DIM if absent |
| resolution | `resolution` | Trend name or `—` | CLR_GREEN if present, CLR_DIM if absent |
| drift | `drift` | Drift type name | CLR_AMBER if non-stable, CLR_DIM if stable |
| orientation | `orientation` | Orientation name | CLR_DIM |
| assimilation | `assimilation` | Depth name | CLR_DIM |

**History display:**
- Reverse chronological
- Format: `{relative_time}    {description}`
- Time: fixed 10-char column, left-aligned
- Max 12 entries shown
- If more: show most recent entries + `⋮` gap + first entry

**Data contract (FullGazeData):**
```rust
struct FullGazeData {
    phase: String,
    tendency: String,
    magnitude: Option<f64>,
    orientation: Option<String>,
    conflict: Option<String>,
    neglect: Option<String>,
    oscillation: Option<String>,
    resolution: Option<String>,
    compensating_strategy: Option<String>,
    assimilation: Option<String>,
    horizon_drift: Option<String>,
    history: Vec<HistoryEntry>,  // max 20
}
```

---

## 6.4 Descended View (Parent Context)

**Purpose:** When descended into a tension's children, the parent frames the entire view.

**Structure:**

```
Build the authentication layer                · 3d ago         Mar 20 ◌◌◦◌●◌
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
│
│ ◇ Design token storage schema                                    ◌◌◌◌◌◎
│ ◆ Implement OAuth2 flow                              Mar 15 ◌◌◦◌●◌
│ ◈ Write migration scripts                            Mar 18 ◌◌◌◦●◌
│
· · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · ·
    ◇ Research session management                                  ◌◌◌◌◌◎
    ◇ Write integration tests                                     ◌◌◌◌◌◎

┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
Using JWT with refresh tokens. Redis for session store.         · 2h ago
```

**Spatial metaphor (literal):**
- **Top** (desire header): where we're going. Bold, present tense.
- **Heavy rule**: commitment boundary. Below = operational space.
- **Trunk line** (│): positioned children structurally committed. Trunk connects to parent's desire.
- **Dotted separator**: boundary between committed and uncommitted.
- **Unpositioned children**: present but not sequenced. Indented 2 chars from trunk.
- **Light rule**: transition back to ground.
- **Bottom** (reality footer): where we are. Dim, grounding, honest.

**Desire header layout (Flex::horizontal):**
```
Constraint::Fill         → desire text (bold)
Constraint::FitContent   → " · 3d ago" (dim, inline age)
Constraint::Fixed(2)     → gap
Constraint::FitContent   → horizon label
Constraint::Fixed(8)     → temporal indicator
```

**Reality footer layout (Flex::horizontal):**
```
Constraint::Fill         → reality text (dim)
Constraint::FitContent   → " · 2h ago" (dim, inline age)
```

**Trunk implementation:**
```rust
Panel::new(positioned_children_group)
    .borders(Borders::LEFT)
    .border_type(BorderType::Square)   // │
    .border_style(STYLES.dim)
    .padding(Sides::left(1))           // 1 cell after border
    .render(positioned_section_rect, frame);
```

**Content area layout (Flex::vertical):**
```
Constraint::FitContent   → desire header (1-3 lines, word-wrapped)
Constraint::Fixed(1)     → heavy rule
Constraint::Fill         → children (scrollable)
Constraint::Fixed(1)     → light rule (if reality exists)
Constraint::FitContent   → reality footer (1-3 lines)
```

---

## 6.5 Lever (Status Bar)

**Purpose:** The operator's grip. Always visible. Shows mode, location, counts.

**Layout:**
```
◆ Build auth › ◇ Token storage                    filter: all  2 insights  ? help
```

**Three regions:**
- **Left:** Breadcrumb path. Format: `glyph name › glyph name › glyph name`. Names truncated to 20 chars.
- **Center:** (empty in Normal mode; shows mode indicator in other modes)
- **Right:** Filter state, insight count, help hint. Items separated by 2 spaces.

**ftui implementation:**
```rust
StatusLine::new()
    .left(StatusItem::Text(&breadcrumb_path))
    .right(StatusItem::KeyHint { key: "?", action: "help" })
    .right(StatusItem::Text(&insight_count))
    .right(StatusItem::Text(&filter_label))
    .separator("  ")
    .style(STYLES.lever)
```

**During agent activity:**
```rust
StatusLine::new()
    .left(StatusItem::Spinner(frame_counter))
    .left(StatusItem::Text(" thinking..."))
    .style(STYLES.cyan)
```

**Mode indicators (shown in lever when not Normal):**
- Adding: `[add] name`
- Editing: `[edit] desire`
- Searching: `[/] search term`
- Reordering: `[reorder] ≡`
- Agent: `[agent] thinking...`
- Review: `[review] 3 mutations`
- Insights: `[insights] 2 pending`

---

## 6.6 Alert Bar

**Purpose:** Persistent, non-scrolling structural signals.

**Layout:** Fixed 1-row region between content and lever. Collapses to 0 rows when no alerts.

**Each alert:** Numbered `Badge` with amber or red styling.

```
1 ⚠ neglect 3w — check reality    2 ⚠ horizon past 5d — extend or close
```

**Implementation:**
```rust
Badge::new(&format!("{} ⚠ {} — {}", num, alert.message, alert.action_hint))
    .with_style(if alert.is_conflict { STYLES.red } else { STYLES.amber })
    .with_padding(1, 1)
```

**Alert computation (from dynamics):**
- Neglect: any tension with last reality update > 3 weeks
- Horizon past: any tension with horizon.range_end() < now
- Conflict: any StructuralConflict detected
- Oscillation: reversals > 4 in window
- Multiple roots: root count > 1 (structural signal, not error)

**Aggregation:** Similar alerts group. "3 tensions neglected" not "tension A neglected, tension B neglected, tension C neglected".

---

## 6.7 Input Surfaces

**Add tension flow:** Panel at insertion point.
```
╭ name ──────────────────────────────────────────────────╮
│ ▏                                                      │
╰────────────────────────────────────────────────────────╯
```

Multi-step: name → (Enter) → desire → (Enter) → reality → (Enter) → horizon (optional, Esc to skip).

**Edit tension flow:** Panel at bottom with field tabs.
```
╭ desire ────────────────────────────────────────────────╮
│ [desire]  reality   horizon                            │
│                                                        │
│ Build the authentication layer▏                        │
╰────────────────────────────────────────────────────────╯
```

Tab cycles active field. Active tab in cyan Badge; inactive in dim Badge.

**Confirm dialog:** Modal widget, centered.
```
╭──────────────────────────────────────────╮
│                                          │
│  resolve "Build authentication layer"?   │
│                                          │
│  y confirm    Esc cancel                 │
│                                          │
╰──────────────────────────────────────────╯
```

---

## 6.8 Search Overlay

**Purpose:** Jump to any tension by name.

```
╭ / ─────────────────────────────────────────────────────╮
│ auth▏                                                  │
│                                                        │
│ ▸ Build the authentication layer       root            │
│   Token storage schema                 › Build auth    │
│   OAuth2 flow                          › Build auth    │
╰────────────────────────────────────────────────────────╯
```

**Full-screen Panel** with TextInput + List. Parent path as dim right-aligned text. `▸` selector from List selection.

---

## 6.9 Help Surface

**Purpose:** Full-screen overlay showing key bindings and visual grammar legend.

**Sections:**
1. NAVIGATION — movement keys
2. ACTS — creation/editing keys
3. READING THE FIELD — glyph legend, temporal dot legend, color meanings

Section headers use `Rule::new().title("NAVIGATION")` — the rule IS the section header.

---

## 6.10 Agent Review Surface

**Purpose:** Review agent-proposed mutations before applying.

```
╭──────────────────────────────────────────────────────────────────╮
│ agent response                                                   │
│ ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ │
│ The authentication layer shows good structural progression.      │
│ Token storage and OAuth2 are advancing. Consider setting         │
│ horizons on the unpositioned children to prevent drift.          │
│ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│ suggested changes                                                │
│                                                                  │
│ ▸ [x] set horizon on "Research session management"    Mar 25     │
│   [x] set horizon on "Write integration tests"       Mar 28     │
│   [ ] add note on "Build auth"                                   │
│                                                                  │
╰──────────────────────────────────────────────────────────────────╯
```

Heavy rule separates reading (agent prose) from doing (mutations). Mutations as List with checkbox selection. Space toggles selection. `a` applies selected. Esc cancels all.

---

## 6.11 Insight Review Surface

**Purpose:** Review watch daemon observations.

Same Panel language as gaze — progressive expand. Space on an insight expands it inline showing observation text + suggested mutations.

---

## 6.12 Empty States

**Root empty state:**
```
╭──────────────────────────────────────────────────────────╮
│                                                          │
│                          ◇                               │
│                                                          │
│                  nothing here yet.                       │
│                                                          │
│              press  a  to name what matters.             │
│                                                          │
╰──────────────────────────────────────────────────────────╯
```

**Descended empty state:**
```
Build the authentication layer                              Mar 20 ◌◌◦◌●◌
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

                        ◇

                no children yet.

            press  a  to decompose.

┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
Using JWT with refresh tokens.                                    · 2h ago
```

"decompose" — not "add." Structural dynamics language: you decompose a tension into sub-tensions.

---

# PART VII: DESIGN TOKENS

```rust
// === Layout ===
const MAX_CONTENT_WIDTH: u16 = 104;
const INDENT: u16 = 2;
const GLYPH_CELL_WIDTH: u16 = 4;          // indent + glyph + space
const TEMPORAL_CELL_WIDTH: u16 = 8;        // 6 dots + 2 spacing
const TRUNK_PADDING_LEFT: u16 = 1;
const MIN_TERMINAL_WIDTH: u16 = 40;
const MIN_TERMINAL_HEIGHT: u16 = 5;

// === Breakpoints (ftui Breakpoint compatible) ===
const BP_SM: u16 = 60;
const BP_MD: u16 = 90;
const BP_LG: u16 = 120;
const BP_XL: u16 = 160;

// === Border types by semantic role ===
const BORDER_STRUCTURAL: BorderType = BorderType::Square;     // trunk, dividers
const BORDER_CONTAINER: BorderType = BorderType::Rounded;     // gaze card, input panel
const BORDER_ACCENT: BorderType = BorderType::Heavy;          // selection indicator
const BORDER_DIVISION: BorderType = BorderType::Heavy;        // operational ↔ analytical
const BORDER_OVERLAY: BorderType = BorderType::Rounded;       // modals, overlays

// === Rule types by semantic role ===
// Heavy rule (━): desire boundary, commitment threshold
// Light rule (─): section divider within containers
// Dotted rule (┄ or ·): positioned/unpositioned boundary, uncertainty

// === Timing ===
const TOAST_DURATION_MS: u64 = 3000;
const TICK_INTERVAL_SECS: u64 = 2;

// === Content limits ===
const BREADCRUMB_MAX_NAME_WIDTH: usize = 20;
const CHILDREN_PREVIEW_MAX: usize = 8;
const HISTORY_DISPLAY_MAX: usize = 12;
const DYNAMICS_LABEL_WIDTH: usize = 13;
const TEMPORAL_DOTS_FULL: usize = 6;
const TEMPORAL_DOTS_COMPACT: usize = 4;

// === Colors ===
const CLR_DEFAULT: PackedRgba = /* #DCDCDC */;
const CLR_DIM: PackedRgba = /* #646464 */;
const CLR_CYAN: PackedRgba = /* #50BED2 */;
const CLR_AMBER: PackedRgba = /* #C8AA3C */;
const CLR_RED: PackedRgba = /* #DC5A5A */;
const CLR_GREEN: PackedRgba = /* #50BE78 */;
const CLR_BG: PackedRgba = /* #000000 */;
const CLR_SELECTED_BG: PackedRgba = /* #23232A */;

// === Glyphs ===
const GLYPH_GERMINATION: char = '◇';
const GLYPH_ASSIMILATION: char = '◆';
const GLYPH_COMPLETION: char = '◈';
const GLYPH_MOMENTUM: char = '◉';
const GLYPH_RESOLVED: char = '✦';
const GLYPH_RELEASED: char = '·';

// === Temporal dots ===
const DOT_EMPTY: char = '◌';
const DOT_NOW: char = '◦';
const DOT_HORIZON: char = '●';
const DOT_STALE: char = '◎';

// === Structural ===
const HEAVY_RULE: char = '━';
const LIGHT_RULE: char = '─';
const TRUNK: char = '│';
const DOTTED_PATTERN: &str = "· · ·";

// === Styles (pre-computed) ===
// STYLES.text_bold  — desire text: CLR_DEFAULT, bold
// STYLES.text       — active content: CLR_DEFAULT
// STYLES.dim        — chrome, labels, resolved: CLR_DIM
// STYLES.label      — dynamics labels: CLR_DIM, fixed-width
// STYLES.cyan       — selection, agent, advancing: CLR_CYAN
// STYLES.amber      — warnings, oscillation: CLR_AMBER
// STYLES.red        — conflict only: CLR_RED
// STYLES.green      — advancement evidence: CLR_GREEN
// STYLES.lever      — status line: CLR_DIM bg, CLR_DEFAULT fg
```

---

# PART VIII: CANONICAL RENDERING STRATEGIES

Complete mapping of every structural dynamic to its rendering at each depth.

| Dynamic | Field (Depth 0) | Gaze (Depth 1) | Analysis (Depth 2) | Primary Widget |
|---------|-----------------|-----------------|---------------------|---------------|
| **Phase** | Glyph shape (◇◆◈◉✦·) | Same glyph in panel heading | `phase` label + name in dynamics table | `Span`, `Badge` |
| **Tendency** | Glyph color (cyan/default/amber) | Same + tendency badge in gaze | `tendency` label + name + Sparkline evidence | `Span`, `Badge`, `Sparkline` |
| **Magnitude** | Not shown (too dense) | Not shown | `magnitude` label + MiniBar(8) + decimal | `MiniBar` |
| **Conflict** | Not shown (alert bar catches it) | Red badge in gaze if present | `conflict` label + pattern name (red) | `Badge`, label |
| **Neglect** | Not shown (alert bar catches it) | Amber badge in gaze if present | `neglect` label + type name (amber) | `Badge`, label |
| **Oscillation** | Not shown (glyph color = amber) | `OSC` badge in gaze | `oscillation` label + reversal count (amber) | `Badge`, label |
| **Resolution** | Not shown | Green badge if resolving | `resolution` label + trend + velocity | `Badge`, label |
| **Horizon drift** | Temporal indicator encodes drift | Horizon badge shows label | `drift` label + type name | `Badge`, label |
| **Urgency** | Temporal indicator color (cyan→amber→red) | Same + urgency badge | Temporal indicator + urgency value | `Span`, `Badge` |
| **Orientation** | Not shown | Not shown | `orientation` label + name | label |
| **Compensating strategy** | Not shown | Amber badge if detected | Named in dynamics or absent | `Badge`, label |
| **Assimilation** | Not shown | Not shown | `assimilation` label + depth | label |
| **History** | Not shown | Last event line | Reverse chronological history list | `Paragraph` |

**Progressive density:** Field shows 2 signals (phase glyph, temporal dots). Gaze adds 3-4 (children, reality, badges). Analysis shows all 13 dynamics + history.

---

# PART IX: ANTI-PATTERNS

Things this design system explicitly forbids:

1. **No manual character-width arithmetic.** All layout through `Flex`/`Layout` constraints. Unicode width handled by ftui internals.

2. **No background-band span padding.** Selection via `Panel::borders(LEFT)` + `.style(bg)`. The Panel handles geometry.

3. **No manual trunk segment insertion.** Trunk via `Panel::borders(LEFT)` wrapping positioned section. Continuous by construction.

4. **No manual cursor rendering** (█ blocks). `TextInput` handles cursor display.

5. **No manual rule repetition** (`"━".repeat(w)`). `Rule` widget fills its rect automatically.

6. **No `chars().count()` for layout math.** ftui handles display width internally.

7. **No clearing rects for overlays.** `Modal` with backdrop handles occlusion.

8. **No mixed concerns in render functions.** Each component maps to one widget composition.

9. **No raw Frame::buffer painting.** All rendering through widget composition.

10. **No color for decoration.** Every color application must map to a semantic meaning from ADR-4.

11. **No prose where structure suffices.** If a badge, glyph, or bar can communicate the information, don't use a sentence.

---

# PART X: FULL-SCREEN ASCII MOCKUP

## Mockup 1: Descended View at Md Width (104 columns)

This is the primary daily view — inside a parent tension, seeing its children.

```
  Build a sustainable revenue engine              · 1w ago       Q2 2026 ◌◦◌◌◌●
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  │
  │ ◆ Design pricing tiers and packaging                       Apr 15 ◌◌◦◌◌●
  │ ◇ Build payment integration with Stripe                    Apr 30 ◌◦◌◌◌●
  ┃ ◆ Launch beta program with 10 customers                    Mar 28 ◌◌◌◦●◌
  │ ◈ Create customer success playbook                         Mar 25 ◌◌◌◌◦●
  │
  · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · ·
      ◇ Analyze competitor pricing                                     ◌◌◌◌◌◎
      ◇ Draft partnership proposal                                     ◌◌◌◌◌◎
      ◇ Research enterprise licensing models                           ◌◌◌◌◌◎

  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  Revenue is $0. We have 3 design partners using free tier. No paying   · 1w ago
  customers yet. Stripe account created but no products configured.

  1 ⚠ 2 tensions neglected — check reality    2 ⚠ horizon approaching on "beta program"
  ◆ Revenue › ◆ Beta program                             filter: active  1 insight  ? help
```

**Reading this mockup:**

- **Top**: Parent desire ("Build a sustainable revenue engine") with age, horizon, temporal indicator
- **Heavy rule**: Commitment boundary
- **Trunk section** (│): 4 positioned children, structurally committed. The `┃` on "Launch beta program" indicates selection (Heavy LEFT border, cyan).
- **Dotted separator**: Boundary between committed and uncommitted
- **Indented section**: 3 unpositioned children, acknowledged but not sequenced
- **Light rule**: Transition to reality
- **Bottom**: Parent reality (dim, grounding, honest) with age
- **Alert bar**: 2 persistent structural signals with numbered actions
- **Lever**: Breadcrumb path, filter, insight count, help hint

---

## Mockup 2: Gaze Card Expanded (Space pressed on "Launch beta program")

```
  Build a sustainable revenue engine              · 1w ago       Q2 2026 ◌◦◌◌◌●
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  │
  │ ◆ Design pricing tiers and packaging                       Apr 15 ◌◌◦◌◌●
  │ ◇ Build payment integration with Stripe                    Apr 30 ◌◦◌◌◌●
  │ ╭──────────────────────────────────────────────────────────────────────────────────╮
  │ │ ◆ Launch beta program with 10 customers                  Mar 28 ◌◌◌◦●◌         │
  │ │ ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ │
  │ │ ◆ Identify and contact 20 potential beta users                                  │
  │ │ ◇ Set up onboarding flow                                                        │
  │ │ ◇ Create feedback collection mechanism                                          │
  │ │ ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ │
  │ │ Have verbal commitments from 5 design partners. No formal beta                  │
  │ │ agreement yet. Onboarding is manual email.                                      │
  │ ╰──────────────────────────────────────────────────────────────────────────────────╯
  │ ◈ Create customer success playbook                         Mar 25 ◌◌◌◌◦●
  │
  · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · ·
      ◇ Analyze competitor pricing                                     ◌◌◌◌◌◎

  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  Revenue is $0. We have 3 design partners using free tier.            · 1w ago

  1 ⚠ 2 tensions neglected — check reality
  ◆ Revenue › ◆ Beta program                             filter: active  1 insight  ? help
```

**Reading this mockup:**
- The gaze card (╭╯ rounded border, cyan) expands inline, pushing siblings down
- Inside: heading line → light rule → children → light rule → reality
- The card is additive — same glyph/horizon/temporal grammar as the field line
- Other siblings remain visible, providing context

---

# PART XI: BEADS

## Foundation Beads (Ring 1, P0)

### Bead 1: Spatial Law Constants and Validation
- **Description**: Define the four-axis spatial law as code constants and validation functions. Create a `spatial.rs` module that exports: `AXIS_VERTICAL` (reality=bottom, desire=top), `AXIS_HORIZONTAL` (basis=left, intention=right), `AXIS_DEPTH` (field=shallow, modal=deep), `AXIS_TIME` (older=left, newer=right). Include a `validate_composition(desired_rect, actual_rect) -> bool` function that verifies desire is above actual in any given layout. This is the foundation all other rendering builds on.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: None
- **Blocks**: 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20
- **Acceptance criteria**: Module compiles. `validate_composition` returns false when desire_rect.y > actual_rect.y. Constants are documented with the four-axis model from ADR-1.
- **Effort**: S

### Bead 2: Design Token Module
- **Description**: Create a `tokens.rs` module exporting all design tokens from Part VII of the plan. This includes: layout constants (MAX_CONTENT_WIDTH=104, INDENT=2, GLYPH_CELL_WIDTH=4, TEMPORAL_CELL_WIDTH=8), breakpoint thresholds (SM=60, MD=90, LG=120, XL=160), border type assignments (STRUCTURAL=Square, CONTAINER=Rounded, ACCENT=Heavy, DIVISION=Heavy), timing constants (TOAST_DURATION=3000ms, TICK_INTERVAL=2s), content limits (BREADCRUMB_MAX=20, CHILDREN_PREVIEW_MAX=8, HISTORY_MAX=12), all glyph characters, all temporal dot characters, and all structural separator characters. Every constant must have a doc comment explaining its semantic role.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: None
- **Blocks**: 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20
- **Acceptance criteria**: Module compiles. Every constant from Part VII is present. Doc comments reference the relevant ADR.
- **Effort**: S

### Bead 3: Color Palette and Style Precomputation
- **Description**: Refactor the existing `theme.rs` to align with ADR-4. Ensure the six foreground colors (DEFAULT=#DCDCDC, DIM=#646464, CYAN=#50BED2, AMBER=#C8AA3C, RED=#DC5A5A, GREEN=#50BE78) and two background colors (BG=#000000, SELECTED_BG=#23232A) are exported as named constants. Create a `Styles` struct with pre-computed ftui `Style` instances: `text_bold` (DEFAULT+bold), `text` (DEFAULT), `dim` (DIM), `label` (DIM), `cyan` (CYAN), `amber` (AMBER), `red` (RED), `green` (GREEN), `lever` (DIM bg + DEFAULT fg). Each style must have a doc comment explaining when to use it per ADR-4 color rules (e.g., "red: conflict only — structural conflict between siblings, nothing else").
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: 2
- **Blocks**: 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20
- **Acceptance criteria**: `STYLES` global is accessible. Each color has correct hex value. Doc comments enforce semantic rules. Existing theme.rs functionality preserved.
- **Effort**: S

### Bead 4: Glyph System Module
- **Description**: Refactor the existing `glyphs.rs` to align with ADR-3. Export: phase glyphs (GERMINATION=◇, ASSIMILATION=◆, COMPLETION=◈, MOMENTUM=◉), terminal glyphs (RESOLVED=✦, RELEASED=·), temporal dots (EMPTY=◌, NOW=◦, HORIZON=●, STALE=◎), structural separators (HEAVY_RULE=━, LIGHT_RULE=─, TRUNK=│). Add a `phase_glyph(phase: CreativeCyclePhase) -> char` function and a `status_glyph(status: TensionStatus, phase: CreativeCyclePhase) -> char` function. Add a `tendency_color(tendency: StructuralTendency) -> PackedRgba` function returning CLR_CYAN for Advancing, CLR_DEFAULT for Stagnant, CLR_AMBER for Oscillating. Preserve the existing `temporal_indicator()` and `compact_horizon()` functions.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: 2, 3
- **Blocks**: 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20
- **Acceptance criteria**: All glyphs match the locked table in ADR-3. `phase_glyph` and `status_glyph` cover all enum variants. `tendency_color` returns correct colors. Existing temporal_indicator and compact_horizon pass their tests.
- **Effort**: S

### Bead 5: Screen Layout Skeleton
- **Description**: Implement the top-level screen decomposition as a `Flex::vertical()` split: Content (Fill) + Alert Bar (FitContent, 0-1 rows) + Lever (Fixed 1). Create a `screen_layout(terminal_rect: Rect, has_alerts: bool) -> (Rect, Option<Rect>, Rect)` function that returns the three regions. Implement the content area width constraint: `content_area(terminal_rect: Rect) -> Rect` that constrains to MAX_CONTENT_WIDTH=104 and centers horizontally. This replaces the ad-hoc content area calculation in the current render.rs.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: 2
- **Blocks**: 6, 7, 8, 9, 10, 14, 15, 16, 17
- **Acceptance criteria**: `screen_layout` correctly splits terminal rect into three regions. Content area never exceeds 104 width. Content area is centered when terminal > 104. Alert bar region is None when no alerts. Lever is always 1 row at bottom.
- **Effort**: S

### Bead 6: Tension Line Renderer (Flex-Based)
- **Description**: Implement the tension line as a `Flex::horizontal()` with four constraints per Section 6.1: Glyph(Fixed 4) + Desire(Fill) + Horizon(FitContent) + Temporal(Fixed 8). Create a `render_tension_line(entry: &FieldEntry, line_rect: Rect, selected: bool, frame: &mut Frame)` function. The glyph region renders `phase_glyph()` colored by `tendency_color()`. The desire region renders desire text as a Paragraph, truncated to fit. The horizon region renders a Badge with compact horizon label (hidden if None). The temporal region renders the six-dot indicator colored by urgency. When `selected`, wrap the entire line in `Panel::borders(Borders::LEFT)` with `BorderType::Heavy` and `CLR_CYAN` border + `CLR_SELECTED_BG` background. This REPLACES the manual char-width arithmetic in the current render.rs tension line code.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: 2, 3, 4, 5
- **Blocks**: 7, 8, 10, 14, 17
- **Acceptance criteria**: Tension line renders correctly at 80, 104, and 140 column widths. Glyph is correct shape and color. Desire text truncates rather than overflows. Horizon badge appears/hides correctly. Temporal dots show correct urgency color. Selection shows left-edge cyan heavy bar. No manual character-width arithmetic — all layout via Flex constraints.
- **Effort**: M

### Bead 7: Descended View Structure
- **Description**: Implement the descended view layout per Section 6.4. Create a `render_descended_view(parent: &Tension, children: &[FieldEntry], content_rect: Rect, frame: &mut Frame)` function using `Flex::vertical()` with constraints: DesireHeader(FitContent) + HeavyRule(Fixed 1) + Children(Fill) + LightRule(Fixed 1, conditional) + RealityFooter(FitContent, conditional). The desire header uses `Flex::horizontal()` with desire text (Fill) + age suffix (FitContent) + gap (Fixed 2) + horizon (FitContent) + temporal (Fixed 8). The reality footer uses `Flex::horizontal()` with reality text (Fill) + age suffix (FitContent). Between positioned and unpositioned children, render a dotted separator Rule. Wrap positioned children in `Panel::borders(Borders::LEFT)` with `BorderType::Square` (trunk line). This REPLACES the manual FieldElement::DesireHeader, FieldElement::HeavyRule, FieldElement::TrunkSegment, FieldElement::DottedSeparator, FieldElement::LightRule, FieldElement::RealityFooter elements.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: 2, 3, 4, 5, 6
- **Blocks**: 8, 10, 14, 17
- **Acceptance criteria**: Desire header is above heavy rule. Heavy rule spans full content width. Positioned children have continuous trunk line on left (from Panel border). Dotted separator appears between positioned and unpositioned groups. Reality footer is below light rule. Reality footer absent when parent.actual is empty. Spatial law validated: desire above reality.
- **Effort**: M

### Bead 8: Gaze Card Renderer
- **Description**: Implement the gaze card per Section 6.2. Create a `render_gaze_card(entry: &FieldEntry, gaze_data: &GazeData, card_rect: Rect, frame: &mut Frame)` function. The card is a `Panel` with `BorderType::Rounded` and `CLR_CYAN` border containing: heading line (same as tension line) → light Rule → positioned children preview → dotted separator → unpositioned children preview → light Rule → reality text (dim). Children are limited to CHILDREN_PREVIEW_MAX=8. Empty gaze card (no children, no reality) shows only the heading line. The gaze card replaces the selected tension line in the field — it occupies the same slot but with larger height. Height calculation must be deterministic for virtual list scrolling.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: 2, 3, 4, 5, 6
- **Blocks**: 9, 10, 14, 17
- **Acceptance criteria**: Card has rounded cyan border. Heading line matches field tension line format. Children show with phase glyphs. Positioned and unpositioned separated by dotted line. Reality text is dim at bottom. Empty card handles gracefully. Height can be pre-calculated given content and width.
- **Effort**: M

### Bead 9: Analysis View Renderer
- **Description**: Implement the analysis expansion per Section 6.3. Create a `render_analysis_section(full_data: &FullGazeData, section_rect: Rect, frame: &mut Frame)` function. This renders below the gaze card content, separated by a heavy Rule (━). Layout is two-column: dynamics table (left, Ratio 1:2) + history (right, Ratio 1:2) with a Fixed(3) divider between them. Dynamics table: each row is `label(13w) + value` for 10 dynamics (phase, tendency, magnitude, conflict, neglect, oscillation, resolution, drift, orientation, assimilation). Magnitude uses MiniBar(8). Absent dynamics show "—" in dim. History: reverse chronological, relative time (10w) + description (fill). Max 12 entries with `⋮` gap for overflow.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: 2, 3, 4, 8
- **Blocks**: 10, 14, 17
- **Acceptance criteria**: Heavy rule separates operational from analytical content. Two columns render without overlap. Dynamics labels are left-aligned at 13 chars. MiniBar renders for magnitude. Absent dynamics are dim dash. History is reverse chronological. Overflow uses ⋮ gap.
- **Effort**: M

### Bead 10: Lever (StatusLine) Renderer
- **Description**: Implement the lever per Section 6.5. Refactor the existing StatusLine rendering to use proper `StatusItem` composition: Left = breadcrumb path (glyph + truncated name, `›` separator), Right = filter state + insight count + "? help". Add mode indicators: when not Normal, show mode tag (e.g., `[add] name`, `[edit] desire`, `[/] search`). During agent activity, show `Spinner` + "thinking...". The lever is always Fixed(1) at the bottom of the screen.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: 2, 3, 4, 5
- **Blocks**: 14, 17
- **Acceptance criteria**: Lever shows breadcrumb with phase glyphs. Lever shows filter, insight count, help hint. Mode indicator appears for non-Normal modes. Spinner appears during agent activity. Lever is always 1 row at bottom.
- **Effort**: S

## Component Beads (Ring 2, P1)

### Bead 11: Alert Bar Renderer
- **Description**: Implement the alert bar per Section 6.6. Create a `render_alert_bar(alerts: &[Alert], bar_rect: Rect, frame: &mut Frame)` function. Each alert is a numbered `Badge` with amber or red styling. Alerts are computed from dynamics: neglect (3+ weeks), horizon past, structural conflict, oscillation (>4 reversals), multiple roots. Similar alerts aggregate ("3 neglected" not "A neglected, B neglected, C neglected"). The alert bar occupies 0-1 rows between content and lever (from Bead 5). Pressing number keys 1-9 in Normal mode triggers the corresponding alert action.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: 2, 3, 5
- **Blocks**: 14, 17
- **Acceptance criteria**: Alert bar renders numbered Badge widgets. Amber for warnings, red for conflict. Alerts aggregate by type. Bar collapses to 0 rows when no alerts. Number keys route to alert actions.
- **Effort**: M

### Bead 12: Rule Widget Integration
- **Description**: Replace all manual rule rendering (repeated characters) with ftui `Rule` widget. Heavy rule: `Rule::new().border_type(BorderType::Heavy).style(STYLES.dim)`. Light rule: `Rule::new().border_type(BorderType::Square).style(STYLES.dim)`. Dotted separator: `Rule::new().border_type(BorderType::Custom(BorderSet { horizontal: '┄', ..BorderSet::SQUARE })).style(STYLES.dim)` — OR if Custom BorderSet doesn't support single-char dot pattern, use a `Paragraph` with `"· · · ..."` as fallback. Remove all `"━".repeat(width)` and `"─".repeat(width)` patterns from render.rs.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: 2, 3
- **Blocks**: 7, 8, 9
- **Acceptance criteria**: No manual rule repetition in codebase. All rules use ftui Rule widget or documented fallback. Heavy, light, and dotted rules visually match design tokens.
- **Effort**: S

### Bead 13: Badge System for Structural Signals
- **Description**: Implement Badge usage for compressed structural signals. Define Badge classes per Codex's vocabulary: phase (glyph + label), tendency (ADV/OSC/STAG), urgency (NOW/SOON/LATE), conflict (CONFLICT, red), neglect (amber), drift (STABLE/POSTP/LOOSE). Create a `badge_for_dynamic(dynamic: &str, value: &str) -> Badge` factory function. Badges use consistent padding (1,1) and semantic colors. Badges appear in: gaze card header cluster, alert bar, analysis dynamics column (where label alone is insufficient).
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: 2, 3
- **Blocks**: 8, 9, 11
- **Acceptance criteria**: Badge factory covers all dynamic types. Badges use correct semantic colors. Padding is consistent. Badges render at correct size in all contexts.
- **Effort**: S

### Bead 14: Input Surface: Add Tension Flow
- **Description**: Implement the multi-step add tension flow per Section 6.7. When `a` is pressed in Normal mode, render a `Panel` with `BorderType::Rounded` and `CLR_CYAN` border at the cursor position containing a `TextInput`. Multi-step flow: Step 1 "name" (title = "name", placeholder = "what matters?") → Enter → Step 2 "desire" (pre-filled with name) → Enter → Step 3 "reality" (placeholder = "what is true now?") → Enter → Step 4 "horizon" (optional, Esc to skip, placeholder = "when? e.g., Mar, Apr 15, 2026"). Esc at any step cancels. Backspace on empty field goes back one step. Each step title shown in Panel title. Current step validates on Enter (name cannot be empty).
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: 2, 3, 5, 6
- **Blocks**: 17
- **Acceptance criteria**: Panel appears at cursor position with correct border. TextInput handles text entry with cursor. Multi-step flow advances correctly. Esc cancels from any step. Backspace on empty goes back. Validation prevents empty name. Horizon parsing uses existing horizon.rs.
- **Effort**: M

### Bead 15: Input Surface: Edit Tension Flow
- **Description**: Implement the edit tension flow per Section 6.7. When `e` is pressed, render a `Panel` at bottom with tab indicators: `[desire]  reality  horizon`. Active tab is cyan Badge, inactive are dim. Tab key cycles between fields. TextInput shows current value of active field. Enter submits. Esc cancels. Field values are loaded from the selected tension. Only changed fields emit mutations.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: 2, 3, 5, 6
- **Blocks**: 17
- **Acceptance criteria**: Panel appears at bottom with three tab indicators. Tab cycles correctly. Active tab is cyan, inactive dim. Current values pre-loaded. Enter submits only changed fields. Esc cancels without mutation.
- **Effort**: M

### Bead 16: Input Surface: Confirm Dialog
- **Description**: Implement resolve/release confirmation as `Modal` widget per Section 6.7. When `r` (resolve) or `x` (release) is pressed, show centered Modal with: action description, tension name, and `y confirm  Esc cancel` hints. `y` confirms and emits mutation. Esc cancels. Modal uses `BorderType::Rounded`, backdrop dimming.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: 2, 3
- **Blocks**: 17
- **Acceptance criteria**: Modal appears centered. Backdrop dims content. y confirms action. Esc cancels. Correct mutation emitted on confirm.
- **Effort**: S

### Bead 17: View Integration: Complete Field View
- **Description**: Integrate all foundation and component beads into a complete field view renderer. Replace the current `render_field()` in render.rs with a composition of: screen_layout (Bead 5) → content_area → descended_view (Bead 7) with tension_lines (Bead 6) and optional gaze_card (Bead 8) and analysis (Bead 9) → alert_bar (Bead 11) → lever (Bead 10). Handle root view (no parent) and descended view (with parent). Handle empty states (Bead 26). Handle all input modes by switching to appropriate input surface (Beads 14-16). This is the main integration point — all previous beads feed into this.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: 5, 6, 7, 8, 9, 10, 11
- **Blocks**: 18, 19, 20
- **Acceptance criteria**: Full screen renders correctly at 80, 104, and 140 widths. Descended view shows desire header, trunk, children, reality footer. Gaze card expands inline. Analysis section shows below gaze. Alert bar shows above lever. Lever shows breadcrumbs. All input modes render correct surfaces. No manual char-width arithmetic remains in render.rs.
- **Effort**: L

### Bead 18: Responsive Layout Adaptation
- **Description**: Implement responsive behavior per ADR-6. Use ftui's Breakpoint system to adapt layout at each tier: Xs (<60) — hide temporal dots, hide horizon labels, minimal desire text only. Sm (60-89) — 4-dot temporal, truncated desire, no horizon labels. Md (90-119) — full layout as designed (the target). Lg (120-159) — same as Md with wider margins. Xl (160+) — same as Lg. [FUTURE: split-pane at Lg/Xl.] Implement a `responsive_config(width: u16) -> ResponsiveConfig` struct that controls: temporal_dots (6/4/0), show_horizon (bool), show_annotations (bool), max_content_width. Use `Visibility` widget to hide/show elements by breakpoint.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: 2, 5, 6, 17
- **Blocks**: 19
- **Acceptance criteria**: At Xs: only glyphs + desire text visible. At Sm: 4-dot temporal added. At Md: full layout. Spatial law holds at all breakpoints (desire above reality). Phase glyphs never hidden. Selection indicator never hidden.
- **Effort**: M

### Bead 19: Search Overlay
- **Description**: Implement the search overlay per Section 6.8. When `/` is pressed, render a full-screen `Panel` with `TextInput` for search query and `List` for results. Results are filtered tensions matching query text (case-insensitive substring). Each result shows desire text + dim parent path on right. `▸` selector from List selection. j/k navigate results. Enter jumps to selected tension (descends into its parent, selects it). Esc closes search.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: 2, 3, 5, 17
- **Blocks**: None
- **Acceptance criteria**: Search overlay appears on `/`. TextInput accepts query. Results filter in real-time. j/k navigate results. Enter jumps to correct tension. Esc closes without navigation. Parent path shown for context.
- **Effort**: M

### Bead 20: Help Overlay
- **Description**: Implement the help overlay per Section 6.9. When `?` is pressed, render a full-screen `Modal` with `Panel` inside. Three sections: NAVIGATION (movement keys table), ACTS (creation/editing keys table), READING THE FIELD (glyph legend, temporal dot legend, color meanings). Section headers use `Rule::new().title("NAVIGATION")`. Key hints: key in cyan, description in default. Press any key to close.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: 2, 3, 12
- **Blocks**: None
- **Acceptance criteria**: Help overlay appears on `?`. Three sections with Rule headers. All key bindings documented. Glyph legend complete. Any key closes overlay.
- **Effort**: M

## Integration Beads (Ring 2-3, P1-P2)

### Bead 21: Agent Review Surface
- **Description**: Implement agent mutation review per Section 6.10. After agent response arrives, render a full-screen `Panel` with: agent prose (Paragraph) → heavy Rule → mutation list (List with checkboxes). Space toggles mutation selection. j/k navigate mutations. `a` applies all selected mutations. Esc cancels all. Each mutation shows: cursor indicator (▸), checkbox ([x]/[ ]), action label, tension name, value. Currently the agent integration uses a custom format — preserve existing parsing but replace rendering with this widget composition.
- **Ring**: 3
- **Priority**: P2
- **Blocked by**: 2, 3, 12, 17
- **Blocks**: None
- **Acceptance criteria**: Agent prose above heavy rule. Mutations below with checkboxes. Space toggles. j/k navigate. `a` applies selected. Esc cancels. Existing agent parsing preserved.
- **Effort**: M

### Bead 22: Insight Review Surface
- **Description**: Implement watch insight review per Section 6.11. When `i` is pressed, render a `Panel` with List of pending insights. Each insight shows: trigger type badge + tension name. Space on an insight expands it inline (progressive disclosure, like gaze) showing: observation text + suggested mutations. j/k navigate. `a` on expanded insight applies its mutations. `d` dismisses. Esc closes. Preserves existing insight file loading from `.werk/watch/pending/`.
- **Ring**: 3
- **Priority**: P2
- **Blocked by**: 2, 3, 12, 17
- **Blocks**: None
- **Acceptance criteria**: Insight list shows with trigger badges. Space expands inline. Mutations shown in expanded view. Apply and dismiss work. Existing file loading preserved.
- **Effort**: M

### Bead 23: Reorder Mode Visual
- **Description**: Implement the reorder mode visual treatment. When Shift+J/K enters reorder mode, the grabbed tension shows: `≡` grab handle (cyan Badge) replacing the phase glyph, left-edge heavy cyan border (same as selection). j/k moves the item visually (swaps with adjacent). Enter commits new position. Esc cancels. The grab handle `≡` is rendered as `Badge::new("≡").with_style(STYLES.cyan)`. Position math preserves the positioned/unpositioned boundary — positioned items can only reorder among positioned items.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: 6, 17
- **Blocks**: None
- **Acceptance criteria**: ≡ grab handle appears on grabbed item. j/k swap visually. Enter commits with correct position values. Esc restores original order. Positioned/unpositioned boundary respected.
- **Effort**: S

### Bead 24: Toast System for Action Outcomes
- **Description**: Implement Toast notifications for action outcomes per ADR-7. When a mutation is applied (add, edit, resolve, release, move, reorder, agent apply), show a `Toast` at BottomCenter for 3 seconds with the outcome message. Success: cyan style. Failure: amber style. Error: red style. Use ftui's `Toast` and `NotificationQueue` widgets. This replaces the current transient message system (which overwrites the lever).
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: 2, 3
- **Blocks**: None
- **Acceptance criteria**: Toast appears on mutation success. Toast auto-dismisses after 3s. Lever remains stable (not overwritten). Success/failure/error use correct semantic colors.
- **Effort**: S

### Bead 25: Spinner for Agent Activity
- **Description**: When an agent invocation is in progress, replace the lever content with `StatusLine::Spinner` showing braille animation (⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏) + "thinking..." in cyan. The spinner provides subtle sign of computational life. When agent response arrives, spinner stops and lever returns to normal. Use ftui's built-in `Spinner` widget.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: 10
- **Blocks**: None
- **Acceptance criteria**: Spinner appears during agent activity. Animation cycles through braille chars. "thinking..." text in cyan. Spinner stops when response arrives.
- **Effort**: S

### Bead 26: Empty State Renderers
- **Description**: Implement empty state displays per Section 6.12. Root empty state: centered `Panel` with ◇ glyph + "nothing here yet." + "press a to name what matters." Descended empty state: desire header + heavy rule + centered ◇ + "no children yet." + "press a to decompose." + light rule + reality footer. Use structural dynamics language ("decompose" not "add", "name what matters" not "create a task"). Both states use CLR_DIM for invitation text.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: 2, 3, 5, 7
- **Blocks**: 17
- **Acceptance criteria**: Root empty state shows ◇ with invitation. Descended empty state preserves parent header/footer framing. Language uses domain terminology. Empty states are centered.
- **Effort**: S

### Bead 27: Filter State Rendering
- **Description**: When filter is set to "All", resolved and released tensions appear with their terminal glyphs (✦ and ·) in CLR_DIM. All content for terminal tensions (desire text, horizon, temporal) is dim. The filter state is shown in the lever ("filter: active" or "filter: all"). `f` key cycles between Active and All filters. Active filter hides resolved/released entirely. All filter shows them dim at the end of the list.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: 3, 4, 6, 10
- **Blocks**: None
- **Acceptance criteria**: Resolved tensions show ✦ in dim. Released show · in dim. All content dim for terminal states. Filter label in lever. f cycles filter. Active filter hides terminal tensions.
- **Effort**: S

## Polish Beads (Ring 2-3, P2-P3)

### Bead 28: Sparkline for Activity History
- **Description**: In the Analysis view (Depth 2), add a `Sparkline` widget showing mutation activity over the last 6-12 time buckets. Each bucket represents one week. Sparkline uses gradient from CLR_DIM (quiet) to CLR_CYAN (active). Sparkline width: 12 characters. Position: in the dynamics column of the analysis view, below the dynamics table. Data source: count mutations per week from FullGazeData history entries.
- **Ring**: 3
- **Priority**: P2
- **Blocked by**: 9
- **Blocks**: None
- **Acceptance criteria**: Sparkline renders in analysis view. Data correctly aggregated per week. Gradient colors applied. Width constrained to 12 chars.
- **Effort**: S

### Bead 29: MiniBar for Magnitude
- **Description**: In the Analysis view dynamics table, render magnitude as `MiniBar` widget with 8-cell width. Filled char: `█`. Empty char: `░`. Colors: CLR_GREEN for high magnitude (large gap = energy), CLR_DEFAULT for mid, CLR_DIM for low (approaching resolution). Show decimal value after the bar (e.g., `████░░░░ .72`). If oscillating, use CLR_AMBER for filled portion.
- **Ring**: 3
- **Priority**: P2
- **Blocked by**: 9
- **Blocks**: None
- **Acceptance criteria**: MiniBar renders 8 cells. Fill level proportional to magnitude (0.0-1.0). Correct colors by magnitude level. Decimal shown. Amber when oscillating.
- **Effort**: S

### Bead 30: Terminal Capability Detection
- **Description**: Implement terminal capability detection: color profile (TrueColor / ANSI256 / ANSI16) from COLORTERM and TERM environment variables, Unicode glyph support testing. If TrueColor not available, map the six-color palette to closest ANSI256 colors. If Unicode not fully supported, provide ASCII fallback glyphs: `o` for ◇, `*` for ◆, `#` for ◈, `@` for ◉, `+` for ✦, `.` for ·. Store capabilities in a global config accessible during rendering.
- **Ring**: 3
- **Priority**: P3
- **Blocked by**: 3, 4
- **Blocks**: None
- **Acceptance criteria**: Color profile detected correctly. Glyph fallbacks work on limited terminals. No panic on any terminal type. Detection runs once at startup.
- **Effort**: S

### Bead 31: Minimum Terminal Size Handling
- **Description**: When terminal width < 40 or height < 5, show a centered "terminal too small" message instead of the normal UI. When width < 20, show just "werk". This prevents panics from negative rect arithmetic and provides graceful degradation at extreme sizes. Handle terminal resize events to switch between normal and minimal views.
- **Ring**: 3
- **Priority**: P3
- **Blocked by**: 5
- **Blocks**: None
- **Acceptance criteria**: "terminal too small" shown at < 40x5. "werk" shown at < 20 width. No panics at any terminal size. Resize restores normal view when large enough.
- **Effort**: S

### Bead 32: Undo Visual Feedback
- **Description**: When `u` (undo) is pressed and a mutation is undone, show a Toast with the undone action description (e.g., "undid: updated reality"). The Toast uses CLR_CYAN styling. If undo is not possible (no mutations to undo), show amber Toast "nothing to undo". This improves feedback for the undo operation.
- **Ring**: 3
- **Priority**: P3
- **Blocked by**: 24
- **Blocks**: None
- **Acceptance criteria**: Undo shows cyan Toast with action description. No-op undo shows amber Toast. Toast auto-dismisses after 3s.
- **Effort**: S

### Bead 33: Note Annotation Surface
- **Description**: When `n` is pressed, render a `Panel` with `TextArea` (multi-line) for adding a note to the selected tension. Panel title: "note". Cyan rounded border. The note is stored as a mutation with field="note". Enter on empty line submits. Esc cancels. This replaces the current single-line annotation prompt with a proper multi-line editing surface.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: 2, 3, 5
- **Blocks**: None
- **Acceptance criteria**: TextArea allows multi-line input. Panel has correct styling. Submit stores note mutation. Esc cancels without mutation.
- **Effort**: S

### Bead 34: Move/Reparent Surface
- **Description**: When `m` is pressed, render a search overlay (similar to Bead 19) but for selecting a destination parent. The selected tension will be moved to become a child of the search result. Results show: desire text + current children count. Enter confirms move. Esc cancels. Self-moves and moves that would create cycles are prevented.
- **Ring**: 2
- **Priority**: P1
- **Blocked by**: 19
- **Blocks**: None
- **Acceptance criteria**: Search overlay shows potential parents. Can't select self or create cycle. Enter moves tension. Esc cancels. Parent relationship updated correctly.
- **Effort**: M

## Hardening Beads (Ring 2-3, P2-P3)

### Bead 35: Scenario Fixture Suite
- **Description**: Create a set of structural scenario fixtures for testing. Each fixture is a known tension tree with mutations that produces specific dynamics states. Minimum fixtures: (1) Fresh root with germinating tension, (2) Healthy advancing parent with assimilating children, (3) Neglected subtree (3+ weeks no reality update), (4) Oscillating sibling pair, (5) Overdue tension with repeated horizon postponement, (6) Multiple roots with no senior organizing principle, (7) Agent mutation review with 3 mixed proposals, (8) Watch insight backlog with 3 pending insights, (9) Resolved/released tensions visible in "All" filter, (10) Deep hierarchy (3+ levels). Each fixture should be loadable programmatically for rendering tests.
- **Ring**: 2
- **Priority**: P2
- **Blocked by**: None
- **Blocks**: 36
- **Acceptance criteria**: 10 fixtures created. Each produces a deterministic dynamics state. Fixtures are loadable in test code. Fixtures cover all major dynamics: phase, tendency, conflict, neglect, oscillation, resolution, drift, urgency.
- **Effort**: M

### Bead 36: Golden Rendering Tests
- **Description**: For each scenario fixture (Bead 35), render the field view, gaze view, and analysis view to a test buffer at 104 columns width. Compare against golden file snapshots. Tests verify: correct glyphs, correct colors (as style attributes), correct spatial layout (desire above reality), correct responsive behavior at 80 and 60 columns. Golden files are committed to the repo and updated explicitly when design changes.
- **Ring**: 2
- **Priority**: P2
- **Blocked by**: 17, 35
- **Blocks**: None
- **Acceptance criteria**: Golden tests pass for all 10 fixtures × 3 views × 3 widths = 90 test cases. Tests detect regressions in glyph rendering, color assignment, and spatial layout. Golden files are human-readable.
- **Effort**: L

### Bead 37: Responsive Invariant Tests
- **Description**: Write automated tests that verify the responsive invariants at each breakpoint (60, 90, 120, 160 columns). For each breakpoint, verify: (1) desire text appears above reality text (check y-coordinates), (2) phase glyph is present and correct shape, (3) selection indicator is visible, (4) alert bar is visible when alerts exist, (5) lever is present at bottom row. These tests use the scenario fixtures from Bead 35.
- **Ring**: 2
- **Priority**: P2
- **Blocked by**: 17, 18, 35
- **Blocks**: None
- **Acceptance criteria**: Tests pass at all four breakpoint widths. Tests verify all five spatial invariants. Tests use fixture data for realistic scenarios. Any regression in spatial law triggers test failure.
- **Effort**: M

### Bead 38: Performance Baseline
- **Description**: Create benchmarks measuring: (1) Time to render field view with 10/50/100/500 tensions, (2) Time to compute full dynamics for a single tension, (3) Time to open gaze card (data loading + rendering), (4) Memory usage for 100/500/1000 tensions in memory. Establish acceptable baselines: field render < 16ms (60fps), dynamics computation < 50ms, gaze open < 100ms. Store baselines for regression detection.
- **Ring**: 3
- **Priority**: P2
- **Blocked by**: 17
- **Blocks**: None
- **Acceptance criteria**: Benchmarks run and produce timing data. Baselines documented. No render exceeds 16ms for typical tension counts (< 100). Performance regression detection possible.
- **Effort**: M

### Bead 39: Widget Prototype Validation
- **Description**: Build minimal prototype programs testing the ftui widget compositions assumed in this plan: (1) VirtualizedList with variable-height items (simulating gaze expansion), (2) Panel containing TextInput (for input surfaces), (3) Badge in a Flex row (for alert bar), (4) Modal with backdrop (for confirm dialogs), (5) Rule with custom BorderSet (for dotted separator), (6) StatusLine with Spinner (for agent activity), (7) Toast with NotificationQueue (for action outcomes). Document findings: what works as designed, what needs adaptation, what needs ftui changes.
- **Ring**: 1
- **Priority**: P0
- **Blocked by**: None
- **Blocks**: 6, 7, 8, 11, 14, 16, 24, 25
- **Acceptance criteria**: 7 prototypes built and tested. Each documents: works/needs-adaptation/needs-ftui-change. Findings fed back into relevant beads. Any ftui gaps documented for upstream contribution.
- **Effort**: M

## Documentation Beads (Ring 2, P2)

### Bead 40: Implementer's Quick Reference
- **Description**: Create a single-page quick reference card for implementers. Include: glyph table, color table with usage rules, border type assignments, design token constants, widget binding table (domain concept → ftui widget), key binding table, mode table. This is a condensed extract from this plan — the reference an implementer keeps open while coding. Format: markdown table-heavy, fits on 2-3 printed pages.
- **Ring**: 2
- **Priority**: P2
- **Blocked by**: All Ring 1 beads
- **Blocks**: None
- **Acceptance criteria**: All tables from this plan condensed into one reference. Every design token included. Usable without reading the full plan.
- **Effort**: S

---

# PART XII: BEAD ANALYSIS

## Critical Path (longest dependency chain)

```
Bead 39 (Widget Prototypes)
  → Bead 2 (Design Tokens)
    → Bead 3 (Color/Style)
      → Bead 4 (Glyphs)
        → Bead 6 (Tension Line)
          → Bead 7 (Descended View)
            → Bead 8 (Gaze Card)
              → Bead 9 (Analysis View)
                → Bead 17 (View Integration)
                  → Bead 18 (Responsive)
                    → Bead 37 (Responsive Tests)
```

**Critical path length:** 11 beads. Estimated effort: S+S+S+S+M+M+M+M+L+M+M = ~11 bead-units.

## Quick Wins (high impact, few blockers)

| Bead | Title | Impact | Blockers | Effort |
|------|-------|--------|----------|--------|
| 2 | Design Token Module | Foundation for everything | None | S |
| 12 | Rule Widget Integration | Eliminates manual rendering | 2, 3 | S |
| 24 | Toast System | Better feedback | 2, 3 | S |
| 25 | Spinner for Agent | Visual polish | 10 | S |
| 26 | Empty State Renderers | First impression | 2,3,5,7 | S |
| 27 | Filter State Rendering | Resolved/released visibility | 3,4,6,10 | S |
| 30 | Terminal Capability Detection | Robustness | 3, 4 | S |
| 31 | Minimum Terminal Size | Safety | 5 | S |

## Foundational Beads (most downstream dependents)

| Bead | Title | Dependents | Ring |
|------|-------|------------|------|
| 2 | Design Token Module | ALL other beads | 1 |
| 3 | Color/Style | Nearly all beads | 1 |
| 5 | Screen Layout Skeleton | All view beads | 1 |
| 6 | Tension Line Renderer | 7, 8, 10, 14, 17 | 1 |
| 39 | Widget Prototype Validation | 6, 7, 8, 11, 14, 16, 24, 25 | 1 |

---

# PART XIII: IMPLEMENTATION PHASES

## Phase 1: Foundation (Beads 1-5, 39)

**What's true when this phase ends:**
- All design tokens exist as code constants
- All colors and styles are pre-computed
- All glyphs have functions mapping domain types to visual symbols
- Screen layout skeleton splits terminal into content + alert bar + lever
- Widget prototypes validate all critical ftui compositions
- The spatial law is codified and testable

**Parallelism:** Beads 1, 2, 39 can start simultaneously. 3 depends on 2. 4 depends on 2+3. 5 depends on 2.

## Phase 2: Core Components (Beads 6-10, 12)

**What's true when this phase ends:**
- Tension line renders via Flex constraints (no char math)
- Descended view has proper header/trunk/footer structure
- Gaze card expands inline with cyan rounded border
- Analysis view shows dynamics table + history
- Lever shows breadcrumbs and mode indicators
- All rules use ftui Rule widget

**Parallelism:** Beads 6, 10, 12 can start once Phase 1 is done. 7 depends on 6. 8 depends on 6. 9 depends on 8.

## Phase 3: Integration (Bead 17, plus 11, 13, 26, 27)

**What's true when this phase ends:**
- Complete field view renders from integrated components
- Alert bar shows persistent structural signals
- Badge system provides compressed signal vocabulary
- Empty states handle gracefully
- Filter state shows resolved/released correctly
- The main daily-use view is complete

**Parallelism:** 11, 13, 26, 27 can work alongside 17 integration.

## Phase 4: Interaction Surfaces (Beads 14-16, 19, 20, 23, 33, 34)

**What's true when this phase ends:**
- All input modes have proper widget-based surfaces
- Add, edit, annotate, confirm, search, help, reorder, move all work
- Modals use ftui Modal with backdrop
- Search uses Panel + TextInput + List

**Parallelism:** All input surface beads can be done in parallel once Phase 3 is complete.

## Phase 5: Polish (Beads 18, 21, 22, 24, 25, 28-32)

**What's true when this phase ends:**
- Responsive behavior verified at all breakpoints
- Agent review and insight review surfaces complete
- Toast notifications for action outcomes
- Spinner for agent activity
- Sparkline and MiniBar visualizations in analysis view
- Terminal capability detection and minimum size handling
- Undo visual feedback

**Parallelism:** Most polish beads are independent.

## Phase 6: Hardening (Beads 35-40)

**What's true when this phase ends:**
- 10 scenario fixtures exist
- Golden rendering tests verify 90+ test cases
- Responsive invariant tests verify spatial law at all breakpoints
- Performance baselines established
- Implementer's quick reference documented
- The design system is tested, documented, and ready for confident iteration

---

# PART XIV: OPEN QUESTIONS

## Requires User Input

1. **Product stance: daily steering vs periodic review?** The plan assumes daily steering as primary use case (Codex flagged this). If periodic review is equally important, the split-pane layout (Ring 3) should be promoted to Ring 2.

2. **Watch/agent: core loop or episodic?** The plan treats watch insights and agent sessions as episodic (Ring 3). If they should be part of the primary loop, they need permanent screen real estate — possibly a third column or persistent badge in the lever.

3. **Two-line vs one-line tension stripe?** Codex proposed a two-line stripe (desired above actual per row). This plan uses one-line (desire text only, with reality in gaze/footer). The two-line approach makes reality visible at Field depth but doubles the vertical space per tension. [UNCERTAIN: prototype both and let operator preference decide.]

4. **CommandPalette scope?** K-operative wants CommandPalette as the primary command surface. This plan keeps it as search-only (Ring 2) with full command integration as Ring 3. If command unification is high priority, Bead 19 should be expanded.

5. **Resolved/Released default visibility?** Current implementation defaults to Active filter. Should "All" be the default so terminal tensions are always visible? This affects the daily experience significantly.

## Requires Prototyping

6. **VirtualizedList variable-height rows?** Bead 39 must validate this. If VirtualizedList can't handle gaze expansion gracefully, the manual vlist approach must be preserved and wrapped.

7. **Panel::borders(LEFT) for trunk?** Prior doc's innovation — must be validated with actual ftui Panel. If Panel doesn't support partial borders well, fall back to manual trunk rendering.

8. **Flex::horizontal() with FitContent for horizon Badge?** The tension line layout depends on FitContent constraint working correctly. Bead 39 validates this.

9. **Custom BorderSet for dotted Rule?** If ftui's Custom(BorderSet) doesn't produce a clean dotted line, fall back to `Paragraph` with `"· · · ..."` pattern.

10. **Analysis two-column layout?** The dynamics + history side-by-side may be too cramped at Md width (104 cols). May need to stack vertically at Md and use columns only at Lg+.

## Requires ftui Changes

11. **Badge in StatusLine right items?** If StatusLine doesn't support Badge as right items, insight count and filter may need to be plain text.

12. **FocusManager for future split-pane?** Multi-pane navigation needs FocusManager to route focus correctly. Not needed for single-column MVP but foundational for Ring 3 split-pane.

13. **Sparkline gradient colors?** If Sparkline doesn't support multi-color gradients (dim→cyan), single-color rendering is acceptable.

---

# PART XV: MIGRATION STRATEGY

The current codebase has a working (if manually-rendered) TUI. Migration must be incremental, not big-bang.

## Migration Phases (aligned with Implementation Phases)

### Migration 1: Tokens & Styles (with Phase 1)
- Add `tokens.rs` alongside existing code
- Refactor `theme.rs` to use new style tokens
- Refactor `glyphs.rs` to add new helper functions
- **Zero behavior change** — existing rendering uses new tokens

### Migration 2: Layout Primitives (with Phase 2)
- Replace manual content area calculation with `screen_layout()`
- Replace manual rule rendering with `Rule` widgets
- Replace manual tension line char math with `Flex` layout
- **Incremental** — replace one element type at a time in render.rs

### Migration 3: Structural Components (with Phase 3)
- Replace manual descended view with composed widgets
- Replace manual gaze card with Panel composition
- Add alert bar between content and lever
- **Section by section** — each FieldElement variant replaced independently

### Migration 4: Input Surfaces (with Phase 4)
- Replace manual cursor rendering with TextInput
- Replace manual confirm prompts with Modal
- Replace manual search with Panel + TextInput + List
- **One mode at a time** — each InputMode variant migrated independently

### Migration 5: Polish & Clean (with Phase 5-6)
- Remove all manual char-width arithmetic
- Remove all `"━".repeat(width)` patterns
- Remove FieldElement enum variants that are replaced by widgets
- Remove VirtualList if VirtualizedList works
- **Final cleanup** — remove old code only after new code is validated

---

# APPENDIX A: COMPLETE DYNAMICS RENDERING REFERENCE

For each of the 13 structural dynamics, the complete rendering specification across all depths.

## A.1 Phase (CreativeCyclePhase)

- **Source:** `ComputedDynamics.phase.phase`
- **Field:** Glyph shape (◇◆◈◉) — always visible, first element on tension line
- **Gaze:** Same glyph in card heading. No additional phase treatment.
- **Analysis:** `phase` label + phase name (e.g., "assimilation") in dynamics table
- **Color:** NOT determined by phase. Phase determines shape only.
- **Widget:** `Span` for glyph, text for label

## A.2 Tendency (StructuralTendency)

- **Source:** `ComputedDynamics.tendency.tendency`
- **Field:** Glyph COLOR (cyan=Advancing, default=Stagnant, amber=Oscillating)
- **Gaze:** Same glyph color + optional tendency Badge (ADV/STAG/OSC) in badge cluster
- **Analysis:** `tendency` label + tendency name in dynamics table, colored by tendency
- **Widget:** `Span` color, `Badge`

## A.3 Structural Tension Magnitude

- **Source:** `ComputedDynamics.tension.magnitude`
- **Field:** Not shown (too dense for one-line scan)
- **Gaze:** Not shown (children and reality provide qualitative sense)
- **Analysis:** `magnitude` label + MiniBar(8) + decimal (e.g., `████░░░░ .72`)
- **Color:** CLR_GREEN (high=energy), CLR_DEFAULT (mid), CLR_DIM (low=approaching resolution)
- **Widget:** `MiniBar`

## A.4 Structural Conflict

- **Source:** `ComputedDynamics.conflict`
- **Field:** Contributes to alert bar ("structural conflict detected")
- **Gaze:** Red CONFLICT Badge in badge cluster if present
- **Analysis:** `conflict` label + pattern name (red) or `—` (dim)
- **Color:** CLR_RED always
- **Widget:** `Badge` (red), alert bar `Badge`

## A.5 Neglect

- **Source:** `ComputedDynamics.neglect`
- **Field:** Contributes to alert bar ("N tensions neglected")
- **Gaze:** Amber neglect Badge if present
- **Analysis:** `neglect` label + type name (amber) or `—` (dim)
- **Color:** CLR_AMBER
- **Widget:** `Badge` (amber), alert bar `Badge`

## A.6 Oscillation

- **Source:** `ComputedDynamics.oscillation`
- **Field:** Glyph color = amber (via tendency = Oscillating)
- **Gaze:** OSC Badge (amber) if present
- **Analysis:** `oscillation` label + reversal count + magnitude (amber) or `—` (dim)
- **Color:** CLR_AMBER
- **Widget:** `Span` (glyph color), `Badge`

## A.7 Resolution

- **Source:** `ComputedDynamics.resolution`
- **Field:** Not shown explicitly (tendency = Advancing covers it)
- **Gaze:** Green resolution Badge if velocity sufficient
- **Analysis:** `resolution` label + trend name + velocity + sufficient/insufficient indicator
- **Color:** CLR_GREEN if sufficient, CLR_AMBER if insufficient
- **Widget:** `Badge`, label

## A.8 Horizon Drift

- **Source:** `ComputedDynamics.horizon_drift`
- **Field:** Temporal indicator encodes drift implicitly (urgency coloring)
- **Gaze:** Horizon badge shows label; drift badge if non-stable
- **Analysis:** `drift` label + type name (STABLE/POSTP/LOOSE/R-POST/OSC)
- **Color:** CLR_AMBER if non-stable, CLR_DIM if stable
- **Widget:** `Badge`, label

## A.9 Urgency

- **Source:** `ComputedDynamics.urgency`
- **Field:** Temporal indicator color gradient (cyan → amber → red)
- **Gaze:** Same temporal dots + urgency badge if high
- **Analysis:** Temporal indicator + urgency value
- **Color:** Gradient based on urgency value: <0.5 CLR_CYAN, 0.5-0.8 CLR_AMBER, >0.8 CLR_RED
- **Widget:** `Span` (temporal dots), `Badge`

## A.10 Orientation

- **Source:** `ComputedDynamics.orientation`
- **Field:** Not shown (secondary signal)
- **Gaze:** Not shown (secondary signal)
- **Analysis:** `orientation` label + name (Creative/ProblemSolving/ReactiveResponsive)
- **Color:** CLR_DIM (informational only)
- **Widget:** label

## A.11 Compensating Strategy

- **Source:** `ComputedDynamics.compensating_strategy`
- **Field:** Not shown (rare, analytical)
- **Gaze:** Amber Badge if detected (e.g., "COMP")
- **Analysis:** Named in dynamics table or `—`
- **Color:** CLR_AMBER if present
- **Widget:** `Badge` (rare), label

## A.12 Assimilation Depth

- **Source:** `ComputedDynamics.assimilation`
- **Field:** Not shown (secondary signal)
- **Gaze:** Not shown (secondary signal)
- **Analysis:** `assimilation` label + depth name (Shallow/Deep/None)
- **Color:** CLR_DIM (informational only)
- **Widget:** label

## A.13 Mutation History

- **Source:** Loaded separately from mutation store
- **Field:** Not shown (scan mode — no history at this density)
- **Gaze:** Last event line (e.g., "2h ago: updated reality")
- **Analysis:** Reverse chronological list, max 12 entries, relative time + description
- **Color:** CLR_DIM for time, CLR_DEFAULT for description
- **Widget:** `Paragraph`, future: `HistoryPanel`

---

# APPENDIX B: ftui WIDGET CAPABILITIES CONFIRMED

Based on source exploration of ftui 0.2.1:

| Widget | Available | Notes |
|--------|-----------|-------|
| `Panel` | YES | Supports Borders (subset), BorderType, border_style, title, padding |
| `Rule` | YES | Supports BorderType, title, title_alignment |
| `Badge` | YES | Supports with_style, with_padding |
| `Paragraph` | YES | Supports Text, Lines, Spans, alignment, scroll |
| `StatusLine` | YES | Supports left/right items, separator, Spinner |
| `TextInput` | YES | Handles cursor, selection, grapheme editing |
| `TextArea` | YES | Multi-line text input |
| `Modal` | YES | Supports ModalPosition, ModalSizeConstraints, BackdropConfig |
| `Toast` | YES | Supports ToastPosition, timeout, animation |
| `NotificationQueue` | YES | Manages multiple toasts |
| `Sparkline` | YES | Takes &[f64], gradient, bounds |
| `MiniBar` | YES | Compact bar visualization |
| `Flex` | YES | Horizontal/vertical, constraints, alignment |
| `Grid` | YES | 2D layout with spanning |
| `Columns` | YES | Multi-column layout |
| `Responsive` | YES | Conditional rendering by breakpoint |
| `Visibility` | YES | Show/hide by breakpoint |
| `Breakpoint` | YES | Xs/Sm/Md/Lg/Xl with configurable thresholds |
| `VirtualizedList` | YES | Height prediction, virtualized rendering |
| `List` | YES | Selectable list items |
| `Table` | YES | Rows, columns, selection |
| `Tree` | YES | Hierarchical data with guides |
| `CommandPalette` | YES | Command search and dispatch |
| `HistoryPanel` | YES | History display |
| `Spinner` | YES | Multiple animation styles |
| `FocusManager` | YES | Keyboard focus navigation |
| `HelpRegistry` | YES | Keybinding help system |
| `ProgressBar` | YES | With animation |
| `Constraint::FitContent` | YES | Content-aware sizing |
| `Constraint::Fill` | YES | Fill remaining space |
| `Custom(BorderSet)` | YES | Custom border characters |
| `Borders::LEFT` | YES | Partial border support |
| `BackdropConfig` | YES | Modal backdrop with opacity |

**Key capability confirms:**
- Panel supports partial borders (Borders::LEFT) — trunk-as-border is valid
- Flex supports FitContent constraint — tension line layout is valid
- VirtualizedList exists with height prediction — may support variable heights
- Modal has backdrop — confirm dialog design is valid
- Custom(BorderSet) — dotted rule is valid

**Needs validation (Bead 39):**
- VirtualizedList with dynamically changing item heights (gaze expansion)
- Panel containing TextInput (composition)
- Badge rendering within StatusLine right items

---

*This plan is V1. Decisions marked [UNCERTAIN] need prototyping or user input. The plan WILL iterate. But it is comprehensive enough that a fresh conversation reviewing it can propose meaningful improvements rather than asking "what about X?" for things never addressed.*

*Quality bar met: The plan specifies every domain concept's rendering, every widget binding, every glyph, every color, every responsive breakpoint, every interaction mode, and provides full-screen ASCII mockups demonstrating the integrated system. Any bead can be picked up and executed without consulting external documents.*
