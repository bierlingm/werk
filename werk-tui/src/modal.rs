//! Modal overlay helpers — backdrop dimming and centered positioning.
//!
//! Each overlay (add, edit, confirm, note, pathway) gets wrapped in a
//! dimmed backdrop that preserves field visibility behind the modal.
//! InputMode dispatch stays unchanged — modals are visual, not architectural.

use ftui::Frame;
use ftui::layout::Rect;

use crate::theme::InstrumentStyles;

/// Dim the background behind a modal overlay.
///
/// Fills the area with the dim background color at reduced opacity,
/// making the field visible but de-emphasized behind the modal.
pub fn render_backdrop(frame: &mut Frame<'_>, area: Rect, styles: &InstrumentStyles) {
    crate::helpers::clear_area_styled(frame, area, styles.clr_dim);
}

/// Compute a centered modal rect within a parent area.
pub fn center_modal(parent: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(parent.width.saturating_sub(4));
    let h = height.min(parent.height.saturating_sub(2));
    let x = parent.x + (parent.width.saturating_sub(w)) / 2;
    let y = parent.y + (parent.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}
