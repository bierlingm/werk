use axum::body::{Body, to_bytes};
use hyper::Request;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;
use tower::ServiceExt;
use werk_core::Store;
use werk_shared::Workspace;

#[tokio::test]
async fn web_matches_cli() {
    let dir = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("HOME", dir.path());
    }
    let workspace = Workspace::init(dir.path(), false).unwrap();
    let store = Store::init(workspace.root()).unwrap();
    let tension = store.create_tension("sigil parity", "baseline").unwrap();
    let short_code = tension.short_code.unwrap();

    let app = werk_web::build_router(workspace.root().to_path_buf()).unwrap();
    let response = app
        .oneshot(
            Request::get(format!(
                "/api/sigil?scope={short_code}&logic=contemplative&seed=7"
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    let web_bytes = to_bytes(response.into_body(), 10_000_000).await.unwrap();

    let cli_output = Command::new(werk_bin())
        .arg("sigil")
        .arg(short_code.to_string())
        .arg("--logic")
        .arg("contemplative")
        .arg("--seed")
        .arg("7")
        .env("HOME", dir.path())
        .current_dir(dir.path())
        .output()
        .expect("failed to run werk");
    assert!(cli_output.status.success());

    let web_norm = normalize_svg(&web_bytes);
    let cli_norm = normalize_svg(&cli_output.stdout);
    assert_eq!(web_norm, cli_norm);
}

fn werk_bin() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/debug/werk");
    if path.exists() {
        return path;
    }
    let status = Command::new("cargo")
        .args(["build", "-p", "werk"])
        .status()
        .expect("failed to build werk");
    assert!(status.success());
    path
}

fn normalize_svg(bytes: &[u8]) -> String {
    let mut svg = String::from_utf8_lossy(bytes).to_string();
    if let Some(start) = svg.find("<generated>")
        && let Some(end) = svg[start..].find("</generated>")
    {
        let end = start + end + "</generated>".len();
        svg.replace_range(start..end, "<generated>fixed</generated>");
    }
    svg
}
