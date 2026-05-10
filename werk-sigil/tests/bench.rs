mod fixtures;

use fixtures::small_tree::fixed_now;
use std::time::Instant;
use werk_sigil::{Ctx, Engine, load_preset};
use werk_core::store::Store;

fn large_tree(store: &Store, count: usize) -> String {
    let root = store.create_tension("root", "actual").unwrap();
    let mut parents = vec![root.id.clone()];
    for idx in 0..count.saturating_sub(1) {
        let parent = parents[idx % parents.len()].clone();
        let name = format!("node {idx}");
        let tension = store
            .create_tension_with_parent(&name, "actual", Some(parent))
            .unwrap();
        parents.push(tension.id.clone());
    }
    root.id
}

#[test]
fn bench_contemplative_50_node_under_100ms() {
    let store = Store::new_in_memory().unwrap();
    let root_id = large_tree(&store, 50);
    let preset = load_preset(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("presets/contemplative.toml"),
    )
    .unwrap();
    let scope = preset
        .logic
        .scope_default
        .clone()
        .into_scope(Some(root_id), None);
    let mut ctx = Ctx::new(fixed_now(), &store, "werk", 0);
    let mut samples = Vec::new();
    for _ in 0..20 {
        let start = Instant::now();
        let _ = Engine::render(scope.clone(), preset.logic.clone(), &mut ctx).unwrap();
        samples.push(start.elapsed());
    }
    samples.sort();
    let median = samples[samples.len() / 2];
    assert!(median.as_millis() < 100, "median {:?} exceeded 100ms", median);
}
