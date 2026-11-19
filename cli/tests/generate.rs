use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use flart::codegen::{self, Kind, Options};
use flart::snapshot;
use serde_json::{Value, json};
use tempfile::TempDir;

fn sample_snapshot() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample/snapshot.json")
}

fn generate(snapshot: &Path, out: &Path, extra: &[&str]) -> Command {
    let mut cmd = Command::cargo_bin("flart").unwrap();
    cmd.args(["--json", "generate"])
        .arg(snapshot)
        .arg("--out")
        .arg(out)
        .args(extra);
    cmd
}

fn files(output: &[u8]) -> Vec<Value> {
    let envelope: Value = serde_json::from_slice(output).unwrap();
    assert_eq!(envelope["type"], "generate");
    envelope["data"]["files"].as_array().unwrap().clone()
}

fn statuses(output: &[u8]) -> Vec<(String, String)> {
    files(output)
        .into_iter()
        .map(|f| {
            (
                f["kind"].as_str().unwrap().to_string(),
                f["status"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn all(status: &str) -> Vec<(String, String)> {
    ["colors", "text_styles", "effects", "variables"]
        .iter()
        .map(|kind| (kind.to_string(), status.to_string()))
        .collect()
}

#[test]
fn generate_writes_four_files() {
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("lib/theme");
    let run = generate(&sample_snapshot(), &out, &["--theme-extensions"])
        .assert()
        .success();
    assert_eq!(statuses(&run.get_output().stdout), all("created"));

    let loaded = snapshot::load(&sample_snapshot()).unwrap();
    let options = Options {
        theme_extensions: true,
        include_font_name: false,
    };
    for kind in Kind::ALL {
        let written = fs::read_to_string(out.join(format!("{}.dart", kind.file_stem()))).unwrap();
        let library = codegen::generate(kind, &loaded, options, None).unwrap();
        assert_eq!(written, library, "{}", kind.file_stem());
    }
}

#[test]
fn dry_run_writes_nothing() {
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("lib/theme");
    let run = generate(&sample_snapshot(), &out, &["--dry-run"])
        .assert()
        .success();
    assert_eq!(statuses(&run.get_output().stdout), all("created"));
    assert!(!out.exists());
}

#[test]
fn second_run_is_unchanged_and_edits_are_detected() {
    let dir = TempDir::new().unwrap();
    generate(&sample_snapshot(), dir.path(), &[])
        .assert()
        .success();
    let again = generate(&sample_snapshot(), dir.path(), &[])
        .assert()
        .success();
    assert_eq!(statuses(&again.get_output().stdout), all("unchanged"));

    fs::write(dir.path().join("colors.dart"), "// edited by hand\n").unwrap();
    let after_edit = generate(&sample_snapshot(), dir.path(), &["--dry-run"])
        .assert()
        .success();
    let expected: Vec<_> = all("unchanged")
        .into_iter()
        .map(|(kind, status)| {
            let status = if kind == "colors" {
                "changed".into()
            } else {
                status
            };
            (kind, status)
        })
        .collect();
    assert_eq!(statuses(&after_edit.get_output().stdout), expected);
    assert_eq!(
        fs::read_to_string(dir.path().join("colors.dart")).unwrap(),
        "// edited by hand\n"
    );
}

#[test]
fn modes_apply_per_kind() {
    let dir = TempDir::new().unwrap();
    let run = generate(&sample_snapshot(), dir.path(), &["--modes", "Mobile"])
        .assert()
        .success();
    let modes: Vec<(String, Value)> = files(&run.get_output().stdout)
        .into_iter()
        .map(|f| (f["kind"].as_str().unwrap().to_string(), f["modes"].clone()))
        .collect();
    assert_eq!(
        modes,
        vec![
            ("colors".to_string(), json!([])),
            ("text_styles".to_string(), json!(["Mobile"])),
            ("effects".to_string(), json!(["Mobile"])),
            ("variables".to_string(), json!(["Mobile"])),
        ]
    );
    let text = fs::read_to_string(dir.path().join("text_styles.dart")).unwrap();
    assert!(text.contains("fontSize: 18,"), "{text}");
}

#[test]
fn unknown_mode_lists_choices() {
    let dir = TempDir::new().unwrap();
    for (modes, missing) in [("Mobile,Tablet", "Tablet"), ("Default", "Default")] {
        let run = generate(&sample_snapshot(), dir.path(), &["--modes", modes])
            .assert()
            .code(8);
        let envelope: Value = serde_json::from_slice(&run.get_output().stdout).unwrap();
        assert_eq!(envelope["data"]["kind"], "unknown_mode");
        let message = envelope["data"]["message"].as_str().unwrap();
        assert!(message.contains(&format!("\"{missing}\"")), "{message}");
        assert!(
            message.ends_with("available: light, dark, dim, Desktop, Mobile"),
            "{message}"
        );
    }
    assert!(fs::read_dir(dir.path()).unwrap().next().is_none());
}

#[test]
fn kind_without_content_is_skipped() {
    let dir = TempDir::new().unwrap();
    let mut raw: Value =
        serde_json::from_str(&fs::read_to_string(sample_snapshot()).unwrap()).unwrap();
    raw["effectStyles"] = json!([]);
    let trimmed = dir.path().join("snapshot.json");
    fs::write(&trimmed, raw.to_string()).unwrap();

    let out = dir.path().join("out");
    let run = generate(&trimmed, &out, &["--only", "effects,colors"])
        .assert()
        .success();
    assert_eq!(
        statuses(&run.get_output().stdout),
        vec![
            ("effects".to_string(), "skipped".to_string()),
            ("colors".to_string(), "created".to_string()),
        ]
    );
    assert!(!out.join("effects.dart").exists());
}

#[test]
fn unknown_kind_is_a_usage_error() {
    let dir = TempDir::new().unwrap();
    generate(&sample_snapshot(), dir.path(), &["--only", "fonts"])
        .assert()
        .code(2);
}
