mod fixtures;

use fixtures::small_tree::{fixed_now, small_tree};
use std::fs;
use std::path::PathBuf;
use werk_sigil::{Ctx, Engine, load_preset};

fn render_preset(preset: &str) -> String {
    let fixture = small_tree();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("presets/{preset}.toml"));
    let preset = load_preset(path).unwrap();
    let scope = preset
        .logic
        .scope_default
        .clone()
        .into_scope(Some(fixture.root_id.clone()), None);
    let mut ctx = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
    let sigil = Engine::render(scope, preset.logic, &mut ctx).unwrap();
    String::from_utf8(sigil.svg.0).unwrap()
}

fn snapshot_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/snapshots/{name}.svg"))
}

fn compare_or_update(name: &str, svg: &str) {
    let path = snapshot_path(name);
    if std::env::var("WERK_UPDATE_SNAPSHOTS").ok().as_deref() == Some("1") {
        fs::write(&path, svg).unwrap();
        return;
    }
    let expected = fs::read_to_string(&path).expect("snapshot missing");
    assert_eq!(expected, svg);
}

#[test]
fn contemplative_snapshot() {
    let svg = render_preset("contemplative");
    compare_or_update("contemplative", &svg);
}

#[test]
fn glance_snapshot() {
    let svg = render_preset("glance");
    compare_or_update("glance", &svg);
}

#[test]
fn snapshot_snapshot() {
    let svg = render_preset("snapshot");
    compare_or_update("snapshot", &svg);
}

#[test]
fn identity_snapshot() {
    let svg = render_preset("identity");
    compare_or_update("identity", &svg);
}

#[test]
fn oracle_snapshot() {
    let svg = render_preset("oracle");
    compare_or_update("oracle", &svg);
}

#[test]
fn color_emphasis_for_stale_nodes() {
    let svg = render_preset("oracle");
    assert!(svg.contains("#cc3333"));
}
