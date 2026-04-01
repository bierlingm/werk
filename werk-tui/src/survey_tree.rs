//! Survey tree: collapsible temporal bands using ftui Tree widget.
//!
//! Wraps the survey data into a Tree structure where:
//! - Root (hidden): the entire survey
//! - Band nodes: one per TimeBand (overdue, imminent, approaching, later, unframed)
//! - Item nodes: survey items within each band (providers and their inheritors)
//!
//! Band expansion state is tracked here and applied during survey rendering.

use std::collections::HashSet;

use ftui::widgets::tree::{Tree, TreeGuides, TreeNode};

use crate::survey::{SurveyItem, TimeBand};

/// Manages band collapse/expand state for the survey view.
///
/// By default, overdue and imminent bands are expanded. Other bands
/// are expanded too but can be collapsed by the user.
pub struct SurveyTreeState {
    /// Collapsed bands. Bands NOT in this set are expanded.
    pub collapsed: HashSet<TimeBand>,
}

impl SurveyTreeState {
    pub fn new() -> Self {
        Self {
            collapsed: HashSet::new(),
        }
    }

    /// Toggle a band's collapsed state. Returns new expanded state.
    pub fn toggle_band(&mut self, band: TimeBand) -> bool {
        if self.collapsed.contains(&band) {
            self.collapsed.remove(&band);
            true // now expanded
        } else {
            self.collapsed.insert(band);
            false // now collapsed
        }
    }

    /// Whether a band is expanded (not collapsed).
    pub fn is_expanded(&self, band: TimeBand) -> bool {
        !self.collapsed.contains(&band)
    }

    /// Collapse a specific band.
    pub fn collapse(&mut self, band: TimeBand) {
        self.collapsed.insert(band);
    }

    /// Get all currently collapsed bands.
    pub fn collapsed_bands(&self) -> Vec<TimeBand> {
        self.collapsed.iter().copied().collect()
    }
}

/// Build a Tree widget from survey items for structural display.
///
/// The tree structure is:
///   (hidden root)
///   ├── overdue (N)
///   │   ├── tension A
///   │   └── tension B
///   ├── imminent (N)
///   │   ├── provider X
///   │   │   ├── inheritor Y
///   │   │   └── inheritor Z
///   │   └── standalone W
///   └── later (N)
///       └── ...
pub fn build_survey_tree(items: &[SurveyItem], state: &SurveyTreeState) -> Tree {
    let mut root = TreeNode::new("survey").with_expanded(true);

    // Group items by band
    let mut i = 0;
    while i < items.len() {
        let band = items[i].band;
        let band_start = i;
        while i < items.len() && items[i].band == band {
            i += 1;
        }
        let band_items = &items[band_start..i];
        let count = band_items.len();

        let label = format!("{} ({})", band.label(), count);
        let expanded = state.is_expanded(band);

        let mut band_node = TreeNode::new(label).with_expanded(expanded);

        // Add items as children
        // Items are already in tree order (from tree_order_within_bands).
        // Providers have empty tree_prefix, inheritors have non-empty prefix.
        // For Tree widget, we build flat children per band (the tree_prefix
        // is already computed for the per-line rendering).
        for item in band_items {
            let item_label = if let Some(sc) = item.short_code {
                format!("#{} {}", sc, truncate(&item.desired, 50))
            } else {
                truncate(&item.desired, 60)
            };
            band_node = band_node.child(TreeNode::new(item_label));
        }

        root = root.child(band_node);
    }

    Tree::new(root)
        .with_show_root(false)
        .with_guides(TreeGuides::Rounded)
}

/// Determine if a survey cursor position points at a band header.
/// Returns Some(band) if so.
pub fn cursor_on_band_header(
    _items: &[SurveyItem],
    cursor: usize,
    band_header_positions: &[(TimeBand, usize)],
) -> Option<TimeBand> {
    band_header_positions
        .iter()
        .find(|(_, pos)| *pos == cursor)
        .map(|(band, _)| *band)
}

fn truncate(s: &str, max: usize) -> String {
    let first_line = s.lines().next().unwrap_or(s).trim();
    if first_line.chars().count() <= max {
        first_line.to_string()
    } else {
        let t: String = first_line.chars().take(max.saturating_sub(1)).collect();
        format!("{}\u{2026}", t)
    }
}
