use axum::body::Body;
use http_body_util::BodyExt;
use hyper::{Request, StatusCode};
use serde_json::json;
use tempfile::TempDir;
use tokio::time::{Duration, timeout};
use tower::ServiceExt;
use werk_core::Store;
use werk_shared::Workspace;

#[tokio::test]
async fn invalidates_on_mutation() {
    let dir = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("HOME", dir.path());
    }
    let workspace = Workspace::init(dir.path(), false).unwrap();
    let store = Store::init(workspace.root()).unwrap();
    let tension = store.create_tension("sigil stream", "baseline").unwrap();
    let id = tension.id.clone();

    let app = werk_web::build_router(workspace.root().to_path_buf()).unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::get("/api/sigil/stream")
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
        "text/event-stream"
    );

    let mut body = response.into_body();
    let app_for_update = app.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let payload = json!({ "value": "updated" });
        let _ = app_for_update
            .oneshot(
                Request::patch(format!("/api/tensions/{id}/desired"))
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await;
    });

    let received = timeout(Duration::from_secs(2), async {
        loop {
            if let Some(frame) = body.frame().await {
                let frame = frame.unwrap();
                if let Some(bytes) = frame.data_ref() {
                    let text = String::from_utf8_lossy(bytes);
                    if text.contains("event: invalidate") {
                        return text.to_string();
                    }
                }
            }
        }
    })
    .await
    .expect("expected invalidate event");

    assert!(received.contains("event: invalidate"));
}

#[tokio::test]
async fn invalidates_on_logic_file_change() {
    let dir = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("HOME", dir.path());
    }
    let workspace = Workspace::init(dir.path(), false).unwrap();
    let store = Store::init(workspace.root()).unwrap();
    let tension = store.create_tension("sigil stream", "baseline").unwrap();
    let scope = tension.short_code.unwrap().to_string();

    let logic_dir = dir.path().join("sigil-logic");
    std::fs::create_dir_all(&logic_dir).unwrap();
    unsafe {
        std::env::set_var(
            "WERK_SIGIL_WATCH_PATHS",
            logic_dir.to_string_lossy().to_string(),
        );
    }

    let logic_path = logic_dir.join("hot.toml");
    let initial = r#"
[meta]
name = "hot"
version = "1"

[scope]
default = { kind = "subtree", depth = 2 }
fallback = { kind = "space", name = "active" }

[pipeline]
selector = "subtree"
featurizer = "tension_tree"
encoder = "structural_default"
layouter = "radial_mandala"
stylist = "ink_brush"
renderer = "svg"
"#;
    std::fs::write(&logic_path, initial.trim()).unwrap();

    let app = werk_web::build_router(workspace.root().to_path_buf()).unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::get("/api/sigil/stream")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let mut body = response.into_body();
    let logic_path_for_update = logic_path.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let updated = format!(
            "{initial}\n[encoder.channels.r]\nliteral = 12.0\n",
            initial = initial.trim()
        );
        std::fs::write(&logic_path_for_update, updated).unwrap();
    });

    let received = timeout(Duration::from_secs(1), async {
        loop {
            if let Some(frame) = body.frame().await {
                let frame = frame.unwrap();
                if let Some(bytes) = frame.data_ref() {
                    let text = String::from_utf8_lossy(bytes);
                    if text.contains("event: invalidate") {
                        return text.to_string();
                    }
                }
            }
        }
    })
    .await
    .expect("expected invalidate event");

    assert!(received.contains("event: invalidate"));

    let _ = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/sigil?scope={scope}&logic={}",
                logic_path.display()
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await;

    let cache_dir = dir.path().join(".werk/sigils/cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let entries_before: Vec<_> = std::fs::read_dir(&cache_dir)
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert_eq!(entries_before.len(), 1);

    let updated = format!(
        "{initial}\n[encoder.channels.r]\nliteral = 18.0\n",
        initial = initial.trim()
    );
    std::fs::write(&logic_path, updated).unwrap();

    let _ = app
        .oneshot(
            Request::get(format!(
                "/api/sigil?scope={scope}&logic={}",
                logic_path.display()
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await;

    let entries_after: Vec<_> = std::fs::read_dir(&cache_dir)
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert_eq!(entries_after.len(), 2);
    let before_name = entries_before[0].path().file_name().unwrap().to_owned();
    let after_names: std::collections::HashSet<_> = entries_after
        .iter()
        .filter_map(|entry| entry.path().file_name().map(|name| name.to_owned()))
        .collect();
    assert!(after_names.contains(&before_name));
}
