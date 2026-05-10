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

    pub fn set_param_number(
        &mut self,
        category: &str,
        stage: &str,
        key: &str,
        value: f64,
    ) {
        self.set_param_value(category, stage, key, toml::Value::Float(value));
    }

    pub fn set_param_string(
        &mut self,
        category: &str,
        stage: &str,
        key: &str,
        value: &str,
    ) {
        self.set_param_value(category, stage, key, toml::Value::String(value.to_string()));
    }

    pub fn set_param_value(
        &mut self,
        category: &str,
        stage: &str,
        key: &str,
        value: toml::Value,
    ) {
        let map = match category {
            "selector" => &mut self.selector,
            "featurizer" => &mut self.featurizer,
            "encoder" => &mut self.encoder,
            "layouter" => &mut self.layouter,
            "stylist" => &mut self.stylist,
            "renderer" => &mut self.renderer,
            _ => return,
        };
        let entry = map
            .entry(stage.to_string())
            .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));
        if let toml::Value::Table(table) = entry {
            table.insert(key.to_string(), value);
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
    pub content_hash: Option<String>,
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

    pub fn version_string(&self) -> String {
        self.content_hash
            .clone()
            .unwrap_or_else(|| self.meta.version.clone())
    }

    pub fn cache_key(&self) -> String {
        format!("{}@{}", self.meta.name, self.version_string())
    }
}
