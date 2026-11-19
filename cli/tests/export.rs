use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use flart::export::{self, Dropped, RESOLVER_FILE};
use flart::{Error, Snapshot, snapshot};
use serde::Deserialize;
use serde_json::{Value, json};
use tempfile::TempDir;

fn sample_snapshot() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample/snapshot.json")
}

fn sample_documents() -> HashMap<String, Value> {
    let loaded = snapshot::load(&sample_snapshot()).unwrap();
    export::documents(&loaded)
        .unwrap()
        .files
        .into_iter()
        .collect()
}

fn export_cmd(out: &Path, extra: &[&str]) -> Command {
    let mut cmd = Command::cargo_bin("flart").unwrap();
    cmd.args(["--json", "export"])
        .arg(sample_snapshot())
        .arg("--out")
        .arg(out)
        .args(extra);
    cmd
}

fn envelope(output: &[u8]) -> Value {
    let envelope: Value = serde_json::from_slice(output).unwrap();
    assert_eq!(envelope["type"], "export");
    envelope["data"].clone()
}

fn token<'a>(document: &'a Value, path: &str) -> &'a Value {
    path.split('.').fold(document, |node, key| &node[key])
}

/// Every token path in a document, as a dotted string.
fn token_paths(node: &Value, prefix: &mut Vec<String>, into: &mut HashSet<String>) {
    let Some(object) = node.as_object() else {
        return;
    };
    if object.contains_key("$value") {
        into.insert(prefix.join("."));
        return;
    }
    for (key, child) in object {
        prefix.push(key.clone());
        token_paths(child, prefix, into);
        prefix.pop();
    }
}

fn references(node: &Value, into: &mut Vec<String>) {
    match node {
        Value::String(text) if text.starts_with('{') && text.ends_with('}') => {
            into.push(text[1..text.len() - 1].to_string());
        }
        Value::Array(items) => items.iter().for_each(|item| references(item, into)),
        Value::Object(object) => object.values().for_each(|value| references(value, into)),
        _ => {}
    }
}

#[test]
fn export_writes_one_file_per_mode_and_a_resolver() {
    let dir = TempDir::new().unwrap();
    let run = export_cmd(dir.path(), &[]).assert().success();
    let data = envelope(&run.get_output().stdout);

    let written: Vec<String> = data["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| {
            assert_eq!(file["status"], "created");
            let path = PathBuf::from(file["path"].as_str().unwrap());
            path.strip_prefix(dir.path()).unwrap().display().to_string()
        })
        .collect();
    assert_eq!(
        written,
        [
            "tokens/theme/light.tokens.json",
            "tokens/theme/dark.tokens.json",
            "tokens/theme/dim.tokens.json",
            "tokens/size/desktop.tokens.json",
            "tokens/size/mobile.tokens.json",
            "tokens/palette.tokens.json",
            "tokens/styles.tokens.json",
            RESOLVER_FILE,
        ]
    );

    let resolver: Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join(RESOLVER_FILE)).unwrap()).unwrap();
    assert_eq!(resolver["version"], "2025.10");
    assert_eq!(resolver["modifiers"]["Theme"]["default"], "light");
    assert_eq!(
        resolver["modifiers"]["Size"]["contexts"]["Mobile"],
        json!([{ "$ref": "tokens/size/mobile.tokens.json" }])
    );
    assert_eq!(
        resolver["resolutionOrder"],
        json!([
            { "$ref": "#/sets/Palette" },
            { "$ref": "#/sets/styles" },
            { "$ref": "#/modifiers/Theme" },
            { "$ref": "#/modifiers/Size" },
        ])
    );
}

#[test]
fn what_dtcg_cannot_express_is_reported() {
    let loaded = snapshot::load(&sample_snapshot()).unwrap();
    let dropped = export::documents(&loaded).unwrap().dropped;
    let sources: Vec<&str> = dropped.iter().map(|d| d.source.as_str()).collect();
    assert_eq!(
        sources,
        [
            "variable name",
            "variable isDark",
            "text style Caption",
            "effect style background-blur",
        ]
    );
    assert!(dropped.contains(&Dropped {
        source: "text style Caption".into(),
        reason: "line height AUTO depends on font metrics and has no DTCG number".into(),
    }));
}

#[test]
fn every_reference_points_at_an_exported_token() {
    let documents = sample_documents();
    let mut paths = HashSet::new();
    let mut found = Vec::new();
    for (name, document) in &documents {
        if name.as_str() == RESOLVER_FILE {
            continue;
        }
        token_paths(document, &mut Vec::new(), &mut paths);
        references(document, &mut found);
    }
    assert!(
        found.len() >= 6,
        "expected aliases in the sample, got {found:?}"
    );
    for reference in found {
        assert!(
            paths.contains(&reference),
            "dangling reference {{{reference}}}"
        );
    }
}

