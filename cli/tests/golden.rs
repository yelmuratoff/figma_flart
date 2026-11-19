use std::fs;
use std::path::{Path, PathBuf};

use flart::codegen::{self, Kind, Options};
use flart::snapshot;

/// Deliberate departures from 1.1.1 output: (kind, 1.1.1 text, flart text).
const DIVERGENCES: &[(Kind, &str, &str)] = &[(
    Kind::Colors,
    "LinearGradient(colors: [Color(0xff9555e1), Color(0xffe0aa28)])",
    "LinearGradient(colors: [Color(0x129555e1), Color(0x12e0aa28)])",
)];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

struct Variant {
    options: Options,
    modes: ModesFlag,
}

enum ModesFlag {
    None,
    First,
    All,
}

fn parse_variant(stem: &str) -> Variant {
    let mut options = Options::default();
    let mut modes = ModesFlag::None;
    for flag in stem.split('-') {
        match flag {
            "theme_ext_on" => options.theme_extensions = true,
            "font_name_on" => options.include_font_name = true,
            "theme_ext_off" | "font_name_off" | "modes_none" => {}
            "modes_first" => modes = ModesFlag::First,
            "modes_all" => modes = ModesFlag::All,
            other => panic!("unknown golden flag {other:?} in {stem}"),
        }
    }
    Variant { options, modes }
}

fn expected(kind: Kind, golden: &str) -> String {
    let mut text = golden.to_string();
    for (divergent_kind, legacy, current) in DIVERGENCES {
        if *divergent_kind == kind && text.contains(legacy) {
            text = text.replace(legacy, current);
        }
    }
    text
}

fn first_difference(expected: &str, actual: &str) -> String {
    let line = expected
        .lines()
        .zip(actual.lines())
        .position(|(e, a)| e != a)
        .unwrap_or_else(|| expected.lines().count().min(actual.lines().count()));
    format!(
        "first difference at line {}:\n  expected: {:?}\n  actual:   {:?}",
        line + 1,
        expected.lines().nth(line),
        actual.lines().nth(line)
    )
}

fn check_case(case: &str) -> usize {
    let snapshot = snapshot::load(
        &root()
            .join("tests/fixtures")
            .join(case)
            .join("snapshot.json"),
    )
    .expect("snapshot loads");
    let mut compared = 0;
    let mut failures = Vec::new();
    let mut divergences_seen = vec![false; DIVERGENCES.len()];

    for kind in Kind::ALL {
        let dir = root()
            .join("tests/golden")
            .join(case)
            .join(kind.file_stem());
        let mut files: Vec<PathBuf> = fs::read_dir(&dir)
            .unwrap_or_else(|_| panic!("missing {}", dir.display()))
            .map(|entry| entry.unwrap().path())
            .collect();
        files.sort();
        for file in files {
            let stem = file.file_stem().unwrap().to_str().unwrap();
            let variant = parse_variant(stem);
            let mode_ids: Vec<String> = codegen::modes(kind, &snapshot)
                .unwrap_or_default()
                .into_iter()
                .map(|mode| mode.mode_id)
                .collect();
            let selected = match variant.modes {
                ModesFlag::None => None,
                ModesFlag::First => Some(mode_ids[..1].to_vec()),
                ModesFlag::All => Some(mode_ids),
            };
            let golden = fs::read_to_string(&file).unwrap();
            for (position, (divergent_kind, legacy, _)) in DIVERGENCES.iter().enumerate() {
                divergences_seen[position] |= *divergent_kind == kind && golden.contains(legacy);
            }
            let actual = codegen::generate(kind, &snapshot, variant.options, selected.as_deref())
                .expect("fixture names are unique");
            let expected = expected(kind, &golden);
            if actual != expected {
                failures.push(format!(
                    "{case}/{}/{stem}: {}",
                    kind.file_stem(),
                    first_difference(&expected, &actual)
                ));
            }
            compared += 1;
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
    for (seen, (kind, legacy, _)) in divergences_seen.iter().zip(DIVERGENCES) {
        assert!(
            seen,
            "{case}: divergence for {kind:?} never matched {legacy:?}"
        );
    }
    compared
}

#[test]
fn golden_sample() {
    assert_eq!(check_case("sample"), 24);
}

#[test]
fn golden_ah_ds_when_present() {
    if !root().join("tests/fixtures/ah_ds").is_dir() {
        eprintln!("skipped: tests/fixtures/ah_ds is local-only and not present");
        return;
    }
    assert_eq!(check_case("ah_ds"), 20);
}
