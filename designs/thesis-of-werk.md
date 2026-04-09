# The Thesis of Werk

## I. The Problem

Modern digital environments structurally recruit human energy toward external aims. Algorithms, notifications, feeds, dashboards, project management tools — each one pulls the user's attention into a frame designed by someone else, optimized for someone else's metric. The user becomes an instrument of the system rather than the system serving as an instrument of the user.

This is not a conspiracy. It is a structural condition. When the tools you use to organize your work are built around coordination topology (who owns what, what blocks what, what's the status for the manager), the tool's structure shapes your cognition toward management concerns even when you are the one doing the work. You think in tickets, sprints, and backlogs because that's what the tool makes thinkable.

Meanwhile, the pace at which reality changes and information travels has accelerated past what any individual's unaided cognitive architecture can process. Desires form on contingent information and may need revision within days. Observations accumulate faster than they can be integrated. The gap between "what I noticed" and "what I've formally acknowledged" widens continuously. The practitioner who doesn't have a structure for holding this — who relies on memory, habit, or the latest notification — loses coherence. Their energy scatters across whatever feels urgent rather than flowing toward what they actually aim at.

Robert Fritz identified the generative mechanism: structural tension between a clearly seen desired outcome and an honestly acknowledged current reality produces the energy for creative action. The structure resolves toward the result. But Fritz developed this in a world where the rate of change allowed a practitioner to hold a few tensions in their head and work them through conversation and paper. That world is gone. The structural dynamics framework is not invalidated — it is under-instrumented.

## II. The Instrument

Werk is an operative instrument for structural dynamics practice. It holds structural tensions — desire-reality pairs — and the practitioner's theory of how to close each gap. It computes temporal facts from user-supplied standards (deadlines, ordering). It surfaces signals by exception. It does not interpret, diagnose, or prescribe.

The word "instrument" is chosen carefully. A piano does not compose music. A compass does not navigate. A stethoscope does not diagnose. Each makes a specific domain of reality legible to a practitioner who brings skill, judgment, and intention. Werk makes the structure of directed action legible — the gaps, the theories, the progress, the drift — so the practitioner can read it and act.

The word "operative" distinguishes werk from management tools. Management coordinates what exists: dependencies, permissions, resources, status reports. Operations closes gaps between where you are and where you aim. Werk serves the person doing the work, not the person overseeing it. It assumes its users face reality and want to move.

### The Sacred Core

Ten principles constrain every design decision. They do not change.

1. **The One Spatial Law** — desired outcome above current reality. Always. The gap between them is the tension that generates movement.

2. **Theory of Closure** — children of a tension are not sub-items. They are the practitioner's current conjecture about how to cross the gap. Hypotheses, not permanent structure.

3. **Positioned and Held** — positioned means temporally committed and sequenced. Held means acknowledged but not yet placed. The distinction between "I will do this next" and "I might need to do this" is structural, not decorative.

4. **Frontier, Envelope, Cursor** — three independent concepts. The frontier is where accomplished meets remaining (a fact). The envelope is the interaction surface around the frontier (a window). The cursor is where attention sits (a choice).

5. **Signal by Exception** — silence is the default. Signals appear only when actionable, only where they are about, and leave when resolved.

6. **Gesture as Unit of Change** — every meaningful act is a gesture. Gestures group mutations. Undo reverses a gesture, not a field. History is a sequence of gestures. The term carries weight from operative traditions (Fernandez) and generative anthropology (Gans).

7. **Locality** — signal propagates one level: self, parent, children. Not globally. Local patterns are high-confidence. Field-level patterns are weaker inference.

8. **Root Tensions as Coherence Generators** — fewer tensions held honestly produce more structural integrity than many tensions held loosely. The root level organizes the user's directed energy.

9. **Structure Determines Behavior** — the instrument is not consulted. It is inhabited. Opening it is stepping into a field that shapes cognition toward the user's aims.

10. **Standard of Measurement** — every computation the instrument produces is anchored to something the user explicitly provided. No user-supplied standard, no instrument-generated inference. This is the epistemological boundary.

### What the Instrument Does Not Do

Werk does not compute dynamics — phase, tendency, oscillation, conflict, compensating strategy. These are interpretive frameworks from the structural dynamics *practice* that require understanding the user's life, intentions, and context. The instrument surfaces honest facts. The human (possibly aided by AI) or their coach reads them.

Werk does not orchestrate agents. It exposes its affordances through three interface surfaces (TUI, CLI, MCP protocol) and any agent — human or artificial — can inhabit those surfaces. The instrument does not launch, manage, or monitor external intelligence.

Werk does not track dependencies between tensions, enforce permissions, or manage coordination topology. If tension B cannot advance because tension A hasn't resolved, that is a fact about B's current reality — write it there.

## III. The Architecture

### Four Frameworks

The instrument is specified by four formal systems, each appropriate to its domain:

**Architecture of Space** — how navigational and visual space is structured. The one spatial law. Four axes: pitch (time), roll (structure), yaw (orientation), zoom (density). The descended view and the survey view as dual projections of the same space.

**Grammar of Action** — how gestures compose into meaningful wholes. Sixteen operative gestures: creating, decomposing, composing, positioning, holding, resolving, releasing, updating reality, evolving desire, noting, reordering, snoozing, recurring, splitting, merging, navigating. Each is a compression of intentionality into enacted form.

**Calculus of Time** — how temporal quantities are set, computed, and produce signals. Two user-set primitives (deadline, order of operations). Six computed properties (implied window, urgency, sequencing pressure, critical path, containment, overdue). Two recorded facts (actual and reported resolution time). Three temporal orientations: taxis (sequence), chronos (calendar), kairos (readiness).

**Logic of Framing** — how context determines visibility, actionability, and signal. Envelope extent rules. Zoom density levels. The relationship between state and available gestures.

### Three Interface Surfaces

**TUI** — the primary experience. A session. The user inhabits the structure. Built on FrankenTUI for terminal-native rendering. Three-pane layout: desire (top), field of action (center), reality (bottom). Deck view for structural depth, survey view for temporal breadth. Tab transposes axes while carrying selection.

**CLI** — every gesture as a command. Human-readable text by default, structured JSON with `--json`. Non-interactive — no stdin prompts, every input passable as a flag. Grouped by framework in `--help`.

**MCP** — protocol surface for AI agents. Stdio transport. Same gestures, same payloads. An agent performing `reality 42 "new state"` via MCP fires the same hooks as a human typing it in the CLI.

### The Data Model

A tension is a desire-reality pair with status (Active, Resolved, Released), optional deadline, optional position, and parent reference. Children form the theory of closure. Edges track containment (parent-child), provenance (split-from, merged-into). Mutations record every field change with gesture association and timestamp.

The mutation log is the source of truth. Tension state is a projection. Epochs snapshot desire, reality, and children at phase transitions. The logbase — the composite lattice of all epoch sequences — is queryable structure, not dead history. Geological strata encoding the project's evolution.

Executable formal specifications in Quint verify the sacred core — state machine invariants, forest properties, temporal containment, gesture atomicity, undo conflict detection — against the same state space the Rust implementation operates on.

### The Event System

Every mutation the Store performs emits a typed Event through the EventBus. Thirteen event types: TensionCreated, RealityConfronted, DesireRevised, TensionResolved, TensionReleased, TensionDeleted, StructureChanged, HorizonChanged, NoteTaken, NoteRetracted, GestureUndone, UrgencyThresholdCrossed, HorizonDriftDetected.

The HookBridge subscribes to the EventBus and fires shell hooks automatically. Adding a new Event variant makes it hookable with zero wiring. Pre-hooks block at the command level (the Store stays a pure data layer). Post-hooks fire automatically through the bridge.

Hook configuration supports chains (multiple commands per event), categories (`post_mutation` fires for any mutation), wildcards (`post_*` fires for everything), filters (`parent:N`), and dual scope (global + workspace, global fires first).

## IV. The Practice

### Desire and Reality

Desire is a trajectory — an intended path through possibility space. Forward-looking, projected, subject to revision. Reality is a trace — the actual path through state space. Backward-looking, factual, subject to reversal. Reality accumulates history but not necessarily ground. The trace only gets longer; experience cannot be un-experienced. But the current position can be behind where you were.

The asymmetry matters. A desire change reorients the entire theory of closure — the destination moved, the path may be wrong. A reality change recalibrates — the aim holds but the position shifted. Desire changes are rare and heavy. Reality changes are frequent and granular. This asymmetry is structural, not accidental.

### Notes as Pre-Structural Signals

Notes are observations that haven't yet been absorbed into the reality or desire fields. They sit at the boundary between perception and formal acknowledgment. A note is simultaneously: to notice (perception), to record (compression), and an atomic unit of performance within the instrument (the musical sense).

In practice, notes are the leading indicator of reality updates. When notes accumulate without a reality update, testimony is piling up without integration. When notes reference constraints not in the desire field, the aim may be shifting without being acknowledged. Notes are pre-structural — they are waiting to become reality updates, desire revisions, new children, or nothing.

### The Wayfinding Connection

The nervous system is a recursive wayfinding architecture that converts sensed difference into indexed, valued, predicted, and selected action under constraint. Doolittle's canonical ladder — sensation, orientation, disambiguation, salience, indexing, valuation, prediction, selection, execution, correction — maps directly onto the practitioner's cycle within the instrument:

- **Sensation**: the event system detects mutations
- **Orientation**: the survey and urgency system direct attention
- **Disambiguation**: notes are the practitioner performing disambiguation — narrowing observation into articulation
- **Salience**: the signal system ranks what matters (urgency, containment, sequencing pressure)
- **Indexing**: the tension tree places observations in structural relation
- **Valuation**: the gap between desire and reality is the valuation — how far from aim
- **Prediction**: the theory of closure is the practitioner's prediction of how to converge
- **Selection**: positioning is the go/no-go gate — committing a step to the sequence
- **Execution**: the gesture is the enacted act
- **Correction**: epochs compare expected against actual — the trace zigzags, the theory updates

The instrument supports each stage. But it does not perform disambiguation, valuation, or prediction for the user. Those are the practitioner's work. The instrument holds the results.

### Activity Signals

The instrument currently surfaces structural signals (containment violations, sequencing pressure) and temporal signals (urgency, overdue, stale). A third layer is emerging: activity signals — patterns in what the practitioner is doing that imply something about the state of the fields.

**Reality pressure** — activity has accumulated on or below a tension (notes taken, children resolved, child realities updated) without a reality update on the tension itself. Reality has moved but the summary hasn't caught up.

**Desire drift** — activity below a tension suggests the aim has shifted without the desire field being revised. Children released, new children that don't serve the stated desire, notes referencing constraints not in the desire field.

**Testimony accumulation** — notes piling up without being absorbed into structure. Observations waiting to become reality updates, desire revisions, or new children.

**Closure momentum** — the rate of child resolution relative to child creation. Converging (resolving faster than creating) or diverging (creating faster than resolving).

These compose: reality pressure with high closure momentum means things are going well, just update the summary. Reality pressure with low closure momentum means stalled — the notes are piling up because the practitioner is stuck. Desire drift with testimony accumulation means the aim is shifting and the notes probably contain the real desire.

## V. The Situation

As of April 2026, werk holds 199 tensions. 35 active, 108 resolved, 56 released. 362 commits since March 2026. Activity rate: 37 mutations per day across 95 tensions in the last week. The instrument is used daily as its own development tracker — building the instrument with the instrument.

The tension tree has one root: #2, "werk is a mature tool for practicing structural dynamics." Under it, 22 active children spanning TUI rendering, documentation, business model, formal specification, hook infrastructure, typed edges, and more. 14 of 22 resolved. The tree is 4 waves deep.

The instrument exists across five surfaces: TUI (FrankenTUI-based terminal app), CLI (37 commands), MCP (35 tools), Web (Axum server), Desktop (Tauri app). The CLI and MCP are complete. The TUI is the active frontier. Web and desktop are scaffolded.

The codebase is ~25,000 lines of Rust across six crates: `sd-core` (data model, store, events, graph, temporal), `werk-shared` (config, hooks, workspace, palette), `werk-cli` (commands), `werk-mcp` (protocol surface), `werk-tui` (terminal interface), `werk-web` (HTTP surface).

The formal specification in Quint verifies seven invariants across state machine, forest, temporal, and gesture domains. The hook system bridges all 13 event types automatically. The signal system surfaces containment violations, sequencing pressure, critical path, urgency collisions, horizon drift, and structural signals (hub, spine, reach).

## VI. The Thesis

Werk's thesis is that the generative mechanism of structural tension — clearly seen desire held against honestly acknowledged reality — is the most powerful engine for directed action available to human beings, and that this mechanism is currently under-instrumented.

The instrument does not replace the practice. It does not automate judgment, interpret patterns, or prescribe action. It holds the structure so the practitioner can read it. It computes facts so the practitioner can see what's true. It surfaces signals so the practitioner knows where to look.

The instrument is thermodynamically advantageous: it reduces the energy required to maintain coherent directed action across multiple evolving tensions and time horizons. Without it, the practitioner must hold desire, reality, theory, frontier, temporal situation, and cross-tension relationships in working memory. With it, those facts are externalized into a structure that persists, computes, and signals.

The deeper thesis is about sovereignty. In a world where desires are manipulated by billion-dollar corporations, where reality changes faster than it can be processed, where algorithms structurally recruit human attention toward external aims — the ability to hold your own structural tensions honestly, to maintain your own theory of closure, to read your own patterns, is an act of self-governance. The instrument supports this by being operative (serving the person doing the work) rather than managerial (serving the person overseeing it), by anchoring every computation to user-supplied standards rather than hidden heuristics, and by drawing a hard line between what the instrument computes and what belongs to the practice.

The instrument is a wayfinding structure. The practitioner enters it, orients by desire and reality, disambiguates through observation, ranks by signal, indexes by structural position, values by gap, predicts by theory of closure, selects by positioning, executes by gesture, and corrects by epoch. The cycle is recursive. The structure supports it. The practice lives in the recursion.

Consciousness of one's own structure is renewed through the craft, not through the instrument's assertions. The instrument holds. The practitioner reads. The practice deepens through contact with honest structure, held over time.
