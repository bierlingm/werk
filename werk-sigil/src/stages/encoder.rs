use std::collections::HashMap;

use crate::ctx::Ctx;
use crate::error::SigilError;
use crate::expr::CompiledExpr;
use crate::ir::IrKind;
use crate::registry::Primitive;
use crate::stages::featurizer::Featurized;
use crate::stages::{ChannelValue, MarkSpec};
use werk_core::ir::{AttributeValue, TensionTree};

pub trait Encoder {
    fn supports(&self, kind: IrKind) -> bool;
    fn encode(&self, ir: &Featurized, ctx: &mut Ctx<'_>) -> Result<Vec<MarkSpec>, SigilError>;
}

#[derive(Debug, Clone)]
pub struct StructuralDefault;

#[derive(Debug, Clone)]
pub struct ShapeByStatus;

#[derive(Debug, Clone)]
pub struct TomlDeclarative {
    pub channels: HashMap<String, ChannelSpec>,
}

#[derive(Debug, Clone)]
pub enum ChannelSpec {
    Literal(ChannelValue),
    Field {
        field: String,
        scale: Option<String>,
        domain: Option<(f64, f64)>,
        range: Option<(f64, f64)>,
    },
    Expr(CompiledExpr),
    Categorical {
        field: String,
        mapping: HashMap<String, String>,
    },
    Threshold {
        field: String,
        threshold: f64,
        low: String,
        high: String,
    },
}

impl Encoder for StructuralDefault {
    fn supports(&self, kind: IrKind) -> bool {
        matches!(kind, IrKind::TensionTree)
    }

    fn encode(&self, ir: &Featurized, ctx: &mut Ctx<'_>) -> Result<Vec<MarkSpec>, SigilError> {
        let tree = match ir {
            Featurized::TensionTree(tree) => tree,
            _ => {
                return Err(SigilError::internal(
                    "structural_default expects TensionTree",
                ));
            }
        };
        encode_tree(tree, ctx, |attrs| {
            let status = match attrs.get("status") {
                Some(AttributeValue::Categorical(value)) => value.as_str(),
                _ => "active",
            };
            let primitive = match status {
                "held" => Primitive::Ellipse,
                "resolved" => Primitive::Glyph,
                "released" => Primitive::Polygon,
                _ => Primitive::Circle,
            };

            let urgency = number_attr(attrs, "urgency").unwrap_or(0.0);
            let gap = number_attr(attrs, "gap_magnitude").unwrap_or(0.0);
            let depth = number_attr(attrs, "depth").unwrap_or(0.0);

            let stroke_width = 0.6 + urgency.max(0.0) * 2.0;
            let fill_opacity = (1.0 - depth * 0.15).clamp(0.1, 1.0);
            let scale = 1.0 + (4.0 - depth).clamp(0.0, 4.0) * 0.05;
            let mut channels = HashMap::new();
            channels.insert("stroke_width".into(), ChannelValue::Number(stroke_width));
            channels.insert("fill_opacity".into(), ChannelValue::Number(fill_opacity));
            channels.insert("stroke_opacity".into(), ChannelValue::Number(1.0));
            channels.insert("scale".into(), ChannelValue::Number(scale));
            channels.insert("r".into(), ChannelValue::Number(6.0 + gap * 6.0));
            Ok((primitive, channels))
        })
    }
}

impl Encoder for ShapeByStatus {
    fn supports(&self, kind: IrKind) -> bool {
        matches!(kind, IrKind::TensionTree)
    }

    fn encode(&self, ir: &Featurized, ctx: &mut Ctx<'_>) -> Result<Vec<MarkSpec>, SigilError> {
        let tree = match ir {
            Featurized::TensionTree(tree) => tree,
            _ => return Err(SigilError::internal("shape_by_status expects TensionTree")),
        };
        encode_tree(tree, ctx, |attrs| {
            let status = match attrs.get("status") {
                Some(AttributeValue::Categorical(value)) => value.as_str(),
                _ => "active",
            };
            let primitive = match status {
                "held" => Primitive::Ellipse,
                "resolved" => Primitive::Glyph,
                "released" => Primitive::Polygon,
                _ => Primitive::Circle,
            };
            Ok((primitive, HashMap::new()))
        })
    }
}

impl Encoder for TomlDeclarative {
    fn supports(&self, kind: IrKind) -> bool {
        matches!(kind, IrKind::TensionTree | IrKind::AttributeGraph)
    }

