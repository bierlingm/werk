//! Canonical glyph registry for werk's CLI visual language.
//!
//! Every glyph in werk's terminal output should be sourced from this module.
//! Commands MUST NOT define their own inline glyph literals — doing so
//! causes drift across `tree.rs`, `show.rs`, `list.rs`, and makes it
//! impossible to evolve the vocabulary coherently.
//!
//! Glyphs are grouped into four categories:
//!
//! - **Status** — lifecycle markers (resolved, released, positioned).
//! - **Signal** — shown by exception when a tension needs attention.
//! - **Tree** — box-drawing characters for hierarchical layout.
//! - **Chart** — bar-chart and arrow characters for stats/log rendering.
//!
//! Every glyph must remain legible without color. The [`super::Palette`]
//! only *amplifies* glyphs; it must never be the sole carrier of meaning.

// ============================================================================
// Status glyphs
// ============================================================================

/// Position marker prefix: renders as `▸3` for a tension in position 3.
pub const STATUS_POSITION: &str = "\u{25b8}"; // ▸

/// Resolved tension — the tension's closure condition has been met.
pub const STATUS_RESOLVED: &str = "\u{2713}"; // ✓

/// Released tension — deliberately let go without resolution.
pub const STATUS_RELEASED: &str = "~";

// ============================================================================
// Signal glyphs (shown by exception only)
// ============================================================================

/// Critical path — zero or negative slack against a deadline.
pub const SIGNAL_CRITICAL_PATH: &str = "\u{2021}"; // ‡

/// Longest structural path — the spine of the field.
pub const SIGNAL_SPINE: &str = "\u{2503}"; // ┃

/// High centrality — structural hub that many paths pass through.
pub const SIGNAL_HUB: &str = "\u{25c9}"; // ◉

/// Wide reach — large transitive descendant count.
pub const SIGNAL_REACH: &str = "\u{25ce}"; // ◎

/// Containment violation — child deadline exceeds parent's deadline.
pub const SIGNAL_CONTAINMENT: &str = "\u{21a5}"; // ↥

/// Sequencing pressure — ordered after something not yet done.
pub const SIGNAL_SEQUENCING: &str = "\u{21c5}"; // ⇅

/// Horizon drift — repeated postponement or oscillating deadlines.
pub const SIGNAL_DRIFT: &str = "\u{219d}"; // ↝

/// Overdue marker for compact list rendering.
pub const SIGNAL_OVERDUE: &str = "!";

// ============================================================================
// Tree drawing
// ============================================================================

/// Branch connector for non-last child: `├── `.
pub const TREE_BRANCH: &str = "\u{251c}\u{2500}\u{2500} ";

/// Branch connector for the final child of a parent: `└── `.
pub const TREE_LAST: &str = "\u{2514}\u{2500}\u{2500} ";

/// Vertical continuation line, four columns wide: `│   `.
pub const TREE_VERTICAL: &str = "\u{2502}   ";

/// Horizontal line segment: `─`. Used for band separators.
pub const TREE_HORIZONTAL: &str = "\u{2500}"; // ─

/// Zone opener — marks the top-left corner of a visual container: `╭─`.
pub const TREE_ZONE_OPEN: &str = "\u{256d}\u{2500}"; // ╭─

/// Zone closer — marks the bottom-left corner of a visual container: `╰─`.
pub const TREE_ZONE_CLOSE: &str = "\u{2570}\u{2500}"; // ╰─

// ============================================================================
// Chart / arrows
// ============================================================================

/// Right arrow — sequencing, critical path, "leads to".
pub const ARROW_RIGHT: &str = "\u{2192}"; // →

/// Full block — filled bar-chart segment.
pub const BAR_FULL: &str = "\u{2588}"; // █

/// Light shade — empty bar-chart segment.
pub const BAR_EMPTY: &str = "\u{2591}"; // ░

/// Unicode horizontal ellipsis — the canonical display truncation mark
/// for new rendering paths. Older shared helpers (`util::truncate`) still
/// emit three ASCII dots for backwards compatibility with callers that
/// depend on a fixed character count; new code should prefer this.
pub const TRUNCATE_ELLIPSIS: &str = "\u{2026}"; // …
