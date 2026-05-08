use std::collections::HashMap;

use crate::ctx::Ctx;
use crate::error::SigilError;
use crate::expr::compile_expr;
use crate::ir::IrKind;
use crate::logic::{Logic, SeedSpec};
use crate::registry::CHANNEL_NAMES;
use crate::scope::{ResolvedScope, Scope};
use crate::sigil::Sigil;
use crate::stages::{
    AttributeGraphFeaturizer, ChannelSpec, Constellation, Encoder, EpochSeriesFeaturizer,
    Featurizer, FractalBranch, Grid, InkBrush, Layouter, MinimalLine, RadialMandala, Selector,
    ShapeByStatus, StructuralDefault, SvgRenderer, TensionListFeaturizer, TensionTreeFeaturizer,
    TomlDeclarative,
};
use rand::SeedableRng;

pub struct Engine;

pub struct CompiledLogic {
    pub logic: Logic,
    pub selector: Box<dyn Selector>,
    pub featurizer: Box<dyn Featurizer>,
    pub encoder: Box<dyn Encoder>,
    pub layouter: Box<dyn Layouter>,
    pub stylist: Box<dyn crate::stages::Stylist>,
    pub renderer: SvgRenderer,
}

impl Engine {
    pub fn compile(logic: Logic) -> Result<CompiledLogic, SigilError> {
        let selector = build_selector(&logic)?;
        let featurizer = build_featurizer(&logic)?;
        let encoder = build_encoder(&logic)?;
        let layouter = build_layouter(&logic)?;
        let stylist = build_stylist(&logic)?;
        let renderer = build_renderer(&logic)?;

        let ir_kind = featurizer.ir_kind();
        if !encoder.supports(ir_kind) {
            return Err(SigilError::IrIncompatible {
                stage: "encoder".into(),
                expected: IrKind::TensionTree,
                actual: ir_kind,
            });
        }
        if layouter.expected_ir() != ir_kind {
            return Err(SigilError::IrIncompatible {
                stage: "layouter".into(),
                expected: layouter.expected_ir(),
                actual: ir_kind,
            });
        }

        Ok(CompiledLogic {
            logic,
            selector,
            featurizer,
            encoder,
            layouter,
            stylist,
            renderer,
        })
    }

    pub fn render(scope: Scope, logic: Logic, ctx: &mut Ctx<'_>) -> Result<Sigil, SigilError> {
        Self::render_with_seed(scope, logic, ctx, None)
    }

    pub fn render_with_seed(
        scope: Scope,
        logic: Logic,
        ctx: &mut Ctx<'_>,
        seed_override: Option<u64>,
    ) -> Result<Sigil, SigilError> {
        let mut compiled = Self::compile(logic)?;
        Self::render_with_compiled(scope, &mut compiled, ctx, seed_override)
    }

    pub fn render_with_compiled(
        scope: Scope,
        compiled: &mut CompiledLogic,
        ctx: &mut Ctx<'_>,
        seed_override: Option<u64>,
    ) -> Result<Sigil, SigilError> {
        let resolved = compiled.selector.select(scope, ctx)?;
        let scope_canonical = canonical_from_resolved(&resolved);
        let seed = seed_override
            .unwrap_or_else(|| derive_seed_from_canonical(&compiled.logic, &scope_canonical));
        ctx.seed = seed;
        ctx.rng = rand_chacha::ChaChaRng::seed_from_u64(seed);
        let ir = compiled.featurizer.featurize(&resolved, ctx)?;
        let marks = compiled.encoder.encode(&ir, ctx)?;
        let layout = compiled.layouter.layout(&ir, marks, ctx)?;
        let styled = compiled.stylist.style(layout, ctx)?;
        compiled
            .renderer
            .render(&compiled.logic, &scope_canonical, seed, styled, ctx)
    }
}

fn canonical_from_resolved(resolved: &ResolvedScope) -> String {
    let mut short_lookup: std::collections::HashMap<&str, i32> = std::collections::HashMap::new();
    for tension in &resolved.tensions {
        if let Some(short) = tension.short_code {
            short_lookup.insert(tension.id.as_str(), short);
        }
    }
    match resolved.scope.kind {
        crate::scope::ScopeKind::Tension => resolved
            .scope
            .root
            .as_deref()
            .and_then(|id| short_lookup.get(id).copied())
            .map(|short| format!("#{short}"))
            .unwrap_or_else(|| resolved.scope.canonical()),
        crate::scope::ScopeKind::Subtree => {
            let depth = resolved.scope.depth.unwrap_or(1);
            let root = resolved
                .scope
                .root
                .as_deref()
                .and_then(|id| short_lookup.get(id).copied())
                .map(|short| format!("#{short}"))
                .unwrap_or_else(|| resolved.scope.root.clone().unwrap_or_default());
            format!("{root}~d{depth}")
        }
        _ => resolved.scope.canonical(),
    }
}

