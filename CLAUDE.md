# werk

An operative instrument for structural dynamics practice.

## Context

Read `designs/werk-conceptual-foundation.md` for the conceptual architecture — the sacred core, the four frameworks, the vocabulary, and the design decisions. That document is the authority; everything else derives from it.

Run `cargo run --bin werk -- tree` to see the current tension structure.

Run `cargo run --bin werk -- show <id>` for details on any tension.

## The Four Frameworks

1. **Architecture of Space** — dimensions, positions, limits, the one spatial law (desired above actual)
2. **Grammar of Action** — gesture primitives, state machine, key bindings per state
3. **Calculus of Time** — two user-set primitives (deadline, order), six computed properties, two recorded facts
4. **Logic of Framing** — what's visible and actionable given context (envelope, zoom, thresholds)

## What's Sacred

- Desired outcome above current reality (the one spatial law)
- Theory of closure (children as composed bridge from reality to desire)
- Frontier of action / operating envelope as primary interaction surface
- Signal by exception (silence as default)
- Gesture as unit of change
- Locality (signal propagates one level)
- Structure determines behavior

## What's Not Sacred

Phase glyphs, color assignments, specific visual chrome, computed dynamics display, breakpoints. See the foundation document for the full list.
