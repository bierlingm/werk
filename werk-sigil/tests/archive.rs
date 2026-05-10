mod fixtures;

use fixtures::small_tree::{fixed_now, small_tree};
use filetime::FileTime;
use std::thread::sleep;
use std::time::Duration;
use werk_sigil::{
    Engine, archive_path, cache_path, cleanup_cache, derive_seed, load_preset, scope_canonical,
    werk_state_revision,
};
use tempfile::TempDir;

#[test]
fn archive_path_uses_now_date() {
    let path = archive_path("#7", "contemplative", 3, fixed_now());
    let path_str = path.to_string_lossy();
    assert!(path_str.contains("/.werk/sigils/2026-05-08/"));
    assert!(path_str.ends_with("-7-contemplative-3.svg"));
}

#[test]
fn cache_key_invalidates_on_state_change() {
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
    let mut ctx = werk_sigil::Ctx::new(fixed_now(), &fixture.store, "werk", 0);
    let compiled = Engine::compile(preset.logic.clone()).unwrap();
    let resolved = compiled.selector.select(scope, &mut ctx).unwrap();
    let scope_canonical = scope_canonical(&resolved);
    let seed = derive_seed(&compiled.logic, &scope_canonical);
    let revision_before = werk_state_revision(&fixture.store, &resolved.tensions).unwrap();
    let path_before = cache_path(
        &scope_canonical,
        &compiled.logic.cache_key(),
        seed,
        &revision_before,
    );

    sleep(Duration::from_millis(10));
    let target = resolved.tensions[0].id.clone();
    fixture
        .store
        .update_desired(&target, "changed desired")
        .unwrap();

    let revision_after = werk_state_revision(&fixture.store, &resolved.tensions).unwrap();
    let path_after = cache_path(
        &scope_canonical,
        &compiled.logic.cache_key(),
        seed,
        &revision_after,
    );

    assert_ne!(path_before, path_after);
}

#[test]
fn cleanup_cache_removes_stale() {
    let dir = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("HOME", dir.path());
    }
    let cache_dir = dir.path().join(".werk/sigils/cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let stale = cache_dir.join("stale.svg");
    let fresh = cache_dir.join("fresh.svg");
    std::fs::write(&stale, "<svg/>").unwrap();
    std::fs::write(&fresh, "<svg/>").unwrap();

    let stale_time = FileTime::from_unix_time(
        (chrono::Utc::now() - chrono::Duration::days(8)).timestamp(),
        0,
    );
    filetime::set_file_mtime(&stale, stale_time).unwrap();

    let report = cleanup_cache(7).unwrap();
    assert_eq!(report.removed, 1);
    assert!(!stale.exists());
    assert!(fresh.exists());
}
