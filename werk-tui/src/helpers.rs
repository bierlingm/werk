//! Shared helpers for the Operative Instrument.

/// Clear an area with a safe base style (dim fg, transparent bg).
///
/// This is the fundamental defense against the "all white" rendering glitch.
/// Cell::default() has fg=WHITE and bg=TRANSPARENT. Any cell left in that
/// state will be emitted by the diff engine as bright white text. By filling
/// with a dim fg cell, un-written cells appear as dim instead of glaring white.
/// Using TRANSPARENT bg lets the terminal's native background show through,
/// which means this works on both dark and light terminals.
pub fn clear_area_styled(frame: &mut ftui::Frame<'_>, area: ftui::layout::Rect) {
    use crate::theme::CLR_DIM;
    let cell = ftui::Cell::from_char(' ')
        .with_fg(CLR_DIM)
        .with_bg(ftui::PackedRgba::TRANSPARENT);
    frame.buffer.fill(area, cell);
}
