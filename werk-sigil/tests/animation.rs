mod fixtures;

use fixtures::small_tree::{fixed_now, small_tree};
use tempfile::TempDir;
use werk_sigil::{
    render_animation, AnimationAxis, AnimationOutput, Ctx, Logic, Scope, ScopeKind, SigilError,
    StageRef, load_preset,
};

fn preset_logic(name: &str) -> Logic {
    load_preset(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("presets/{name}.toml")),
    )
    .unwrap()
    .logic
}

fn subtree_scope(root_id: String) -> Scope {
    Scope {
        kind: ScopeKind::Subtree,
        root: Some(root_id),
        depth: Some(2),
        name: None,
        status: None,
        members: Vec::new(),
        at: None,
    }
}

#[test]
fn seed_sweep_respects_start_end_step() {
    let fixture = small_tree();
    let logic = preset_logic("contemplative");
    let scope = subtree_scope(fixture.root_id.clone());
    let mut ctx = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
    let dir = TempDir::new().unwrap();
    render_animation(
        scope,
        logic,
        AnimationAxis::SeedSweep {
            start: 2,
            end: 8,
            step: 2,
        },
        AnimationOutput::FrameSequence {
            dir: dir.path().to_path_buf(),
        },
        &mut ctx,
    )
    .unwrap();

    let mut files = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    files.sort();
    assert_eq!(files.len(), 3);

    let seeds = [2, 4, 6];
    for (path, seed) in files.iter().zip(seeds.iter()) {
        let svg = std::fs::read_to_string(path).unwrap();
        assert!(svg.contains(&format!("<seed>{seed}</seed>")));
    }
}

#[test]
fn param_sweep_linear_interp() {
    let fixture = small_tree();
    let mut logic = preset_logic("contemplative");
    logic
        .params
        .set_param_number("layouter", "radial_mandala", "ring_step", 60.0);
    let scope = subtree_scope(fixture.root_id.clone());
    let mut ctx = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
    let dir = TempDir::new().unwrap();
    render_animation(
        scope,
        logic,
        AnimationAxis::ParamSweep {
            stage: StageRef::Layouter,
            param: "ring_step".into(),
            from: 60.0,
            to: 100.0,
            frames: 5,
        },
        AnimationOutput::FrameSequence {
            dir: dir.path().to_path_buf(),
        },
        &mut ctx,
    )
    .unwrap();

    let mut names = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    names.sort();
    let expected = ["60.0", "70.0", "80.0", "90.0", "100.0"];
    for value in expected {
        assert!(names.iter().any(|name| name.contains(value)));
    }
}

#[test]
fn animated_svg_unsupported() {
    let fixture = small_tree();
    let logic = preset_logic("contemplative");
    let scope = subtree_scope(fixture.root_id.clone());
    let mut ctx = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
    let err = render_animation(
        scope,
        logic,
        AnimationAxis::SeedSweep {
            start: 0,
            end: 1,
            step: 1,
        },
        AnimationOutput::AnimatedSvg,
        &mut ctx,
    )
    .unwrap_err();
    assert!(matches!(err, SigilError::Unsupported { .. }));
}

#[test]
fn epoch_range_unsupported() {
    let fixture = small_tree();
    let logic = preset_logic("contemplative");
    let scope = subtree_scope(fixture.root_id.clone());
    let mut ctx = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
    let err = render_animation(
        scope,
        logic,
        AnimationAxis::EpochRange {
            from: "e1".into(),
            to: "e2".into(),
        },
        AnimationOutput::FrameSequence {
            dir: TempDir::new().unwrap().path().to_path_buf(),
        },
        &mut ctx,
    )
    .unwrap_err();
    assert!(matches!(err, SigilError::Unsupported { .. }));
}

#[test]
fn param_sweep_rejects_unknown_or_non_numeric() {
    let fixture = small_tree();
    let mut logic = preset_logic("contemplative");
    logic
        .params
        .set_param_string("layouter", "radial_mandala", "ring_step", "wide");
    let scope = subtree_scope(fixture.root_id.clone());
    let mut ctx = Ctx::new(fixed_now(), &fixture.store, "werk", 0);

    let unknown = render_animation(
        scope.clone(),
        logic.clone(),
        AnimationAxis::ParamSweep {
            stage: StageRef::Layouter,
            param: "unknown".into(),
            from: 1.0,
            to: 2.0,
            frames: 2,
        },
        AnimationOutput::FrameSequence {
            dir: TempDir::new().unwrap().path().to_path_buf(),
        },
        &mut ctx,
    )
    .unwrap_err();
    assert!(matches!(unknown, SigilError::Construction { .. }));

    let non_numeric = render_animation(
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
            dir: TempDir::new().unwrap().path().to_path_buf(),
        },
        &mut ctx,
    )
    .unwrap_err();
    assert!(matches!(non_numeric, SigilError::Construction { .. }));
}
