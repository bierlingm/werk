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
#[derive(Debug, Clone)]
pub struct PaneRects {
    /// Desire anchor — breadcrumb + desire text + rule. Top of the spatial axis.
    pub desire: Rect,
    /// Field of action — route, console, accumulated. The operating surface.
    pub field: Rect,
    /// Reality anchor — reality text + rule. Bottom of the spatial axis.
    pub reality: Rect,
    /// Lever bar — 1 line status.
    pub lever: Rect,
    /// Key hints — 1 line, hidden when too short.
    pub hints: Rect,
}

impl PaneRects {
    pub fn has_hints(&self) -> bool {
        self.hints.height > 0
    }
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

impl LayoutState {
    /// Recompute regime from terminal dimensions.
    pub fn update_regime(&mut self, width: u16, height: u16) {
        self.regime = SizeRegime::detect(width, height);
    }

    /// Constrain area to max content width, centered horizontally on wide terminals.
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

    /// Split the full terminal area into pane rects.
    ///
    /// `desire_lines` and `reality_lines` are the content-fit line counts
    /// for the parent's desired/actual text. When there is no parent (root level),
    /// pass 0 for both — desire and reality panes collapse to zero height.
    pub fn split(&self, area: Rect, desire_lines: u16, reality_lines: u16) -> PaneRects {
        let show_hints = area.height >= 6;

        // Desire height: user override, or content-fit (0 = no parent = collapsed)
        let desire_h = self.desire_height.unwrap_or(desire_lines);
        let reality_h = self.reality_height.unwrap_or(reality_lines);

        let mut constraints = vec![
            Constraint::Fixed(desire_h),    // desire anchor
            Constraint::Fill,               // field (gets remaining)
            Constraint::Fixed(reality_h),   // reality anchor
            Constraint::Fixed(1),           // lever bar
        ];
        if show_hints {
            constraints.push(Constraint::Fixed(1)); // hints
        }

        let layout = Flex::vertical().constraints(constraints);
        let rects = layout.split(area);

        PaneRects {
            desire: rects[0],
            field: rects[1],
            reality: rects[2],
            lever: rects[3],
            hints: if show_hints { rects[4] } else { Rect::default() },
        }
    }
}
