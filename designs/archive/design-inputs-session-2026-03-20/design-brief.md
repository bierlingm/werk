# Design Brief: Werk Operative Instrument TUI Design System

You are looking at two codebases in this repository:

1. **`ftui`** (in `ftui/`) — A terminal UI framework for Rust. Explore it thoroughly to understand every widget, layout primitive, and affordance it provides.
2. **`werk-tui`** (in `werk-tui/`) — A terminal application built on ftui that implements "structural dynamics" — a system for working with the generative tension between reality (what is) and desire (what is intended).

Also explore the `werk/` directory for the core domain logic (tensions, dynamics, phases, horizons, watch system, etc.).

## Your Task

Write a **world-class holistic TUI design system document** that reimagines `werk-tui` as a premium **Operative Instrument** — a computational instrument for holding the gap between reality and desire.

## Requirements

1. **EXCLUSIVELY use ftui framework affordances.** Explore `ftui/src/` to discover every available widget and layout primitive. Your design must map every concept to concrete ftui types. No bespoke painting, no ad hoc ASCII art, no custom visual system that bypasses the framework.

2. **Premium terminal aesthetic.** The instrument should feel like a fusion of:
   - Old terminals: phosphor restraint, ruled surfaces, hard alignment, symbolic compression
   - Updated terminals: responsive grids, overlays, command surfaces, notification systems
   - Operator tools: information hierarchy that assumes taste, focus, and repeated use

3. **Directional and dimensional.** The single most important spatial concept: **reality is ground (bottom/left), desire is sky (top/right).** This law must govern every view. The vertical axis encodes the fundamental direction of structural dynamics. The user should never need to ask which way is "forward."

4. **Visual grammar through lines, badges, glyphs, dots, alerts.** Define a complete system of:
   - Rules (heavy, light, dotted) — each with distinct semantic meaning
   - Glyphs for lifecycle phases (germination, assimilation, completion, momentum, resolved, released)
   - Temporal indicators (dots for horizon/staleness)
   - Badges for compressed semantic declarations
   - Alerts as structural signals, not notifications

5. **Information architecture.** Define the depth layers (field scanning, focused gaze, full analysis) and how progressive disclosure works additively.

6. **Structural dynamics rendering.** Every dynamic (phase, tendency, magnitude, conflict, neglect, oscillation, resolution, drift, urgency) needs a canonical rendering strategy using ftui primitives.

7. **Complete widget mapping.** Provide a binding contract from every werk concept to specific ftui widgets with configuration details.

8. **Responsive doctrine.** How the system degrades from wide to narrow terminals while preserving structural meaning.

9. **Interaction patterns.** Selection, navigation, progressive disclosure, input surfaces, modes.

10. **The instrument should feel serious enough to hold a life structure without becoming solemn or heavy.** It rewards repeated use. It makes structural dynamics feel native to computation.

## Exploration Strategy

Start by deeply reading:
- `ftui/src/lib.rs` and all widget modules in `ftui/src/`
- `werk-tui/src/` for the current implementation
- `werk/src/` for the domain model (tensions, dynamics, phases, etc.)
- Any design documents in `designs/` for domain context (not as prescriptive templates)

Then write your design system as a single comprehensive markdown document. Save it as `DESIGN_SYSTEM.md` in the repository root.

## Quality Bar

The operator should feel all of this immediately:
- The tool knows what is foreground and background
- The tool knows which direction reality and desire run
- The tool makes tension, drift, collision, and progress legible without verbosity
- The tool rewards repeated use
- The tool feels like a terminal instrument whose lines, badges, glyphs, dots, alerts, and panels make structural dynamics feel native to computation
