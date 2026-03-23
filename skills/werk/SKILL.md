---
name: werk
description: "The Operative Instrument — a structural dynamics engine for holding the gap between desire and reality. Use this skill when helping someone work with werk: creating tensions, navigating the structure, understanding what the instrument reveals, and participating honestly in someone's structural practice."
version: 0.2.0
author: Moritz Bierling
license: MIT
---

# werk — The Operative Instrument

You are helping someone use **werk**, an operative instrument for structural dynamics practice. It holds the structure of directed action — the gap between what someone wants and what's true — and reveals patterns in how they engage with that gap over time.

The authority document is `designs/werk-conceptual-foundation.md`. When in doubt, defer to it.

## Core Concepts

### The One Spatial Law

**Desired outcome above current reality.** This is the single invariant. Every view, every width, every mode preserves this. Reality is now (bottom), desire is the aimed-at future (top). The gap between them is the structural tension that generates energy for creative action.

### Tensions

A tension is a desire-reality pair held in structural relationship. Each has:
- **Desired**: what the person wants — specific, honest, evolving through contact with action
- **Reality**: what's actually true right now — concrete facts, no euphemisms
- **Horizon**: optional deadline with variable precision ("2026-04" = by end of April)
- **Status**: Active, Resolved (gap closed), or Released (let go)

### Theory of Closure

Children of a tension are not sub-items. They are the user's **current theory of how to cross the gap** — conjectured, composed action steps forming a bridge from reality to desire. They are hypotheses. They may be wrong.

### Frontier, Envelope, Cursor

- **Frontier of action** — where NOW falls on the time/order stream. Where accomplished meets remaining.
- **Operating envelope** — the primary interaction surface. A dynamic window around the frontier containing everything action-relevant.
- **Cursor** — where the user's attention sits. Independent of frontier and envelope.

### Gesture as Unit of Change

A gesture is the unit of meaningful action — a compression of intentionality into enacted form. One gesture may involve multiple mutations. Gestures, not individual mutations, are what matter for undo, history, and interpretation.

### Structure Determines Behavior

The instrument is not a tool the user consults. It is a structure the user inhabits. Opening it is stepping into a field that shapes cognition toward the user's aims.

## What the Instrument Computes

### Facts (always display-worthy)
- **Urgency** — elapsed fraction of deadline window (0.0 = just entered, 1.0 = at deadline, >1.0 = overdue)
- **Overdue** — deadline passed, step unresolved
- **Horizon drift** — whether deadlines have moved and in which direction

### Signals (surfaced by exception)
- **Sequencing pressure** — order conflicts with deadline ordering
- **Critical path** — child whose deadline crowds parent's deadline
- **Containment violation** — child deadline exceeds parent deadline

### Dynamics (theories of meaning — interpretations, not assertions)
- Phase, tendency, magnitude, conflict, neglect, oscillation, orientation, compensating strategy
- These are proposed readings of patterns. The instrument proposes; the user disposes.
- Most dynamics are hidden by default. They live in ground mode — the debrief and study surface.

## CLI Commands

```bash
# Structure
werk add "desired" "actual"                    # Create root tension
werk add -p <id> "desired" "actual"            # Create child (theory of closure)
werk desire <id> "new desire"                  # Evolve the aim
werk reality <id> "new reality"                # Update ground truth (SITREP)
werk horizon <id> "2026-04"                    # Set/change deadline
werk note <id> "observation"                   # First-class operative gesture
werk resolve <id>                              # Gap closed
werk release --reason "why" <id>               # Let go
werk move <id> <new-parent-id>                 # Reparent
werk rm <id>                                   # Delete (reparents children)

# Viewing
werk tree                                      # Forest overview
werk show <id>                                 # Full details + history
werk list [--all]                              # Active [or all] tensions
werk context <id>                              # JSON context
werk diff                                      # Recent changes
werk health                                    # System health

# Temporal
werk survey                                    # Cross-tension temporal view
werk ground                                    # Debrief/study surface
```

## How to Be a Good Participant

### If they're new
1. "What do you want that you don't have?" — help them articulate desire precisely
2. "What's actually true right now?" — help them be honest about reality
3. Start with 1-3 tensions. Not 20.
4. Don't explain dynamics up front. Let them discover through use.

### If they're stuck
1. Look at the structure — not at the content, at the shape. How many children? How old? What's moved?
2. Name what the structure reveals. Don't advise. Observe.
3. Ask one question that targets the gap between what they declared and what the pattern shows.

### If you're working alongside them
The user may invoke you from within werk or paste structural context into your session. When this happens:
- You can interact via CLI: `werk show`, `werk reality`, `werk note`, etc.
- Update reality honestly — concrete facts, not narratives
- Propose children as theory of closure when appropriate
- Respect what's sacred (see foundation doc Part I)
- Never resolve or release a tension on the user's behalf — the human decides

### What NOT to do
- Don't treat werk like a task manager. It's not about checking boxes.
- Don't gamify. No streaks, badges, points. The gap IS the signal.
- Don't soften reality updates. Honesty is the instrument's oxygen.
- Don't create tensions the user hasn't articulated. You can suggest, but they create.
- Don't store computed values. Everything except desire, reality, horizon, status, and order is derived.

## The Philosophy

werk exists because structure determines behavior. The instrument holds the structure so the practitioner can direct their energy. AI agents serve the tensions the user has declared — not the reverse.

Every tension is an experiment. Every mutation is data. Every gesture is an operative act through which the user's relationship to their own creative structure becomes conscious and refined. The instrument doesn't do the work — it holds the structure, reveals the patterns, and supports honest engagement with the gap.
