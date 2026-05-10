use assert_cmd::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

fn setup_workspace() -> (TempDir, String) {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("sigil test")
        .arg("baseline")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let short_code = extract_short_code(&stdout).expect("should extract short code");
    (dir, short_code)
}

fn extract_short_code(output: &str) -> Option<String> {
    let hash = output.find('#')?;
    let rest = &output[hash + 1..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
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

#[test]
fn renders_to_stdout() {
    let (dir, short_code) = setup_workspace();
    let output = cargo_bin_cmd!("werk")
        .arg("sigil")
        .arg(&short_code)
        .arg("--logic")
        .arg("contemplative")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.starts_with("<?xml"));
}

#[test]
fn writes_to_out_path() {
    let (dir, short_code) = setup_workspace();
    let out_path = dir.path().join("out.svg");

    let output = cargo_bin_cmd!("werk")
        .arg("sigil")
        .arg(&short_code)
        .arg("--out")
        .arg(out_path.to_string_lossy().to_string())
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        !stdout.starts_with("<?xml"),
        "stdout should not contain SVG bytes when --out is used"
    );
    let file = std::fs::read_to_string(&out_path).unwrap();
    assert!(file.starts_with("<?xml"));
}

#[test]
fn json_output_shape() {
    let (dir, short_code) = setup_workspace();
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("sigil")
        .arg(&short_code)
        .arg("--seed")
        .arg("7")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("sigil json should parse");
    assert!(json.get("scope").is_some());
    assert!(json.get("logic").is_some());
    assert!(json.get("logic_version").is_some());
    assert!(json.get("seed").is_some());
    assert!(json.get("warnings").is_some());
    assert!(json.get("svg").is_some());
    assert!(json.get("path").is_none());
}

#[test]
fn dry_run_does_not_write() {
    let (dir, short_code) = setup_workspace();
    let out_path = dir.path().join("dry-run.svg");
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("sigil")
        .arg(&short_code)
        .arg("--dry-run")
        .arg("--out")
        .arg(out_path.to_string_lossy().to_string())
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert!(!out_path.exists());
    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("dry-run json should parse");
    assert_eq!(json.get("dry_run").and_then(|v| v.as_bool()), Some(true));
}

#[test]
fn seed_override_changes_output() {
    let (dir, short_code) = setup_workspace();
    let output_7 = cargo_bin_cmd!("werk")
        .arg("sigil")
        .arg(&short_code)
        .arg("--seed")
        .arg("7")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output_8 = cargo_bin_cmd!("werk")
        .arg("sigil")
        .arg(&short_code)
        .arg("--seed")
        .arg("8")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output_7b = cargo_bin_cmd!("werk")
        .arg("sigil")
        .arg(&short_code)
        .arg("--seed")
        .arg("7")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let norm_7 = normalize_svg(&output_7);
    let norm_8 = normalize_svg(&output_8);
    let norm_7b = normalize_svg(&output_7b);
    assert_ne!(norm_7, norm_8);
    assert_eq!(norm_7, norm_7b);
}

#[test]
fn not_found_json_error_shape() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("sigil")
        .arg("99999")
        .current_dir(dir.path())
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("error json should parse");
    let error = json.get("error").expect("error object");
    assert_eq!(
        error.get("code").and_then(|v| v.as_str()),
        Some("NOT_FOUND")
    );
}

#[test]
fn help_includes_examples() {
    let output = cargo_bin_cmd!("werk")
        .arg("sigil")
        .arg("--help")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Examples:"));
    let example_lines = stdout
        .lines()
        .filter(|line| line.trim_start().starts_with("werk sigil"));
    assert!(example_lines.count() >= 3);
}

#[test]
fn default_logic_is_contemplative() {
    let (dir, short_code) = setup_workspace();
    let output_default = cargo_bin_cmd!("werk")
        .arg("sigil")
        .arg(&short_code)
        .arg("--seed")
        .arg("7")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output_named = cargo_bin_cmd!("werk")
        .arg("sigil")
        .arg(&short_code)
        .arg("--logic")
        .arg("contemplative")
        .arg("--seed")
        .arg("7")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let norm_default = normalize_svg(&output_default);
    let norm_named = normalize_svg(&output_named);
    assert_eq!(norm_default, norm_named);
}
