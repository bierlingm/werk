//! Session telemetry — records every significant action during a TUI session.
//!
//! A ring buffer of timestamped events. Lightweight enough to stay on for every
//! session. Captures navigation, cursor moves, mode changes, reorder operations,
//! gestures, and render timing. Dumpable to file for post-mortem analysis.

use std::collections::VecDeque;
use std::fmt;
use std::time::Instant;

/// Maximum events retained in the ring buffer.
const MAX_EVENTS: usize = 2000;

/// A single telemetry event.
#[derive(Debug, Clone)]
pub struct TelemetryEvent {
    /// Monotonic timestamp (relative to session start).
    pub elapsed_ms: u64,
    /// Event category.
    pub category: Category,
    /// Human-readable description.
    pub detail: String,
}

/// Event categories for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// Session lifecycle (start, end).
    Session,
    /// Navigation (descend, ascend, cursor move).
    Nav,
    /// Mode transitions (normal, reorder, add, edit, etc.).
    Mode,
    /// Reorder-specific operations (enter, move, commit, cancel).
    Reorder,
    /// Data gestures (add, resolve, release, edit, etc.).
    Gesture,
    /// Rendering events (frame timing, plan stats).
    Render,
    /// State snapshots (positions, frontier classification).
    State,
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Category::Session => write!(f, "SESSION"),
            Category::Nav => write!(f, "NAV"),
            Category::Mode => write!(f, "MODE"),
            Category::Reorder => write!(f, "REORDER"),
            Category::Gesture => write!(f, "GESTURE"),
            Category::Render => write!(f, "RENDER"),
            Category::State => write!(f, "STATE"),
        }
    }
}

/// The session log — a timestamped ring buffer of events.
pub struct SessionLog {
    events: VecDeque<TelemetryEvent>,
    start: Instant,
    /// Total events recorded (including those that rolled off the buffer).
    total_count: usize,
    /// Store session ID for correlation with the structural session record.
    store_session_id: Option<String>,
}

impl SessionLog {
    /// Create a new session log.
    pub fn new() -> Self {
        let mut log = Self {
            events: VecDeque::with_capacity(MAX_EVENTS),
            start: Instant::now(),
            total_count: 0,
            store_session_id: None,
        };
        log.record(Category::Session, "session started");
        log
    }

    /// Set the store session ID for correlation.
    pub fn set_store_session_id(&mut self, id: String) {
        self.store_session_id = Some(id);
    }

    /// Record an event.
    pub fn record(&mut self, category: Category, detail: impl Into<String>) {
        let elapsed = self.start.elapsed().as_millis() as u64;
        let event = TelemetryEvent {
            elapsed_ms: elapsed,
            category,
            detail: detail.into(),
        };
        if self.events.len() >= MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(event);
        self.total_count += 1;
    }

    /// Record an event with formatted detail.
    pub fn record_fmt(&mut self, category: Category, args: fmt::Arguments<'_>) {
        self.record(category, args.to_string());
    }

    /// Get all events (oldest first).
    pub fn events(&self) -> &VecDeque<TelemetryEvent> {
        &self.events
    }

    /// Get recent events (last N).
    pub fn recent(&self, n: usize) -> impl Iterator<Item = &TelemetryEvent> {
        let skip = self.events.len().saturating_sub(n);
        self.events.iter().skip(skip)
    }

    /// Get events filtered by category.
    pub fn by_category(&self, category: Category) -> Vec<&TelemetryEvent> {
        self.events
            .iter()
            .filter(|e| e.category == category)
            .collect()
    }

    /// Total events recorded (including rolled-off).
    pub fn total_count(&self) -> usize {
        self.total_count
    }

    /// Format the log for file dump.
    pub fn dump(&self) -> String {
        let mut out = String::new();
        if let Some(ref sid) = self.store_session_id {
            out.push_str(&format!(
                "=== Session Log [{}] ({} events, {} total) ===\n",
                &sid[..13.min(sid.len())],
                self.events.len(),
                self.total_count
            ));
        } else {
            out.push_str(&format!(
                "=== Session Log ({} events, {} total) ===\n",
                self.events.len(),
                self.total_count
            ));
        }
        for event in &self.events {
            out.push_str(&format!(
                "{:>8}ms  {:<8} {}\n",
                event.elapsed_ms, event.category, event.detail
            ));
        }
        out
    }

    /// Dump to a file. Returns the path on success.
    pub fn dump_to_file(&self) -> Result<std::path::PathBuf, std::io::Error> {
        let dir = std::env::current_dir().unwrap_or_default().join(".werk");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("session.log");
        std::fs::write(&path, self.dump())?;
        Ok(path)
    }
}

/// Convenience macro for recording telemetry with format args.
#[macro_export]
macro_rules! tlog {
    ($log:expr, $cat:expr, $($arg:tt)*) => {
        $log.record_fmt($cat, format_args!($($arg)*))
    };
}
