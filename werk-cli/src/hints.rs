//! Footer hint helpers — contextual one-line nudges shown at the end
//! of reading commands.
//!
//! Hints are dim, single-line, actionable suggestions about "what to do
//! next" given what was just displayed. They're inspired by GitButler's
//! end-of-output hints. They are *suggestions*, not warnings, and must:
//!
//! - render in chrome (dim) — they should never compete with the actual
//!   output for attention,
//! - never wrap (one line, no paragraphs),
//! - not appear in JSON output,
//! - not appear when stdout is not a terminal (piped, redirected),
//! - not appear when the palette is disabled (NO_COLOR or test harness).
//!
//! The third-and-fourth conditions are folded into the
//! [`Palette::is_enabled`] check, since the palette is built from the
//! same TTY/NO_COLOR detection at command entry. Calling
//! `print_hint(&palette, ...)` is therefore safe in all contexts: it's
//! a no-op whenever the surrounding output is not headed for an
//! interactive terminal.

use werk_shared::cli_display::Palette;

/// Print a contextual footer hint, dim, on its own line.
///
/// No-op when the palette is disabled, which means no-op for JSON
/// output, NO_COLOR, piped commands, and the test harness — exactly
/// the contexts where a "hint" would be either invisible noise or a
/// test-disrupting extra line.
pub fn print_hint(palette: &Palette, hint: &str) {
    if !palette.is_enabled() {
        return;
    }
    println!();
    println!("{}", palette.chrome(&format!("hint: {}", hint)));
}
