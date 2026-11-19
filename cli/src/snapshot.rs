use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::canonical::{canonical_json, checksum};
use crate::error::Error;

pub const SCHEMA_VERSION: u64 = 1;
const PART_MARKER: &str = "snapshot-part";
const SNAPSHOT_MARKER: &str = "snapshot";

/// One `use_figma` call's worth of snapshot data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartKind {
    Styles,
    Collections,
    Remote,
    Variables { collection_id: String },
}

impl FromStr for PartKind {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "styles" => Ok(PartKind::Styles),
            "collections" => Ok(PartKind::Collections),
            "remote" => Ok(PartKind::Remote),
            _ => match text.strip_prefix("variables:") {
                Some(id) if is_collection_id(id) => Ok(PartKind::Variables {
                    collection_id: id.to_string(),
                }),
                _ => Err(format!(
                    "unknown part {text:?}; expected styles, collections, remote, or variables:<collection id>"
                )),
            },
        }
    }
}

fn is_collection_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '/' | '-' | '_'))
}

impl fmt::Display for PartKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PartKind::Styles => f.write_str("styles"),
            PartKind::Collections => f.write_str("collections"),
            PartKind::Remote => f.write_str("remote"),
            PartKind::Variables { collection_id } => write!(f, "variables:{collection_id}"),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub collections: Vec<Collection>,
    pub variables: Vec<Variable>,
    pub text_styles: Vec<TextStyle>,
    pub paint_styles: Vec<PaintStyle>,
    pub effect_styles: Vec<EffectStyle>,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub remote: bool,
    pub default_mode_id: String,
    pub modes: Vec<Mode>,
    pub variable_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mode {
    pub mode_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub id: String,
    pub name: String,
    pub remote: bool,
    pub variable_collection_id: String,
    pub resolved_type: ResolvedType,
    pub values_by_mode: BTreeMap<String, VariableValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResolvedType {
    Color,
    Float,
    String,
    Boolean,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum VariableValue {
    Alias(Alias),
    Color(Rgba),
    Number(f64),
    Bool(bool),
    Text(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Alias {
    #[serde(rename = "type")]
    pub kind: AliasKind,
    pub id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum AliasKind {
    #[serde(rename = "VARIABLE_ALIAS")]
    VariableAlias,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Rgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextStyle {
    pub id: String,
    pub name: String,
    pub font_size: f64,
    pub text_decoration: String,
    pub font_name: FontName,
    pub line_height: LineHeight,
    pub letter_spacing: LetterSpacing,
    #[serde(default)]
    pub bound_variables: BTreeMap<String, Alias>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FontName {
    pub family: String,
    pub style: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct LineHeight {
    pub unit: LineHeightUnit,
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LineHeightUnit {
    Pixels,
    Percent,
    Auto,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct LetterSpacing {
    pub unit: LetterSpacingUnit,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LetterSpacingUnit {
    Pixels,
    Percent,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaintStyle {
    pub id: String,
    pub name: String,
    pub paints: Vec<Paint>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paint {
    #[serde(rename = "type")]
    pub kind: PaintKind,
    pub color: Option<Rgba>,
    pub opacity: Option<f64>,
    pub gradient_stops: Option<Vec<ColorStop>>,
    #[serde(default)]
    pub bound_variables: BTreeMap<String, Alias>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaintKind {
    Solid,
    GradientLinear,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorStop {
    pub color: Rgba,
    pub position: f64,
    #[serde(default)]
    pub bound_variables: BTreeMap<String, Alias>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EffectStyle {
    pub id: String,
    pub name: String,
    pub effects: Vec<Effect>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Effect {
    #[serde(rename = "type")]
    pub kind: EffectKind,
    pub radius: f64,
    pub color: Option<Rgba>,
    pub offset: Option<Offset>,
    pub spread: Option<f64>,
    pub bound_variables: Option<IndexMap<String, Alias>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EffectKind {
    DropShadow,
    InnerShadow,
    LayerBlur,
    BackgroundBlur,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Offset {
    pub x: f64,
    pub y: f64,
}

/// Counts that let a reader confirm the snapshot covers the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub collections: usize,
    pub variables: usize,
    pub aliases: usize,
    pub remote_variables: usize,
    pub text_styles: usize,
    pub paint_styles: usize,
    pub effect_styles: usize,
}

/// A merged snapshot: the raw JSON written to disk and its typed view.
#[derive(Debug, Clone)]
pub struct Assembled {
    pub raw: Value,
    pub snapshot: Snapshot,
}

/// Verifies each part's marker, schema version, and checksum, then merges the
/// parts into one snapshot.
pub fn assemble(parts: &[PathBuf]) -> Result<Assembled, Error> {
    let mut styles = None;
    let mut collections = None;
    let mut remote = None;
    let mut variables: Vec<(String, String, Value)> = Vec::new();

    for path in parts {
        let label = path.display().to_string();
        let (kind, data) = read_part(path)?;
        let slot = match &kind {
            PartKind::Styles => &mut styles,
            PartKind::Collections => &mut collections,
            PartKind::Remote => &mut remote,
            PartKind::Variables { collection_id } => {
                if variables.iter().any(|(id, _, _)| id == collection_id) {
                    return Err(invalid(&label, format!("second part for {kind}")));
                }
                variables.push((collection_id.clone(), label, data));
                continue;
            }
        };
        if slot.is_some() {
            return Err(invalid(&label, format!("second part for {kind}")));
        }
        *slot = Some(data);
    }

    let styles = styles.ok_or_else(|| invalid("snapshot", "missing the styles part"))?;
    let collections =
        collections.ok_or_else(|| invalid("snapshot", "missing the collections part"))?;
    let remote = remote.ok_or_else(|| invalid("snapshot", "missing the remote part"))?;

    let local_collections = array_field(&collections, "collections", "collections part")?;
    let mut merged_variables = Vec::new();
    for collection in &local_collections {
        let id = collection
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("collections part", "a collection has no id"))?;
        let position = variables
            .iter()
            .position(|(collection_id, _, _)| collection_id == id)
            .ok_or_else(|| invalid("snapshot", format!("missing the variables:{id} part")))?;
        let (_, label, data) = variables.remove(position);
        if data.get("collectionId").and_then(Value::as_str) != Some(id) {
            return Err(invalid(&label, format!("data.collectionId is not {id}")));
        }
        merged_variables.extend(array_field(&data, "variables", &label)?);
    }
    if let Some((id, label, _)) = variables.first() {
        return Err(invalid(label, format!("{id} is not a local collection")));
    }

    let mut all_collections = local_collections;
    all_collections.extend(array_field(&remote, "collections", "remote part")?);
    merged_variables.extend(array_field(&remote, "variables", "remote part")?);

    let mut raw = Map::new();
    raw.insert("flart".into(), SNAPSHOT_MARKER.into());
    raw.insert("schemaVersion".into(), SCHEMA_VERSION.into());
    raw.insert("collections".into(), Value::Array(all_collections));
    raw.insert("variables".into(), Value::Array(merged_variables));
    for key in ["textStyles", "paintStyles", "effectStyles"] {
        raw.insert(
            key.into(),
            Value::Array(array_field(&styles, key, "styles part")?),
        );
    }
    raw.insert(
        "missing".into(),
        Value::Array(array_field(&remote, "missing", "remote part")?),
    );
    let raw = Value::Object(raw);
    let snapshot = typed(&raw, "snapshot")?;
    Ok(Assembled { raw, snapshot })
}

/// Reads a snapshot written by `assemble`.
pub fn load(path: &Path) -> Result<Snapshot, Error> {
    let label = path.display().to_string();
    let raw = read_json(path)?;
    if raw.get("flart").and_then(Value::as_str) != Some(SNAPSHOT_MARKER) {
        return Err(invalid(
            &label,
            "not a flart snapshot (flart != \"snapshot\")",
        ));
    }
    check_schema_version(&raw, &label)?;
    typed(&raw, &label)
}

/// Writes the merged snapshot as canonical JSON.
pub fn write(path: &Path, raw: &Value) -> Result<(), Error> {
    let mut text = canonical_json(raw);
    text.push('\n');
    std::fs::write(path, text).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Confirms every alias and variable binding resolves inside the snapshot.
pub fn check(snapshot: &Snapshot) -> Result<Report, Error> {
    if let Some(target) = snapshot.missing.first() {
        return Err(Error::DanglingAlias {
            source_name: "the remote part".into(),
            target: target.clone(),
        });
    }

    let known: HashSet<&str> = snapshot.variables.iter().map(|v| v.id.as_str()).collect();
    let resolve = |source_name: String, target: &str| {
        if known.contains(target) {
            Ok(())
        } else {
            Err(Error::DanglingAlias {
                source_name,
                target: target.to_string(),
            })
        }
    };

    let mut aliases = 0;
    for variable in &snapshot.variables {
        for value in variable.values_by_mode.values() {
            if let VariableValue::Alias(alias) = value {
                resolve(format!("variable {}", variable.name), &alias.id)?;
                if !variable.remote {
                    aliases += 1;
                }
            }
        }
    }
    for style in &snapshot.text_styles {
        for alias in style.bound_variables.values() {
            resolve(format!("text style {}", style.name), &alias.id)?;
        }
    }
    for style in &snapshot.paint_styles {
        for paint in &style.paints {
            let stops = paint.gradient_stops.iter().flatten();
            let bindings = paint
                .bound_variables
                .values()
                .chain(stops.flat_map(|stop| stop.bound_variables.values()));
            for alias in bindings {
                resolve(format!("paint style {}", style.name), &alias.id)?;
            }
        }
    }
    for style in &snapshot.effect_styles {
        for effect in &style.effects {
            for alias in effect.bound_variables.iter().flat_map(IndexMap::values) {
                resolve(format!("effect style {}", style.name), &alias.id)?;
            }
        }
    }

    ensure_no_alias_cycle(snapshot)?;

    let local = snapshot.collections.iter().filter(|c| !c.remote);
    Ok(Report {
        collections: local.clone().count(),
        variables: local.map(|c| c.variable_ids.len()).sum(),
        aliases,
        remote_variables: snapshot.variables.iter().filter(|v| v.remote).count(),
        text_styles: snapshot.text_styles.len(),
        paint_styles: snapshot.paint_styles.len(),
        effect_styles: snapshot.effect_styles.len(),
    })
}

/// Follows each variable's default-mode alias chain, the path generation
/// resolves, and rejects one that returns to a variable it already visited.
fn ensure_no_alias_cycle(snapshot: &Snapshot) -> Result<(), Error> {
    let variables: HashMap<&str, &Variable> = snapshot
        .variables
        .iter()
        .map(|v| (v.id.as_str(), v))
        .collect();
    let default_modes: HashMap<&str, &str> = snapshot
        .collections
        .iter()
        .map(|c| (c.id.as_str(), c.default_mode_id.as_str()))
        .collect();
    fn next<'a>(variable: &'a Variable, default_modes: &HashMap<&str, &str>) -> Option<&'a str> {
        let mode = default_modes.get(variable.variable_collection_id.as_str())?;
        match variable.values_by_mode.get(*mode)? {
            VariableValue::Alias(alias) => Some(alias.id.as_str()),
            _ => None,
        }
    }
    for start in &snapshot.variables {
        let mut path = vec![start.name.as_str()];
        let mut visited = HashSet::from([start.id.as_str()]);
        let mut current: &Variable = start;
        while let Some(&target) = next(current, &default_modes).and_then(|id| variables.get(id)) {
            path.push(target.name.as_str());
            if !visited.insert(target.id.as_str()) {
                return Err(invalid(
                    "snapshot",
                    format!("alias cycle: {}", path.join(" → ")),
                ));
            }
            current = target;
        }
    }
    Ok(())
}

fn read_part(path: &Path) -> Result<(PartKind, Value), Error> {
    let label = path.display().to_string();
    let mut envelope = read_json(path)?;
    if envelope.get("flart").and_then(Value::as_str) != Some(PART_MARKER) {
        return Err(invalid(
            &label,
            "not a flart snapshot part (flart != \"snapshot-part\")",
        ));
    }
    check_schema_version(&envelope, &label)?;
    let kind: PartKind = envelope
        .get("part")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(&label, "missing \"part\""))?
        .parse()
        .map_err(|message: String| invalid(&label, message))?;
    let expected = envelope
        .get("checksum")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(&label, "missing \"checksum\""))?
        .to_string();
    let data = envelope
        .get_mut("data")
        .map(Value::take)
        .filter(Value::is_object)
        .ok_or_else(|| invalid(&label, "missing \"data\" object"))?;
    let actual = checksum(&canonical_json(&data));
    if actual != expected {
        return Err(Error::ChecksumMismatch {
            path: label,
            expected,
            actual,
        });
    }
    Ok((kind, data))
}

fn read_json(path: &Path) -> Result<Value, Error> {
    let text = std::fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&text)
        .map_err(|error| invalid(&path.display().to_string(), format!("not JSON: {error}")))
}

fn check_schema_version(value: &Value, label: &str) -> Result<(), Error> {
    match value.get("schemaVersion").and_then(Value::as_u64) {
        Some(SCHEMA_VERSION) => Ok(()),
        Some(other) => Err(invalid(
            label,
            format!("schemaVersion {other} is not supported; this flart reads {SCHEMA_VERSION}"),
        )),
        None => Err(invalid(label, "missing \"schemaVersion\"")),
    }
}

fn array_field(value: &Value, key: &str, label: &str) -> Result<Vec<Value>, Error> {
    value
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| invalid(label, format!("missing \"{key}\" array")))
}

fn typed(raw: &Value, label: &str) -> Result<Snapshot, Error> {
    Snapshot::deserialize(raw).map_err(|error| invalid(label, error.to_string()))
}

fn invalid(path: &str, message: impl Into<String>) -> Error {
    Error::InvalidSnapshot {
        path: path.to_string(),
        message: message.into(),
    }
}
