//! Semantic color palette for werk's CLI.
//!
//! # Design
//!
//! Seven color roles plus a bold emphasis channel. Every role means
//! exactly one thing; there is no decorative color. When the palette is
//! disabled (NO_COLOR set, non-TTY stdout, `--json` output, piped to a
//! file), every method returns the input unchanged as a `String` — the
//! output is byte-identical to plain uncolored text.
//!
//! # Usage
//!
//! ```no_run
//! use std::io::IsTerminal;
//! use werk_shared::cli_display::Palette;
//!
//! let enabled = std::io::stdout().is_terminal()
//!     && std::env::var("NO_COLOR").is_err();
//! let palette = Palette::new(enabled);
//!
//! println!("{}", palette.identity("write a novel"));
//! println!("{}", palette.chrome("(12 days remaining)"));
//! println!("{}", palette.danger("OVERDUE"));
//! ```
//!
//! # Why return `String`?
//!
//! `owo_colors` produces deeply-generic styled wrappers that are awkward
//! to pass through function boundaries. Eager rendering to `String`
//! trades a small allocation for a dramatically simpler API — render
//! functions can compose palette calls without threading generic
//! lifetimes through every helper.

use owo_colors::OwoColorize;

/// Semantic color palette. Cheap to copy; construct once per command.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    enabled: bool,
}

impl Palette {
    /// Create a palette.
    ///
    /// Pass `true` only when the caller has confirmed the output is going
    /// to a color-capable TTY. The canonical check is:
    ///
    /// ```no_run
    /// use std::io::IsTerminal;
    /// # use werk_shared::cli_display::Palette;
    /// let enabled = std::io::stdout().is_terminal()
    ///     && std::env::var("NO_COLOR").is_err();
    /// let palette = Palette::new(enabled);
    /// ```
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// A palette that never emits ANSI — useful as a default when routing
    /// through a JSON writer or a non-terminal destination.
    pub const fn disabled() -> Self {
        Self { enabled: false }
    }

    /// Whether this palette currently emits ANSI escape codes.
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// **Identity** — the desire, the primary actionable text. Rendered
    /// at default terminal weight; no transformation applied. This exists
    /// as a method (rather than a no-op) so renderers can uniformly
    /// "pass text through the palette" and remain symmetric with other
    /// roles.
    pub fn identity(&self, s: &str) -> String {
        s.to_string()
    }

    /// **Chrome** — parentheses, metadata, timestamps, structural
    /// punctuation. Anything the eye should skim past. Dimmed.
    pub fn chrome(&self, s: &str) -> String {
        if self.enabled {
            s.dimmed().to_string()
        } else {
            s.to_string()
        }
    }

    /// **Danger** — overdue, containment violation, destructive actions.
    /// Red. Use sparingly: overuse defeats signal-by-exception.
    pub fn danger(&self, s: &str) -> String {
        if self.enabled {
            s.red().to_string()
        } else {
            s.to_string()
        }
    }

    /// **Warning** — sequencing pressure, horizon drift, things to
    /// watch but not immediately actionable. Yellow.
    pub fn warning(&self, s: &str) -> String {
        if self.enabled {
            s.yellow().to_string()
        } else {
            s.to_string()
        }
    }

    /// **Structure** — hub, spine, reach, zone boundaries, arrows,
    /// critical path, section headers. Cyan.
    pub fn structure(&self, s: &str) -> String {
        if self.enabled {
            s.cyan().to_string()
        } else {
            s.to_string()
        }
    }

    /// **Resolved** — completed tensions, success glyphs, filled bar
    /// segments. Green.
    pub fn resolved(&self, s: &str) -> String {
        if self.enabled {
            s.green().to_string()
        } else {
            s.to_string()
        }
    }

    /// **Testimony** — notes, first-person statements, quoted practice
    /// content. Magenta.
    pub fn testimony(&self, s: &str) -> String {
        if self.enabled {
            s.magenta().to_string()
        } else {
            s.to_string()
        }
    }

    /// **Bold** — reserved for tension IDs and critical danger signals.
    /// Emphasis, not decoration. Using bold for "everything important"
    /// defeats its purpose.
    pub fn bold(&self, s: &str) -> String {
        if self.enabled {
            s.bold().to_string()
        } else {
            s.to_string()
        }
    }
}

impl Default for Palette {
    /// Returns a disabled palette. Safe default: never emit color unless
    /// the caller has explicitly opted in.
    fn default() -> Self {
        Self::disabled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_palette_is_identity() {
        let p = Palette::disabled();
        assert_eq!(p.identity("hello"), "hello");
        assert_eq!(p.chrome("hello"), "hello");
        assert_eq!(p.danger("hello"), "hello");
        assert_eq!(p.warning("hello"), "hello");
        assert_eq!(p.structure("hello"), "hello");
        assert_eq!(p.resolved("hello"), "hello");
        assert_eq!(p.testimony("hello"), "hello");
        assert_eq!(p.bold("hello"), "hello");
    }

    #[test]
    fn default_is_disabled() {
        assert!(!Palette::default().is_enabled());
    }

    #[test]
    fn enabled_palette_wraps_with_ansi() {
        let p = Palette::new(true);
        let colored = p.danger("oops");
        // Contains the ANSI reset sequence
        assert!(colored.contains("\u{1b}["));
        assert!(colored.contains("oops"));
    }

    #[test]
    fn identity_is_always_untouched() {
        // Identity never applies color, even when enabled.
        let p = Palette::new(true);
        assert_eq!(p.identity("write a novel"), "write a novel");
    }
}
