//! Shared helpers for the Operative Instrument.

/// Clear an area in the frame buffer by filling it with space characters.
pub fn clear_area(frame: &mut ftui::Frame<'_>, area: ftui::layout::Rect) {
    frame.buffer.fill(area, ftui::Cell::default());
}
