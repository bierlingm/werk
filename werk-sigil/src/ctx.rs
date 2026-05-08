use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use werk_core::store::Store;

#[derive(Debug, Clone, Default)]
pub struct Diagnostics {
    warnings: Arc<Mutex<Vec<String>>>,
}

impl Diagnostics {
    pub fn warn(&self, message: impl Into<String>) {
        let mut warnings = self
            .warnings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        warnings.push(message.into());
    }

    pub fn warnings(&self) -> Vec<String> {
        let warnings = self
            .warnings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        warnings.clone()
    }

    pub fn warning_count(&self) -> usize {
        let warnings = self
            .warnings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        warnings.len()
    }
}

pub struct Ctx<'a> {
    pub now: DateTime<Utc>,
    pub store: &'a Store,
    pub workspace_name: String,
    pub seed: u64,
    pub rng: ChaChaRng,
    pub diagnostics: Diagnostics,
}

impl<'a> Ctx<'a> {
    pub fn new(
        now: DateTime<Utc>,
        store: &'a Store,
        workspace_name: impl Into<String>,
        seed: u64,
    ) -> Self {
        Self {
            now,
            store,
            workspace_name: workspace_name.into(),
            seed,
            rng: ChaChaRng::seed_from_u64(seed),
            diagnostics: Diagnostics::default(),
        }
    }
}
