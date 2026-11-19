//! Export to the Design Tokens Community Group format 2025.10: a token file per
//! collection (one per mode when a collection has several), a styles file, and
//! a resolver document that selects one mode per collection.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::codegen::naming::infer_font_weight;
use crate::error::Error;
use crate::generate::{Status, write_if_changed};
use crate::snapshot::{
    Alias, Collection, Effect, EffectKind, EffectStyle, LetterSpacingUnit, LineHeightUnit,
    PaintKind, PaintStyle, ResolvedType, Rgba, Snapshot, TextStyle, Variable, VariableValue,
};

pub const RESOLVER_FILE: &str = "tokens.resolver.json";
const VERSION: &str = "2025.10";
const STYLES_SET: &str = "styles";
const TEXT_STYLES: &str = "text-styles";
const PAINT_STYLES: &str = "paint-styles";
const EFFECT_STYLES: &str = "effect-styles";
const ROOT: &str = "$root";
const MAX_ALIAS_DEPTH: usize = 64;

#[derive(Debug, Clone, Serialize)]
pub struct ExportedFile {
    pub path: PathBuf,
    pub status: Status,
}

/// Something in the snapshot that DTCG 2025.10 has no way to express.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Dropped {
    pub source: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Export {
    pub files: Vec<ExportedFile>,
    pub dropped: Vec<Dropped>,
}

/// Writes the token files and `tokens.resolver.json` under `out`. With
/// `dry_run` the statuses are computed and nothing is written.
pub fn export(snapshot: &Snapshot, out: &Path, dry_run: bool) -> Result<Export, Error> {
    let documents = documents(snapshot)?;
    let mut files = Vec::new();
    for (relative, document) in &documents.files {
        let path = out.join(relative);
        if !dry_run {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|source| Error::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
        }
        let mut text = serde_json::to_string_pretty(document).expect("JSON values serialize");
        text.push('\n');
        let status = write_if_changed(&path, &text, dry_run)?;
        files.push(ExportedFile { path, status });
    }
    Ok(Export {
        files,
        dropped: documents.dropped,
    })
}

/// The exported documents keyed by their path relative to the output directory.
pub struct Documents {
    pub files: Vec<(String, Value)>,
    pub dropped: Vec<Dropped>,
}

pub fn documents(snapshot: &Snapshot) -> Result<Documents, Error> {
    let mut builder = Builder::new(snapshot)?;
    let files = builder.build()?;
    Ok(Documents {
        files,
        dropped: builder.dropped,
    })
}

struct Builder<'a> {
    snapshot: &'a Snapshot,
    variables: HashMap<&'a str, &'a Variable>,
    collections: HashMap<&'a str, &'a Collection>,
    paths: HashMap<String, Vec<String>>,
    dropped: Vec<Dropped>,
}

impl<'a> Builder<'a> {
    fn new(snapshot: &'a Snapshot) -> Result<Self, Error> {
        let mut builder = Self {
            snapshot,
            variables: snapshot
                .variables
                .iter()
                .map(|v| (v.id.as_str(), v))
                .collect(),
            collections: snapshot
                .collections
                .iter()
                .map(|c| (c.id.as_str(), c))
                .collect(),
            paths: HashMap::new(),
            dropped: Vec::new(),
        };
        builder.paths = builder.token_paths()?;
        Ok(builder)
    }

