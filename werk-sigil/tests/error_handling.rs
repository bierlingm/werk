mod fixtures;

use fixtures::small_tree::{fixed_now, small_tree};
use tempfile::TempDir;
use werk_sigil::{
    AnimationAxis, AnimationOutput, Ctx, Engine, Logic, Meta, Pipeline, SeedSpec, SigilError,
    StageParams, StageRef, SvgRenderer, Layout, MarkSpec, PlacedMark, StyledScene, Primitive,
    render_animation, load_preset, load_preset_str,
};
use std::collections::HashMap;

fn minimal_logic() -> Logic {
    Logic {
        meta: Meta {
            name: "bad".into(),
            version: "1".into(),
            description: None,
            purpose: None,
        },
        scope_default: werk_sigil::ScopeSpec {
            kind: werk_sigil::ScopeKind::Subtree,
            root: None,
            depth: Some(2),
            name: None,
            status: None,
            members: None,
        },
        scope_fallback: werk_sigil::ScopeSpec {
            kind: werk_sigil::ScopeKind::Space,
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
            encoder: "toml_declarative".into(),
            layouter: "radial_mandala".into(),
            stylist: "ink_brush".into(),
            renderer: "svg".into(),
        },
        params: StageParams::empty(),
        seed: SeedSpec::Auto,
        content_hash: None,
    }
}

#[test]
fn error_handling_schema_parse_loud() {
    let err = load_preset_str("not a valid pipeline = section").unwrap_err();
    assert!(matches!(err, SigilError::Construction { .. }));
}

#[test]
fn error_handling_expr_parse_loud() {
    let mut logic = minimal_logic();
    let mut table = toml::value::Table::new();
    table.insert(
        "r".into(),
        toml::Value::Table(
            [("expr".into(), toml::Value::String("(((".into()))]
                .into_iter()
                .collect(),
        ),
    );
    logic.params.encoder.insert("channels".into(), toml::Value::Table(table));
    let err = match Engine::compile(logic) {
        Ok(_) => panic!("expected compile error"),
        Err(err) => err,
    };
    assert!(matches!(err, SigilError::Construction { .. }));
}

#[test]
fn error_handling_unknown_channel_loud() {
    let mut logic = minimal_logic();
    let mut table = toml::value::Table::new();
    table.insert(
        "bogus".into(),
        toml::Value::Table(
            [("literal".into(), toml::Value::Float(3.0))]
                .into_iter()
                .collect(),
        ),
    );
    logic.params.encoder.insert("channels".into(), toml::Value::Table(table));
    let err = match Engine::compile(logic) {
        Ok(_) => panic!("expected compile error"),
        Err(err) => err,
    };
    assert!(matches!(err, SigilError::UnknownChannel { .. }));
}

#[test]
fn error_handling_ir_shape_mismatch_loud() {
    let mut logic = minimal_logic();
    logic.pipeline.featurizer = "attribute_graph".into();
    let err = match Engine::compile(logic) {
        Ok(_) => panic!("expected compile error"),
        Err(err) => err,
    };
    assert!(matches!(err, SigilError::IrIncompatible { .. }));
}

#[test]
fn error_handling_missing_field_graceful() {
    let fixture = small_tree();
    let mut logic = minimal_logic();
    let mut table = toml::value::Table::new();
    table.insert(
        "r".into(),
        toml::Value::Table(
            [("field".into(), toml::Value::String("urgency".into()))]
                .into_iter()
                .collect(),
        ),
    );
    logic.params.encoder.insert("channels".into(), toml::Value::Table(table));
    let mut ctx = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
    let mut compiled = Engine::compile(logic).unwrap();
    let scope = compiled.logic.scope_default.clone().into_scope(Some(fixture.root_id), None);
    let sigil = Engine::render_with_compiled(scope, &mut compiled, &mut ctx, None).unwrap();
    let svg = String::from_utf8(sigil.svg.0).unwrap();
    assert!(svg.contains("warnings count=\""));
    assert!(ctx.diagnostics.warning_count() > 0);
}

#[test]
fn error_handling_construction_failures_have_no_side_effects() {
    let fixture = small_tree();
    let mut logic = minimal_logic();
    logic
        .params
        .set_param_string("layouter", "radial_mandala", "ring_step", "wide");
    let scope = logic
        .scope_default
        .clone()
        .into_scope(Some(fixture.root_id.clone()), None);
    let mut ctx = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
    let dir = TempDir::new().unwrap();
    let err = render_animation(
        scope,
        logic,
        AnimationAxis::ParamSweep {
            stage: StageRef::Layouter,
            param: "ring_step".into(),
            from: 1.0,
            to: 2.0,
            frames: 2,
        },
        AnimationOutput::FrameSequence {
            dir: dir.path().to_path_buf(),
        },
        &mut ctx,
    )
    .unwrap_err();
    assert!(matches!(err, SigilError::Construction { .. }));
    let count = std::fs::read_dir(dir.path()).unwrap().count();
    assert_eq!(count, 0);
}

#[test]
fn error_handling_renderer_internal_error_not_panic() {
    let scene = StyledScene {
        layout: Layout {
            marks: vec![PlacedMark {
                mark: MarkSpec {
                    id: "x".into(),
                    primitive: Primitive::Circle,
                    channels: HashMap::new(),
                },
                cx: f64::NAN,
                cy: 0.0,
                rotation: 0.0,
                scale: 1.0,
            }],
            structural: Vec::new(),
        },
        background: None,
        stroke_color: "#000".into(),
        fill_color: "#000".into(),
        glyph_color: "#000".into(),
        filter: None,
        palette_name: "ink".into(),
        stroke_only: false,
        glyph_mirror: false,
    };
    let renderer = SvgRenderer {
        viewbox: (0.0, 0.0, 600.0, 600.0),
        margin: 40.0,
        embed_metadata: false,
    };
    let logic = load_preset(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("presets/contemplative.toml"),
    )
    .unwrap()
    .logic;
    let store = werk_core::store::Store::new_in_memory().unwrap();
    let ctx = Ctx::new(fixed_now(), &store, "werk", 0);
    let err = renderer
        .render(&logic, "scope", 1, scene, &ctx)
        .unwrap_err();
    assert!(matches!(err, SigilError::Internal { .. }));
}
