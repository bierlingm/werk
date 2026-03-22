# Convergence & Divergence Analysis: Plan A vs Plan B

**Date:** 2026-03-19
**Subject:** Two competing TUI design system plans for `werk-tui`
**Plan A:** `tui-design-system-plan-a.md` — 80 beads, 27 sections, five phases. Codex-generated.
**Plan B:** `tui-design-system-plan-b.md` — 40 beads, 15 parts + appendices. Multi-model synthesis.

---

# PART 1: AGREEMENT MAP

The two plans agree on a remarkable amount. This section documents convergence briefly and moves on.

## 1.1 Product Stance

**Agreement: Strong.**
Both plans lock the same position: `werk-tui` is a daily field instrument with on-demand depth, not a dashboard, tree explorer, or analytics cockpit.

- Plan A §3: "werk-tui is a daily field instrument with on-demand depth."
- Plan B §3 (Best-of-All-Worlds): "Mirror not dashboard" philosophy adopted from Gemini.

Both explicitly reject: dashboard-first, tree-as-primary, maximum-information-density opening screen.

## 1.2 Spatial Law

**Agreement: Strong.**
Both lock desire-above-actual as absolute and non-negotiable across all widths.

- Plan A ADR-01: "The vertical axis always encodes actual/reality below desired/desire. This invariant is absolute."
- Plan B ADR-1: "Reality is ground (bottom/left), desire is sky (top/right). This is ABSOLUTE."

Both agree on collapsible horizontal semantics (left=basis/history, right=intention/projection). Plan B adds explicit depth and time axes (see Divergences).

## 1.3 Depth Model

**Agreement: Strong.**
Three additive layers with identical names:

| | Plan A (ADR-02) | Plan B (ADR-2) |
|---|---|---|
| Depth 0 | Field | Field |
| Depth 1 | Gaze | Gaze |
| Depth 2 | Analysis | Analysis |

Both agree:
- Gaze expands inline beneath the selected row, never replaces it.
- Analysis may replace the field on narrow widths.
- Disclosure is additive in semantics.
- K-operative's four-layer model is acknowledged but treated as orthogonal purpose taxonomy, not a fourth depth.

## 1.4 Phase Glyphs (Core Four)

**Agreement: Strong.**
Identical across both plans:

| Phase | Glyph |
|---|---|
| Germination | ◇ |
| Assimilation | ◆ |
| Completion | ◈ |
| Momentum | ◉ |

## 1.5 Color Semantics (Six-Color Palette)

**Agreement: Strong.**
Both lock the same six foreground semantic roles:

| Role | Plan A (ADR-04) | Plan B (ADR-4) |
|---|---|---|
| Default | primary content | active content |
| Dim | chrome, labels, resolved | structure/chrome |
| Cyan | selection, focus, advisory | agency/selection |
| Amber | caution, neglect, stagnation | attention/warning |
| Red | conflict, breach | conflict ONLY |
| Green | healthy motion, progress | advancement |