    /// Every exported token's path, keyed by variable or style id. A path that
    /// is also a group gets the `$root` segment so both can coexist.
    fn token_paths(&self) -> Result<HashMap<String, Vec<String>>, Error> {
        let mut entries: Vec<(String, Vec<String>, String)> = Vec::new();
        for variable in self.exportable_variables() {
            let collection = self.collection_name(variable);
            let mut path = vec![segment(collection)];
            path.extend(segments(&variable.name));
            entries.push((variable.id.clone(), path, variable.name.clone()));
        }
        let styles = [
            (
                TEXT_STYLES,
                names(&self.snapshot.text_styles, |s| (&s.id, &s.name)),
            ),
            (
                PAINT_STYLES,
                names(&self.snapshot.paint_styles, |s| (&s.id, &s.name)),
            ),
            (
                EFFECT_STYLES,
                names(&self.snapshot.effect_styles, |s| (&s.id, &s.name)),
            ),
        ];
        for (group, styles) in styles {
            for (id, name) in styles {
                let mut path = vec![group.to_string()];
                path.extend(segments(name));
                entries.push((id.to_string(), path, name.to_string()));
            }
        }

        let mut seen: HashMap<Vec<String>, String> = HashMap::new();
        let mut prefixes: HashSet<Vec<String>> = HashSet::new();
        for (_, path, source) in &entries {
            if let Some(first) = seen.insert(path.clone(), source.clone()) {
                return Err(Error::DuplicateIdentifier {
                    scope: "DTCG token paths".to_string(),
                    name: path.join("."),
                    first,
                    second: source.clone(),
                });
            }
            for end in 1..path.len() {
                prefixes.insert(path[..end].to_vec());
            }
        }
        Ok(entries
            .into_iter()
            .map(|(id, mut path, _)| {
                if prefixes.contains(&path) {
                    path.push(ROOT.to_string());
                }
                (id, path)
            })
            .collect())
    }

    fn exportable_variables(&self) -> impl Iterator<Item = &'a Variable> + use<'a> {
        let snapshot: &'a Snapshot = self.snapshot;
        snapshot
            .variables
            .iter()
            .filter(|v| matches!(v.resolved_type, ResolvedType::Color | ResolvedType::Float))
    }