    fn encode(&self, ir: &Featurized, ctx: &mut Ctx<'_>) -> Result<Vec<MarkSpec>, SigilError> {
        match ir {
            Featurized::TensionTree(tree) => encode_tree(tree, ctx, |attrs| {
                apply_channel_specs(&self.channels, attrs)
            }),
            Featurized::AttributeGraph(graph) => encode_attr_map(&graph.attributes, ctx, |attrs| {
                apply_channel_specs(&self.channels, attrs)
            }),
            _ => Err(SigilError::internal(
                "toml_declarative expects TensionTree or AttributeGraph",
            )),
        }
    }
}

fn apply_channel_specs(
    specs: &HashMap<String, ChannelSpec>,
    attrs: &werk_core::ir::Attributes,
) -> Result<(Primitive, HashMap<String, ChannelValue>), SigilError> {
    let mut channels = HashMap::new();
    let mut primitive = Primitive::Circle;
    for (name, spec) in specs {
        match spec {
            ChannelSpec::Literal(value) => {
                channels.insert(name.clone(), value.clone());
            }
            ChannelSpec::Field {
                field,
                scale,
                domain,
                range,
            } => {
                let raw = number_attr(attrs, field)
                    .ok_or_else(|| SigilError::render(format!("missing attribute {field}")))?;
                let mut value = raw;
                if let Some(scale) = scale {
                    value = match scale.as_str() {
                        "sqrt" => raw.sqrt(),
                        _ => raw,
                    };
                }
                if let (Some(domain), Some(range)) = (domain, range) {
                    value = scale_linear(value, domain.0, domain.1, range.0, range.1);
                }
                channels.insert(name.clone(), ChannelValue::Number(value));
            }
            ChannelSpec::Expr(expr) => {
                let vars = attrs
                    .keys()
                    .filter_map(|key| number_attr(attrs, key).map(|value| (key.clone(), value)))
                    .collect::<Vec<_>>();
                let value = expr
                    .eval(&vars)
                    .map_err(|_| SigilError::render("expression eval error"))?;
                channels.insert(name.clone(), ChannelValue::Number(value));
            }
            ChannelSpec::Categorical { field, mapping } => {
                let value = match attrs.get(field) {
                    Some(AttributeValue::Categorical(value)) => value.as_str(),
                    Some(AttributeValue::Text(value)) => value.as_str(),
                    _ => "",
                };
                if let Some(mapped) = mapping.get(value) {
                    if name == "primitive" {
                        if let Some(mapped) = Primitive::from_str(mapped) {
                            primitive = mapped;
                        }
                    } else {
                        channels.insert(name.clone(), ChannelValue::Text(mapped.clone()));
                    }
                }
            }
            ChannelSpec::Threshold {
                field,
                threshold,
                low,
                high,
            } => {
                let value = number_attr(attrs, field)
                    .ok_or_else(|| SigilError::render(format!("missing attribute {field}")))?;
                let color = if value >= *threshold { high } else { low };
                channels.insert(name.clone(), ChannelValue::Text(color.clone()));
            }
        }
    }
    Ok((primitive, channels))
}

fn encode_attr_map<F>(
    attributes: &HashMap<String, werk_core::ir::Attributes>,
    ctx: &mut Ctx<'_>,
    mut builder: F,
) -> Result<Vec<MarkSpec>, SigilError>
where
    F: FnMut(
        &werk_core::ir::Attributes,
    ) -> Result<(Primitive, HashMap<String, ChannelValue>), SigilError>,
{
    let mut marks = Vec::new();
    for (id, attrs) in attributes.iter() {
        let label = label_from_attrs(id, attrs);
        match builder(attrs) {
            Ok((primitive, mut channels)) => {
                if let Some(short_code) = short_code_value(attrs) {
                    channels.insert("short_code".into(), ChannelValue::Number(short_code as f64));
                }
                marks.push(MarkSpec {
                    id: id.clone(),
                    primitive,
                    channels,
                });
            }
            Err(err) => {
                ctx.diagnostics.warn(format!("skipped {label}: {err}"));
            }
        }
    }
    Ok(marks)
}

fn encode_tree<F>(
    tree: &TensionTree,
    ctx: &mut Ctx<'_>,
    mut builder: F,
) -> Result<Vec<MarkSpec>, SigilError>
where
    F: FnMut(
        &werk_core::ir::Attributes,
    ) -> Result<(Primitive, HashMap<String, ChannelValue>), SigilError>,
{
    let mut marks = Vec::new();
    for (id, attrs) in tree.attributes.iter() {
        let label = label_from_attrs(id, attrs);
        match builder(attrs) {
            Ok((primitive, mut channels)) => {
                if let Some(short_code) = short_code_value(attrs) {
                    channels.insert("short_code".into(), ChannelValue::Number(short_code as f64));
                }
                marks.push(MarkSpec {
                    id: id.clone(),
                    primitive,
                    channels,
                });
            }
            Err(err) => {
                ctx.diagnostics.warn(format!("skipped {label}: {err}"));
            }
        }
    }
    Ok(marks)
}

fn number_attr(attrs: &werk_core::ir::Attributes, key: &str) -> Option<f64> {
    match attrs.get(key) {
        Some(AttributeValue::Number(value)) => Some(*value),
        Some(AttributeValue::Bool(value)) => Some(if *value { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn short_code_value(attrs: &werk_core::ir::Attributes) -> Option<i32> {
    match attrs.get("short_code") {
        Some(AttributeValue::Number(value)) => Some(*value as i32),
        _ => None,
    }
}

fn label_from_attrs(id: &str, attrs: &werk_core::ir::Attributes) -> String {
    if let Some(short_code) = short_code_value(attrs) {
        format!("#{short_code}")
    } else {
        id.to_string()
    }
}

fn scale_linear(value: f64, d0: f64, d1: f64, r0: f64, r1: f64) -> f64 {
    if (d1 - d0).abs() < f64::EPSILON {
        return r0;
    }
    let t = (value - d0) / (d1 - d0);
    r0 + (r1 - r0) * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use werk_core::store::Store;
    use werk_core::tree::Forest;

    fn sample_tree() -> TensionTree {
        let store = Store::new_in_memory().unwrap();
        let root = store.create_tension("root", "root actual").unwrap();
        let child = store
            .create_tension_with_parent("child", "child actual", Some(root.id.clone()))
            .unwrap();
        let forest = Forest::from_tensions(vec![root.clone(), child.clone()]).unwrap();
        let mut attributes = HashMap::new();
        let mut root_attrs = werk_core::ir::Attributes::new();
        root_attrs.insert("status", AttributeValue::Categorical("active".into()));
        root_attrs.insert("urgency", AttributeValue::Number(0.2));
        root_attrs.insert("gap_magnitude", AttributeValue::Number(0.1));
        root_attrs.insert("depth", AttributeValue::Number(0.0));
        root_attrs.insert("child_count", AttributeValue::Number(1.0));
        attributes.insert(root.id.clone(), root_attrs);
        let mut child_attrs = werk_core::ir::Attributes::new();
        child_attrs.insert("status", AttributeValue::Categorical("resolved".into()));
        child_attrs.insert("urgency", AttributeValue::Number(0.9));
        child_attrs.insert("gap_magnitude", AttributeValue::Number(1.0));
        child_attrs.insert("depth", AttributeValue::Number(2.0));
        child_attrs.insert("child_count", AttributeValue::Number(0.0));
        attributes.insert(child.id.clone(), child_attrs);
        TensionTree { forest, attributes }
    }

    #[test]
    fn channel_mappings() {
        let tree = sample_tree();
        let ir = Featurized::TensionTree(tree);
        let store = Store::new_in_memory().unwrap();
        let mut ctx = Ctx::new(
            Utc.with_ymd_and_hms(2026, 5, 8, 0, 0, 0).unwrap(),
            &store,
            "werk",
            0,
        );
        let encoder = StructuralDefault;
        let marks = encoder.encode(&ir, &mut ctx).unwrap();
        assert_eq!(marks.len(), 2);
        let mut by_id: HashMap<String, MarkSpec> =
            marks.into_iter().map(|m| (m.id.clone(), m)).collect();
        let root = by_id
            .remove(&tree_placeholder_id("root"))
            .unwrap_or_else(|| by_id.values().next().unwrap().clone());
        let stroke_width = channel_value(&root.channels, "stroke_width").unwrap();
        let fill_opacity = channel_value(&root.channels, "fill_opacity").unwrap();
        assert!(stroke_width >= 0.6);
        assert!(fill_opacity <= 1.0);
    }

    #[test]
    fn per_status_primitives() {
        let tree = sample_tree();
        let ir = Featurized::TensionTree(tree);
        let store = Store::new_in_memory().unwrap();
        let mut ctx = Ctx::new(Utc::now(), &store, "werk", 0);
        let encoder = ShapeByStatus;
        let marks = encoder.encode(&ir, &mut ctx).unwrap();
        let mut has_glyph = false;
        for mark in marks {
            if mark.primitive == Primitive::Glyph {
                has_glyph = true;
            }
        }
        assert!(has_glyph);
    }

    #[test]
    fn literal_channel() {
        let tree = sample_tree();
        let ir = Featurized::TensionTree(tree);
        let store = Store::new_in_memory().unwrap();
        let mut ctx = Ctx::new(Utc::now(), &store, "werk", 0);
        let mut channels = HashMap::new();
        channels.insert("r".into(), ChannelSpec::Literal(ChannelValue::Number(18.0)));
        let encoder = TomlDeclarative { channels };
        let marks = encoder.encode(&ir, &mut ctx).unwrap();
        assert!(
            marks
                .iter()
                .all(|m| channel_value(&m.channels, "r") == Some(18.0))
        );
    }

    #[test]
    fn e2_field_with_sqrt_scale() {
        let tree = sample_tree();
        let ir = Featurized::TensionTree(tree);
        let store = Store::new_in_memory().unwrap();
        let mut ctx = Ctx::new(Utc::now(), &store, "werk", 0);
        let mut channels = HashMap::new();
        channels.insert(
            "r".into(),
            ChannelSpec::Field {
                field: "urgency".into(),
                scale: Some("sqrt".into()),
                domain: Some((0.0, 1.0)),
                range: Some((4.0, 36.0)),
            },
        );
        let encoder = TomlDeclarative { channels };
        let marks = encoder.encode(&ir, &mut ctx).unwrap();
        for mark in marks {
            let value = channel_value(&mark.channels, "r").unwrap();
            assert!((4.0..=36.0).contains(&value));
        }
    }

    #[test]
    fn e3_expression_eval() {
        let tree = sample_tree();
        let ir = Featurized::TensionTree(tree);
        let store = Store::new_in_memory().unwrap();
        let mut ctx = Ctx::new(Utc::now(), &store, "werk", 0);
        let compiled = crate::expr::compile_expr(
            "sqrt(urgency + 0.1) * 26 + ln(child_count + 1) * 4 + 4",
            1,
            1,
        )
        .unwrap();
        let mut channels = HashMap::new();
        channels.insert("r".into(), ChannelSpec::Expr(compiled));
        let encoder = TomlDeclarative { channels };
        let marks = encoder.encode(&ir, &mut ctx).unwrap();
        assert!(!marks.is_empty());
    }

    #[test]
    fn categorical_mapping() {
        let tree = sample_tree();
        let ir = Featurized::TensionTree(tree);
        let store = Store::new_in_memory().unwrap();
        let mut ctx = Ctx::new(Utc::now(), &store, "werk", 0);
        let mut mapping = HashMap::new();
        mapping.insert("active".into(), "circle".into());
        mapping.insert("resolved".into(), "glyph".into());
        let mut channels = HashMap::new();
        channels.insert(
            "primitive".into(),
            ChannelSpec::Categorical {
                field: "status".into(),
                mapping,
            },
        );
        let encoder = TomlDeclarative { channels };
        let marks = encoder.encode(&ir, &mut ctx).unwrap();
        assert!(marks.iter().any(|m| m.primitive == Primitive::Glyph));
    }

    fn tree_placeholder_id(id: &str) -> String {
        id.to_string()
    }

    fn channel_value(channels: &HashMap<String, ChannelValue>, key: &str) -> Option<f64> {
        match channels.get(key) {
            Some(ChannelValue::Number(value)) => Some(*value),
            _ => None,
        }
    }
}
