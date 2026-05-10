use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod attribute_graph;
pub mod epoch_series;
pub mod tension_list;
pub mod tension_tree;
pub use attribute_graph::AttributeGraph;
pub use epoch_series::{EpochSeries, EpochSeriesScope};
pub use tension_list::{TensionList, TensionListEntry};
pub use tension_tree::TensionTree;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IrKind {
    TensionList,
    TensionTree,
    AttributeGraph,
    EpochSeries,
}

pub trait Ir {
    fn kind(&self) -> IrKind;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AttributeValue {
    Number(f64),
    Text(String),
    Bool(bool),
    Categorical(String),
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(transparent)]
pub struct Attributes(pub HashMap<String, AttributeValue>);

impl Attributes {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, key: impl Into<String>, value: AttributeValue) {
        self.0.insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> Option<&AttributeValue> {
        self.0.get(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.0.keys()
    }
}

#[derive(Debug, Clone)]
pub struct IrContext {
    pub now: DateTime<Utc>,
    pub workspace_name: String,
    pub diagnostics: Diagnostics,
}

impl IrContext {
    pub fn new(now: DateTime<Utc>, workspace_name: impl Into<String>) -> Self {
        Self {
            now,
            workspace_name: workspace_name.into(),
            diagnostics: Diagnostics::default(),
        }
    }

    pub fn workspace_name(&self) -> &str {
        &self.workspace_name
    }

    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }
}

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

#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct IrError {
    message: String,
}

impl IrError {
    pub fn unknown_attribute(name: &str) -> Self {
        Self {
            message: format!("unknown attribute: {name}"),
        }
    }

    pub fn unsupported_epoch_series_scope(scope: &str) -> Self {
        Self {
            message: format!("unsupported epoch series scope: {scope}"),
        }
    }

    pub fn invalid_epoch_snapshot(err: impl std::fmt::Display) -> Self {
        Self {
            message: format!("invalid epoch snapshot: {err}"),
        }
    }
}

impl From<crate::store::StoreError> for IrError {
    fn from(err: crate::store::StoreError) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}

impl From<crate::tree::TreeError> for IrError {
    fn from(err: crate::tree::TreeError) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}

const REGISTRY_ATTRIBUTE_NAMES: &[&str] = &[
    "id",
    "short_code",
    "space",
    "desire",
    "actual",
    "status",
    "is_held",
    "is_resolved",
    "is_released",
    "created_at",
    "updated_at",
    "deadline",
    "last_pulse_at",
    "age_seconds",
    "time_to_deadline_seconds",
    "urgency",
    "urgency_raw",
    "staleness",
    "gap_magnitude",
    "frequency_per_day",
    "frequency_trend",
    "gap_trend",
    "mutation_count",
    "is_projectable",
    "depth",
    "child_count",
    "descendant_count",
    "parent_id",
    "parent_short_code",
    "note_count",
    "has_children",
];

#[derive(Debug, Clone)]
pub struct AttributeBuilder {
    requested: Vec<String>,
}

impl AttributeBuilder {
    pub fn new<S: AsRef<str>>(requested: &[S]) -> Result<Self, IrError> {
        let allowed: HashSet<&'static str> = REGISTRY_ATTRIBUTE_NAMES.iter().copied().collect();
        let mut names = Vec::with_capacity(requested.len());
        for name in requested {
            let name_ref = name.as_ref();
            if !allowed.contains(name_ref) {
                return Err(IrError::unknown_attribute(name_ref));
            }
            names.push(name_ref.to_string());
        }
        Ok(Self { requested: names })
    }

    pub fn registry_attribute_names() -> &'static [&'static str] {
        REGISTRY_ATTRIBUTE_NAMES
    }

    pub fn requested(&self) -> &[String] {
        &self.requested
    }
}

pub(crate) struct AttributeContext<'a> {
    pub(crate) now: DateTime<Utc>,
    pub(crate) workspace_name: &'a str,
    pub(crate) forest: &'a crate::Forest,
    pub(crate) patterns: HashMap<String, crate::projection::MutationPattern>,
    pub(crate) note_counts: HashMap<String, usize>,
    pub(crate) last_mutations: HashMap<String, DateTime<Utc>>,
    pub(crate) parent_short_codes: HashMap<String, Option<i32>>,
    pub(crate) held_ids: HashSet<String>,
}

impl AttributeContext<'_> {
    pub(crate) fn last_mutation(&self, tension_id: &str, fallback: DateTime<Utc>) -> DateTime<Utc> {
        self.last_mutations
            .get(tension_id)
            .copied()
            .unwrap_or(fallback)
    }

    pub(crate) fn note_count(&self, tension_id: &str) -> usize {
        self.note_counts.get(tension_id).copied().unwrap_or(0)
    }

    pub(crate) fn pattern(&self, tension_id: &str) -> crate::projection::MutationPattern {
        self.patterns
            .get(tension_id)
            .cloned()
            .unwrap_or(crate::projection::MutationPattern {
                tension_id: tension_id.to_string(),
                mean_interval_seconds: None,
                mutation_count: 0,
                frequency_per_day: 0.0,
                frequency_trend: 0.0,
                gap_trend: 0.0,
                gap_samples: Vec::new(),
                is_projectable: false,
            })
    }
}

pub(crate) fn count_active_notes(mutations: &[crate::Mutation]) -> usize {
    let mut retracted: HashSet<String> = HashSet::new();
    for mutation in mutations {
        if mutation.field() == "note_retracted" {
            retracted.insert(mutation.new_value().to_owned());
        }
    }

    mutations
        .iter()
        .filter(|mutation| {
            mutation.field() == "note" && !retracted.contains(&mutation.timestamp().to_rfc3339())
        })
        .count()
}