    fn collection_name(&self, variable: &Variable) -> &'a str {
        self.collections
            .get(variable.variable_collection_id.as_str())
            .map_or("unknown", |c| c.name.as_str())
    }

    fn reference(&self, id: &str) -> Option<String> {
        self.paths
            .get(id)
            .map(|path| format!("{{{}}}", path.join(".")))
    }

    fn record_dropped(&mut self, source: String, reason: impl Into<String>) {
        self.dropped.push(Dropped {
            source,
            reason: reason.into(),
        });
    }

    fn build(&mut self) -> Result<Vec<(String, Value)>, Error> {
        let snapshot: &'a Snapshot = self.snapshot;
        for variable in &snapshot.variables {
            if matches!(
                variable.resolved_type,
                ResolvedType::String | ResolvedType::Boolean
            ) {
                let kind = match variable.resolved_type {
                    ResolvedType::String => "STRING",
                    _ => "BOOLEAN",
                };
                self.record_dropped(
                    format!("variable {}", variable.name),
                    format!("{kind} variables have no DTCG type"),
                );
            }
        }

        let text_style_home = self.text_style_homes();
        let mut files: Vec<(String, Value)> = Vec::new();
        let mut file_names: HashMap<String, String> = HashMap::new();
        let mut sets = Map::new();
        let mut modifiers = Map::new();

        for collection in &snapshot.collections {
            let variables: Vec<&Variable> = self
                .exportable_variables()
                .filter(|v| v.variable_collection_id == collection.id)
                .collect();
            let homed: Vec<&TextStyle> = snapshot
                .text_styles
                .iter()
                .filter(|s| text_style_home.get(s.id.as_str()) == Some(&collection.id.as_str()))
                .collect();
            if variables.is_empty() && homed.is_empty() {
                continue;
            }
            let stem = slug(&collection.name);
            if collection.modes.len() <= 1 {
                let mode_id = collection
                    .modes
                    .first()
                    .map_or(collection.default_mode_id.as_str(), |m| m.mode_id.as_str());
                let tree = self.collection_tree(&variables, &homed, collection, mode_id)?;
                let relative = format!("tokens/{stem}.tokens.json");
                claim(&mut file_names, &relative, &collection.name)?;
                insert_unique(
                    &mut sets,
                    &collection.name,
                    json!({ "sources": [{ "$ref": relative }] }),
                )?;
                files.push((relative, Value::Object(tree)));
            } else {
                let mut contexts = Map::new();
                for mode in &collection.modes {
                    let tree =
                        self.collection_tree(&variables, &homed, collection, &mode.mode_id)?;
                    let relative = format!("tokens/{stem}/{}.tokens.json", slug(&mode.name));
                    claim(&mut file_names, &relative, &mode.name)?;
                    insert_unique(&mut contexts, &mode.name, json!([{ "$ref": relative }]))?;
                    files.push((relative, Value::Object(tree)));
                }
                let default = collection
                    .modes
                    .iter()
                    .find(|m| m.mode_id == collection.default_mode_id)
                    .unwrap_or(&collection.modes[0]);
                insert_unique(
                    &mut modifiers,
                    &collection.name,
                    json!({ "contexts": contexts, "default": default.name }),
                )?;
            }
        }

        let styles = self.styles_tree(&text_style_home);
        if !styles.is_empty() {
            let relative = "tokens/styles.tokens.json".to_string();
            claim(&mut file_names, &relative, STYLES_SET)?;
            insert_unique(
                &mut sets,
                STYLES_SET,
                json!({ "sources": [{ "$ref": relative }] }),
            )?;
            files.push((relative, Value::Object(styles)));
        }

        let order: Vec<Value> = sets
            .keys()
            .map(|name| json!({ "$ref": format!("#/sets/{}", pointer_escape(name)) }))
            .chain(
                modifiers
                    .keys()
                    .map(|name| json!({ "$ref": format!("#/modifiers/{}", pointer_escape(name)) })),
            )
            .collect();
        let resolver = json!({
            "version": VERSION,
            "sets": sets,
            "modifiers": modifiers,
            "resolutionOrder": order,
        });
        files.push((RESOLVER_FILE.to_string(), resolver));
        Ok(files)
    }

    fn collection_tree(
        &mut self,
        variables: &[&'a Variable],
        homed: &[&'a TextStyle],
        collection: &Collection,
        mode_id: &str,
    ) -> Result<Map<String, Value>, Error> {
        let mut tree = Map::new();
        for variable in variables {
            if let Some(token) = self.variable_token(variable, mode_id, collection) {
                insert(&mut tree, &self.paths[&variable.id], token);
            }
        }
        let modes = HashMap::from([(collection.id.as_str(), mode_id)]);
        for style in homed {
            if let Some(token) = self.typography_token(style, &modes) {
                insert(&mut tree, &self.paths[&style.id], token);
            }
        }
        Ok(tree)
    }

    fn variable_token(
        &mut self,
        variable: &Variable,
        mode_id: &str,
        collection: &Collection,
    ) -> Option<Value> {
        let source = || format!("variable {}", variable.name);
        let mode_name = || {
            collection
                .modes
                .iter()
                .find(|m| m.mode_id == mode_id)
                .map_or(mode_id.to_string(), |m| m.name.clone())
        };
        let Some(raw) = variable.values_by_mode.get(mode_id) else {
            let reason = format!("no value in mode {}", mode_name());
            self.record_dropped(source(), reason);
            return None;
        };
        let value = match raw {
            VariableValue::Alias(alias) => match self.reference(&alias.id) {
                Some(reference) => json!(reference),
                None => {
                    self.record_dropped(source(), "aliases a variable that was not exported");
                    return None;
                }
            },
            VariableValue::Color(rgba) => color(*rgba, 1.0),
            VariableValue::Number(number) => dimension(*number),
            VariableValue::Bool(_) | VariableValue::Text(_) => return None,
        };
        let kind = match variable.resolved_type {
            ResolvedType::Color => "color",
            _ => "dimension",
        };
        Some(json!({ "$type": kind, "$value": value }))
    }

    /// Text styles bound to a multi-mode collection live in that collection's
    /// mode files, resolved per mode, because DTCG typography takes a line
    /// height ratio that no reference can express.
    fn text_style_homes(&self) -> HashMap<&'a str, &'a str> {
        let mut homes = HashMap::new();
        for style in &self.snapshot.text_styles {
            let home = style.bound_variables.values().find_map(|alias| {
                let variable = self.variables.get(alias.id.as_str())?;
                let collection = self
                    .collections
                    .get(variable.variable_collection_id.as_str())?;
                (collection.modes.len() > 1).then_some(collection.id.as_str())
            });
            if let Some(home) = home {
                homes.insert(style.id.as_str(), home);
            }
        }
        homes
    }

    fn styles_tree(&mut self, text_style_home: &HashMap<&str, &str>) -> Map<String, Value> {
        let mut tree = Map::new();
        let no_modes = HashMap::new();
        let snapshot = self.snapshot;
        for style in &snapshot.text_styles {
            if text_style_home.contains_key(style.id.as_str()) {
                continue;
            }
            if let Some(token) = self.typography_token(style, &no_modes) {
                insert(&mut tree, &self.paths[&style.id], token);
            }
        }
        for style in &snapshot.paint_styles {
            match self.paint_token(style) {
                Ok(token) => insert(&mut tree, &self.paths[&style.id], token),
                Err(reason) => self.record_dropped(format!("paint style {}", style.name), reason),
            }
        }
        for style in &snapshot.effect_styles {
            if let Some(token) = self.effect_token(style) {
                insert(&mut tree, &self.paths[&style.id], token);
            }
        }
        tree
    }

    /// A variable's value with aliases followed: `modes` picks the mode for a
    /// collection, any other collection is read in its default mode.
    fn resolve(&self, id: &str, modes: &HashMap<&str, &str>) -> Option<&'a VariableValue> {
        let mut variable = *self.variables.get(id)?;
        for _ in 0..MAX_ALIAS_DEPTH {
            let collection = self
                .collections
                .get(variable.variable_collection_id.as_str())?;
            let mode = modes
                .get(collection.id.as_str())
                .copied()
                .unwrap_or(collection.default_mode_id.as_str());
            match variable.values_by_mode.get(mode)? {
                VariableValue::Alias(next) => variable = *self.variables.get(next.id.as_str())?,
                value => return Some(value),
            }
        }
        None
    }

    fn bound_number(
        &self,
        bound: &std::collections::BTreeMap<String, Alias>,
        field: &str,
        modes: &HashMap<&str, &str>,
    ) -> Option<f64> {
        match self.resolve(&bound.get(field)?.id, modes)? {
            VariableValue::Number(number) => Some(*number),
            _ => None,
        }
    }

    fn typography_token(
        &mut self,
        style: &TextStyle,
        modes: &HashMap<&str, &str>,
    ) -> Option<Value> {
        let bound = &style.bound_variables;
        let font_size = self
            .bound_number(bound, "fontSize", modes)
            .unwrap_or(style.font_size);
        let font_family = match bound
            .get("fontFamily")
            .and_then(|alias| self.resolve(&alias.id, modes))
        {
            Some(VariableValue::Text(family)) => family.clone(),
            _ => style.font_name.family.clone(),
        };
        let font_weight = self
            .bound_number(bound, "fontWeight", modes)
            .unwrap_or_else(|| f64::from(infer_font_weight(&style.font_name.style)));
        let line_height = match style.line_height.unit {
            LineHeightUnit::Auto => {
                self.record_dropped(
                    format!("text style {}", style.name),
                    "line height AUTO depends on font metrics and has no DTCG number",
                );
                return None;
            }
            unit => {
                let value = self
                    .bound_number(bound, "lineHeight", modes)
                    .or(style.line_height.value)
                    .unwrap_or(font_size);
                match unit {
                    LineHeightUnit::Percent => value / 100.0,
                    _ => value / font_size,
                }
            }
        };
        let spacing = self
            .bound_number(bound, "letterSpacing", modes)
            .unwrap_or(style.letter_spacing.value);
        let letter_spacing = match style.letter_spacing.unit {
            LetterSpacingUnit::Pixels => spacing,
            LetterSpacingUnit::Percent => spacing / 100.0 * font_size,
        };
        Some(json!({
            "$type": "typography",
            "$value": {
                "fontFamily": font_family,
                "fontSize": dimension(font_size),
                "fontWeight": number(font_weight),
                "letterSpacing": dimension(derived(letter_spacing)),
                "lineHeight": number(derived(line_height)),
            }
        }))
    }

    fn paint_token(&self, style: &PaintStyle) -> Result<Value, String> {
        let [paint] = style.paints.as_slice() else {
            return Err(format!(
                "{} paints; a DTCG token holds one value",
                style.paints.len()
            ));
        };
        let opacity = paint.opacity.unwrap_or(1.0);
        match paint.kind {
            PaintKind::Solid => {
                let rgba = paint.color.ok_or("solid paint without a color")?;
                let value = self.color_or_reference(rgba, opacity, &paint.bound_variables);
                Ok(json!({ "$type": "color", "$value": value }))
            }
            PaintKind::GradientLinear => {
                let stops: Vec<Value> = paint
                    .gradient_stops
                    .iter()
                    .flatten()
                    .map(|stop| {
                        json!({
                            "color": self.color_or_reference(stop.color, opacity, &stop.bound_variables),
                            "position": number(stop.position),
                        })
                    })
                    .collect();
                Ok(json!({ "$type": "gradient", "$value": stops }))
            }
            PaintKind::Other => {
                Err("only solid and linear gradient paints have a DTCG type".into())
            }
        }
    }

    /// The bound variable's reference when the paint adds no opacity of its
    /// own, otherwise the literal color with that opacity applied.
    fn color_or_reference(
        &self,
        rgba: Rgba,
        opacity: f64,
        bound: &std::collections::BTreeMap<String, Alias>,
    ) -> Value {
        match bound.get("color").and_then(|a| self.reference(&a.id)) {
            Some(reference) if opacity == 1.0 => json!(reference),
            _ => color(rgba, opacity),
        }
    }

    fn effect_token(&mut self, style: &EffectStyle) -> Option<Value> {
        let mut shadows = Vec::new();
        for effect in &style.effects {
            match effect.kind {
                EffectKind::DropShadow | EffectKind::InnerShadow => {
                    shadows.push(self.shadow(effect));
                }
                kind => self.record_dropped(
                    format!("effect style {}", style.name),
                    format!("{} effects have no DTCG type", effect_name(kind)),
                ),
            }
        }
        match shadows.len() {
            0 => None,
            1 => Some(json!({ "$type": "shadow", "$value": shadows.remove(0) })),
            _ => Some(json!({ "$type": "shadow", "$value": shadows })),
        }
    }

    fn shadow(&self, effect: &Effect) -> Value {
        let empty = IndexMap::new();
        let bound = effect.bound_variables.as_ref().unwrap_or(&empty);
        let reference = |field: &str| bound.get(field).and_then(|a| self.reference(&a.id));
        let length = |field: &str, value: f64| {
            reference(field).map_or_else(|| dimension(value), Value::String)
        };
        let offset = effect
            .offset
            .unwrap_or(crate::snapshot::Offset { x: 0.0, y: 0.0 });
        let mut shadow = json!({
            "color": reference("color").map_or_else(
                || color(effect.color.unwrap_or_default(), 1.0),
                Value::String,
            ),
            "offsetX": length("offsetX", offset.x),
            "offsetY": length("offsetY", offset.y),
            "blur": length("radius", effect.radius),
            "spread": length("spread", effect.spread.unwrap_or(0.0)),
        });
        if effect.kind == EffectKind::InnerShadow {
            shadow["inset"] = json!(true);
        }
        shadow
    }
}