fn derive_seed_from_canonical(logic: &Logic, scope_canonical: &str) -> u64 {
    match logic.seed {
        SeedSpec::Fixed { value } => value,
        SeedSpec::Auto => {
            let canonical = format!("{}{}", scope_canonical, logic.canonical());
            let hash = blake3::hash(canonical.as_bytes());
            let bytes = hash.as_bytes();
            u64::from_le_bytes(bytes[0..8].try_into().unwrap())
        }
    }
}

fn build_selector(logic: &Logic) -> Result<Box<dyn Selector>, SigilError> {
    match logic.pipeline.selector.as_str() {
        "subtree" => Ok(Box::new(crate::stages::SubtreeSelector { depth: 4 })),
        "space" => Ok(Box::new(crate::stages::SpaceSelector)),
        "query" => Ok(Box::new(crate::stages::QuerySelector)),
        "union" => Ok(Box::new(crate::stages::UnionSelector)),
        other => Err(SigilError::unsupported(format!("selector {other}"))),
    }
}

fn build_featurizer(logic: &Logic) -> Result<Box<dyn Featurizer>, SigilError> {
    match logic.pipeline.featurizer.as_str() {
        "tension_tree" => {
            let attrs = parse_attributes(logic, "tension_tree");
            Ok(Box::new(TensionTreeFeaturizer { attributes: attrs }))
        }
        "tension_list" => {
            let attrs = parse_attributes(logic, "tension_list");
            Ok(Box::new(TensionListFeaturizer { attributes: attrs }))
        }
        "attribute_graph" => {
            let attrs = parse_attributes(logic, "attribute_graph");
            Ok(Box::new(AttributeGraphFeaturizer { attributes: attrs }))
        }
        "epoch_series" => Ok(Box::new(EpochSeriesFeaturizer)),
        other => Err(SigilError::unsupported(format!("featurizer {other}"))),
    }
}

fn build_encoder(logic: &Logic) -> Result<Box<dyn Encoder>, SigilError> {
    match logic.pipeline.encoder.as_str() {
        "structural_default" => Ok(Box::new(StructuralDefault)),
        "shape_by_status" => Ok(Box::new(ShapeByStatus)),
        "toml_declarative" => {
            let channels = parse_channels(logic)?;
            Ok(Box::new(TomlDeclarative { channels }))
        }
        other => Err(SigilError::unsupported(format!("encoder {other}"))),
    }
}

fn build_layouter(logic: &Logic) -> Result<Box<dyn Layouter>, SigilError> {
    match logic.pipeline.layouter.as_str() {
        "radial_mandala" => Ok(Box::new(RadialMandala {
            ring_step: 80.0,
            inner_padding: 0.08,
            root_radius: 12.0,
            center: (300.0, 300.0),
            parent_child_curves: true,
            ring_guides: true,
            respect_status: true,
        })),
        "fractal_branch" => Ok(Box::new(FractalBranch {
            center: (300.0, 300.0),
            branch_step: 120.0,
        })),
        "constellation" => Ok(Box::new(Constellation {
            center: (300.0, 300.0),
        })),
        "grid" => Ok(Box::new(Grid {
            center: (300.0, 300.0),
            columns: 2,
            rows: 2,
            cell_size: 200.0,
        })),
        other => Err(SigilError::unsupported(format!("layouter {other}"))),
    }
}

fn build_stylist(logic: &Logic) -> Result<Box<dyn crate::stages::Stylist>, SigilError> {
    match logic.pipeline.stylist.as_str() {
        "ink_brush" => Ok(Box::new(InkBrush {
            palette: "ink_on_cream".into(),
            background: "#f5efe1".into(),
            stroke_color: "#1a1818".into(),
            fill_color: "#1a1818".into(),
            glyph_color: "#1a1818".into(),
            filter_mode: "filter".into(),
        })),
        "minimal_line" => Ok(Box::new(MinimalLine)),
        "glyphic" => Ok(Box::new(crate::stages::Glyphic { mirror: true })),
        other => Err(SigilError::unsupported(format!("stylist {other}"))),
    }
}