#[test]
fn values_and_typography_follow_the_mode() {
    let documents = sample_documents();
    let desktop = &documents["tokens/size/desktop.tokens.json"];
    let mobile = &documents["tokens/size/mobile.tokens.json"];
    assert_eq!(
        token(mobile, "Size.text.size-lg.$value"),
        &json!({ "value": 18, "unit": "px" })
    );
    assert_eq!(
        token(desktop, "text-styles.Body.Large.$value.lineHeight"),
        &json!(1.4)
    );
    assert_eq!(
        token(mobile, "text-styles.Body.Large.$value"),
        &json!({
            "fontFamily": "Inter",
            "fontSize": { "value": 18, "unit": "px" },
            "fontWeight": 400,
            "letterSpacing": { "value": 0, "unit": "px" },
            "lineHeight": 1.3333,
        })
    );
    let styles = &documents["tokens/styles.tokens.json"];
    assert_eq!(
        token(styles, "text-styles.Display.1.$value.letterSpacing"),
        &json!({ "value": -1.92, "unit": "px" })
    );
}

#[test]
fn colors_keep_aliases_and_alpha() {
    let documents = sample_documents();
    let light = &documents["tokens/theme/light.tokens.json"];
    let dim = &documents["tokens/theme/dim.tokens.json"];
    assert_eq!(
        token(light, "Theme.text.primary"),
        &json!({ "$type": "color", "$value": "{Palette.gray.900}" })
    );
    assert_eq!(
        token(dim, "Theme.text.primary.$value"),
        &json!({
            "colorSpace": "srgb",
            "components": [1, 1, 1],
            "alpha": 0.800000011920929,
            "hex": "#ffffff",
        })
    );
    let styles = &documents["tokens/styles.tokens.json"];
    assert_eq!(
        token(styles, "paint-styles.brand.solid.$value"),
        &json!("{Theme.surface.brand}")
    );
    assert_eq!(
        token(styles, "paint-styles.scrim.$value.alpha"),
        &json!(0.5)
    );
    assert_eq!(
        token(styles, "effect-styles.shadow.small.$value")[0]["blur"],
        json!("{Size.spacing.md}")
    );
    assert_eq!(
        token(styles, "effect-styles.shadow.small.$value")[1]["inset"],
        json!(true)
    );
}

#[test]
fn dry_run_writes_nothing_and_a_rerun_is_unchanged() {
    let dir = TempDir::new().unwrap();
    let dry = export_cmd(dir.path(), &["--dry-run"]).assert().success();
    assert_eq!(envelope(&dry.get_output().stdout)["dryRun"], true);
    assert!(!dir.path().join(RESOLVER_FILE).exists());

    export_cmd(dir.path(), &[]).assert().success();
    let again = export_cmd(dir.path(), &[]).assert().success();
    let data = envelope(&again.get_output().stdout);
    for file in data["files"].as_array().unwrap() {
        assert_eq!(file["status"], "unchanged", "{}", file["path"]);
    }
}

fn snapshot_with(variables: Value) -> Snapshot {
    Snapshot::deserialize(json!({
        "collections": [{
            "id": "local-c",
            "name": "Base",
            "remote": false,
            "defaultModeId": "m",
            "modes": [{ "modeId": "m", "name": "Default" }],
            "variableIds": [],
        }],
        "variables": variables,
        "textStyles": [],
        "paintStyles": [],
        "effectStyles": [],
        "missing": [],
    }))
    .unwrap()
}

fn float(id: &str, name: &str, value: f64) -> Value {
    json!({
        "id": id,
        "name": name,
        "remote": false,
        "variableCollectionId": "local-c",
        "resolvedType": "FLOAT",
        "valuesByMode": { "m": value },
    })
}

#[test]
fn a_token_that_is_also_a_group_moves_to_root() {
    let snapshot = snapshot_with(json!([
        float("v1", "border", 1.0),
        float("v2", "border/width", 2.0),
        float("v3", "opacity/0.5", 0.5),
    ]));
    let documents: HashMap<String, Value> = export::documents(&snapshot)
        .unwrap()
        .files
        .into_iter()
        .collect();
    let base = &documents["tokens/base.tokens.json"];
    assert_eq!(token(base, "Base.border.$root.$value.value"), &json!(1));
    assert_eq!(token(base, "Base.border.width.$value.value"), &json!(2));
    assert_eq!(token(base, "Base.opacity.0_5.$value.value"), &json!(0.5));
}

#[test]
fn names_that_collide_after_sanitizing_fail() {
    let snapshot = snapshot_with(json!([
        float("v1", "opacity/0.5", 0.5),
        float("v2", "opacity/0_5", 0.5),
    ]));
    let error = export::documents(&snapshot).err().unwrap();
    assert!(
        matches!(error, Error::DuplicateIdentifier { .. }),
        "{error}"
    );
    assert_eq!(error.exit_code(), 7);
}
