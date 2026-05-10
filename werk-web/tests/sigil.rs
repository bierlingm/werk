use axum::body::{Body, to_bytes};
use hyper::{Request, StatusCode};
use tempfile::TempDir;
use tower::ServiceExt;
use werk_core::Store;
use werk_shared::Workspace;
use werk_sigil::{Ctx, Engine, cache_path, load_preset, scope_canonical, werk_state_revision};

#[tokio::test]
async fn get_returns_svg() {
    let dir = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("HOME", dir.path());
    }
    let workspace = Workspace::init(dir.path(), false).unwrap();
    let store = Store::init(workspace.root()).unwrap();
    let tension = store.create_tension("sigil web", "baseline").unwrap();
    let short_code = tension.short_code.unwrap();

    let app = werk_web::build_router(workspace.root().to_path_buf()).unwrap();
    let response = app
        .oneshot(
            Request::get(format!("/api/sigil?scope={short_code}&logic=contemplative"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "image/svg+xml"
    );

    let body = to_bytes(response.into_body(), 10_000_000).await.unwrap();
    let text = String::from_utf8_lossy(&body);
    assert!(text.starts_with("<?xml"));
}

#[tokio::test]
async fn caches_on_second_call() {
    let dir = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("HOME", dir.path());
    }
    let workspace = Workspace::init(dir.path(), false).unwrap();
    let store = Store::init(workspace.root()).unwrap();
    let tension = store.create_tension("sigil web", "baseline").unwrap();
    let short_code = tension.short_code.unwrap();

    let app = werk_web::build_router(workspace.root().to_path_buf()).unwrap();
    let uri = format!("/api/sigil?scope={short_code}&logic=contemplative&seed=7");

    let response = app
        .clone()
        .oneshot(Request::get(&uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let first = to_bytes(response.into_body(), 10_000_000).await.unwrap();

    let preset = load_preset(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../werk-sigil/presets/contemplative.toml"),
    )
    .unwrap();
    let scope = preset
        .logic
        .scope_default
        .clone()
        .into_scope(Some(tension.id.clone()), None);
    let mut ctx = Ctx::new(chrono::Utc::now(), &store, "werk", 0);
    let compiled = Engine::compile(preset.logic.clone()).unwrap();
    let resolved = compiled.selector.select(scope, &mut ctx).unwrap();
    let scope_canonical = scope_canonical(&resolved);
    let revision = werk_state_revision(&store, &resolved.tensions).unwrap();
    let cache = cache_path(&scope_canonical, &preset.logic.cache_key(), 7, &revision);
    assert!(cache.exists());

    let response = app
        .oneshot(Request::get(&uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let second = to_bytes(response.into_body(), 10_000_000).await.unwrap();
    assert_eq!(first, second);
}