fn build_renderer(_logic: &Logic) -> Result<SvgRenderer, SigilError> {
    Ok(SvgRenderer {
        viewbox: (0.0, 0.0, 600.0, 600.0),
        margin: 40.0,
        embed_metadata: true,
    })
}

fn parse_attributes(logic: &Logic, name: &str) -> Vec<String> {
    logic
        .params
        .for_stage("featurizer", name)
        .and_then(|value| value.get("attributes"))
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_else(|| {
            werk_core::ir::AttributeBuilder::registry_attribute_names()
                .iter()
                .map(|name| name.to_string())
                .collect()
        })
}

fn parse_channels(
    logic: &Logic,
) -> Result<std::collections::HashMap<String, ChannelSpec>, SigilError> {
    let Some(channel_table) = logic
        .params
        .for_stage("encoder", "channels")
        .and_then(|value| value.as_table())
    else {
        return Ok(HashMap::new());
    };
    let allowed: std::collections::HashSet<&str> = CHANNEL_NAMES.iter().copied().collect();
    let mut channels = HashMap::new();
    for (name, value) in channel_table {
        if !allowed.contains(name.as_str()) {
            return Err(SigilError::UnknownChannel { name: name.clone() });
        }
        let table = value
            .as_table()
            .ok_or_else(|| SigilError::construction("channel spec must be table", 1, 1))?;
        if let Some(literal) = table.get("literal") {
            if let Some(num) = literal.as_float() {
                channels.insert(
                    name.clone(),
                    ChannelSpec::Literal(crate::stages::ChannelValue::Number(num)),
                );
                continue;
            }
            if let Some(text) = literal.as_str() {
                channels.insert(
                    name.clone(),
                    ChannelSpec::Literal(crate::stages::ChannelValue::Text(text.to_string())),
                );
                continue;
            }
        }
        if let Some(field) = table.get("field").and_then(|v| v.as_str()) {
            let scale = table
                .get("scale")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let domain = table.get("domain").and_then(parse_range);
            let range = table.get("range").and_then(parse_range);
            if let Some(kind) = table.get("kind").and_then(|v| v.as_str()) {
                if kind == "categorical" {
                    let mapping = table
                        .get("mapping")
                        .and_then(|v| v.as_table())
                        .map(|map| {
                            map.iter()
                                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                                .collect()
                        })
                        .unwrap_or_default();
                    channels.insert(
                        name.clone(),
                        ChannelSpec::Categorical {
                            field: field.to_string(),
                            mapping,
                        },
                    );
                    continue;
                }
                if kind == "threshold" {
                    let threshold = table
                        .get("threshold")
                        .and_then(|v| v.as_float())
                        .unwrap_or(0.5);
                    let mapping = table
                        .get("mapping")
                        .and_then(|v| v.as_table())
                        .cloned()
                        .unwrap_or_default();
                    let low = mapping
                        .get("low")
                        .and_then(|v| v.as_str())
                        .unwrap_or("#1a1818")
                        .to_string();
                    let high = mapping
                        .get("high")
                        .and_then(|v| v.as_str())
                        .unwrap_or("#cc3333")
                        .to_string();
                    channels.insert(
                        name.clone(),
                        ChannelSpec::Threshold {
                            field: field.to_string(),
                            threshold,
                            low,
                            high,
                        },
                    );
                    continue;
                }
            }
            channels.insert(
                name.clone(),
                ChannelSpec::Field {
                    field: field.to_string(),
                    scale,
                    domain,
                    range,
                },
            );
            continue;
        }
        if let Some(expr) = table.get("expr").and_then(|v| v.as_str()) {
            let compiled = compile_expr(expr, 1, 1)?;
            channels.insert(name.clone(), ChannelSpec::Expr(compiled));
            continue;
        }
    }
    Ok(channels)
}

