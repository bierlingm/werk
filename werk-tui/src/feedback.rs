//! Implicit relevance feedback for palette action learning.
//!
//! Tracks which palette actions the practitioner selects, boosting frequently-used
//! actions in future default ordering. Boosts decay exponentially so recent usage
//! matters more than distant history.
//!
//! Uses the same JSON schema as frankensearch-fusion's FeedbackCollector for
//! forward compatibility — a snapshot with `schema_version`, `exported_elapsed_secs`,
//! and per-document boost entries.

use std::collections::HashMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};

/// Per-action boost entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoostEntry {
    pub raw_boost: f64,
    pub positive_signals: u64,
    pub negative_signals: u64,
    pub last_signal_secs: f64,
}

/// Serialized snapshot for persistence — matches frankensearch-fusion format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackSnapshot {
    pub schema_version: u32,
    pub exported_elapsed_secs: f64,
    pub boosts: HashMap<String, BoostEntry>,
}

/// Configuration for the feedback collector.
pub struct FeedbackConfig {
    pub decay_halflife_hours: f64,
    pub max_boost: f64,
    pub min_boost: f64,
    pub select_weight: f64,
}

impl Default for FeedbackConfig {
    fn default() -> Self {
        Self {
            decay_halflife_hours: 24.0,
            max_boost: 3.0,
            min_boost: 0.5,
            select_weight: 3.0,
        }
    }
}

/// Tracks action usage with exponential-decay boosts.
pub struct FeedbackCollector {
    config: FeedbackConfig,
    epoch: Instant,
    boosts: HashMap<String, BoostEntry>,
}

impl FeedbackCollector {
    pub fn new(config: FeedbackConfig) -> Self {
        Self {
            config,
            epoch: Instant::now(),
            boosts: HashMap::new(),
        }
    }

    fn elapsed_secs(&self) -> f64 {
        self.epoch.elapsed().as_secs_f64()
    }

    /// Record that an action was selected.
    pub fn record_select(&mut self, action_id: &str) {
        let now = self.elapsed_secs();
        let entry = self.boosts.entry(action_id.to_string()).or_insert(BoostEntry {
            raw_boost: 1.0,
            positive_signals: 0,
            negative_signals: 0,
            last_signal_secs: now,
        });
        entry.raw_boost = (entry.raw_boost + self.config.select_weight)
            .min(self.config.max_boost);
        entry.positive_signals += 1;
        entry.last_signal_secs = now;
    }

    /// Get the effective boost for an action, applying exponential decay.
    pub fn get_boost(&self, action_id: &str) -> f64 {
        let Some(entry) = self.boosts.get(action_id) else {
            return 1.0;
        };
        let age_hours = (self.elapsed_secs() - entry.last_signal_secs) / 3600.0;
        let decay = f64::powf(2.0, -age_hours / self.config.decay_halflife_hours);
        let boost = 1.0 + (entry.raw_boost - 1.0) * decay;
        boost.clamp(self.config.min_boost, self.config.max_boost)
    }

    /// Export boost map to JSON for persistence.
    pub fn export_boost_map(&self) -> Result<String, serde_json::Error> {
        let snapshot = FeedbackSnapshot {
            schema_version: 1,
            exported_elapsed_secs: self.elapsed_secs(),
            boosts: self.boosts.clone(),
        };
        serde_json::to_string(&snapshot)
    }

    /// Import boost map from JSON, rebasing timestamps to current epoch.
    pub fn import_boost_map(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let now_secs = self.elapsed_secs();
        let mut snapshot: FeedbackSnapshot = serde_json::from_str(json)?;
        let exported_elapsed = if snapshot.exported_elapsed_secs.is_finite()
            && snapshot.exported_elapsed_secs >= 0.0
        {
            snapshot.exported_elapsed_secs
        } else {
            0.0
        };
        // Rebase timestamps to this collector's epoch
        for entry in snapshot.boosts.values_mut() {
            let age_secs = (exported_elapsed - entry.last_signal_secs).max(0.0);
            entry.last_signal_secs = now_secs - age_secs;
        }
        // Filter non-finite entries
        snapshot.boosts.retain(|_, e| {
            e.raw_boost.is_finite() && e.last_signal_secs.is_finite()
        });
        self.boosts = snapshot.boosts;
        Ok(())
    }
}
