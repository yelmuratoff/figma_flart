use flart::codegen::{self, Kind, Options};
use flart::{Error, Snapshot, snapshot};
use serde::Deserialize;
use serde_json::{Value, json};

fn alias(id: &str) -> Value {
    json!({ "type": "VARIABLE_ALIAS", "id": id })
}

fn rgba(r: f64, g: f64, b: f64, a: f64) -> Value {
    json!({ "r": r, "g": g, "b": b, "a": a })
}

fn collection(id: &str, name: &str, remote: bool, modes: &[(&str, &str)], ids: &[&str]) -> Value {
    json!({
        "id": id,
        "name": name,
        "remote": remote,
        "defaultModeId": modes[0].0,
        "modes": modes.iter().map(|(id, name)| json!({ "modeId": id, "name": name })).collect::<Vec<_>>(),
        "variableIds": ids,
    })
}

fn variable(id: &str, name: &str, collection: &str, kind: &str, values: Value) -> Value {
    json!({
        "id": id,
        "name": name,
        "remote": !collection.starts_with("local"),
        "variableCollectionId": collection,
        "resolvedType": kind,
        "valuesByMode": values,
    })
}

fn snapshot_of(
    collections: Vec<Value>,
    variables: Vec<Value>,
    paint_styles: Vec<Value>,
) -> Snapshot {
    Snapshot::deserialize(json!({
        "collections": collections,
        "variables": variables,
        "textStyles": [],
        "paintStyles": paint_styles,
        "effectStyles": [],
        "missing": [],
    }))
    .unwrap()
}

fn variables_dart(snapshot: &Snapshot) -> String {
    codegen::generate(Kind::Variables, snapshot, Options::default(), None).unwrap()
}

#[test]
fn alias_into_multimode_collection_uses_its_default_mode() {
    let mut primitives = collection(
        "lib",
        "Primitives",
        true,
        &[("p:0", "Light"), ("p:1", "Dark")],
        &[],
    );
    primitives["defaultModeId"] = json!("p:1");
    let snapshot = snapshot_of(
        vec![
            collection("local:theme", "Theme", false, &[("1:0", "light")], &["t1"]),
            primitives,
        ],
        vec![
            variable(
                "t1",
                "text/primary",
                "local:theme",
                "COLOR",
                json!({ "1:0": alias("gray") }),
            ),
            variable(
                "gray",
                "gray/900",
                "lib",
                "COLOR",
                json!({ "p:0": rgba(0.0, 0.0, 0.0, 1.0), "p:1": rgba(1.0, 1.0, 1.0, 1.0) }),
            ),
        ],
        vec![],
    );

    let dart = variables_dart(&snapshot);
    assert!(
        dart.contains("static const Color textPrimary = Color(0xffffffff);"),
        "{dart}"
    );
}

#[test]
fn alias_chain_resolves_to_the_final_value() {
    let snapshot = snapshot_of(
        vec![
            collection("local:theme", "Theme", false, &[("1:0", "light")], &["t1"]),
            collection("lib", "Palette", true, &[("1:0", "Default")], &[]),
        ],
        vec![
            variable(
                "t1",
                "text/primary",
                "local:theme",
                "COLOR",
                json!({ "1:0": alias("a") }),
            ),
            variable(
                "a",
                "semantic/ink",
                "lib",
                "COLOR",
                json!({ "1:0": alias("b") }),
            ),
            variable(
                "b",
                "gray/900",
                "lib",
                "COLOR",
                json!({ "1:0": rgba(0.10980392247438431, 0.10980392247438431, 0.10980392247438431, 1.0) }),
            ),
        ],
        vec![],
    );

    let dart = variables_dart(&snapshot);
    assert!(
        dart.contains("static const Color textPrimary = Color(0xff1c1c1c);"),
        "{dart}"
    );
}