fn parse_range(value: &toml::Value) -> Option<(f64, f64)> {
    let array = value.as_array()?;
    if array.len() != 2 {
        return None;
    }
    let a = array[0].as_float()?;
    let b = array[1].as_float()?;
    Some((a, b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctx::Ctx;
    use crate::logic::{Pipeline, StageParams};
    use crate::scope::ScopeKind;
    use crate::scope::ScopeSpec;
    use crate::stages::Stylist as _;
    use crate::stages::TomlDeclarative;
    use crate::stages::{ChannelSpec, InkBrush, RadialMandala, SvgRenderer};
    use crate::stages::{Featurized, TensionTreeFeaturizer};
    use crate::toml_schema::load_preset;
    use std::collections::HashMap;
    use werk_core::ir::AttributeValue;

    use chrono::{TimeZone, Utc};
    use werk_core::store::Store;
    use werk_core::tension::TensionStatus;

    #[test]
    fn render_is_pure_and_deterministic() {
        let fixture = small_tree();
        let preset = load_preset(preset_path("contemplative")).unwrap();
        let scope = preset
            .logic
            .scope_default
            .clone()
            .into_scope(Some(fixture.root_id.clone()), None);
        let mut ctx = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
        let first = Engine::render(scope.clone(), preset.logic.clone(), &mut ctx).unwrap();
        let mut ctx2 = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
        let second = Engine::render(scope, preset.logic, &mut ctx2).unwrap();
        assert_eq!(first.svg.0, second.svg.0);
        assert!(fixture.store.list_sigils().unwrap().is_empty());
    }

    #[test]
    fn default_seed_from_canonical_inputs() {
        let fixture = small_tree();
        let preset = load_preset(preset_path("contemplative")).unwrap();
        let scope = preset
            .logic
            .scope_default
            .clone()
            .into_scope(Some(fixture.root_id.clone()), None);
        let mut ctx = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
        let first = Engine::render(scope.clone(), preset.logic.clone(), &mut ctx).unwrap();
        let mut ctx2 = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
        let second = Engine::render(scope, preset.logic.clone(), &mut ctx2).unwrap();
        assert_eq!(first.svg.0, second.svg.0);

        let other_scope = Scope {
            kind: ScopeKind::Subtree,
            root: Some("different".into()),
            depth: Some(2),
            name: None,
            status: None,
            members: Vec::new(),
            at: None,
        };
        let mut ctx3 = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
        let third = Engine::render(other_scope, preset.logic, &mut ctx3).unwrap();
        assert_ne!(first.svg.0, third.svg.0);
    }

    #[test]
    fn metadata_contains_provenance() {
        let fixture = small_tree();
        let preset = load_preset(preset_path("contemplative")).unwrap();
        let scope = preset
            .logic
            .scope_default
            .clone()
            .into_scope(Some(fixture.root_id.clone()), None);
        let mut ctx = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
        let sigil = Engine::render(scope, preset.logic, &mut ctx).unwrap();
        let svg = String::from_utf8(sigil.svg.0).unwrap();
        assert!(svg.contains("<werk-sigil>"));
        assert!(svg.contains("<scope>"));
        assert!(svg.contains("<logic>"));
        assert!(svg.contains("<seed>"));
        assert!(svg.contains("<generated>"));
        assert!(svg.contains("<warnings count="));
    }

    #[test]
    fn render_skips_missing_data_with_warning() {
        let fixture = small_tree();
        let scope = Scope {
            kind: ScopeKind::Subtree,
            root: Some(fixture.root_id.clone()),
            depth: Some(2),
            name: None,
            status: None,
            members: Vec::new(),
            at: None,
        };
        let mut ctx = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
        let resolved = crate::stages::SubtreeSelector { depth: 2 }
            .select(scope, &mut ctx)
            .unwrap();
        let featurizer = TensionTreeFeaturizer {
            attributes: vec!["status".into(), "depth".into(), "urgency".into()],
        };
        let Featurized::TensionTree(mut tree) = featurizer.featurize(&resolved, &mut ctx).unwrap()
        else {
            panic!("expected tree");
        };
        for attrs in tree.attributes.values_mut() {
            attrs.insert("urgency", AttributeValue::Number(0.5));
        }
        if let Some((_, attrs)) = tree.attributes.iter_mut().next() {
            attrs.insert("urgency", AttributeValue::Unknown);
        }
        let mut channels = HashMap::new();
        channels.insert(
            "r".into(),
            ChannelSpec::Field {
                field: "urgency".into(),
                scale: None,
                domain: None,
                range: None,
            },
        );
        let encoder = TomlDeclarative { channels };
        let marks = encoder
            .encode(&Featurized::TensionTree(tree), &mut ctx)
            .unwrap();
        let layouter = RadialMandala {
            ring_step: 80.0,
            inner_padding: 0.08,
            root_radius: 12.0,
            center: (300.0, 300.0),
            parent_child_curves: false,
            ring_guides: false,
            respect_status: false,
        };
        let layout = layouter
            .layout(
                &Featurized::TensionTree(tree_placeholder(&marks)),
                marks,
                &mut ctx,
            )
            .unwrap();
        let stylist = InkBrush {
            palette: "ink_on_cream".into(),
            background: "#f5efe1".into(),
            stroke_color: "#1a1818".into(),
            fill_color: "#1a1818".into(),
            glyph_color: "#1a1818".into(),
            filter_mode: "filter".into(),
        };
        let styled = stylist.style(layout, &mut ctx).unwrap();
        let renderer = SvgRenderer {
            viewbox: (0.0, 0.0, 600.0, 600.0),
            margin: 40.0,
            embed_metadata: true,
        };
        let sigil = renderer
            .render(&dummy_logic(), "scope", 1, styled, &ctx)
            .unwrap();
        let svg = String::from_utf8(sigil.svg.0).unwrap();
        assert!(svg.contains("warnings count=\"1\""));
    }

    #[test]
    fn explicit_seed_override_is_effective() {
        let fixture = small_tree();
        let preset = load_preset(preset_path("contemplative")).unwrap();
        let scope = preset
            .logic
            .scope_default
            .clone()
            .into_scope(Some(fixture.root_id.clone()), None);
        let mut ctx = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
        let seed7 =
            Engine::render_with_seed(scope.clone(), preset.logic.clone(), &mut ctx, Some(7))
                .unwrap();
        let mut ctx2 = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
        let seed8 =
            Engine::render_with_seed(scope.clone(), preset.logic.clone(), &mut ctx2, Some(8))
                .unwrap();
        let mut ctx3 = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
        let seed7_again =
            Engine::render_with_seed(scope, preset.logic, &mut ctx3, Some(7)).unwrap();
        assert_ne!(seed7.svg.0, seed8.svg.0);
        assert_eq!(seed7.svg.0, seed7_again.svg.0);
    }

    fn dummy_logic() -> Logic {
        Logic {
            meta: crate::logic::Meta {
                name: "dummy".into(),
                version: "1".into(),
                description: None,
                purpose: None,
            },
            scope_default: ScopeSpec {
                kind: ScopeKind::Subtree,
                root: None,
                depth: Some(2),
                name: None,
                status: None,
                members: None,
            },
            scope_fallback: ScopeSpec {
                kind: ScopeKind::Space,
                root: None,
                depth: None,
                name: Some("active".into()),
                status: None,
                members: None,
            },
            scope_at: None,
            pipeline: Pipeline {
                selector: "subtree".into(),
                featurizer: "tension_tree".into(),
                encoder: "structural_default".into(),
                layouter: "radial_mandala".into(),
                stylist: "ink_brush".into(),
                renderer: "svg".into(),
            },
            params: StageParams::empty(),
            seed: SeedSpec::Auto,
        }
    }

    fn tree_placeholder(marks: &[crate::stages::MarkSpec]) -> werk_core::ir::TensionTree {
        let forest = werk_core::tree::Forest::new();
        let mut attrs = HashMap::new();
        for mark in marks {
            let mut map = werk_core::ir::Attributes::new();
            map.insert("depth", AttributeValue::Number(0.0));
            attrs.insert(mark.id.clone(), map);
        }
        werk_core::ir::TensionTree {
            forest,
            attributes: attrs,
        }
    }

    struct Fixture {
        store: Store,
        root_id: String,
    }

    fn small_tree() -> Fixture {
        let store = Store::new_in_memory().unwrap();
        let root = store.create_tension("root", "root actual").unwrap();
        let c1 = store
            .create_tension_with_parent("child 1", "child 1 actual", Some(root.id.clone()))
            .unwrap();
        let c2 = store
            .create_tension_with_parent("child 2", "child 2 actual", Some(root.id.clone()))
            .unwrap();
        let _g1 = store
            .create_tension_with_parent("grand 1", "grand 1 actual", Some(c1.id.clone()))
            .unwrap();
        let g2 = store
            .create_tension_with_parent("grand 2", "grand 2 actual", Some(c1.id.clone()))
            .unwrap();
        let _g3 = store
            .create_tension_with_parent("grand 3", "grand 3 actual", Some(c2.id.clone()))
            .unwrap();
        let _leaf = store
            .create_tension_with_parent("leaf", "leaf actual", Some(g2.id.clone()))
            .unwrap();
        store
            .update_status(&c2.id, TensionStatus::Resolved)
            .unwrap();
        store
            .update_status(&g2.id, TensionStatus::Released)
            .unwrap();
        Fixture {
            store,
            root_id: root.id,
        }
    }

    fn fixed_now() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 8, 10, 0, 0).unwrap()
    }

    fn preset_path(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("presets/{name}.toml"))
    }
}