Plan B adds concrete hex values (#DCDCDC, #646464, #50BED2, #C8AA3C, #DC5A5A, #50BE78) and two background colors (CLR_BG, CLR_SELECTED_BG). Plan A references existing `theme.rs` constants.

Both agree: no new hue without an ADR. Both agree red is reserved for conflict.

## 1.6 Widget Binding Contract

**Agreement: Strong.**
Both lock the ftui-first principle:

- Plan A ADR-05: "Every rendered element must map either directly to an ftui widget type or to a named werk composite widget built only from ftui."
- Plan B Constraint 5: "Every visual element maps to a named ftui widget type. No raw frame.buffer painting."

Both agree: current ad hoc rendering is debt, not precedent. Both propose a component layer for named composites.

## 1.7 Interaction Model

**Agreement: Strong.**
Both lock vim-first navigation with identical core keybindings:

| Key | Both Plans |
|---|---|
| j/k | Move selection |
| l/Enter | Descend/activate |
| h/Backspace | Ascend |
| Space | Toggle gaze |
| Tab | Toggle/cycle analysis |
| Esc | Dismiss/return |
| / | Search |
| : (A) or ? (B) | Command palette / Help |
| a, e, n, r, x, m | Create, edit, note, resolve, release, move |

Minor binding differences exist (see Divergences) but the architecture is identical.

## 1.8 Migration Strategy

**Agreement: Strong.**
Both explicitly require incremental migration, not big-bang rewrite.

- Plan A §19: "The migration path must be incremental. A big-bang rewrite would reintroduce hidden assumptions."
- Plan B Part XV: "Migration must be incremental, not big-bang."

Both agree on what changes first (tokens, component contracts, composites) and what changes later (vlist replacement, full analysis redesign).

## 1.9 Structural Alerts as Persistent

**Agreement: Strong.**
Both agree structural alerts (conflict, neglect) must persist until structurally cleared and never rely solely on toasts.

- Plan A ADR-07: "Structural alerts persist in view until structurally cleared; they are never toast-only."
- Plan B ADR-7: "Alerts are persistent (not scrollable)."

## 1.10 Watch/Agent as Advisory, Not Auto-Apply

**Agreement: Strong.**
Both agree watch insights are advisory-only, agent proposals require review before application.

- Plan A Constraint 17-18: "Watch insights are advisory-only and cannot auto-apply mutations. Agent proposals are always reviewable."
- Plan B §6.10-6.11: Review surfaces with explicit accept/dismiss.

---

# PART 2: DIVERGENCE MAP

## 2.1 Glyph System: Tendency and Terminal States

| Decision | Plan A Position | Plan B Position | Severity | Recommendation |
|----------|----------------|-----------------|----------|----------------|
| **Tendency representation** | Separate inline glyphs: `→` advancing, `↔` oscillating, `·` stagnant (ADR-03, §12.4) | Tendency encoded as glyph COLOR on the phase glyph: cyan=advancing, default=stagnant, amber=oscillating (ADR-3) | **Structural** | See §4.1 |
| **Resolved glyph** | No dedicated glyph. Status expressed through "style and explicit badges, not alternate lifecycle glyphs" (ADR-03, Constraint 12) | Dedicated glyph: `✦` (U+2726) for resolved (ADR-3) | **Structural** | See §4.2 |
| **Released glyph** | `·` used as stagnant/filler, not a terminal-state glyph. Released uses dim styling + badge. (ADR-03) | `·` (U+00B7) as explicit released glyph (ADR-3) | **Structural** | See §4.2 |
| **Glyph count** | 4 phase + 3 tendency + 2 trail = 9 glyphs total | 6 phase/status + 4 temporal dots = 10 glyphs total | Cosmetic | N/A |

**Analysis:** This is the single most consequential visual divergence. Plan A keeps phase and tendency as separate visual channels (glyph shape + separate tendency token). Plan B merges tendency INTO the phase glyph via color, saving horizontal space but coupling two distinct signals into one visual element.

## 2.2 Temporal Dots

| Decision | Plan A (§12.5) | Plan B (Part VII) | Severity | Recommendation |
|----------|---------------|-------------------|----------|----------------|
| **Dot vocabulary** | Binary: `●` (mutated) / `○` (no mutation) | Quaternary: `◌` (empty) / `◦` (now) / `●` (horizon) / `◎` (stale) | **Structural** | See §4.3 |
| **Dot semantics** | Activity-only: did mutation happen in this bucket? | Mixed: activity + urgency + staleness encoded in dot shape | Structural | See §4.3 |
| **Urgency encoding** | Not in dots; urgency is a separate signal | Temporal indicator COLOR encodes urgency (cyan→amber→red) | Structural | See §4.3 |

## 2.3 Breakpoints

| Decision | Plan A (ADR-06, §12.1) | Plan B (ADR-6) | Severity | Recommendation |
|----------|------------------------|-----------------|----------|----------------|
| **Number of tiers** | 3 app-level: Compact (80-119), Standard (120-159), Expanded (160+) | 5 tiers matching ftui: Xs (<60), Sm (60-89), Md (90-119), Lg (120-159), Xl (160+) | **Architectural** | See §4.4 |
| **Sub-80 handling** | "<80 is emergency fallback only and not a stop-ship design target" | Xs (<60) and Sm (60-89) are explicit design targets with specified behavior | Architectural | See §4.4 |
| **ftui default override** | "use ftui_layout::Breakpoints::new_with_xl(80, 120, 160, 200) — do not inherit 60/90/120/160 blindly" | Uses ftui's native 60/90/120/160 breakpoints directly | Architectural | See §4.4 |

## 2.4 Content Width Cap

| Decision | Plan A | Plan B (Constraint 13, Part VII) | Severity | Recommendation |
|----------|--------|----------------------------------|----------|----------------|
| **Max content width** | No cap specified. Content fills available width within responsive tier. | 104 characters max, centered. `MAX_CONTENT_WIDTH: u16 = 104` | **Structural** | See §4.5 |

## 2.5 Alert Bar Architecture

| Decision | Plan A (ADR-07) | Plan B (ADR-7, §6.6) | Severity | Recommendation |
|----------|-----------------|----------------------|----------|----------------|
| **Alert bar** | No dedicated alert bar. Alerts distributed: inline on rows (local), lever (aggregate counts), review cards (action). | Dedicated persistent 1-row alert bar between content and lever. Numbered Badge widgets. | **Structural** | See §4.6 |
| **Alert hotkeys** | "Number-key shortcuts may target visible actionable alerts in context" (ADR-07) — tentative | "Press 1-9 to act on" alerts in alert bar — concrete | Cosmetic | See §4.6 |
| **Alert aggregation** | Aggregate counts in lever | "3 tensions neglected" not individual badges — explicit aggregation rule | Cosmetic | N/A |

## 2.6 Selection Indicator

| Decision | Plan A | Plan B (§6.1) | Severity | Recommendation |
|----------|--------|----------------|----------|----------------|
| **Selection mechanism** | Not specified at widget level. "Selection state" is a visible element but implementation left to components. | `Panel::borders(Borders::LEFT)` + `BorderType::Heavy` + cyan border + `CLR_SELECTED_BG` background | Cosmetic | See §4.7 |

## 2.7 Tension Line Layout

| Decision | Plan A (§14.3) | Plan B (§6.1) | Severity | Recommendation |
|----------|----------------|----------------|----------|----------------|
| **Layout mechanism** | Composite over Paragraph + optional Badge. Width budget described conceptually. | `Flex::horizontal()` with 4 explicit constraints: Glyph(Fixed 4) + Desire(Fill) + Horizon(FitContent) + Temporal(Fixed 8) | **Structural** | See §4.8 |
| **Tendency slot** | "Optional tendency token" in stripe — separate visual element | No tendency token; tendency encoded in glyph color | Structural | Coupled to §4.1 |

## 2.8 Trunk Rendering

| Decision | Plan A | Plan B (§6.4, Constraint 6) | Severity | Recommendation |
|----------|--------|------------------------------|----------|----------------|
| **Trunk lines** | Not specified at implementation level | `Panel::borders(Borders::LEFT)` wrapping positioned children — "No FieldElement::TrunkSegment" | Cosmetic | See §4.9 |

## 2.9 Analysis Layout

| Decision | Plan A (§14.5) | Plan B (§6.3) | Severity | Recommendation |
|----------|----------------|----------------|----------|----------------|
| **Analysis internal layout** | Sections listed abstractly: header, value planes, signal summary, analysis rows, history, children/siblings, review | Explicit two-column: dynamics table (left) + history (right). Heavy rule separating operational from analytical. Concrete data contracts. | **Structural** | See §4.10 |

## 2.10 Design Tokens

| Decision | Plan A (§12) | Plan B (Part VII) | Severity | Recommendation |
|----------|-------------|-------------------|----------|----------------|
| **Token specificity** | Semantic names (`fg_default`, `fg_dim`, etc.) and conceptual tables. No Rust constants. | Concrete Rust `const` declarations with exact values, widths, timing, and doc comments. ~50 constants. | **Structural** | See §4.11 |

## 2.11 Empty States

| Decision | Plan A (§13.25) | Plan B (§6.12) | Severity | Recommendation |
|----------|-----------------|----------------|----------|----------------|
| **Empty state copy** | "sparse instructional state with one obvious first act" — concept without specific text | Root: "nothing here yet. press a to name what matters." Descended: "no children yet. press a to decompose." — exact copy with domain language | Cosmetic | See §4.12 |

## 2.12 Anti-Patterns

| Decision | Plan A | Plan B (Part IX) | Severity | Recommendation |
|----------|--------|------------------|----------|----------------|
| **Anti-patterns list** | No equivalent section | 11 explicit anti-patterns (no char math, no manual trunk, no manual cursor, no manual rule repetition, no raw Frame painting, etc.) | Cosmetic | See §4.13 |

## 2.13 Bead Architecture (View-Model-First vs Widget-First)

| Decision | Plan A | Plan B | Severity | Recommendation |
|----------|--------|--------|----------|----------------|
| **Ring 1 focus** | View-model-first: typed data models (field row VM, gaze VM, analysis VM, review queue VM) then components, then integration | Widget-first: concrete widget renderers (tension line, descended view, gaze card, analysis view) implemented early with data contracts inline | **Architectural** | See §4.14 |
| **Total beads** | 80 beads across 5 phases | 40 beads across 6 phases | Architectural | See §5 |

## 2.14 ASCII Mockups

| Decision | Plan A (§16) | Plan B (Part X) | Severity | Recommendation |
|----------|-------------|-----------------|----------|----------------|
| **Mockup completeness** | 3 mockups: standard field (120 col), expanded field+analysis (160+), watch insight review | 2 full mockups: descended view at Md (104 col), gaze card expansion. Plus inline diagrams for every component. | Cosmetic | N/A |

## 2.15 Gaze Card Border Style

| Decision | Plan A (§14.4) | Plan B (§6.2) | Severity | Recommendation |
|----------|----------------|----------------|----------|----------------|
| **Gaze card border** | "Panel only if border weight remains quiet — otherwise Paragraph + Rule composition" (cautious) | `Panel` with `BorderType::Rounded` and `CLR_CYAN` border — definitive | Cosmetic | See §4.15 |

## 2.16 Four Axes vs Three

| Decision | Plan A (ADR-01) | Plan B (ADR-1) | Severity | Recommendation |
|----------|-----------------|-----------------|----------|----------------|
| **Axis model** | Two primary axes: vertical (absolute) + horizontal (canonical but collapsible). Depth and time implied but not formalized as axes. | Four explicit axes: vertical, horizontal, depth, time. Each with directional semantics. | Cosmetic | See §4.16 |

## 2.17 Minimum Terminal Size

| Decision | Plan A | Plan B (Constraint 14) | Severity | Recommendation |
|----------|--------|------------------------|----------|----------------|
| **Minimum size** | Not specified | 40 columns, 5 rows. Below 20 columns: show "werk" only. | Cosmetic | N/A (Plan B's specificity is simply additive) |

## 2.18 Stability Ring Placement of Responsive Doctrine and Alert Architecture

| Decision | Plan A (§11) | Plan B (Part V) | Severity | Recommendation |
|----------|-------------|-----------------|----------|----------------|
| **Responsive doctrine ring** | Ring 1 (Sacred Core) | Ring 2 (Reusable Components) | Cosmetic | See §4.17 |
| **Alert architecture ring** | Ring 1 (Sacred Core) | Ring 2 (Reusable Components) | Cosmetic | See §4.17 |

---

# PART 3: RISK ASSESSMENT

## 3.1 Tendency Representation (§2.1)

**Which survives contact with the codebase?** Plan A's separate tendency token is safer. The existing codebase already uses `→ ↔ ○` as separate glyphs. Plan B's color-encoding requires touching every glyph rendering path to inject tendency-aware coloring.

**Downstream rework if wrong?** Plan B's color coupling is harder to undo — if color becomes overloaded (tendency + urgency + selection all compete for glyph color), untangling requires reworking the entire glyph rendering pipeline. Plan A's separate token is trivially removable.

**Easier to prototype?** Plan A — it's already implemented in the current codebase.

## 3.2 Resolved/Released Glyphs (§2.1)

**Which survives?** Plan B's `✦` and `·` as dedicated terminal-state glyphs are more scannable. However, Plan A's concern about "semantic overload" on the phase glyph position is valid — if the leftmost glyph means both "what phase" and "what status," users must learn a larger symbol set.

**Downstream rework if wrong?** Low either way — it's a glyph table change.

**Easier to prototype?** Both trivial. But Plan B's approach means the "All" filter view is immediately legible without badges.

## 3.3 Temporal Dots (§2.2)

**Which survives?** Plan B's quaternary system (◌◦●◎) encodes more information but is harder to scan. Plan A's binary (●○) is simpler to read at field speed.

**Downstream rework?** Medium — temporal rendering touches every field row.

**Easier to prototype?** Plan A's binary system.

## 3.4 Breakpoints (§2.3)

**Which survives?** Plan B's 5-tier system aligns with ftui's native breakpoints, meaning less custom code. But Plan A's 3-tier system is more pragmatic — terminals below 80 columns are genuinely edge cases for this product.

**Downstream rework?** High for breakpoints — every responsive behavior references these tiers.

**Easier to prototype?** Plan B — uses ftui defaults directly.

## 3.5 Content Width Cap (§2.4)

**Which survives?** Plan B's 104-character cap prevents text from becoming unreadable on ultra-wide terminals. Without a cap, 200-column terminals produce uncomfortably long scan lines.

**Downstream rework?** Low — adding a cap later is easy. Removing one is also easy.

**Easier to prototype?** Plan B — one constraint line.

## 3.6 Alert Bar (§2.5)

**Which survives?** Plan B's dedicated alert bar is more concrete and testable. Plan A's distributed approach is conceptually cleaner but leaves implementation ambiguous — "inline with the relevant tension" requires each component to know about alerts.

**Downstream rework?** Medium — changing alert architecture after components are built requires touching many surfaces.

**Easier to prototype?** Plan B — one widget row.

## 3.7 Tension Line Layout (§2.7)

**Which survives?** Plan B's explicit `Flex::horizontal()` with 4 constraints eliminates all manual character-width arithmetic. This is the anti-pattern Plan B explicitly forbids.

**Downstream rework?** High — the tension line is the atomic rendering unit.

**Easier to prototype?** Plan B — the Flex constraint model is directly testable.

## 3.8 View-Model-First vs Widget-First (§2.13)

**Which survives?** Both are viable strategies. Plan A's view-model-first approach produces cleaner architecture but delays visible progress. Plan B's widget-first approach produces working screens faster but may require refactoring if data contracts shift.

**Downstream rework?** Plan A: rework if view models don't match actual rendering needs. Plan B: rework if data contracts are wrong and widgets need replumbing.

**Easier to prototype?** Plan B — you see results immediately.

---

# PART 4: UNIFIED RECOMMENDATION (Decision Table)

| # | Decision | Winner | Rationale |
|---|----------|--------|-----------|
| 4.1 | **Tendency representation** | **Plan A** (separate tokens) | Keeps phase glyph as a pure phase indicator. Color is already overloaded with selection, urgency, and semantic role. Separate `→ ↔ ·` tokens are already in the codebase and proven. Plan B's color-coupling is elegant but fragile under the existing six-color constraint. |
| 4.2 | **Resolved/Released glyphs** | **Plan B** (`✦` resolved, `·` released) | Terminal states deserve first-class visual identity at scan speed. Users filtering to "All" need instant recognition without reading badges. The glyph vocabulary expands from 4→6, which is manageable. Plan A's concern about overload is mitigated by terminal states being dim-styled and rare in the active filter. |
| 4.3 | **Temporal dots** | **Plan A** (binary `● ○`) | Simpler. The field is a scan surface — binary "something happened / nothing happened" is all you need at 1-second glance speed. Plan B's quaternary system (◌◦●◎) encodes urgency and staleness into dots, but urgency is already carried by color in Plan B's own system, creating redundancy. Urgency encoding should use color applied to binary dots, not dot shape. |
| 4.4 | **Breakpoints** | **Hybrid: Plan B's tiers, Plan A's minimum** | Use ftui's native 5-tier system (Xs/Sm/Md/Lg/Xl at 60/90/120/160) to avoid fighting the framework. But adopt Plan A's stance that <80 is emergency fallback, not a design target. Design effort focuses on Md (90-119) as the primary target and Lg (120-159) as comfortable. Xs/Sm get graceful degradation, not bespoke layouts. |
| 4.5 | **Content width cap** | **Plan B** (104 characters centered) | Prevents scan-line fatigue on wide terminals. 104 is a sensible design target matching typical prose line lengths. Easy to relax later if needed. |
| 4.6 | **Alert bar** | **Plan B** (dedicated persistent bar between content and lever) | Concrete, testable, visible. The numbered-badge action model is elegant. Plan A's distributed approach leaves too much implementation ambiguity. The bar takes 0-1 rows — zero cost when no alerts exist. |
| 4.7 | **Selection indicator** | **Plan B** (`Panel::borders(LEFT)` + Heavy cyan) | Specific enough to implement without interpretation. Eliminates manual selection rendering. Directly uses ftui primitives. |
| 4.8 | **Tension line layout** | **Plan B** (`Flex::horizontal()` with 4 constraints) | The most consequential implementation decision. Eliminates all character-width arithmetic — the single largest source of rendering bugs. The 4-constraint model (glyph/desire/horizon/temporal) is clean and testable. **Modification:** Add a 5th constraint for the tendency token per §4.1: Glyph(Fixed 4) + Desire(Fill) + Tendency(Fixed 2) + Horizon(FitContent) + Temporal(Fixed 8). |
| 4.9 | **Trunk rendering** | **Plan B** (`Panel::borders(LEFT)`) | Innovative and proven by ftui's confirmed support for partial borders. Eliminates manual trunk segment management. |
| 4.10 | **Analysis layout** | **Plan B** (two-column dynamics+history) | Concrete and implementable. The heavy-rule separation between operational and analytical content is a strong spatial metaphor. Plan A's abstract section list needs further specification before coding. **Risk:** two-column may be cramped at 104 chars — validate at prototype stage, fall back to stacked if needed. |
| 4.11 | **Design tokens** | **Plan B** (concrete Rust constants) | Implementers need constants, not semantic names alone. Plan B's ~50 `const` declarations with doc comments are directly usable. Plan A's semantic names become the doc comments on Plan B's constants. |
| 4.12 | **Empty state copy** | **Plan B** (specific domain language) | "press a to decompose" is better than "one obvious first act." Domain-specific language ("decompose" not "add", "name what matters" not "create a task") reinforces the instrument identity. |
| 4.13 | **Anti-patterns list** | **Plan B** (11 explicit anti-patterns) | Invaluable for preventing regression. Plan A's constraints (§9) partially cover this territory but don't call them out as anti-patterns explicitly. Adopt Plan B's list as a binding appendix. |
| 4.14 | **Bead architecture** | **Hybrid** | Plan A's view-model layer is architecturally correct — typed view models prevent rendering logic from reaching into domain structs. Plan B's widget-first approach is faster to produce visible results. **Resolution:** Adopt Plan A's view-model beads as the foundation layer but merge them with Plan B's concrete widget specifications. Each view model bead should include the Rust data contract (from Plan B) and the ftui widget mapping (from Plan B). This is "view-model-first execution with widget-binding specifications." |
| 4.15 | **Gaze card border** | **Plan B** (Panel with Rounded cyan border — definitive) | Plan A's hedging ("only if border weight remains quiet") introduces implementation ambiguity. Lock the decision: Rounded cyan Panel. Validate in prototype. If too noisy, that's a future ADR amendment, not an implementation-time choice. |
| 4.16 | **Axis model** | **Plan B** (four explicit axes) | The depth axis (field→gaze→analysis = shallow→deep) and time axis (older left → newer right) are already implicit in Plan A. Making them explicit costs nothing and adds clarity for layout decisions. |
| 4.17 | **Stability ring placement** | **Plan A** (responsive + alerts in Ring 1) | Responsive doctrine and alert architecture are more "sacred" than Plan B suggests. Changing breakpoints or alert architecture after components are built creates widespread rework. These belong in Ring 1. |

---

# PART 5: BEAD RECONCILIATION

## 5.1 Granularity Comparison

| Dimension | Plan A (80 beads) | Plan B (40 beads) |
|-----------|-------------------|-------------------|
| Ring 1 (Foundation) | 20 beads — heavy on view models, fixture data, snapshot harness, focus contract | 10 beads — tokens, glyphs, styles, screen skeleton, widget prototypes, core renderers |
| Ring 2 (Components + Integration) | ~55 beads — components, integration, layout tiers, full interaction surfaces | ~24 beads — components, input surfaces, integration, polish |
| Ring 3 (Polish/Extras) | 5 beads — extensions, handbook | 6 beads — sparkline, minibar, terminal detection, min-size, undo, performance |
| View Model beads | 8 dedicated beads (10-17) | 0 dedicated beads (data contracts inline with renderers) |
| Integration beads | ~19 beads (53-71) | 1 mega-bead (17: View Integration) |
| Layout tier beads | 3 dedicated beads (58, 59, 60) | 1 bead (18: Responsive Layout Adaptation) |

## 5.2 Beads Unique to Plan A (No Plan B Equivalent)

| Bead | Description | Value Assessment |
|------|-------------|------------------|
| 5: Domain Visibility Matrix | Data-driven matrix of which concepts appear at which depth | **High** — prevents ad hoc depth decisions in every component |
| 8: Focus and Modal Ownership Contract | Explicit focus stack for overlays | **High** — prevents interaction bugs in nested surfaces |
| 11: Structural Signal Severity Model | Normalized signal ranking | **Medium** — useful but could be inline with signal rail component |
| 15: Review Queue View Models | Typed models for watch/agent review | **Medium** — useful abstraction layer |
| 17: Tree Surface View Model | Typed tree model | **Low** — tree is secondary |
| 18: Renderer Debt Audit | Explicit audit of what to delete | **High** — prevents ghost code |
| 36: Signal Rail Component | Persistent local signal list | **Medium** — Plan B distributes this across alert bar and inline badges |
| 50: Tree Surface | `ftui` Tree widget integration | **Low** — Ring 3 feature |
| 51: Trace Surface | Deep history review | **Low** — Ring 3 feature |
| 55: Selection and Cursor Preservation | Explicit bead for return semantics | **High** — critical UX concern not addressed in Plan B |
| 71: External Change Reload | Handling external mutations | **Medium** — important but deferrable |
| 75: VirtualizedList PoC | Decisive proof of concept | **High** — resolves the biggest framework question |
| 76: Cross-Terminal Validation Pack | Manual terminal testing checklist | **Medium** |
| 78: End-to-End Session Walkthrough | Scripted full session test | **High** — validates coherence |

## 5.3 Beads Unique to Plan B (No Plan A Equivalent)

| Bead | Description | Value Assessment |
|------|-------------|------------------|
| 1: Spatial Law Constants | Four-axis spatial law as code with validation function | **Medium** — elegant but Plan A's constraints cover this |
| 5: Screen Layout Skeleton | Top-level `Flex::vertical()` split: Content + Alert Bar + Lever | **High** — foundational layout missing from Plan A |
| 12: Rule Widget Integration | Replace all manual rule rendering | **High** — anti-pattern elimination |
| 13: Badge System | Badge factory function for all dynamics | **Medium** |
| 23: Reorder Mode Visual | `≡` grab handle, visual swap | **Medium** |
| 24: Toast System | `Toast` + `NotificationQueue` for action outcomes | **Medium** |
| 25: Spinner for Agent Activity | Braille spinner in lever | **Low** — polish |
| 28: Sparkline for Activity History | Analysis view enhancement | **Low** — Ring 3 |
| 29: MiniBar for Magnitude | Analysis view enhancement | **Low** — Ring 3 |
| 30: Terminal Capability Detection | COLORTERM sniffing, glyph fallback | **Medium** |
| 31: Minimum Terminal Size | Graceful degradation at extreme sizes | **Low** |
| 39: Widget Prototype Validation | 7 critical ftui composition tests | **Critical** — resolves all framework uncertainty |

## 5.4 Critical Path Comparison

**Plan A Critical Path (15 beads):**
```
Bead 1 (Breakpoints) → 5 (Visibility Matrix) → 6 (Fixtures) → 7 (Snapshots) →
9 (Component Scaffolding) → 10 (Field Row VM) → 21 (TensionStripe) →
53 (Root Field) → 55 (Selection) → 56 (Gaze Integration) →
57 (Analysis Integration) → 58 (Compact Layout) → 59 (Standard Layout) →
77 (Golden Snapshots) → 80 (Handbook)
```

**Plan B Critical Path (11 beads):**
```
Bead 39 (Widget Prototypes) → 2 (Tokens) → 3 (Colors) → 4 (Glyphs) →
6 (Tension Line) → 7 (Descended View) → 8 (Gaze Card) → 9 (Analysis) →
17 (View Integration) → 18 (Responsive) → 37 (Responsive Tests)
```

**Key difference:** Plan A front-loads view models and infrastructure; Plan B front-loads widget prototyping and concrete renderers. Plan B reaches a working screen 4-5 beads sooner, but Plan A's infrastructure (fixtures, snapshots, focus contract) prevents rework later.

## 5.5 Which Granularity Serves Swarm-Safe Execution Better?

**Plan A's 80 beads** are better for swarm parallelism because:
- More beads = more independent work units
- View model beads can execute in parallel with component scaffolding
- Integration beads are separated by layout tier, enabling parallel width validation
- Explicit dependency graphs with `Blocked by` / `Blocks` lists

**Plan B's 40 beads** are more implementation-efficient because:
- Fewer context switches
- Each bead produces visible, testable output
- Less overhead managing bead dependencies
- Concrete data contracts inline with renderers reduce the need for separate VM beads

## 5.6 Recommended Unified Bead Strategy

**Adopt Plan A's granularity for the foundation layer and Plan B's granularity for the implementation layer.**

Specifically:
1. **Foundation (Plan A style, ~15 beads):** Breakpoint model, token module, style module, glyph module, visibility matrix, fixture dataset, snapshot harness, focus/modal contract, screen layout skeleton, widget prototype validation, renderer debt audit.
2. **Core Components (Plan B style, ~12 beads):** Tension line (with Flex layout from Plan B), descended view, gaze card, analysis view, lever, alert bar, rule integration, badge system, empty states, selection mechanism, filter rendering.
3. **Interaction Surfaces (Plan A style, ~10 beads):** Separate beads for add, edit, note, confirm, search, help, move, reorder, agent review, insight review.
4. **Integration (Plan A style, ~8 beads):** Root field integration, descended field integration, gaze interaction, analysis interaction, compact layout, standard layout, expanded layout, selection/cursor preservation.
5. **Hardening (Hybrid, ~8 beads):** Golden snapshots, responsive invariant tests, terminal capability, unicode fallback, performance baseline, cross-terminal validation, end-to-end walkthrough, handbook.

**Total: ~53 beads** — between the two plans' counts, favoring actionable granularity over either extreme.

---

# PART 6: OPEN QUESTIONS (Unified)

## Resolved by One Plan, Open in the Other

| Question | Plan A Status | Plan B Status | Resolution |
|----------|--------------|---------------|------------|
| Gaze card border style | Open ("Panel only if...") | Resolved: Rounded cyan Panel | Adopt Plan B |
| Content width cap | Not addressed | Resolved: 104 chars centered | Adopt Plan B |
| Selection indicator implementation | Not specified | Resolved: Panel::borders(LEFT) Heavy cyan | Adopt Plan B |
| Trunk rendering | Not specified | Resolved: Panel::borders(LEFT) Square dim | Adopt Plan B |
| Minimum terminal size | Not specified | Resolved: 40x5 minimum | Adopt Plan B |
| Empty state copy | Concept only | Resolved: specific domain language | Adopt Plan B |
| Resolved/Released visual | Badges + dim style | Resolved: dedicated glyphs ✦ · | Adopt Plan B |

## Open in Both Plans

| Question | Plan A Reference | Plan B Reference | Notes |
|----------|-----------------|------------------|-------|
| **Pinned analysis at 160+: opt-in, sticky, or auto?** | §21.1 | ADR-6 (deferred to Ring 3) | Both defer. Recommend: opt-in, defaulting to off. |
| **Released tensions: visible in default filter or behind "All"?** | §21.1 | Part XIV Q5 | Both flag this. Recommend: default to Active filter, matching current codebase behavior. |
| **Watch insights: dedicated persistent surface or lever+review only?** | §21.1 | Part XIV Q2 | Both defer. Recommend: lever + review only (Ring 2), dedicated surface as Ring 3. |
| **VirtualizedList variable-height rows?** | §21.2 (needs prototyping) | Part XIV Q6 (Bead 39 validates) | Both flag. Plan B's Bead 39 is the right approach — prototype early, decide once. |
| **HistoryPanel vs custom trace list?** | §21.2 | Not explicitly flagged | Plan A's question stands. Resolve during analysis view implementation. |
| **Borderless vs panelized gaze?** | §21.2 | Resolved (panelized) | Plan B decided; accept that decision. |
| **Two-line vs one-line tension stripe?** | Not discussed | Part XIV Q3 | Plan B flags Codex's two-line proposal. Both plans use one-line. The question is closed: one-line wins. |
| **CommandPalette scope?** | §14.10 (registered but secondary) | Part XIV Q4 (Ring 3) | Both agree it's deferred. Ring 3. |
| **Analysis two-column cramped at 104?** | N/A | Part XIV Q10 | Valid concern. Prototype at Md width; fall back to stacked if cramped. |
| **ftui focus choreography for nested review+edit?** | §21.3 | Part XIV Q12 (FocusManager) | Both flag. Plan A's Bead 8 (Focus Contract) addresses this directly. |
| **Terminal glyph/border consistency?** | §21.3 | Part XIV Q7, Q9 | Both flag. Address via widget prototype validation (Plan B Bead 39) + terminal validation pack (Plan A Bead 76). |
| **Time-travel replay surface?** | §21.4 | Not discussed | Plan A's question. Defer to Ring 3 extension seams. |
| **Theme variants?** | §21.4 (Ring 3) | Part V (Ring 3) | Both agree: Ring 3, if ever. |

## Questions Raised by This Analysis

| Question | Source |
|----------|--------|
| Should the unified glyph system use 6 glyphs (4 phase + ✦ + ·) with SEPARATE tendency tokens (→ ↔ ·), combining Plan A's tendency approach with Plan B's terminal-state glyphs? | §4.1 + §4.2 recommendation |
| Can the Flex tension line support a 5th constraint (tendency token) without breaking the width budget at 80 columns? | §4.8 recommendation |
| Should the alert bar support Esc-dismissal for individual alerts, or only structural clearing? | §4.6 |
| How should the alert bar and signal rail (Plan A) relate? Is the signal rail the expanded-width version of the alert bar? | §2.5 vs Plan A §14.7 |

---

*This analysis is a comparison instrument, not a new plan. The decision table in Part 4 provides binding positions for each divergence. The bead reconciliation in Part 5 provides a strategy for merging the two decompositions. A follow-up plan should implement these decisions.*
