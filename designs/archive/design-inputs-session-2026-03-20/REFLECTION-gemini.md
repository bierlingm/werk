# Reflection: Werk Operative Instrument Design System

This reflection captures the synthesis process, the technical and conceptual hurdles, and the recommended path forward for implementing the Operative Instrument.

## 1. What was Easy and what was Hard

### Easy
- **The Spatial Metaphor:** The "Reality is Ground, Desire is Sky" law felt immediately native to both the domain (Structural Dynamics) and the terminal's coordinate system. It provided a clear north star for every layout decision.
- **Glyph Mapping:** The progression from open (`◇`) to solid (`◆`) to textured (`◈`) symbols for the creative cycle phases felt intuitive and provided a clear visual "weight" to the lifecycle of a tension.
- **The "Three Depths" Model:** Progressive disclosure (Line → Gaze → Study) is a proven TUI pattern. Mapping this to the user's attention levels (Scanning → Focusing → Analyzing) came together naturally.

### Hard
- **Framework Specification:** Mapping concepts to *concrete* `ftui` types was challenging because the framework source was not directly available in the workspace (only as a dependency). I had to infer the widget API from its usage in `werk-tui/src/render.rs`.
- **Visualizing Abstraction:** Some structural dynamics—like "Drift" or "Coherence"—are highly abstract. Finding a way to render them that wasn't just "text with a color" required careful thought to ensure they felt "native to computation" rather than just a dashboard label.
- **Balancing Chrome and Content:** The mandate for a "Premium Aesthetic" often pulls towards adding more visual flourishes (borders, headers, icons), while the "Operative Instrument" mandate pulls towards extreme restraint. Finding the point where the tool feels "serious" but not "solemn" was a delicate balance.

## 2. Questions without Clear Answers (Ambiguities)

- **`ftui` Layout Fluidity:** It wasn't clear if `ftui`'s `List` widget natively supports variable-height items or smooth expansion for the "Gaze" view. The existence of `VirtualList` in `werk-tui` suggests that expansion logic currently sits outside the core framework, which creates a maintenance burden.
- **Agent Integration Depth:** The "Agent Session" is described as a modal transformation. It’s unclear how `ftui` handles complex focus management and state isolation for such a session, or if the "Proposals" (JSON to Widget) require a bespoke rendering engine.
- **Domain Thresholds:** While the visual cues for "Stagnation" or "Conflict" are defined, the exact numerical thresholds (in `sd-core`) that trigger these visuals are still parameters that need tuning to ensure the "mirror" isn't too noisy or too quiet.

## 3. Core Decisions Hierarchy & Order

To implement this design system, decisions should be made in this order:

1.  **The Layout Architecture (Base):** Lock down the `ftui::layout` constraints that enforce the vertical "Desire/Reality" vector. This is the foundation for all views.
2.  **The Expansion Mechanism (Interaction):** Decide if `Gaze` expansion belongs in the framework (`ftui::widgets::List`) or the application (`VirtualList`). This affects every scrolling and navigation gesture.
3.  **The Symbol & Theme Contract (Visual):** Standardize the `GLYPH_*` and `CLR_*` constants across the codebase to ensure consistency.
4.  **The Agent Protocol (Structural):** Define the structured communication between `sd-core` (the engine) and `ftui` (the renderer) for agent proposals. How does a JSON suggestion become an actionable TUI `Panel`?

## 4. Recommendations for Hardening

- **Widget Gallery / Storybook for `ftui`:** To truly use the framework's affordances, a visual reference of all `ftui` widgets and their configurations is needed.
- **Snapshot Testing for Dynamics:** Implement snapshot tests that render a tension in various states (High Conflict, Oscillating, Germinating). This ensures the "visual grammar" remains legible as the code evolves.
- **Performance Budget:** Enforce a "50ms to Field" startup time. An "Instrument" must be as ready as a piano; any loading lag breaks the mental model of a tool.
- **Refine the "Gaze" Content:** The current Gaze view is a draft. It needs iterative testing to ensure the "Children Summary" and "Gap Bar" provide the right amount of signal without overwhelming the user during a quick scan.
- **Domain-Specific `ftui` Widgets:** Consider building `werk`-specific widgets (like a `StructuralTensionLine` or `DynamicsGrid`) directly as `ftui` extensions to keep the `render.rs` logic clean and declarative.
