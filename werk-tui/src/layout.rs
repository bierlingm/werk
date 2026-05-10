//! Spatial layout for the Operative Instrument.
//!
//! Three-pane model: desire (top), field (middle), reality (bottom).
//! The one spatial law — desired above actual — made literal.

use ftui::layout::{Constraint, Flex, Rect};

/// Maximum content width. Wide terminals get centered margin.
const MAX_CONTENT_WIDTH: u16 = 104;
/// Left/right edge margin so text doesn't press against terminal edges.
const EDGE_MARGIN: u16 = 1;

/// Terminal size regime drives what chrome is visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeRegime {
    /// < 80 cols or < 24 rows — gutter hidden, ages hidden, labels abbreviated.
    Compact,
    /// 80–120 cols, 24–40 rows — standard layout.
    Standard,
    /// > 120 cols or > 40 rows — wider left column, ages visible.
    Expansive,
}

impl SizeRegime {
    pub fn detect(width: u16, height: u16) -> Self {
        if width < 80 || height < 24 {
            Self::Compact
        } else if width > 120 || height > 40 {
            Self::Expansive
        } else {
            Self::Standard
        }
    }

    pub fn show_gutter(&self) -> bool {
        !matches!(self, Self::Compact)
    }

    pub fn show_ages(&self) -> bool {
        matches!(self, Self::Expansive)
    }
}

/// Computed pane rects for the three-zone spatial model.
///
/// Only the content panes — lever and hints are handled by the outer frame in view().
#[derive(Debug, Clone)]
pub struct PaneRects {
    /// Desire anchor — breadcrumb + desire text + rule. Top of the spatial axis.
    pub desire: Rect,
    /// Field of action — route, console, accumulated. The operating surface.
    pub field: Rect,
    /// Reality anchor — reality text + rule. Bottom of the spatial axis.
    pub reality: Rect,
}

/// Layout state tracked across frames.
pub struct LayoutState {
    pub regime: SizeRegime,
    /// Desire pane height override (from future drag-resize). None = auto.
    pub desire_height: Option<u16>,
    /// Reality pane height override. None = auto.
    pub reality_height: Option<u16>,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            regime: SizeRegime::Standard,
            desire_height: None,
            reality_height: None,
        }
    }
}

/// Minimum field height — below this the field is unusable.
const MIN_FIELD_HEIGHT: u16 = 4;

impl LayoutState {
    /// Recompute regime from terminal dimensions.
    pub fn update_regime(&mut self, width: u16, height: u16) {
        self.regime = SizeRegime::detect(width, height);
    }

    /// Constrain area to max content width, centered horizontally on wide terminals.
    /// Adds edge margins and top padding on tall terminals.
    pub fn content_area(&self, area: Rect) -> Rect {
        let usable_width = area.width.saturating_sub(EDGE_MARGIN * 2);
        let width = usable_width.min(MAX_CONTENT_WIDTH);
        let x_offset = if usable_width > MAX_CONTENT_WIDTH {
            EDGE_MARGIN + (usable_width - MAX_CONTENT_WIDTH) / 2
        } else {
            EDGE_MARGIN
        };
        let top_pad = if area.height > 30 { 1 } else { 0 };
        Rect::new(
            area.x + x_offset,
            area.y + top_pad,
            width,
            area.height.saturating_sub(top_pad),
        )
    }

    /// Split a content area into the three spatial panes: desire / field / reality.
    ///
    /// `desire_h` and `reality_h` are the natural content-fit heights.
    /// When there is no parent (root level), pass 0 for both — desire and reality
    /// collapse to zero height and the field gets everything.
    ///
    /// Heights are capped to ensure the field gets at least MIN_FIELD_HEIGHT lines.
    pub fn split(&self, area: Rect, desire_h: u16, reality_h: u16) -> PaneRects {
        // Apply user overrides if set (future drag-resize).
        let desire_natural = self.desire_height.unwrap_or(desire_h);
        let reality_natural = self.reality_height.unwrap_or(reality_h);

        // Cap desire/reality to ensure field gets minimum space.
        let available = area.height;
        let desire_capped =
            desire_natural.min(available.saturating_sub(MIN_FIELD_HEIGHT + reality_natural));
        let reality_capped =
            reality_natural.min(available.saturating_sub(MIN_FIELD_HEIGHT + desire_capped));

        let layout = Flex::vertical()
            .constraints([
                Constraint::Fixed(desire_capped),
                Constraint::Fill,
                Constraint::Fixed(reality_capped),
            ])
            .split(area);

        PaneRects {
            desire: layout[0],
            field: layout[1],
            reality: layout[2],
        }
    }
}
