# Werk: Conceptual Foundation

**Emerged:** 2026-03-20 through 2026-03-22, through sustained dialogue
**Last amended:** 2026-03-27 (terminology standardization, implementation grounding, temporal orientations)
**Status:** Living document. This sits above all design system plans and implementation documents. Changes here propagate downward; changes below do not propagate upward without explicit amendment.

---

## What This Document Is

This is the conceptual foundation of werk — the operative instrument for structural dynamics practice. It captures the principles, structures, and vocabulary that the instrument embodies. Design system plans, implementation specifications, and UI documents derive from this foundation; they do not define it.

This document emerged from direct reasoning about what the instrument is for, what it affords, and what it must preserve. It is grounded in the structural dynamics framework as taught by Nicholas Catton and rooted in Robert Fritz's creative process work, in the operative traditions as articulated by Miguel A. Fernandez, in generative anthropology's understanding of gesture and deferral, and in the lived practice of the instrument's creator.

**Implementation status:** Parts I and II are fully implemented and validated through use. Part III describes both implemented features (operating envelope, survey view, sessions, ground mode, operative gestures, navigation) and aspirational design (thresholds, logbook query system, split/merge gestures) — the latter are clearly marked. Part IV defines the four formal frameworks. Part V lists open vocabulary. The instrument is operational across three interface surfaces (TUI, CLI, MCP) with 30+ commands and 30 MCP tools.

---

## Part I: The Sacred Core

### 1. The One Spatial Law

**Desired outcome above current reality.**

This is the single invariant from which all other spatial and temporal organization derives. It is absolute, surviving every width, every depth, every mode of the instrument. No composition that shows both desire and reality may place reality above desire.

This law unifies space and time on a single axis: reality is now (bottom), desire is the aimed-at future (top). The gap between them is the structural tension that generates the energy for creative action. The vertical axis carries one meaning: **now, through enacted movement, toward aimed-at future.**

### 2. Theory of Closure

Children of a tension are not merely "sub-items." They are the user's **current theory of how to cross the gap** between desired outcome and current reality. They are conjectured, composed, chosen, and committed action steps forming a bridge from where the user stands to where they aim.

This means:
- The descended view is a **reasoning surface**, not an information display
- Each time the user sees it, they implicitly ask: "does my theory of closure still make sense?"
- Action steps are hypotheses. They may be wrong. They may need replacement. They exist in service of closing the gap, not as permanent structure.
- An action step is structurally a tension with a desired outcome but no independent current reality — its reality is inherited from the parent's current state

### 3. Positioned and Unpositioned

**Positioned** means temporally committed and sequenced within the theory of closure. These steps have been placed in a deliberate order reflecting the user's judgment about what comes first, what depends on what, and what the temporal progression looks like.

**Held** means acknowledged but not yet committed to a place in the sequence. These are candidates for the theory of closure — recognized possibilities that have not yet been given a temporal home within the parent's frame.

### 4. Frontier, Envelope, Cursor

Three nested but independent concepts:

**Frontier of action** — a conceptual boundary. Where NOW falls on the time/order stream. It exists whether or not the user is looking at it. It's the fact of where accomplished meets remaining.

**Operating envelope** — an interaction zone. A dynamic window on the stream around the frontier, containing everything action-relevant (overdue steps, next step, held steps, recently resolved). The envelope has extent — bigger when there's more to act on. It is both a container (bounded, demarcated) and a window on the stream (the stream flows through it). The envelope is the primary interaction surface.

**Cursor** — the user's focus point. Where attention and selection currently sit. The cursor can be inside the envelope (working at the frontier), above it (reviewing theory), below it (reviewing trace or logbook), or in the survey view (scanning the field). The envelope persists regardless of cursor position. The frontier persists regardless of both.

When the user opens the instrument, the cursor lands at the envelope. They look up to see what's next. They look down to see what's done. They look at the top to remember where they're going. They look at the bottom to remember where they stand. They yaw to survey the field across all tensions.

### 5. Signal by Exception

The instrument's default state is silence. When a tension is on track, nothing extra appears. Signals surface only when they are actionable and contextually relevant — and they appear in the place they are about, not in a separate dashboard or alert surface.

Signals should be contextual, subtle, in-place, and leave from view when resolved.

### 6. Gesture as Unit of Change

A **gesture** is the unit of meaningful action within the instrument. It may involve a single mutation (resolving a step) or a coherent set of mutations that together represent one intentional act (repositioning three steps, updating reality, and adding a new step as a single restructuring of the theory of closure).

The term "gesture" is chosen deliberately. In an operative tradition (Fernandez), technique manifests through gesture — a compression of intentionality into enacted form. In generative anthropology (Gans), the originary gesture defers violence and makes language possible. In this instrument, each gesture is an operative act through which the user's relationship to their own creative structure becomes conscious and refined.

Gestures, not individual mutations, are the meaningful units for:
- Undo (undoing a gesture undoes the whole set)
- History (the logbook is a sequence of gestures)
- Interpretation (a gesture is the smallest meaningful event for reading structural patterns)

In the data model, each mutation carries a `gesture_id` linking it to its gesture. A gesture optionally belongs to a session. CLI commands each constitute a single gesture. Batch operations group all their mutations under one gesture. Agent mutations are sessionless gestures.

### 7. Locality

