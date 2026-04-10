//! Shared visual language for werk's CLI surfaces.
//!
//! This module is the single source of truth for the terminal rendering
//! vocabulary: the [`glyphs`] registry (20 canonical characters with
//! fixed meanings) and the [`Palette`] struct (seven semantic color roles
//! plus bold emphasis).
//!
//! ## Why centralize?
//!
//! Before this module existed, the same glyphs were re-encoded inline in
//! `tree.rs`, `show.rs`, and `list.rs`; a single change required touching
//! three files, and the three files drifted. Color was ad-hoc owo_colors
//! calls only in `tree.rs`, with no consistent policy for NO_COLOR or
//! non-TTY output. The shared module enforces:
//!
//! - **Every glyph has one meaning.** `‡` is always critical-path, `◉`
//!   is always hub, `╭─` is always a zone opener. Never decorative.
//! - **Color is amplification, not encoding.** Every glyph must be
//!   interpretable without color. NO_COLOR must leave output fully
//!   readable.
//! - **Palette construction centralizes TTY detection.** Commands ask
//!   "should I emit ANSI?" exactly once, at the entry point, and pass the
//!   resulting [`Palette`] through their render pipeline.
//!
//! ## Non-goals
//!
//! This module does **not** contain layout logic, terminal width detection,
//! or text truncation. Those belong in each command's renderer (which can
//! import the constants and palette freely). The `werk-shared` crate is
//! kept pure of I/O — no `std::io::IsTerminal`, no `terminal_size`.

pub mod glyphs;
pub mod palette;

pub use palette::Palette;
