use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod tension_list;
pub use tension_list::{TensionList, TensionListEntry};

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
}

impl IrContext {
    pub fn new(now: DateTime<Utc>, workspace_name: impl Into<String>) -> Self {
        Self {
            now,
            workspace_name: workspace_name.into(),
        }
    }

    pub fn workspace_name(&self) -> &str {
        &self.workspace_name
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