#[test]
fn numeric_and_keyword_names_get_the_collection_prefix() {
    let ids = ["s1", "s2", "s3"];
    let snapshot = snapshot_of(
        vec![collection(
            "local:size",
            "Size",
            false,
            &[("2:0", "Desktop")],
            &ids,
        )],
        vec![
            variable("s1", "1", "local:size", "FLOAT", json!({ "2:0": 4 })),
            variable("s2", "default", "local:size", "FLOAT", json!({ "2:0": 8 })),
            variable("s3", "map", "local:size", "FLOAT", json!({ "2:0": 12 })),
        ],
        vec![],
    );

    let dart = variables_dart(&snapshot);
    for line in [
        "static const double size1 = 4;",
        "static const double sizeDefault = 8;",
        "static const double sizeMap = 12;",
    ] {
        assert!(dart.contains(line), "missing {line:?} in\n{dart}");
    }
}

#[test]
fn duplicate_identifiers_fail() {
    let snapshot = snapshot_of(
        vec![collection(
            "local:theme",
            "Theme",
            false,
            &[("1:0", "light")],
            &["t1", "t2"],
        )],
        vec![
            variable(
                "t1",
                "text/primary",
                "local:theme",
                "COLOR",
                json!({ "1:0": rgba(0.0, 0.0, 0.0, 1.0) }),
            ),
            variable(
                "t2",
                "text-primary",
                "local:theme",
                "COLOR",
                json!({ "1:0": rgba(1.0, 1.0, 1.0, 1.0) }),
            ),
        ],
        vec![],
    );

    let error =
        codegen::generate(Kind::Variables, &snapshot, Options::default(), None).unwrap_err();
    assert!(matches!(error, Error::DuplicateIdentifier { .. }));
    let message = error.to_string();
    assert!(message.contains("\"text/primary\""), "{message}");
    assert!(message.contains("\"text-primary\""), "{message}");
    assert!(message.contains("textPrimary"), "{message}");
    assert_eq!(error.exit_code(), 7);
}

#[test]
fn alias_cycle_fails_the_check() {
    let snapshot = snapshot_of(
        vec![
            collection("local:theme", "Theme", false, &[("1:0", "light")], &["t1"]),
            collection("lib", "Palette", true, &[("1:0", "Default")], &[]),
        ],
        vec![
            variable(
                "t1",
                "text/primary",
                "local:theme",
                "COLOR",
                json!({ "1:0": alias("a") }),
            ),
            variable("a", "a", "lib", "COLOR", json!({ "1:0": alias("b") })),
            variable("b", "b", "lib", "COLOR", json!({ "1:0": alias("a") })),
        ],
        vec![],
    );

    let error = snapshot::check(&snapshot).unwrap_err();
    assert_eq!(error.exit_code(), 3);
    assert!(error.to_string().contains("alias cycle"), "{error}");
}

#[test]
fn paint_mode_color_follows_alias() {
    let snapshot = snapshot_of(
        vec![
            collection(
                "local:theme",
                "Theme",
                false,
                &[("1:0", "light"), ("1:1", "dark")],
                &["t1"],
            ),
            collection("lib", "Palette", true, &[("1:0", "Default")], &[]),
        ],
        vec![
            variable(
                "t1",
                "surface/brand",
                "local:theme",
                "COLOR",
                json!({ "1:0": alias("white"), "1:1": rgba(0.0, 0.0, 0.0, 1.0) }),
            ),
            variable(
                "white",
                "gray/50",
                "lib",
                "COLOR",
                json!({ "1:0": rgba(1.0, 1.0, 1.0, 1.0) }),
            ),
        ],
        vec![json!({
            "id": "S:brand,",
            "name": "brand",
            "paints": [{
                "type": "SOLID",
                "opacity": 1,
                "color": { "r": 0.5, "g": 0.5, "b": 0.5 },
                "boundVariables": { "color": alias("t1") },
            }],
        })],
    );

    let light = vec!["1:0".to_string()];
    let dart =
        codegen::generate(Kind::Colors, &snapshot, Options::default(), Some(&light)).unwrap();
    assert!(
        dart.contains("static const Color brand = Color(0xffffffff);"),
        "{dart}"
    );
}