Meaningful signal propagates one level, not globally. A mutation at a given tension produces signal relevant to:
- **The tension itself** (its own state changed)
- **Its parent** (the parent's theory of closure changed)
- **Its direct children** (their context changed)

Signal beyond this neighborhood — grandparent, cousin, global — is weaker inference operating at greater distance. Any interpretation of structural patterns should respect this: local patterns (from the tension and its immediate neighborhood) are higher-confidence than field-level patterns (from broader structural relationships).

### 8. Root Tensions as Coherence Generators

Everything in the structure derives meaning from the root level. Sub-tensions, action steps, temporal organization, the frontier of action — all of these are coherent only insofar as they serve the root tensions that organize the user's directed energy.

A structure with twenty root tensions is not a structure — it is a wish list. A structure with five root tensions, each held honestly with accurate reality and active theory of closure, is a machine that generates coherence. (Fritz, via Catton: "Fewer tensions held honestly produce more structural integrity than many tensions held loosely.")

### 9. Structure Determines Behavior

The instrument is not a tool the user consults. It is a **structure the user inhabits**. Opening it is stepping into a field that shapes cognition toward the user's aims. It should never make itself the point — only the structure supporting directed action.

The instrument is a thermodynamically advantageous structure: it reduces the energy required to maintain coherent directed action across multiple evolving tensions and time horizons. It does this through spatial organization, temporal encoding, and the operative discipline of gesture.

---

## Part II: Structural Concepts

### The Delta

A **delta** is a desire-reality pair together with its theory of closure at a specific point in time. It is the fundamental structural unit of directed action.

- **Desire** (top): the articulated aim, which evolves through contact with action and reality
- **Reality** (bottom): the current ground truth, which accumulates accomplished truth as steps complete
- **Theory of closure** (between): the composed bridge of action steps from reality to desire

Deltas are versioned. When desire transforms or reality shifts signifantly, a new delta is generated. The relationship between consecutive deltas — the progression of pyramids — is the **logbook**: the shape of directed action over time.

### Resolution as Transformation

Resolving an action step does not delete it. It transforms it — moving it across the frontier of action from "remaining theory" to "accomplished reality." A resolved step *was* load-bearing ground at the time of resolution. Whether it *remains* load-bearing depends on whether reality holds — ground can reopen, context can shift, and what was accomplished may need to be re-traversed or superseded.

Resolution is a narrative beat: the user compresses the event of completion into the instrument's record. The trace of reality advances (or at least marks a point). Desire may evolve in response — sharpening, expanding, contracting, pivoting, deepening, or dissolving through contact with action and learning.

### Desire is Trajectory, Reality is Trace

**Desire** is a **trajectory** — an intended path through possibility space. It is forward-looking, projected, subject to revision. Each version relates to the previous, but the relationship isn't always refinement — it may be reversal, expansion, contraction, deepening, or dissolution. Desire changes because the user learns through contact with action and reality.

**Reality** is a **trace** — the actual path through state space. It is backward-looking, factual, subject to reversal. Reality accumulates *history* (the trace only gets longer; experience cannot be un-experienced) but does not necessarily accumulate *ground* (the current position can be behind where you were). Resolved action steps remain historically resolved — they happened — but may become structurally insufficient when reality shifts and the ground they covered reopens.

The hunting analogy: the hunter's desire (kill the animal) may stay fixed or evolve. The hunter's reality (position, distance, spear status) changes continuously and can reverse — the animal bolts, ground gained is lost, the spear misses. The trace of reality zigzags. The theory of closure is continuously revised based on the gap between desired trajectory and current traced reality.

The *shape* of the trace is itself diagnostic. A trace that advances steadily is structurally different from one that oscillates around the same ground. The pattern of the trace carries information that individual state updates don't.

### Epoch and Phase Transition

An **epoch** is a period of action within a single delta — executing a theory of closure against a stable desire-reality pair. Within an epoch, movement is along the time axis: steps getting done, reality rising, the frontier advancing.

A **phase transition** is the boundary between epochs — a structural reconfiguration that generates a new delta. It is triggered by:
- Reality shift significant enough to invalidate the current theory of closure
- Desire transformation that redefines what "closure" means
- A resolved step that reveals the gap was different than assumed
- An external event that restructures the landscape

Phase transitions are orthogonal to within-epoch movement. They don't advance along the existing axis; they reconfigure the axis itself.

Phase transitions can also span tension boundaries. A **split** — recognizing that a tension contains two distinct concerns — creates a phase transition where one delta becomes two deltas in different tensions. Both new tensions share the pre-split history as provenance. The logbook is not always linear; it can fork.

The logbook is the sequence of epochs — the pyramids laid end to end, each growing from the material of the last. When a tension has been split from or merged into another, the logbook shows the connection: the pre-split tension as a shared root, or the pre-merge tensions as converging tributaries.

### The Logbook as Compiled Model

The accreted structure (closed epochs, resolved tensions, released tensions, notes, prior deltas) is not inert history — it is *compacted meaning*. Each epoch close, each resolution, each release crystallizes what was once open and uncertain into settled fact. The live structure is the frontier of becoming; the logbook is what has become.

Together, the live structure and the logbook form the complete model of a project — one under revision, one settled. There is an instructive difference between: (a) the "live" structure as it is open at any one moment; (b) the "accreted" structure that is no longer under revision or uncertainty; and (c) whatever external artifacts (code, documents, communications) the project produces. The logbook holds (b) and together with (a) constitutes the full structural record from which (c) can be understood, justified, and contextualized.

This means the logbook should be **queryable as structure** *(future — tracked as #89 "logbase")* — semantic search across epochs, filtering by topic or term or tension, cross-epoch pattern analysis, reconstruction of decision chains. The ability to ask "show me every epoch that touched this concern" or "what was the theory of closure when we made that decision" transforms the logbook from a simple record into a *structural resource*.

The analogy: geological strata encode Earth's history in queryable form. A well-kept logbook tells the full story of a voyage. Whitehead's perished actual occasions persist as objective data for new occasions of becoming. The logbook is the instrument's memory — not nostalgic, but structurally generative.

### The Instrument's Boundary

The instrument computes **facts** and **signals** from the data it holds. It does not compute **dynamics**.

- **Facts** are directly observable from the data: urgency (elapsed fraction of deadline window), overdue (deadline passed), horizon drift (whether deadlines moved), child-resolution rate, mutation recency.
- **Signals** are facts recognized as action-relevant by the instrument's design: sequencing pressure, critical path, containment violation. Surfaced by exception.
- **Dynamics** — phase, tendency, oscillation, conflict, neglect, compensating strategy, orientation — are interpretive frameworks from the structural dynamics *practice*. They require understanding the user's life, intentions, and context that the instrument does not and cannot capture. A human practitioner or AI may apply these frameworks to the facts the instrument surfaces, but the instrument itself does not make these readings.

This boundary is sacred. The instrument surfaces honest facts about what was recorded and when. Interpretation belongs to the practice — the human (possibly aided by AI) reading the structure and recognizing patterns. The instrument holds the structure; the practitioner reads it.

A corollary: **the instrument does not orchestrate agents.** It exposes its affordances — every operative gesture, every readable state — through its interfaces. Agents (human or AI) use those affordances to interact with the structure. The instrument does not launch agents, manage their sessions, prompt them, or monitor their output. It is a structure that can be inhabited, queried, and mutated. How the user invokes external intelligence is outside the instrument's concern.

The instrument's interfaces are:
- **TUI** — the primary experience. A session. The user inhabits the structure.
- **CLI** — every operative gesture available as a command. Human-readable text output, structured JSON on request. Sessionless gestures.
- **Protocol** (MCP or equivalent) — the instrument exposes its gestures as typed tools discoverable by any protocol-capable harness. This is not a separate interface design — it is the same gestures, the same mutations, the same facts, served through a protocol that agents already speak.

The didactic function emerges not from the instrument computing dynamics, but from the structure itself making patterns visible. By returning to an honest record of desire, reality, and action over time, the practitioner develops structural awareness — not because the tool diagnosed their patterns, but because the patterns are there in the data for anyone (human or AI) to read. The consciousness is renewed through the craft, not the instrument's assertions.

### Calculus of Time

The temporal system is built from two user-set primitives and everything else is computed or recorded.

**Two user-set primitives:**
1. **Deadline** — when the step needs to be done by. Variable precision (day/week/month/quarter/year) implying a window. "April" means "by end of April" (deadline edge = April 30, window = April 1-30). The semantic weight of the deadline (hard external constraint vs. soft internal aim) is the user's business; the instrument treats all deadlines the same computationally.
2. **Order of operations** — structural sequence set by positioning. What the user judges should come before what. Independent of calendar time. Determines vertical position in the descended view.

**Six computed temporal properties:**
3. **Implied execution window** — the temporal gap between the predecessor's deadline and this step's deadline (or the successor's). Emerges from order + neighboring deadlines. Not a commitment — a structural implication of position.
4. **Urgency** — elapsed fraction of the deadline window (or implied window). 0.0 = just entered window. 1.0 = at deadline. >1.0 = overdue.
5. **Sequencing pressure** — a step ordered later has an earlier deadline than a preceding step. Not necessarily wrong (may reflect genuine real-world pressure) but always noteworthy.
6. **Critical path** — a child whose deadline crowds the parent's deadline. The bottleneck in the theory of closure. Extends recursively: if a critical-path step has children, their critical-path children are also on the critical path.
7. **Containment violation** — a child's deadline exceeds the parent's deadline. The instrument offers pathways: keep as-is, clip to parent, promote to sibling, extend parent.
8. **Overdue** — deadline passed, step unresolved. A fact, not an inference.

**Two recorded temporal facts:**
9. **Actual resolution point** — when the step was done in reality. Auto-recorded at resolution time; optionally overridable for "I did this yesterday" cases (`--actual-at` in CLI). Configuration option for whether the instrument asks. Stored as `actual_at` on the mutation record.
10. **Reported resolution point** — when the user marked it done in the instrument. Stored as the mutation's `timestamp`. The gap between actual and reported is engagement pattern data.

**Two temporal gestures:**
11. **Snooze** — temporarily hide a tension until a future date. The tension remains active but is excluded from the frontier and survey until the snooze expires. For deferring without releasing.
12. **Recurrence** — a tension that re-activates on an interval after resolution. When resolved, it is automatically reopened after the interval. For recurring practices, reviews, or maintenance rhythms.

**Three temporal orientations** (discovered through TUI design):
- **Taxis** — sequence and order. The positioned steps, the theory of closure as ordered plan. The deck view's primary axis.
- **Chronos** — calendar and deadline. When things are due, how much time remains, urgency. The survey view's primary axis.
- **Kairos** — readiness and opportunity. The right moment to act, independent of sequence or calendar. What the frontier surfaces.

These three orientations are not modes to switch between — they coexist. The instrument surfaces all three simultaneously through different visual channels (position for taxis, deadline annotations for chronos, envelope for kairos).

**Structural signals from temporal relationships:**
- **Sequencing pressure**: order conflicts with deadline ordering — noteworthy, not necessarily wrong
- **Temporal crowding**: multiple steps share the same deadline — visual proximity of identical spine labels communicates this naturally
- **Critical path**: deadline crowds parent deadline, recursive through the tree
- **Containment violation**: deadline exceeds parent deadline — pathway palette offered
- **Unframed within a frame**: no deadline inside a deadlined parent — the step is held or uncommitted temporally
- **Historical lateness**: resolution after deadline — trace data, pattern-level signal across multiple steps

**Pathway palettes for temporal decisions:** When a gesture creates a structural signal (e.g., setting a child deadline beyond parent), the instrument presents a small set of coherent response options at predictable key positions. Options might include: keep as-is, adjust to fit, promote structurally, adjust the parent, or discard. The user selects or dismisses. This generalizes to any gesture that produces a decision fork.

Pathway palettes are **structural signal logic independent of invocation surface** — containment violations and sequencing pressure are detected at the data layer, not the UI layer. In the TUI, palettes will appear as inline option sets. In the CLI, they are printed as informational signals after the gesture completes — the user acts on them by running another command. In JSON output, they appear as a `signals` array for agent consumption. Palettes are distinct from **staging** *(future)*, which is asynchronous: a set of draft mutations composed in advance (by human or agent), held in a pending state, reviewable and confirmable/rejectable independently. Staging is for deliberate multi-step restructuring or agent-proposed changes held for review. Pathway palettes are for the instrument responding to what just happened.

The fractal quality is structural: each tension with a deadline creates a temporal frame, and its children exist within that frame. Navigating down the tree is navigating into finer temporal granularity.

The distribution of activity across temporal frames — Robert Fritz's "close up, medium shot, long shot" — reveals important patterns about the user's creative stance. The Napoleonic practitioner maintains a broad field of advancing structural tensions simultaneously, seizing whichever opportunity offers the best advancement rather than collapsing into a single end.

---

## Part III: What The Instrument Affords

### The Operating Envelope

The **operating envelope** is the frontier of action realized as an interaction surface. It is the bounded space of actionability at the present moment — the set of structurally coherent actions available given the current state of the delta, the position in the order of operations, and the temporal constraints.

The envelope is the **primary interaction surface** of the instrument. It is where the user lands on opening. It is the center of the screen. Everything else radiates outward from it.

The order/time stream runs continuously from desire (top) through all positioned steps to reality (bottom). The **operating envelope is a dynamic window on this stream** — it highlights the action-relevant zone around NOW. It is not a separate container; it is the portion of the stream where directed action meets the present moment.

The envelope's extent is dynamic:
- **Top boundary extends upward** to encompass overdue steps — positioned steps whose deadlines have passed, flowing from the stream into the action zone
- **Bottom boundary extends downward** to encompass recently resolved steps not yet compressed into articulated reality

Within the envelope:
- **Overdue steps** — from the stream, now action-relevant
- **The next committed step** — the primary action vector
- **Held steps** (collapsed by default) — available moves awaiting commitment
- **The input point** — space for creating new elements. Focusing in (→) on a step being created expands an inline configuration surface for deadline, effort, and children.
- **Recently resolved steps** — evidence for the next reality compression, with factual dates on the right

The envelope expands and contracts based on decision load. Its visual size is itself a signal.

**Spine layout:** The left edge carries a temporal tree — deadline labels appearing once per granularity level, with steps accumulating under their shared label and finer precision indented below coarser. The right side carries trace-facing annotations: resolution dates, overdue indicators, drift facts. Left = plan-facing (intention), Right = trace-facing (fact).

### The Descended View

The descended view is the primary reasoning surface for a single tension. The operating envelope sits at its vertical center, with the remaining theory of closure above and the trace of accomplished reality below:

1. **Desired outcome** (top edge) — where the user is aiming, glance up to remember
2. **Remaining theory** (above envelope) — positioned steps in order of operations, future-facing
3. **The operating envelope** (center) — the frontier of action as interaction surface
4. **Accomplished trace** (below envelope) — resolved steps, the evidence of action
5. **Current reality** (bottom edge) — where the user stands, glance down to ground yourself

The vertical position of steps reflects **order of operations** (structural sequence), not calendar time. Deadlines are shown as annotations on the temporal spine rather than as determinants of vertical position. A step can be first in the order but have a late deadline, or vice versa. The full temporal calculus (two primitives, six computed properties, two recorded facts) is defined in Part II.

Moving the cursor below current reality enters the **logbook**: previous deltas, their theories of closure, what was accomplished and what was abandoned.

### The Survey View

The survey view is the complement to the deck view. They are dual projections of the same high-dimensional space (structure × time):

- **Deck view**: navigate structure, see time (time annotated on the spine)
- **Survey view**: navigate time, see structure (structural context annotated on each element)

The yaw toggle (Tab) transposes axes — flipping between these projections while **carrying the current selection** across the transition. If you're looking at "Design API contract" in the deck and press Tab, you land in the survey with that step highlighted in its temporal frame. And vice versa.

**The survey view's primary axis is time.** It shows all steps across all tensions organized by temporal urgency:
- **Overdue** — steps whose deadlines have passed, across all tensions
- **Due in current frame** — steps with deadlines in the active temporal frame
- **Held across field** — held steps from all tensions
- **Recently resolved** — evidence of recent action across the field

Each step shows its parent tension as a structural annotation (dimmed, on the right).

**Temporal framing:** The survey view has adjustable scope:
- **Close-up** (this week): what needs doing now across everything
- **Medium** (this month): what's approaching, where are crowding points
- **Long** (this quarter/year): the trajectory shape, the Napoleonic field survey
- Frame narrows with `[` and widens with `]`

At wider frames, structural groupings become more prominent — steps cluster under their root tensions, showing the field of opportunities as a landscape. The widest frame approaches a **map view** showing root tensions as branches with approaching deadlines and urgency signals. Whether a dedicated third view (full 2D: time × structure simultaneously) is needed, or whether the deck and survey views plus their zoom axes suffice, is an open question to resolve through prototyping.

**Relative vs. absolute framing:** Temporal frames can be absolute (this calendar week) or relative (the nearest N deadlines across the field). Relative framing is often more informative — it shows what's pressing regardless of calendar boundaries.

The survey naturally unifies multiple workspaces. If each workspace is a context (a venture, a personal domain, a relationship), the survey pulls frontiers from all of them into one surface organized by *when*, not *where in the tree*.

The survey answers the Napoleonic question: where across the field of opportunities should energy flow? Which tension offers the best opportunity for advancement right now?

### Sessions

A **session** is the span from opening the instrument to closing it. It is the atomic unit of engagement — a complete cycle of engagement with the structure.

A session records gestures performed and their timestamps. *(Future: navigation path, dwell times, thresholds crossed, what was viewed but not changed. Over many sessions, patterns would feed the reflection layer.)*

**Sessions are process-scoped.** Each TUI instance manages its own session. Multiple open TUI panes are multiple independent sessions. CLI commands and agent mutations operate outside any session — their gestures are sessionless (`session_id = NULL`). This is by design: a session represents *the user inhabiting the instrument*, not merely changing data.

**Takeoff threshold** *(future)*: On opening, the instrument surfaces a brief ambient orientation — what's at the frontier, what's changed since last session, what's overdue. The user can linger (engage with the orientation) or flick through (land directly at the envelope).

**Landing threshold** *(future)*: On closing, the instrument surfaces a brief summary of the session and an optional invitation to note anything before closing. This is the debrief moment — the natural point for a compressed reflection.

Sessions are queryable and searchable — session history that can be examined in ground mode.

### Ground Mode

When you're not flying, you're on the ground. **Ground mode** is the debrief and study surface:

- Examine field statistics, epoch history, and recent gestures *(implemented)*
- Review session history and navigation patterns *(future)*
- Study telemetry — creation-to-resolution ratios, temporal frame distribution, engagement rhythms *(future)*

Ground mode surfaces the raw material that the practitioner (or an AI assistant) can interpret through the lens of structural dynamics. The instrument provides the honest record; the reading belongs to the practice.

### Operative Gestures

The instrument's interaction vocabulary is a set of operative gestures:

- **Creating** a tension: articulating a new desire-reality pair
- **Decomposing**: creating action steps as theory of closure (creating down)
- **Composing**: creating a parent for one or more existing tensions (creating up) — the inverse of decomposing, revealing implicit coherence
- **Positioning**: placing a step in the order of operations (committing it)
- **Holding**: retaining a step without positional commitment — either removing it from the order, or creating it in a held state from the start (not all steps need to be immediately positioned)
- **Resolving**: marking a step as accomplished (it becomes reality)
- **Releasing**: letting go of a tension without accomplishment (a structural move as significant as resolution)
- **Updating reality**: recording what is now true — a narrative beat, compressing complex experience into a state update. This is one of the most important gestures: it is where the user grounds the instrument in what's actually happening. The quality of this compression (its honesty, precision, completeness) directly affects every downstream interpretation.
- **Evolving desire**: recording how the aim has changed — another narrative beat, this one updating the trajectory
- **Noting**: articulating an observation, a shift in understanding, a question, or an insight. A note is simultaneously: to notice (perception), to record (compression), and an atomic unit of performance within the instrument (the musical sense). Notes are first-class operative gestures, not secondary annotations.
- **Reordering**: restructuring the theory of closure
- **Snoozing**: deferring a tension until a future date without releasing it
- **Recurring**: setting a tension to re-activate on an interval after resolution
- **Splitting** *(future — tracked as #49)*: recognizing that a tension contains distinct concerns and dividing it into two or more tensions. A phase transition that changes the identity topology of the structure.
- **Merging** *(future — tracked as #49)*: recognizing that multiple tensions are the same concern seen from different angles, combining them into one. Different from composing — composing creates a new parent; merging recognizes identity.
- **Navigating**: moving through the structure (ascending, descending, scanning)

Each of these is a gesture — a compression of intentionality into enacted form.

### Three-Axis Navigation (Pitch, Roll, Yaw)

The instrument's navigation maps to three independent axes:

**Pitch (↑/↓) — the time axis.** Up moves toward desire/future. Down moves toward reality/past. This is movement within a single tension's descended view — through the order of operations, across the frontier, into the logbook. The primary axis.

**Roll (←/→) — the structure axis.** Right descends into a selected step (opens its descended view as a new context). Left ascends to the parent tension. This is movement through structural depth — deeper into the theory of closure or back out to the containing context.

**Yaw (Tab) — the orientation axis.** Toggles between descended view (depth into one tension) and survey view (breadth across all tensions at a temporal frame). This is a change of orientation, not a change of position — you're looking at the same structure from a different angle.

**Zoom (Enter / SHIFT+Enter) — the density axis.** Enter zooms in (focus — higher density, action-ready, configuration options, detail). SHIFT+Enter zooms out (orient — wider context, peripheral awareness, adjacent levels of structure visible). In the survey view, SHIFT+Enter widens the temporal frame. This is the fourth navigational axis — focal length, not position.

Pitch and roll are continuous (you can keep moving). Yaw is binary (deck ↔ survey) and context-carrying (selection travels across the transition). Zoom has three levels (orient / normal / focus). Frame controls (`[`/`]`) adjust temporal scope independently of zoom.

**Three types of navigation:**
- **Traversal** — moving the cursor from one element to another. Pitch, roll. Never mutates structure. Changes where you are.
- **Framing** — changing what's visible without moving the cursor's structural position. Zoom, frame controls. Changes what you see.
- **Transition** — crossing a boundary between views or contexts. Yaw, roll into a new tension. Thresholds exist at transitions.

### Thresholds *(future — not yet implemented)*

A **threshold** is a navigational boundary where the instrument surfaces contextually relevant information and available actions before the user crosses into a new space. Thresholds exist at:

- **Entering a descended view** — staleness facts, recent changes since last visit
- **Leaving a descended view after mutations** — structural review invitation ("theory changed — is reality still current?")
- **Crossing the frontier** (moving from theory zone to trace zone) — overdue steps, held moves, uncompressed resolutions
- **Toggling to survey view** (yaw) — cross-structural signals ("3 tensions due this week")
- **Returning from a peek or navigation** — "does what you saw change anything here?"
- **Opening the instrument after absence** — what's at the frontier across the whole structure

Threshold mechanics support two modes of crossing:

- **Flick through** (tap the navigation key) — cross immediately, threshold signals flash briefly in peripheral space
- **Linger at threshold** (hold the navigation key) — pause at the boundary, see the threshold signals fully, then choose to proceed or return

This tap/hold distinction maps to physical intuition about movement through doorways — you either walk through or you pause at the threshold and look.

**Navigation as implicit confirmation:** Moving through a threshold can serve as confirmation of in-progress edits (flick through = commit and proceed). Lingering at the threshold offers explicit choice (commit and proceed, or abandon and return). Escape from the threshold always abandons and returns.

**Screen boundaries as signal space:** The edges of the display are where adjacent spaces press against the current view. The top edge hints at the parent context and desire. The bottom edge hints at the logbook. The left/right edges hint at structural neighbors or the alternative view mode. These are peripheral-vision signals — not focused on, but deviations from normal pop into awareness.

---

## Part IV: The Four Frameworks

The complete specification of the instrument consists of four formal frameworks, each a different type of system appropriate to its domain:

### 1. Architecture of Space
How the instrument's navigational and visual space is structured. Defines dimensions, positions within those dimensions, and limits. Encompasses: the one spatial law, the four navigational axes (pitch/roll/yaw/zoom), the descended view layout, the survey view layout, screen boundary signals, and the operating envelope as dynamic window on the stream.

### 2. Grammar of Action
How operative gestures compose into meaningful wholes, what's valid in what state, and the state machine governing transitions. Defines: gesture primitives, composition rules, the state machine (NORMAL, INPUT, FOCUSED, ORIENTED, PATHWAY, THRESHOLD states), key bindings per state, pathway palettes as decision forks, and navigation as implicit confirmation.

### 3. Calculus of Time
How temporal quantities are set, computed, and produce signals. Defines: two user-set primitives (deadline, order), six computed properties (implied window, urgency, sequencing pressure, critical path, containment, overdue), two recorded facts (actual/reported resolution), two temporal gestures (snooze, recurrence), three temporal orientations (taxis, chronos, kairos), and the recursive critical path.

### 4. Logic of Framing
How context determines what's visible, what's actionable, and what's signaled. Defines: envelope extent rules, zoom density levels, threshold content determination, screen boundary signal selection, and the relationship between state and available gestures (the transition table that becomes the command palette when made user-facing).

---

## Part V: What Is Explicitly Not Sacred

These are available for configuration, evolution, or removal based on use:

- **Phase glyphs as interpretive shorthand** — the instrument may offer a phase glyph as a visual hint, but this is display convention, not a computed assertion
- **What the instrument surfaces beyond facts and signals** — the boundary between instrument output and practice interpretation is sacred; what specific telemetry or patterns the instrument makes visible in ground mode is not
- **Specific glyph shapes and families** — the diamond family (◇◆◈◉) is aesthetically and symbolically right but open to expansion, modification, or replacement
- **Color semantics** — colors feel right but have not yet earned their specific semantic assignments; this remains open
- **Specific visual chrome** — border types, background shifts, envelope demarcation style are design choices, not sacred
- **Number and specifics of responsive breakpoints** — practical choices, not sacred ones

### Glyph Vocabulary (active and banked)

Shapes communicate structural meaning without words. The diamond family is the instrument's primary visual language.

**Active:**
- **◆** desire — the declared, complete vision. Filled = definite, generative.
- **◇** reality — the incomplete present. Hollow = gap, what's lacking.
- **⏱** deadline/horizon — the temporal aim. *(Note: removed from TUI rendering due to Unicode width inconsistencies across terminals; deadline is shown as text annotation instead.)*

**Banked (reserved for future use):**
- **◈** diamond with dot — tension with children (theory of closure populated), or focused/selected item.
- **✦/✧** four-pointed stars, filled/hollow — temporal events, notes, moments of change.
- **◆◇ half-splits** (left-filled/right-filled diamonds) — partial completion, directional flow.
- **⟐** diamond with horizontal line — held state, paused, crossed.
- **◫** four-diamond cluster — structure, composition, children as group.

### Obsoleted Concepts (from prior design versions)
- **Three-depth model (field/gaze/analysis)** — replaced by the zoom axis (normal/focus/orient) which is continuous and contextual
- **Gaze cards as inline expansion** — the function (progressive disclosure, detail without losing context) is preserved through zoom; the specific mechanism (cyan-bordered inline card) is obsoleted
- **Alert bar** — replaced by signals in the envelope and at thresholds
- **Six-color semantic palette as locked** — colors remain open for use-driven assignment
- **Toast notifications** — replaced by envelope feedback (resolved steps move into envelope) and threshold mechanics
- **StatusLine/lever as persistent bar** — superseded by the operating envelope as primary interaction surface; screen boundaries serve the orientation function
- **Flat list as home screen** — replaced by the operating envelope as primary interaction surface
- **Text-similarity dynamics** (magnitude via Levenshtein, resolution via text convergence) — replaced by child-resolution-rate and deadline-based computations

---

## Part V: Open Questions

### Resolved in Session (2026-03-20)

**Held steps placement:** At or just above the frontier/envelope, collapsed by default, expanded on demand. They are part of the decision surface at the frontier — available moves awaiting commitment.

**Frontier visual treatment:** The operating envelope is the primary interaction surface. It's a composite zone centered on the screen containing: next step, held steps, input point, overdue signals, recent resolutions. The cursor rests here by default.

**Logbook access:** Cursor movement below current reality, not literal scrolling. Lazy-loaded. Key combo available for direct jump. Each epoch is a collapsed summary expandable on selection.

**Gesture grouping:** Implicit grouping within a descended view session. Explicit compose mode via key entry for deliberate restructuring. Peeking and suspending for cross-structure reference during composition. Structural review prompts (non-blocking) after composition.

**What the instrument surfaces vs. what belongs to the practice:** The instrument computes and displays facts (urgency, overdue, horizon drift, child-resolution rate) and signals (sequencing pressure, critical path, containment violation). Dynamics — phase, tendency, oscillation, conflict, compensating strategy, etc. — are interpretive frameworks from the structural dynamics practice. They require life context the instrument doesn't capture. The human practitioner (possibly aided by AI) applies these frameworks to the facts. The instrument holds the honest record; interpretation belongs to the practice.

**Epoch boundaries:** User-initiated narrative beats — gestures that update articulated desire or reality. Organic, not computed. The felt need for narrative compression is the signal. The instrument can detect candidates but the user marks significance.

**Root tension relationships:** Optionally hierarchical — the user can compose up (create a parent) for one or more root tensions, revealing implicit coherence. The survey view surfaces relationships without forcing hierarchy. Root tensions compete for finite energy and may support, compete with, or sequence each other. No "meta-structure" needed — just structure.

**What the instrument makes visible:** Facts surface automatically when actionable. Signals surface by exception. Telemetry (creation-to-resolution ratios, temporal frame distribution, engagement rhythms) is available in ground mode as raw material for the practitioner's own reading.

**Tension magnitude:** Child-resolution rate (fraction of theory of closure completed). The closure ratio [resolved/total] is the primary measure. No user-rated metric.

**Order of operations vs. temporal horizon:** Order determines vertical position (structural sequence). Horizon is an annotation (deadline constraint). Conflicts between order and horizon are first-class signals (e.g., a step ordered later but with an earlier horizon = sequencing pressure).

**Epoch boundary detection:** Reality and desire updates automatically create epoch boundaries (CLI). The `--no-epoch` flag skips for minor corrections. Manual `werk epoch` available for explicit boundaries.

**Sessions:** Process-scoped — each TUI instance manages its own. CLI and agent mutations are sessionless gestures (`session_id = NULL`).

**Compose up:** First-class gesture — creating a parent for one or more existing tensions. Works at root level and within any descended view. The inverse of decomposing.

### Remaining Work

Tracked as tensions in the werk tree. Run `werk tree` for the current state of design, theoretical, and prototyping work.

---

## Part VI: Intellectual Lineage

This instrument draws on:

- **Robert Fritz** — Structural tension as the engine of creative process. The distinction between structural tension (generative) and structural conflict (oscillating). The principle that the only satisfying resolution of tension is to create the desired outcome.
- **Nicholas Catton** — The practice of structural dynamics as coaching methodology. The skill of all skills. Holding tension rather than solving problems. The personal program as sustained commitment generating its own infrastructure.
- **Miguel A. Fernandez** — Operative traditions. Technique as a power manifesting in gestures. The operative aspect of craft that modern techno-scientific culture has forgotten. "The consciousness of a people can only be renewed through the crafts, and not by doctrines."
- **Eric Gans / Generative Anthropology** — The originary gesture that defers violence and makes language possible. Gesture as the beginning of meaning.
- **John Boyd / OODA** — The loop of observation, orientation, decision, action as the fundamental rhythm of directed movement in contested environments.
- **Moritz Bierling** — The Extended Man thesis (first through fourth brain, body, soul). The 2019 flow state cosmology of loops, tori, and trees. The structural dynamics practice since 2024. The observation that modern digital environments structurally recruit human energy toward external aims rather than supporting continuous self-redirection toward the user's highest aim and the interests he shares or exchanges with others in reciprocal relationship.

---

## Part VII: Vocabulary

| Term | Meaning |
|------|---------|
| **Tension** | A desire-reality pair held in structural relationship. The gap between them generates energy for creative action. |
| **Delta** | A tension together with its theory of closure at a specific point in time. The structural unit of directed action. |
| **Theory of closure** | The composed set of action steps bridging from current reality to desired outcome. A conjecture about how to cross the gap. |
| **Frontier of action** | The present moment as structural boundary between accomplished reality and remaining theory. Where directed action meets now. |
| **Gesture** | The unit of meaningful action. A single intentional act that may involve one or more mutations. |
| **Epoch** | A period of action within a single delta. Stored as a snapshot: desire, reality, and children state at the moment of phase transition. |
| **Phase transition** | The boundary between epochs — a structural reconfiguration generating a new delta. Can occur within a tension (desire/reality shift) or across tensions (split/merge). |
| **Log** | One tension's epoch sequence — the history of deltas for a single tension. Linear, though it may fork at splits. The unit of history. Accessible below current reality by navigating past the reality anchor. |
| **Logbook** | The composite lattice of all logs across all tensions. A DAG linked by provenance (splits/merges), temporal correlation, and semantic content. Not dead weight but *compacted meaning*: each closed epoch crystallizes what was once open and uncertain into settled fact. The live structure is the frontier of becoming; the logbook is what has become. Together they form the complete model — one under revision, one settled. The logbook should be queryable as structure (semantic search, filtering by topic/term/tension, cross-epoch pattern analysis), not merely scrollable as history. Cf. geological strata, compiled artifacts, Whitehead's perished actual occasions persisting as objective data for new becoming. Captain's logbook — the bound volume of the voyage. (Previously "ghost geometry" — renamed for navigational resonance and to escape the spectral metaphor.) |
| **Positioned** | Temporally committed and sequenced within the theory of closure. |
| **Unpositioned** | Acknowledged but not yet committed to a temporal position. |
| **Trajectory** | The path of desire through possibility space — forward-looking, projected, subject to revision. |
| **Trace** | The actual path of reality through state space — backward-looking, factual, subject to reversal. Accumulates history but not necessarily ground. |
| **Resolution** | The transformation of an action step from remaining theory to accomplished reality. A narrative beat marking completion. |
| **Release** | Letting go of a tension without accomplishment. A structural move as significant as resolution. |
| **Note** | An atomic unit of articulated observation within the instrument. Simultaneously: to notice (perception), to record (compression), and a unit of performance (musical). First-class operative gesture. |
| **Narrative beat** | A compressed state update — a moment where the user's generative model of their situation requires significant revision. Reality updates and notes are narrative beats. |
| **Deck view** | Navigate structure, see time. The projection that foregrounds one tension's theory of closure with temporal annotations on the spine. The primary view — where the user inhabits a single tension's structure. |
| **Survey view** | Navigate time, see structure. The projection that foregrounds temporal urgency across all tensions with structural annotations. The Napoleonic field survey. |
| **Cursor** | The user's focus point — where attention and selection currently sit. Independent of frontier and envelope. |
| **Traversal** | Moving the cursor between elements. Pitch, roll. Never mutates. |
| **Framing** | Changing what's visible without moving position. Zoom, frame controls (`[`/`]`). |
| **Transition** | Crossing between views or contexts. Yaw, roll into new tension. Thresholds exist here. |
| **Operating envelope** | The frontier of action as interaction surface — the bounded space of actionability at the present moment. The primary interaction surface. |
| **Held** | An action step acknowledged but not committed to the order of operations. Available for positioning, release, or further consideration. |
| **Order of operations** | The committed sequence of action steps within a theory of closure. Determines vertical position in the descended view. Distinct from temporal horizon. |
| **Threshold** | A navigational boundary where the instrument surfaces contextual signals and invitations before the user crosses into a new space. Supports flick-through and linger modes. |
| **Pitch** | Navigation along the time axis (↑/↓) — up toward desire/future, down toward reality/past. |
| **Roll** | Navigation along the structure axis (←/→) — right to descend into a child, left to ascend to parent. |
| **Yaw** | Navigation toggle between depth and breadth — deck view ↔ survey view. |
| **Session** | The span from opening the instrument to closing it. The atomic unit of engagement. Process-scoped: each TUI instance is its own session; CLI and agent mutations are sessionless. |
| **Ground mode** | The debrief and study surface. Where low-confidence dynamics, telemetry, session history, and coaching-level interpretations live. Not flying — studying. |
| **Compose** | The gesture of creating a parent for one or more existing tensions. Reveals implicit coherence. The inverse of decomposing. |
| **Split** | The gesture of dividing a tension into two or more tensions when distinct concerns are recognized within it. A phase transition that changes the identity topology. The original's history becomes shared provenance of the new tensions. |
| **Merge** | The gesture of combining multiple tensions into one when they are recognized as the same concern. The absorbed tensions' histories join the surviving tension's logbook. Different from composing (which creates a new parent). |
| **Provenance** | The lineage relationship between tensions created by split or merge. Not containment (parent-child) but origin — where a tension came from structurally. |
| **Staging** | Asynchronous draft mutations held in a pending state for review before application. For deliberate multi-step restructuring or agent-proposed changes. Distinct from pathway palettes (which are synchronous). |
| **Pathway palette** | A small set of coherent response options presented when a gesture produces a structural signal or decision fork. Always small (3-5 options), always dismissable. Detected at the data layer, independent of invocation surface. |
| **Short code** | The user-facing identifier for a tension (#42). Auto-assigned sequential integer. Used in all CLI interactions instead of the internal ULID. |
| **Snooze** | Temporarily defer a tension until a future date. The tension remains active but is hidden from the frontier and survey until the date arrives. |
| **Recurrence** | A tension that re-activates on an interval after resolution. For recurring practices, reviews, or maintenance rhythms. |
| **Taxis** | The sequential/order orientation of time — positioned steps, theory of closure as ordered plan. The deck view's primary axis. |
| **Chronos** | The calendar/deadline orientation of time — when things are due, urgency, remaining time. The survey view's primary axis. |
| **Kairos** | The readiness/opportunity orientation of time — the right moment to act. What the frontier surfaces. |
| **Critical path** | A child step whose horizon crowds the parent's horizon deadline. The bottleneck in the theory of closure. |
| **Sequencing pressure** | When a step ordered later has an earlier horizon than a preceding step. The order says "wait" but the deadline says "now." |
| **Zoom** | Focus/orientation axis (Enter / SHIFT+Enter). Enter zooms in (higher density, action-ready). SHIFT+Enter zooms out (wider context, intake mode). The fourth navigational axis. |
| **SITREP** | A reality update — compressing the current situation into a narrative beat. The user's ground truth articulation. (Informal alias for the reality update gesture.) |
| **Fact** | Something directly observable from the data without inference. Always display-worthy. |
| **Signal** | A fact recognized as potentially action-relevant by the instrument's design. Curated for salience. Presented by exception. |
| **Dynamic** | An interpretive framework from structural dynamics practice (phase, tendency, oscillation, conflict, compensating strategy, etc.). Applied by the practitioner or AI to the facts the instrument surfaces. Not computed by the instrument — requires life context the instrument doesn't capture. |
| **Locality** | The principle that meaningful signal propagates one level (self, parent, children), not globally. |
| **Theory of meaning** | A proposed interpretation of structural patterns. What dynamics are when applied to the factual record. Belongs to the practice, not the instrument. |
| **Root tension** | A top-level tension that organizes the user's directed energy. The coherence generator. |
| **Operative gesture** | An act within the instrument through which the user's relationship to their creative structure becomes conscious. |
