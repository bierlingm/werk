# Werk Operative Instrument: TUI Design System

This document defines the visual language, spatial logic, and structural grammar of the **Operative Instrument** — a computational mirror for holding the generative tension between reality and desire.

## I. Core Philosophy: The Instrument as Mirror

The Operative Instrument is not a dashboard; it is a mirror. It does not "display data" for passive consumption; it reflects the structure of a life for active creation.

1.  **Transparency:** The interface should disappear during use, rewarding repeated gestures until they become subconscious.
2.  **Density as Earned:** Information appears only when the user's attention reaches for it.
3.  **Sovereignty:** The instrument observes and reflects but never judges. It protects the user's agency over their own tensions.

---

## II. Spatial Law: The Vector of Desire

The single most important concept in the design system is the **Directional Law**:

> **Reality is Ground (Bottom/Left). Desire is Sky (Top/Right).**

This law governs every view and layout in the instrument. It encodes the fundamental direction of structural dynamics into the coordinate system of the terminal.

*   **Vertical Axis:** Movement from bottom to top represents the transition from what is (actual) to what is intended (desired).
*   **Horizontal Axis:** Movement from left to right represents the progression of time and the unfolding of complexity.
*   **Result:** The "forward" direction is always towards the upper-right. A tension that is "advancing" moves towards the sky.

---

## III. Visual Grammar: Primitives & Symbols

The instrument uses a fusion of old-world terminal restraint and modern responsive layout. It communicates through a strictly defined set of glyphs, lines, and badges.

### 1. Glyphs: Lifecycle Phases
Phases are the heartbeat of the creative cycle. They are rendered as distinct geometric symbols:

| Phase | Glyph | `ftui` Constant | Semantic Meaning |
| :--- | :---: | :--- | :--- |
| **Germination** | `◇` | `GLYPH_GERMINATION` | Open, new, still forming. |
| **Assimilation** | `◆` | `GLYPH_ASSIMILATION` | Solid, being worked on, actively digested. |
| **Completion** | `◈` | `GLYPH_COMPLETION` | Textured, complex, nearing resolution. |
| **Momentum** | `◉` | `GLYPH_MOMENTUM` | Full, radiating, self-sustaining. |
| **Resolved** | `✦` | `GLYPH_RESOLVED` | Achieved. The gap has closed. |
| **Released** | `·` | `GLYPH_RELEASED` | Let go. Acknowledged without closing. |

### 2. Rules: Semantic Separators
Rules define the boundaries of attention and the hierarchy of structure.

| Rule Type | Character | `ftui::widgets::borders::BorderType` | Usage |
| :--- | :---: | :--- | :--- |
| **Light** | `┄` | `BorderType::Plain` (Custom: `LIGHT_RULE`) | Separates siblings in the Field or Gaze. |
| **Medium** | `─` | `BorderType::Rounded` (Custom: `RULE`) | Separates major content blocks (e.g., Header from Field). |
| **Heavy** | `━` | `BorderType::Thick` (Custom: `HEAVY_RULE`) | Defines the "Study" boundary or Agent session. |

### 3. Temporal Indicators: The Trail
Neglect and tendency are made visible through **Trail Dots** (`●`, `○`).
*   **Active (`●`):** A mutation occurred in this time bucket (default: 1 week).
*   **Quiet (`○`):** No mutation. Attention did not touch this tension.
*   **Stale (`◎`):** Used in temporal indicators to show how long since the last reality check.

### 4. Badges: Semantic Declarations
`ftui::widgets::badge::Badge` is used for compressed metadata that demands quick scanning.
*   **Urgency:** Amber/Red badges for tensions nearing their horizon.
*   **Conflict:** Red `⚡` badges for competing structural forces.
*   **Phase:** Muted badges for explicit phase labels when in Depth 2 (Study).

---

## IV. Information Architecture: The Three Depths

Information is disclosed progressively as the user's focus narrows. This preserves the "Field" as a clean, confrontational space.

### Depth 0: Field Scanning (The Line)
**Goal:** Quick assessment of the entire structure.
*   **Widget:** `ftui::widgets::list::List`
*   **Affordances:** Single-line items containing `[Glyph] [Name] [Trail]`.
*   **Style:** Minimal. Truncation for long names. No labels.

### Depth 1: Focused Gaze (The Expansion)
**Goal:** Understanding a single tension's current state without losing context.
*   **Widget:** `ftui::widgets::panel::Panel` (inset within the list)
*   **Layout:** `ftui::layout::Grid` or `ftui::layout::Columns`.
*   **Content:**
    *   **Reality (Bottom/Left):** `ftui::widgets::paragraph::Paragraph` describing what is.
    *   **Desire (Top/Right):** `ftui::widgets::paragraph::Paragraph` describing what is intended.
    *   **Gap Bar:** Visual representation of magnitude (`ftui::widgets::sparkline::Sparkline`).
    *   **Children Summary:** Tendency counts (Advancing, Stagnant, Oscillating).

