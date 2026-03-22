# Reflection on the Werk Operative Instrument TUI Design System

## 1. What Was Easy and What Was Hard?

### What came together naturally

Several things were unusually coherent once the repository structure was clear.

- The core directional law came together fast. The domain already strongly implies that desire is “upstream” and reality is “ground,” and the existing `werk-tui` code already leans in that direction. Making `desired` live above `actual`, and treating left as basis and right as intent/projection, felt native to the model rather than imposed on it.
- The visual grammar was also relatively easy to stabilize. `werk-tui/src/glyphs.rs` already contains a strong seed: lifecycle glyphs, three rule weights, temporal dots, and restrained separators. That made it possible to promote an existing language rather than invent a new one.
- The domain itself is richer than the current UI. `sd-core` already exposes meaningful computed dynamics: phase, tendency, conflict, neglect, oscillation, resolution, urgency, drift, assimilation, orientation, compensating strategy, projections, and events. That meant the design problem was mostly “how should this appear?” rather than “what should exist?”
- `ftui` was stronger than expected on composition. Once its actual widget surface was inspected, it became clear that a serious system could be built from `Panel`, `Rule`, `Badge`, `VirtualizedList`, `Table`, `Tree`, `Modal`, `CommandPalette`, `HistoryPanel`, `Sparkline`, and `StatusLine` without falling back to custom drawing.

### What was harder

The hard parts were mostly about choosing the right level of commitment.

- The biggest difficulty was the gap between domain sophistication and UI maturity. The domain can justify a very rich instrument, but the current app is still structurally simple. The design system therefore had to aim higher without pretending the implementation is already there.
- The second hard part was staying honest about the “ftui only” constraint. The temptation is to describe bespoke terminal compositions in conceptual terms, but the brief explicitly forbids bypassing the framework. That forced careful verification of which `ftui` types actually exist and what they can plausibly do.
- The hardest design judgment was deciding how much of the analysis layer should be visible by default. The domain supports deep diagnostics, but the instrument needs repeated-use clarity, not analytical sprawl. Finding a layering model that preserves seriousness without becoming dense for density’s sake was the main design tension.
- There was also some uncertainty around what should be primary versus occasional. For example, `Tree` is excellent for topology, but not necessarily the best primary working surface. `Table` is excellent for comparison, but can quickly become too “report-like” if overused.

## 2. What Questions Did Not Have Clear Answers?

### Repository and source-of-truth ambiguity

The brief names `ftui/` and `werk/` as local directories, but this workspace does not contain them in that form.

- `ftui` is an external dependency, not a first-party directory in the repo.
- the role of `werk/` is actually split across `sd-core/`, `werk-shared/`, and parts of `werk-cli/`

That did not block the work, but it matters because “what is canonical?” is already slightly ambiguous at the repository level.

### Domain ambiguities

The domain is strong, but some parts are still under-specified from a UI perspective.

- It is not fully clear which dynamics are meant to be persistent first-class operator signals versus secondary analytic interpretation. For example, conflict and neglect clearly deserve persistent visibility; compensating strategy is less obvious.
- Projections exist in `sd-core/src/projection.rs`, but it is not clear how central they are meant to be in everyday operation versus occasional inspection.
- Watch insights and agent suggestions exist, but their desired role in the instrument is not fully settled. Are they adjunct review queues, or are they intended to become native parts of field operation?
- The current domain distinguishes between resolved and released, but it is less explicit about whether terminal states should remain visibly “in field” by default or be collapsed away in everyday operation.

### Framework ambiguities

`ftui` exposes a surprisingly broad surface, but some important things remain uncertain until implementation.

- `VirtualizedList` appears capable enough to replace the bespoke list implementation, but the real ergonomics of variable-height, highly structured rows in this app are not yet proven.
- The pane layout model in `ftui-layout` is promising, but it is not yet clear whether it is mature enough to be a foundational part of the operator workspace or should remain a later enhancement.
- `Rule` and custom borders allow a strong rule grammar, but the exact visual consistency of heavy/light/dotted rules across all terminal environments still needs practical verification.
- `CommandPalette` is powerful, but the right split between palette-driven commands and direct-key operations is still a product decision, not a framework fact.

### Product ambiguities

The design system had to make assumptions on a few product-level questions.

- Is the instrument primarily for daily steering, for periodic review, or equally for both?
- Should the default experience be breadth-first field scanning or single-tension depth?
- How much prose should the system emit by default when structural signals are already present?
- Is watch/agent functionality central enough to deserve permanent screen real estate, or should it stay modal and episodic?

Those questions materially affect layout, mode architecture, and progressive disclosure.

## 3. What Core Decisions Need to Be Made, and in What Order?

There is a clear dependency hierarchy. Some decisions are foundational. Others should not be made before the foundations are locked.

### 1. Lock the product stance

This comes first because everything depends on it.

Decide, explicitly:

- Is `werk-tui` primarily a field instrument, a command console, or a hybrid?
- Is the design center “repeated daily operation” or “deep periodic analysis”?
- Are watch and agent systems core to the primary loop or secondary review systems?

Without these answers, layout and mode decisions will drift.

### 2. Lock the directional invariants

This is the most important visual contract.

Decide and document:

- `desired` is always above `actual`
- left is basis/history/grounded context
- right is intention/projection/action
- these invariants survive every breakpoint and mode

