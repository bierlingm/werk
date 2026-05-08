use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::scope::ScopeSpec;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicVersion(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub selector: String,
    pub featurizer: String,
    pub encoder: String,
    pub layouter: String,
    pub stylist: String,
    pub renderer: String,
}

#[derive(Debug, Clone)]
pub struct StageParams {
    pub selector: HashMap<String, toml::Value>,
    pub featurizer: HashMap<String, toml::Value>,
    pub encoder: HashMap<String, toml::Value>,
    pub layouter: HashMap<String, toml::Value>,
    pub stylist: HashMap<String, toml::Value>,
    pub renderer: HashMap<String, toml::Value>,
}

impl StageParams {
    pub fn empty() -> Self {
        Self {
            selector: HashMap::new(),
            featurizer: HashMap::new(),
            encoder: HashMap::new(),
            layouter: HashMap::new(),
            stylist: HashMap::new(),
            renderer: HashMap::new(),
        }
    }

    pub fn for_stage(&self, category: &str, name: &str) -> Option<&toml::Value> {
        match category {
            "selector" => self.selector.get(name),
            "featurizer" => self.featurizer.get(name),
            "encoder" => self.encoder.get(name),
            "layouter" => self.layouter.get(name),
            "stylist" => self.stylist.get(name),
            "renderer" => self.renderer.get(name),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum SeedSpec {
    Auto,
    Fixed { value: u64 },
}

#[derive(Debug, Clone)]
pub struct Logic {
    pub meta: Meta,
    pub scope_default: ScopeSpec,
    pub scope_fallback: ScopeSpec,
    pub scope_at: Option<String>,
    pub pipeline: Pipeline,
    pub params: StageParams,
    pub seed: SeedSpec,
}

impl Logic {
    pub fn id(&self) -> LogicId {
        LogicId(self.meta.name.clone())
    }

    pub fn version(&self) -> LogicVersion {
        LogicVersion(self.meta.version.clone())
    }

    pub fn canonical(&self) -> String {
        format!("{}@{}", self.meta.name, self.meta.version)
    }
}
