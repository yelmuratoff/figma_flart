use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use flart::canonical::{canonical_json, checksum};
use predicates::str::contains;
use serde_json::{Value, json};
use tempfile::TempDir;

const PARTS: [&str; 5] = [
    "styles.json",
    "collections.json",
    "variables-theme.json",
    "variables-size.json",
    "remote.json",
];

fn fixture_parts(case: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(case)
        .join("parts")
}

fn flart() -> Command {
    Command::cargo_bin("flart").unwrap()
}

/// Copies a case's parts into a scratch dir so a test can alter one of them.
fn scratch_parts_of(case: &str) -> (TempDir, Vec<PathBuf>) {
    let dir = TempDir::new().unwrap();
    let paths = PARTS
        .iter()
        .map(|name| {
            let target = dir.path().join(name);
            fs::copy(fixture_parts(case).join(name), &target).unwrap();
            target
        })
        .collect();
    (dir, paths)
}

fn scratch_parts() -> (TempDir, Vec<PathBuf>) {
    scratch_parts_of("sample")
}

fn read(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

/// Rewrites a part's data and signs it the way the collector would.
fn resign(path: &Path, edit: impl FnOnce(&mut Value)) {
    let mut part = read(path);
    edit(&mut part["data"]);
    part["checksum"] = json!(checksum(&canonical_json(&part["data"])));
    fs::write(path, part.to_string()).unwrap();
}

fn assemble(parts: &[PathBuf], output: &Path) -> Command {
    let mut cmd = flart();
    cmd.args(["--json", "snapshot", "assemble", "-o"])
        .arg(output)
        .args(parts);
    cmd
}

fn envelope(output: &[u8]) -> Value {
    serde_json::from_slice(output).unwrap()
}

fn assert_report(case: &str, report: Value) {
    let (dir, parts) = scratch_parts_of(case);
    let snapshot = dir.path().join("snapshot.json");
    let assembled = assemble(&parts, &snapshot).assert().success();
    let out = envelope(&assembled.get_output().stdout);
    assert_eq!(out["apiVersion"], 1);
    assert_eq!(out["type"], "report");
    assert_eq!(out["data"]["report"], report);

    let checked = flart()
        .args(["--json", "check"])
        .arg(&snapshot)
        .assert()
        .success();
    assert_eq!(
        envelope(&checked.get_output().stdout)["data"]["report"],
        report
    );
}

#[test]
fn check_json_envelope() {
    assert_report(
        "sample",
        json!({
            "collections": 2,
            "variables": 9,
            "aliases": 4,
            "remoteVariables": 3,
            "textStyles": 3,
            "paintStyles": 3,
            "effectStyles": 2
        }),
    );
}

#[test]
fn ah_ds_report_when_present() {
    if !fixture_parts("ah_ds").is_dir() {
        eprintln!("skipped: tests/fixtures/ah_ds is local-only and not present");
        return;
    }
    assert_report(
        "ah_ds",
        json!({
            "collections": 2,
            "variables": 101,
            "aliases": 231,
            "remoteVariables": 52,
            "textStyles": 15,
            "paintStyles": 1,
            "effectStyles": 4
        }),
    );
}

#[test]
fn assemble_rejects_corrupted_part() {
    let (dir, parts) = scratch_parts();
    let theme = dir.path().join("variables-theme.json");
    let text = fs::read_to_string(&theme).unwrap();
    let corrupted = text.replacen("0.800000011920929", "0.800000011920928", 1);
    assert_ne!(text, corrupted);
    fs::write(&theme, corrupted).unwrap();

    let snapshot = dir.path().join("snapshot.json");
    let failed = assemble(&parts, &snapshot).assert().code(4);
    let out = envelope(&failed.get_output().stdout);
    assert_eq!(out["data"]["kind"], "checksum_mismatch");
    assert!(
        out["data"]["message"]
            .as_str()
            .unwrap()
            .contains("variables-theme.json")
    );
    assert!(!snapshot.exists());
}

#[test]
fn reformatted_part_still_matches() {
    let (dir, parts) = scratch_parts();
    let size = dir.path().join("variables-size.json");
    fs::write(&size, serde_json::to_string_pretty(&read(&size)).unwrap()).unwrap();

    assemble(&parts, &dir.path().join("snapshot.json"))
        .assert()
        .success();
}

#[test]
fn dangling_alias() {
    let (dir, parts) = scratch_parts();
    resign(&dir.path().join("remote.json"), |data| {
        let variables = data["variables"].as_array_mut().unwrap();
        let gray_900 = variables
            .iter()
            .position(|v| v["name"] == "gray/900")
            .unwrap();
        variables.remove(gray_900);
    });

    let snapshot = dir.path().join("snapshot.json");
    let failed = assemble(&parts, &snapshot).assert().code(5);
    let out = envelope(&failed.get_output().stdout);
    assert_eq!(out["data"]["kind"], "dangling_alias");
    assert!(
        out["data"]["message"]
            .as_str()
            .unwrap()
            .contains("VariableID:lib-gray900/9:2")
    );
    assert!(!snapshot.exists());
}

#[test]
fn unresolved_remote_ids_fail() {
    let (dir, parts) = scratch_parts();
    resign(&dir.path().join("remote.json"), |data| {
        data["missing"] = json!(["VariableID:deleted/1:1"]);
    });

    assemble(&parts, &dir.path().join("snapshot.json"))
        .assert()
        .code(5)
        .stdout(contains("VariableID:deleted/1:1"));
}

#[test]
fn missing_collection_part_fails() {
    let (dir, parts) = scratch_parts();
    let without_size: Vec<_> = parts
        .into_iter()
        .filter(|p| !p.ends_with("variables-size.json"))
        .collect();

    assemble(&without_size, &dir.path().join("snapshot.json"))
        .assert()
        .code(3)
        .stdout(contains("variables:VariableCollectionId:1:3"));
}

#[test]
fn duplicate_part_fails() {
    let (dir, mut parts) = scratch_parts();
    parts.push(parts[0].clone());

    assemble(&parts, &dir.path().join("snapshot.json"))
        .assert()
        .code(3)
        .stdout(contains("second part for styles"));
}

#[test]
fn collector_substitutes_the_part() {
    let printed = flart()
        .args(["collector", "--part", "variables:VariableCollectionId:1:12"])
        .assert()
        .success();
    let script = String::from_utf8(printed.get_output().stdout.clone()).unwrap();
    assert!(script.starts_with("const PART = \"variables:VariableCollectionId:1:12\";"));
    assert!(!script.contains("__FLART_PART__"));
}

#[test]
fn collector_rejects_unknown_part() {
    flart()
        .args(["collector", "--part", "pages"])
        .assert()
        .code(2)
        .stderr(contains("unknown part"));
}

#[test]
fn collector_rejects_ids_that_break_the_script() {
    for part in [
        "variables:1:12\";figma.root.remove();\"",
        "variables:1:12\nx",
        "variables:",
    ] {
        flart()
            .args(["collector", "--part", part])
            .assert()
            .code(2)
            .stderr(contains("unknown part"));
    }
}
