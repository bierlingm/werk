# Addressing, Sessions, and the Gesture Vocabulary

**Opened:** 2026-03-28

**Status:** Exploratory. No commitments. Three intertwined design threads being held together to see what emerges.

---

## What This Document Is

Three threads surfaced simultaneously from issue discussion and have proven impossible to pull apart. Rather than force premature separation, this document holds them together as an open exploration.

The threads:
1. **Deep addressability** — making everything in the instrument referenceable
2. **Expanded gesture vocabulary** — what kinds of acts the instrument recognizes
3. **Session semantics** — what a session is, what accumulates inside it, what it means

These three together form much of the foundation for the multi-player instrument. They also bear on authority, provenance, knowledge, and memory — all of which have been raised as feature requests and all of which touch the instrument's identity.

---

## Thread 1: Deep Addressability

### The Current State

Today, the primary addressing scheme is the **short code** (`#42`) — a sequential integer identifying a tension. Within a tension, action steps have their own short codes scoped to the parent. Gestures have internal IDs (ULIDs) but are not user-addressable. Notes are sequential within a tension but not independently addressable from outside. Epochs are numbered but not externally referenceable.

### The Question

What if everything were addressable?

- A gesture: `g:01JQXYZ...` or a human-friendly scheme
- A note on a tension: `#42.n3` (third note on tension 42)
- A tension at a point in time: `#42@2026-03` (tension 42 as of March 2026)
- A tension in a specific epoch: `#42~e3` (tension 42, third epoch)
- A query result: addressable as the thing that was asked and what came back
- A session: `s:20260328-1` (first session on March 28)

Deep addressability enables:
- **Precise reference** in notes, reality updates, and cross-tension commentary ("see #18~e2 for why this changed")
- **Stable links** between the instrument's internal state and external artifacts (commits, documents, conversations)
- **Ground mode queries** that return specific structural states, not just current snapshots
- **LogBase** queries that traverse the full structural history
- **Agent-composable intelligence** — agents can reference, retrieve, and combine addressed elements into briefings, telemetry, situational reports

### Open Sub-Questions