fn names<T>(items: &[T], key: impl Fn(&T) -> (&String, &String)) -> Vec<(&str, &str)> {
    items
        .iter()
        .map(|item| {
            let (id, name) = key(item);
            (id.as_str(), name.as_str())
        })
        .collect()
}

fn effect_name(kind: EffectKind) -> &'static str {
    match kind {
        EffectKind::DropShadow => "DROP_SHADOW",
        EffectKind::InnerShadow => "INNER_SHADOW",
        EffectKind::LayerBlur => "LAYER_BLUR",
        EffectKind::BackgroundBlur => "BACKGROUND_BLUR",
        EffectKind::Other => "other",
    }
}

/// Figma's `/`-separated name as DTCG path segments. DTCG forbids `.`, `{`
/// and `}` in names and reserves a leading `$`.
fn segments(name: &str) -> Vec<String> {
    name.split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(segment)
        .collect()
}

fn segment(name: &str) -> String {
    let replaced: String = name
        .trim()
        .chars()
        .map(|ch| {
            if matches!(ch, '.' | '{' | '}') {
                '_'
            } else {
                ch
            }
        })
        .collect();
    match replaced.strip_prefix('$') {
        Some(rest) => format!("_{rest}"),
        None => replaced,
    }
}

fn slug(name: &str) -> String {
    let mut slug = String::new();
    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_alphanumeric() {
            slug.push(ch);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-').to_string();
    if slug.is_empty() {
        "unnamed".to_string()
    } else {
        slug
    }
}

fn pointer_escape(name: &str) -> String {
    name.replace('~', "~0").replace('/', "~1")
}

fn claim(names: &mut HashMap<String, String>, path: &str, source: &str) -> Result<(), Error> {
    if let Some(first) = names.insert(path.to_string(), source.to_string()) {
        return Err(Error::DuplicateIdentifier {
            scope: "DTCG files".to_string(),
            name: path.to_string(),
            first,
            second: source.to_string(),
        });
    }
    Ok(())
}

fn insert_unique(map: &mut Map<String, Value>, key: &str, value: Value) -> Result<(), Error> {
    if map.contains_key(key) {
        return Err(Error::DuplicateIdentifier {
            scope: "DTCG resolver".to_string(),
            name: key.to_string(),
            first: key.to_string(),
            second: key.to_string(),
        });
    }
    map.insert(key.to_string(), value);
    Ok(())
}

fn insert(tree: &mut Map<String, Value>, path: &[String], token: Value) {
    let (last, groups) = path.split_last().expect("token paths are never empty");
    let mut node = tree;
    for group in groups {
        node = node
            .entry(group.clone())
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .expect("token paths that are prefixes end in $root");
    }
    node.insert(last.clone(), token);
}

fn color(rgba: Rgba, opacity: f64) -> Value {
    let alpha = rgba.a.unwrap_or(1.0) * opacity;
    let hex: String = [rgba.r, rgba.g, rgba.b]
        .iter()
        .map(|channel| format!("{:02x}", (channel.clamp(0.0, 1.0) * 255.0).round() as u8))
        .collect();
    json!({
        "colorSpace": "srgb",
        "components": [number(rgba.r), number(rgba.g), number(rgba.b)],
        "alpha": number(alpha),
        "hex": format!("#{hex}"),
    })
}

fn dimension(value: f64) -> Value {
    json!({ "value": number(value), "unit": "px" })
}

/// Integers print without a fraction so `16` does not become `16.0`.
fn number(value: f64) -> Value {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        json!(value as i64)
    } else {
        json!(value)
    }
}

/// Values flart computes (ratios, percent of a font size) carry float noise
/// such as `-1.9200000000000002`; four decimals keep them readable.
fn derived(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_replace_forbidden_characters() {
        assert_eq!(segments("opacity/0.5"), ["opacity", "0_5"]);
        assert_eq!(segments(" Body / Large "), ["Body", "Large"]);
        assert_eq!(segments("$private/{x}"), ["_private", "_x_"]);
    }

    #[test]
    fn slug_is_lowercase_and_dashed() {
        assert_eq!(slug("Theme"), "theme");
        assert_eq!(slug("AH DS / Size"), "ah-ds-size");
        assert_eq!(slug("--"), "unnamed");
    }

    #[test]
    fn color_carries_alpha_and_hex() {
        let value = color(
            Rgba {
                r: 1.0,
                g: 0.5,
                b: 0.0,
                a: Some(0.5),
            },
            0.5,
        );
        assert_eq!(value["hex"], "#ff8000");
        assert_eq!(value["alpha"], json!(0.25));
        assert_eq!(value["components"], json!([1, 0.5, 0]));
    }
}