### Depth 2: Full Analysis (The Study)
**Goal:** Total structural immersion.
*   **Widget:** Full-screen `ftui::layout::Grid` with named areas.
*   **Content:** All 13 dynamics (Orientation, Coherence, Saturation, etc.) and full mutation history.
*   **Affordances:** Scrollable `ftui::widgets::paragraph::Paragraph` for history; `ftui::widgets::badge::Badge` for dynamics.

---

## V. Structural Dynamics Rendering

Every abstract dynamic has a canonical rendering strategy using `ftui` primitives:

| Dynamic | `ftui` Rendering Strategy | Visual Cue |
| :--- | :--- | :--- |
| **Phase** | `ftui::text::Span` | The primary phase glyph (`◇`, `◆`, etc.). |
| **Tendency** | `ftui::text::Span` | Arrow glyphs: `→` (Advancing), `↔` (Oscillating), `○` (Stagnant). |
| **Magnitude** | `ftui::widgets::sparkline::Sparkline` | The Gap Bar: `████░░░░`. |
| **Conflict** | `ftui::widgets::badge::Badge` | `Style::new().fg(CLR_RED)` with the `⚡` symbol. |
| **Neglect** | `ftui::text::Span` | Amber trail dots (`○○○○`) and "Stale" labels. |
| **Oscillation** | `ftui::text::Span` | `↔` glyph and "Oscillating" badge. |
| **Resolution** | `ftui::text::Span` | `✦` glyph and "Resolving" status. |
| **Urgency** | `ftui::widgets::badge::Badge` | Temporal dots with urgency-based coloring (Amber/Red). |

---

## VI. Complete Widget Mapping

| Werk Concept | `ftui` Widget / Type | Configuration Details |
| :--- | :--- | :--- |
| **The Field** | `ftui::widgets::list::List` | `ListState` for selection; `VirtualList` for variable height. |
| **The Lever** | `ftui::widgets::status_line::StatusLine` | Path breadcrumbs in `Dim` style; position on the right. |
| **Desire Field** | `ftui::widgets::paragraph::Paragraph` | Top-right alignment; `Style` with bold name. |
| **Reality Field** | `ftui::widgets::paragraph::Paragraph` | Bottom-left alignment; `Style` with dim metadata. |
| **Act Prompt** | `ftui::widgets::input::TextInput` | Inline, with focused cursor (`ftui::style::Style::new().fg(CLR_CYAN)`). |
| **Agent Card** | `ftui::widgets::panel::Panel` | `BorderType::Thick`, cyan title, containing `TextInput` or `Badge`. |
| **Tension Item** | `ftui::widgets::list::ListItem` | Custom rendering logic for Glyph + Name + Trail. |
| **Dynamics Grid** | `ftui::layout::Grid` | Responsive areas for 13 dynamics. |

---

## VII. Responsive Doctrine: Structural Degradation

The instrument must preserve structural meaning even on minimal terminals.

1.  **Width 120+ (Optimal):** Full layout. Names, trails, full dynamics labels.
2.  **Width 80-120 (Standard):** Names truncate; trails shorten to 6 dots; Gaze stacks fields vertically.
3.  **Width < 80 (Minimal):** Names only; trails hidden; Gaze shows only Desire/Reality text.
4.  **Height < 20:** Breadcrumbs hidden; Study view condensed to essential dynamics.

*Mechanism:* Use `ftui::layout::Constraint` with `Percentage` and `Min`/`Max` to ensure fluid resizing.

---

## VIII. Interaction Patterns: The Act of Gesture

All modifications are **Acts**, not edits. They are intended to be deliberate and weighty.

*   **Navigation (`j/k`, `l/h`):** Standard Vim-like movement. Navigation is **spatial descent**, not just expansion.
*   **The Gaze (`Space`):** A toggleable expansion. Uses `ftui`'s layout engine to shift siblings down.
*   **The Act (`a`, `e`, `r`, `x`):** Opens a focused `TextInput`. Context is preserved by dimming unaffected areas.
*   **The Agent (`@`):** A modal-like transformation. The tension is isolated; the `TextInput` becomes the primary focal point.
*   **Undo (`u`):** A semantic act of reversal, immediately reflected in the Field.

---

## IX. Tone & Aesthetic: Premium Instrument

*   **Palette:** Restrained. White/Default for active; Dim/Gray for stable; Amber for neglect; Red for conflict; Cyan for Agent.
*   **Rhythm:** Snappy. No animations. Instant cuts. Respect for the user's temporal flow.
*   **Density:** Generous whitespace. The left margin is sacred (column 3 start).
*   **Feeling:** Like a high-quality physical tool (a typewriter, a piano, a lathe). It feels solid under the fingers and serious enough to hold a life.