This must be fixed before any component work. Otherwise different screens will invent different spatial semantics.

### 3. Lock the visual token system

Before building screens, define a canonical token catalog:

- rule meanings: heavy, light, dotted
- lifecycle glyphs
- terminal state glyphs
- temporal dots
- badge classes
- severity and color semantics

This should become a small documented design token layer. Without it, implementation will fragment quickly.

### 4. Lock the information architecture

Decide the primary depth layers:

- field scan
- focused gaze
- full analysis

Then decide what is visible by default at each layer, and what graduates from one layer to the next. This is the decision that keeps the instrument from becoming either too sparse or too report-heavy.

### 5. Lock the mode model

Only after the layer model is clear should the team decide:

- which operations are inline
- which use the command palette
- which use modal write surfaces
- which use dedicated review surfaces

If this is done too early, the app will accrete modes around implementation convenience rather than operator clarity.

### 6. Lock the application state architecture

Once the above is stable, decide how the app state should be shaped.

This includes:

- selected tension and viewport state
- derived field/gaze/analysis view models
- focus graph and modal stack
- persistence of panel/layout preferences
- integration model for watch backlog and agent review queues

This is where implementation can go wrong if the state shape reflects current rendering hacks rather than the intended surface model.

### 7. Lock the migration path from the current renderer

At that point, the team can decide:

- whether to replace `vlist.rs` immediately with `VirtualizedList`
- whether to migrate incrementally screen by screen
- whether to ship a new scan surface first or first stabilize the interaction model

This should be treated as a staged architecture decision, not an opportunistic refactor.

### Practical dependency order

The most defensible order is:

1. product stance
2. directional invariants
3. visual tokens
4. layer model
5. interaction/mode model
6. state architecture
7. component and layout composition
8. migration sequencing
9. test strategy and rollout

## 4. What Do I Recommend to Harden Further Work?

### A. Create a canonical design token and semantics spec

The design system is still one large document. The next hardening step should be a compact companion spec that lists:

- every rule type
- every badge class
- every glyph
- every dot grammar
- every color role
- every allowed border treatment

This should be short, explicit, and implementation-facing.

### B. Write a component contract for each primary surface

The next missing layer is not more vision. It is component-level rigor.

At minimum, define canonical contracts for:

- tension stripe
- field plane
- gaze plane
- analysis plane
- signal rail
- review modal
- command palette categories

Each should state:

- required data
- optional data
- breakpoint behavior
- focus behavior
- what never changes

### C. Clarify domain display priorities

The domain currently offers more signals than a first implementation should show at once.

Decide:

- which signals are primary and persistent
- which are secondary and on-demand
- which are debug/expert only

If this is not decided, the UI will overfit to available data rather than operator need.

### D. Make watch and agent roles explicit

These subsystems need product clarity.

Write one short document answering:

- what enters the operator’s primary loop
- what stays in a review queue
- what is advisory versus actionable
- what gets permanent signal treatment

Until this is explicit, those features will remain visually awkward.

### E. Add scenario fixtures and golden-state rendering tests

This project needs concrete structural scenarios, not just unit tests.

Create fixtures for cases like:

- new germinating root
- healthy advancing parent with assimilating child
- neglected subtree
- oscillating sibling pair
- urgent overdue tension with repeated postponement
- multiple roots with no senior organizing principle
- agent mutation review with mixed actions
- watch insight backlog

Then render those scenarios into deterministic UI states and use snapshot or golden tests at the view-model and rendering levels.

### F. Test the responsive invariants directly

Responsive behavior is central to the design, not a secondary concern.

Write tests that verify:

- `desired` remains above `actual`
- signal badges survive breakpoints
- the same tension remains legible from `Xl` to `Xs`
- analysis collapses before core structural meaning does

If these invariants are not tested, the design will degrade exactly where it matters most.

### G. Reduce ad hoc rendering logic early

The current renderer still contains substantial bespoke layout logic. That is the main implementation risk.

I would recommend an early migration plan that prioritizes:

- `VirtualizedList` for the field
- `Panel` and `Rule` for composed surfaces
- `Modal` for edit/review surfaces
- `HistoryPanel` for history
- `CommandPalette` for search/jump/action unification

The longer custom rendering remains central, the harder it becomes to enforce a coherent system.

### H. Add explicit implementation notes for `ftui` gaps

Where the framework is uncertain, do not work from memory. Track gaps explicitly.

Keep a live note of:

- widgets that are confirmed usable
- widgets that need a proof-of-concept
- places where `ftui` needs extension
- places where the design should adapt to the framework rather than fight it

This avoids a common failure mode: designing past the framework and then backing into compromises later.

### I. Treat the next iteration as a structured prototype, not a final build

The next step should not be “implement the whole system.”

It should be:

1. build a canonical field scan surface
2. build a canonical gaze surface
3. validate on real fixtures
4. refine the token grammar
5. then add analysis and review systems

That sequence will expose the right problems early.

## Final Assessment

The project is in a strong position conceptually. The domain is real, the current glyph language is promising, and `ftui` appears capable enough to support a serious instrument.

The main risks are not lack of ideas. They are:

- ambiguity about what is primary
- insufficiently explicit token semantics
- a renderer architecture that is still more bespoke than systematic
- lack of scenario-based validation for the structural cases that matter most

If those are addressed in order, the next iteration can move from “compelling direction” to “coherent instrument.”
