mod fixtures;

use fixtures::small_tree::{fixed_now, small_tree};
use werk_sigil::{Ctx, Engine, load_preset};

#[test]
fn contemplative_pipeline_smoke() {
    let fixture = small_tree();
    let preset = load_preset(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("presets/contemplative.toml"),
    )
    .unwrap();
    let scope = preset
        .logic
        .scope_default
        .clone()
        .into_scope(Some(fixture.root_id.clone()), None);
    let mut ctx = Ctx::new(fixed_now(), &fixture.store, "werk", 0);
    let sigil = Engine::render(scope, preset.logic, &mut ctx).unwrap();
    let svg = String::from_utf8(sigil.svg.0).unwrap();
    assert!(svg.contains("<circle") || svg.contains("class=\"glyph\""));
    assert!(svg.contains("<path"));
}
