//! Virtual list for variable-height rows.
//!
//! Handles the cursor-to-rendered-line mapping when a Gaze expansion
//! inserts extra lines below one item in an otherwise uniform list.

/// A virtual list that maps logical items (tensions) to rendered lines.
pub struct VirtualList {
    /// Height of each item in rendered lines. Most are 1; the gazed item is taller.
    heights: Vec<usize>,
    /// Total number of logical items.
    pub count: usize,
    /// Current cursor position (index into logical items).
    pub cursor: usize,
    /// First rendered line visible in the viewport.
    pub scroll_offset: usize,
}

impl VirtualList {
    /// Create a new virtual list with N items, all height 1.
    pub fn new(count: usize) -> Self {
        Self {
            heights: vec![1; count],
            count,
            cursor: 0,
            scroll_offset: 0,
        }
    }

    /// Set the height of a specific item (e.g., when Gaze expands it).
    pub fn set_height(&mut self, index: usize, height: usize) {
        if index < self.heights.len() {
            self.heights[index] = height;
        }
    }

    /// Reset all heights to 1 (e.g., when Gaze closes).
    pub fn reset_heights(&mut self) {
        for h in &mut self.heights {
            *h = 1;
        }
    }

    /// Rebuild for a new item count. Resets heights and clamps cursor.
    pub fn rebuild(&mut self, count: usize) {
        self.count = count;
        self.heights = vec![1; count];
        if self.cursor >= count && count > 0 {
            self.cursor = count - 1;
        }
    }

    /// Total rendered height (sum of all item heights).
    pub fn total_height(&self) -> usize {
        self.heights.iter().sum()
    }

    /// Rendered line position of a given item index.
    pub fn item_y(&self, index: usize) -> usize {
        self.heights.iter().take(index).sum()
    }

    /// Rendered line position of the cursor.
    pub fn cursor_y(&self) -> usize {
        self.item_y(self.cursor)
    }

    /// Ensure the cursor is visible within the viewport.
    pub fn ensure_visible(&mut self, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }
        let cy = self.cursor_y();
        let cursor_height = self.heights.get(self.cursor).copied().unwrap_or(1);

        // Scroll up if cursor is above viewport
        if cy < self.scroll_offset {
            self.scroll_offset = cy;
        }
        // Scroll down if cursor bottom is below viewport
        if cy + cursor_height > self.scroll_offset + viewport_height {
            self.scroll_offset = (cy + cursor_height).saturating_sub(viewport_height);
        }
    }

    /// Move cursor up. Returns true if cursor moved.
    pub fn up(&mut self) -> bool {
        if self.cursor > 0 {
            self.cursor -= 1;
            true
        } else {
            false
        }
    }

    /// Move cursor down. Returns true if cursor moved.
    pub fn down(&mut self) -> bool {
        if self.cursor + 1 < self.count {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    /// Jump to top.
    pub fn top(&mut self) {
        self.cursor = 0;
    }

    /// Jump to bottom.
    pub fn bottom(&mut self) {
        if self.count > 0 {
            self.cursor = self.count - 1;
        }
    }
}
