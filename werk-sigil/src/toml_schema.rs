use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::error::SigilError;
use crate::logic::{Logic, Meta, Pipeline, SeedSpec, StageParams};
use crate::scope::ScopeSpec;

#[derive(Debug, Clone)]
pub struct PresetSpec {
    pub logic: Logic,
}

#[derive(Debug, Deserialize)]
struct TomlScope {
    default: ScopeSpec,
    fallback: ScopeSpec,
    at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TomlRoot {
    meta: Option<Meta>,
    scope: Option<TomlScope>,
    pipeline: Option<Pipeline>,
    seed: Option<SeedSpec>,
    #[serde(flatten)]
    rest: HashMap<String, toml::Value>,
}

pub fn load_preset(path: impl AsRef<Path>) -> Result<PresetSpec, SigilError> {
    let content = fs::read_to_string(&path)
        .map_err(|e| SigilError::io(format!("failed to read preset: {e}")))?;
    load_preset_str(&content)
}

pub fn load_preset_str(content: &str) -> Result<PresetSpec, SigilError> {
    let root: TomlRoot =
        toml::from_str(content).map_err(|e| SigilError::construction(e.to_string(), 1, 1))?;

    let meta = root
        .meta
        .ok_or_else(|| SigilError::construction("missing [meta]", 1, 1))?;
    let pipeline = root
        .pipeline
        .ok_or_else(|| SigilError::construction("missing [pipeline]", 1, 1))?;
    let scope = root
        .scope
        .ok_or_else(|| SigilError::construction("missing [scope]", 1, 1))?;
    let seed = root.seed.unwrap_or(SeedSpec::Auto);

    let params = parse_stage_params(root.rest);

    Ok(PresetSpec {
        logic: Logic {
            meta,
            scope_default: scope.default,
            scope_fallback: scope.fallback,
            scope_at: scope.at,
            pipeline,
            params,
            seed,
        },
    })
}

fn parse_stage_params(rest: HashMap<String, toml::Value>) -> StageParams {
    let mut params = StageParams::empty();
    for (key, value) in rest {
        if matches!(
            key.as_str(),
            "selector" | "featurizer" | "encoder" | "layouter" | "stylist" | "renderer"
        ) && let toml::Value::Table(table) = value
        {
            for (stage, stage_value) in table {
                match key.as_str() {
                    "selector" => params.selector.insert(stage, stage_value),
                    "featurizer" => params.featurizer.insert(stage, stage_value),
                    "encoder" => params.encoder.insert(stage, stage_value),
                    "layouter" => params.layouter.insert(stage, stage_value),
                    "stylist" => params.stylist.insert(stage, stage_value),
                    "renderer" => params.renderer.insert(stage, stage_value),
                    _ => None,
                };
            }
            continue;
        }
        let parts: Vec<&str> = key.split('.').collect();
        if parts.len() != 2 {
            continue;
        }
        let category = parts[0];
        let name = parts[1];
        match category {
            "selector" => {
                params.selector.insert(name.to_string(), value);
            }
            "featurizer" => {
                params.featurizer.insert(name.to_string(), value);
            }
            "encoder" => {
                params.encoder.insert(name.to_string(), value);
            }
            "layouter" => {
                params.layouter.insert(name.to_string(), value);
            }
            "stylist" => {
                params.stylist.insert(name.to_string(), value);
            }
            "renderer" => {
                params.renderer.insert(name.to_string(), value);
            }
            _ => {}
        }
    }
    params
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Engine;
    use crate::error::SigilError;

    #[test]
    fn rejects_missing_meta() {
        let err = load_preset_str("[pipeline]\nselector = \"subtree\"\nfeaturizer = \"tension_tree\"\nencoder = \"structural_default\"\nlayouter = \"radial_mandala\"\nstylist = \"ink_brush\"\nrenderer = \"svg\"").unwrap_err();
        matches!(err, SigilError::Construction { .. });
    }

    #[test]
    fn rejects_missing_pipeline() {
        let err = load_preset_str("[meta]\nname = \"x\"\nversion = \"1\"\n[scope]\ndefault = { kind = \"subtree\", depth = 2 }\nfallback = { kind = \"space\", name = \"active\" }\n").unwrap_err();
        matches!(err, SigilError::Construction { .. });
    }

    #[test]
    fn contemplative_loads_clean() {
        let preset = load_preset(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("presets/contemplative.toml"),
        )
        .unwrap();
        let compiled = Engine::compile(preset.logic);
        assert!(compiled.is_ok());
    }

    #[test]
    fn rejects_incompatible_pipeline() {
        let preset = load_preset_str(
            r#"
[meta]
name = "bad"
version = "1"
[scope]
default = { kind = "subtree", depth = 2 }
fallback = { kind = "space", name = "active" }
[pipeline]
selector = "subtree"
featurizer = "attribute_graph"
encoder = "structural_default"
layouter = "radial_mandala"
stylist = "ink_brush"
renderer = "svg"
"#,
        )
        .unwrap();
        assert!(matches!(
            Engine::compile(preset.logic),
            Err(SigilError::IrIncompatible { .. })
        ));
    }
}