- What is the addressing syntax? It needs to be typeable, readable, unambiguous, and composable.
- Are addresses stable across time? If a tension is renumbered (it shouldn't be, but), does the address break?
- What's the relationship between an address and a hash? A hash guarantees content-identity. An address is a locator. Do we need both? When?
- How do addresses interact with the multi-player future? Does `#42.n3` mean the same thing to different users? (It should — it's structural, not perspectival.)

---

## Thread 2: Expanded Gesture Vocabulary

### The Current State

Gestures today are mutations — acts that change the structure: creating, resolving, releasing, updating reality, evolving desire, noting, positioning, holding, reordering, snoozing, recurring. Each gesture is recorded with a timestamp and gesture ID.

### The Question

What if the gesture vocabulary extended beyond mutation?

**Queries as gestures.** A query — "show me everything related to the API redesign" or "what was the theory of closure on #18 in January?" — is an intentional act within the instrument. It reveals what the user is attending to, what they're trying to understand, what structural relationships they're exploring. If recorded as a gesture (with the query and its results), it becomes:
- Part of the session's engagement record
- Evidence of attention patterns (what the user keeps asking about)
- A retrievable artifact (the answer to a question at a point in time)
- Input to ground mode's telemetry (what are you studying?)

**Observations as gestures.** Distinct from notes (which are about a specific tension), an observation might be a field-level reading: "I notice I'm avoiding #12" or "The relationship between #5 and #18 has shifted." Currently these have no natural home — they're not about a single tension. If observations were a gesture type, they could live at the session or field level.

**Views as gestures?** This is more speculative. Does the act of looking at a tension — entering its descended view, dwelling there, examining its epochs — constitute a gesture? The TUI could record navigation as a lightweight gesture stream. This approaches telemetry territory (session dwell times, navigation paths) but there may be a meaningful distinction between the ambient telemetry of navigation and the deliberate act of *examining*.

### Implications for the Standard of Measurement

An important constraint from the sacred core: the instrument only reasons from standards the user explicitly provides. An expanded gesture vocabulary must respect this. Recording queries and observations doesn't violate it — those are user-initiated acts with explicit content. Recording navigation as gesture is more ambiguous — the user navigated, but did they intend the navigation itself to be meaningful? The line between "I looked at this" and "I deliberately examined this" matters.

### Open Sub-Questions

- Where is the line between gesture (deliberate, meaningful, recorded) and telemetry (ambient, pattern-level, possibly recorded)?
- If queries are gestures, does the instrument store query results? Results are ephemeral (they depend on the state at query time). But that's also what makes them valuable to store — they're a snapshot of what the instrument showed you at that moment.
- Does expanding the gesture vocabulary change what undo means? You can undo a mutation. Can you undo a query? (Probably not — it's not a mutation. But it could be retracted from the record if the user doesn't want it.)

---

## Thread 3: Session Semantics

### The Current State

A session is process-scoped: one TUI instance = one session. CLI and agent mutations are sessionless. Sessions record gestures and timestamps. Future plans include navigation paths, dwell times, thresholds crossed, and takeoff/landing thresholds.

### The Question

How seriously do we take the session?

**Session as engagement unit.** At minimum, a session is the span of one sitting with the instrument. It records what happened. This is what exists today.

**Session as accumulation surface.** If queries and observations become gestures, the session becomes richer — not just "what did you change?" but "what did you examine, what did you ask, what did you notice?" The session becomes a structured record of an engagement episode. Over many sessions, patterns emerge: what you keep returning to, what you avoid, how your attention distributes.

**Session as knowledge substrate.** This is where the knowledge-base question (#5) might actually be answered. If sessions accumulate structured engagement records (mutations, queries, observations, navigation), and if these records are deeply addressable, then the "knowledge" about a tension isn't stored in a parallel system — it's the accumulated trace of every session that touched it. You don't need a `memories` table. You need sessions that remember what happened in them and a query system that can traverse sessions for a given tension, topic, or time range.

**Session and the multi-player question.** When multiple people use the instrument, whose session is whose? Sessions are inherently individual — one person's engagement episode. But a shared instrument means sessions overlap: two people examining the same tension in the same hour, making competing reality assertions. This is where provenance enters — every gesture carries the session it came from, and every session carries its author. The instrument doesn't resolve disagreements (that's management); it records them honestly (that's operations).

**Session vs. agent session.** Agent mutations are currently sessionless. But an agent working with the instrument over an extended interaction — reading tensions, composing queries, making updates — is arguably having a session. Should it? What would it mean for an agent session to have the same structure as a human one? The harness session (the agent's conversation context) is external to the instrument; the instrument session is internal. Are they the same? Different? Related?

### Open Sub-Questions

- Is the session the unit where knowledge accumulates, or is it the tension? (Probably both — knowledge about a tension is distributed across every session that engaged with it.)
- What does it mean to "review a session"? Today it's a gesture list. With an expanded gesture vocabulary, it's a structured episode with mutations, queries, observations, maybe navigation. Is that enough to be useful in ground mode?
- How do sessions compose? A week of sessions. A month. The concept of "engagement rhythm" implies patterns across sessions. Does the instrument compute anything from session patterns, or is that practice-level interpretation? (The Standard of Measurement principle says: only if the user provides a standard.)

---

## The Convergence

These three threads converge on a single question: **what does the instrument remember, and how do you get at it?**

- Deep addressability says: everything has a name you can use to find it again.
- Expanded gestures say: more kinds of acts are worth remembering.
- Session semantics say: the context in which those acts occurred is itself meaningful.

Together they suggest that the knowledge-base need, the memory need, the trajectory-analysis need, and the multi-player provenance need might all be aspects of the same design problem: **a richly addressable, deeply structured record of engagement over time, queryable from any surface of the instrument.**

This is the LogBase vision extended. Not just epoch history queryable by tension — but the full engagement record, including what was asked and observed, addressable down to individual gestures, traversable by tension, time, author, session, or semantic content.

Whether this converges on a single design or reveals irreducible tensions of its own remains to be seen.

---

## Related

- **LogBase** (#89) — the searchable substrate of epoch history. A subset of what's explored here.
- **Ground mode** — the debrief surface. Would consume the richer session data explored here.
- **Authority** (#1) — provenance and multi-player identity. Sessions carry authorship.
- **Memory research** — Michael Levin's bioelectric networks, predictive coding frameworks. Memory as active structural model rather than passive storage. Documented in `designs/werk-business-foundation.md`.
- **Epochs and time spans** — existing design work on temporal addressing. This exploration extends it.
