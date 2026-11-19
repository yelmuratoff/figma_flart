use super::js::{template, to_hex};
use super::naming::{
    ByNameKind, by_name_lookup, common_dart_type, format_class_name, format_variable_name,
    to_dart_identifier,
};
use super::{HEADER, Index, Options};
use crate::snapshot::{Collection, Mode, ResolvedType, Rgba, Snapshot, Variable, VariableValue};

pub fn modes(snapshot: &Snapshot) -> Option<Vec<Mode>> {
    local_collections(snapshot)
        .find(|c| c.modes.len() > 1 && !c.variable_ids.is_empty())
        .map(|c| c.modes.clone())
}

pub fn generate(
    index: &Index,
    snapshot: &Snapshot,
    options: Options,
    selected: Option<&[String]>,
) -> String {
    let generator = Generator { index };
    let mut code = HEADER.to_string();
    for collection in local_collections(snapshot) {
        let class = format_class_name(&collection.name);
        let ids = &collection.variable_ids;
        if options.theme_extensions && !ids.is_empty() && collection.modes.len() > 1 {
            code.push_str(&generator.interface(&class, ids));
            code.push_str(&generator.implementation(&class, collection, selected));
        }
        code.push_str(&generator.standard_classes(&class, collection, selected));
    }
    code
}

pub(super) fn local_collections(snapshot: &Snapshot) -> impl Iterator<Item = &Collection> {
    snapshot.collections.iter().filter(|c| !c.remote)
}

/// The Dart field name of a variable, prefixed with its collection name when
/// the bare name is not a usable identifier.
pub(super) fn identifier(index: &Index, variable: &Variable) -> String {
    let collection_name = index
        .collection(&variable.variable_collection_id)
        .map_or("value", |c| c.name.as_str());
    let prefix = match format_variable_name(collection_name) {
        prefix if prefix.is_empty() => "value".to_string(),
        prefix => prefix,
    };
    to_dart_identifier(&format_variable_name(&variable.name), &prefix)
}

fn dart_type(resolved: Option<ResolvedType>) -> &'static str {
    match resolved {
        Some(ResolvedType::Color) => "Color",
        Some(ResolvedType::Float) => "double",
        Some(ResolvedType::String) => "String",
        Some(ResolvedType::Boolean) => "bool",
        None => "dynamic",
    }
}

fn is_lookup_type(dart_type: &str) -> bool {
    dart_type == "Color" || dart_type == "double"
}

fn color_code(color: &Rgba) -> String {
    format!(
        "Color(0x{}{}{}{})",
        to_hex(color.a.unwrap_or(1.0)),
        to_hex(color.r),
        to_hex(color.g),
        to_hex(color.b)
    )
}

fn dart_string_literal(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('$', "\\$")
        .replace('\n', "\\n");
    format!("'{escaped}'")
}

/// `formatValue` after alias resolution.
fn format_value(value: Option<&VariableValue>, resolved: ResolvedType) -> String {
    match (value, resolved) {
        (Some(VariableValue::Color(color)), ResolvedType::Color) => color_code(color),
        (Some(VariableValue::Text(text)), ResolvedType::String) => dart_string_literal(text),
        _ => template(value),
    }
}

/// The selected modes of this collection, or all of them when the selection
/// names none of its modes.
fn select_modes<'a>(collection: &'a Collection, selected: Option<&[String]>) -> Vec<&'a Mode> {
    let chosen: Vec<&Mode> = collection
        .modes
        .iter()
        .filter(|mode| selected.is_some_and(|ids| ids.contains(&mode.mode_id)))
        .collect();
    if chosen.is_empty() {
        collection.modes.iter().collect()
    } else {
        chosen
    }
}

struct Generator<'i, 'a> {
    index: &'i Index<'a>,
}

struct Field<'a> {
    variable: Option<&'a Variable>,
    name: String,
    dart_type: &'static str,
}

impl<'a> Generator<'_, 'a> {
    fn identifier(&self, variable: &Variable) -> String {
        identifier(self.index, variable)
    }

    fn field(&self, id: &str) -> Field<'a> {
        let variable = self.index.variable(id);
        Field {
            variable,
            name: variable.map_or_else(String::new, |v| self.identifier(v)),
            dart_type: dart_type(variable.map(|v| v.resolved_type)),
        }
    }

    fn fields(&self, ids: &[String]) -> Vec<Field<'a>> {
        ids.iter().map(|id| self.field(id)).collect()
    }

    fn lookup(&self, ids: &[String]) -> (Vec<String>, String) {
        let fields: Vec<Field> = self
            .fields(ids)
            .into_iter()
            .filter(|f| f.variable.is_some() && is_lookup_type(f.dart_type))
            .collect();
        let types: Vec<&str> = fields.iter().map(|f| f.dart_type).collect();
        let dart_type = common_dart_type(&types);
        (fields.into_iter().map(|f| f.name).collect(), dart_type)
    }

    fn interface(&self, class: &str, ids: &[String]) -> String {
        let mut code = format!(
            "abstract interface class IApp{class} extends ThemeExtension<IApp{class}> {{\n"
        );
        for field in self.fields(ids) {
            code.push_str(&format!(
                "  abstract final {} {};\n",
                field.dart_type, field.name
            ));
        }
        let (names, dart_type) = self.lookup(ids);
        code.push_str(&format!("  List<Map<String, {dart_type}>> get map;\n"));
        code.push_str(&by_name_lookup(&names, &dart_type, ByNameKind::Abstract));
        code.push_str("}\n\n");
        code
    }

    fn implementation(
        &self,
        class: &str,
        collection: &Collection,
        selected: Option<&[String]>,
    ) -> String {
        let ids = &collection.variable_ids;
        let all = self.fields(ids);
        let present: Vec<&Field> = all.iter().filter(|f| f.variable.is_some()).collect();

        let mut code = format!("@immutable\nfinal class App{class} implements IApp{class} {{\n");
        code.push_str(&format!("  const App{class}({{\n"));
        for field in &present {
            code.push_str(&format!("    required this.{},\n", field.name));
        }
        code.push_str("  });\n\n");
        for field in &present {
            code.push_str(&format!(
                "  @override\n  final {} {};\n",
                field.dart_type, field.name
            ));
        }

        code.push_str(&format!("\n  @override\n  App{class} copyWith({{\n"));
        for field in &present {
            code.push_str(&format!("    {}? {},\n", field.dart_type, field.name));
        }
        code.push_str(&format!("  }}) => App{class}(\n"));
        for field in &present {
            code.push_str(&format!("    {0}: {0} ?? this.{0},\n", field.name));
        }
        code.push_str("  );\n\n");

        code.push_str(&format!(
            "  @override\n  App{class} lerp(App{class}? other, double t) {{\n"
        ));
        code.push_str("    if (other == null) return this;\n");
        code.push_str(&format!("    return App{class}(\n"));
        for field in &present {
            let n = &field.name;
            code.push_str(&match field.dart_type {
                "Color" => format!("      {n}: Color.lerp({n}, other.{n}, t)!,\n"),
                "double" => format!("      {n}: lerpDouble({n}, other.{n}, t)!,\n"),
                "String" | "bool" => format!("      {n}: t < 0.5 ? {n} : other.{n},\n"),
                _ => format!("      {n}: other.{n} ?? this.{n},\n"),
            });
        }
        code.push_str("    );\n  }\n");

        code.push_str("\n  @override\n  bool operator ==(Object other) {\n");
        code.push_str("    if (identical(this, other)) return true;\n");
        code.push_str(&format!("    return other is App{class} &&\n"));
        let conditions: Vec<String> = present
            .iter()
            .map(|f| format!("      other.{0} == {0}", f.name))
            .collect();
        code.push_str(&conditions.join(" &&\n"));
        code.push_str(";\n  }\n");

        code.push_str("\n  @override\n  int get hashCode =>\n    Object.hashAll([\n");
        for field in &present {
            code.push_str(&format!("      {},\n", field.name));
        }
        code.push_str("    ]);\n\n");

        code.push_str("  @override\n  Object get type => runtimeType;\n\n");

        for mode in select_modes(collection, selected) {
            let getter = format_variable_name(&mode.name);
            let mode_class = format!("App{}{class}", format_class_name(&mode.name));
            code.push_str(&format!(
                "  static IApp{class} get {getter}{class} => const App{class}(\n"
            ));
            for field in &all {
                code.push_str(&format!("    {0}: {mode_class}.{0},\n", field.name));
            }
            code.push_str("  );\n\n");
        }

        let (names, dart_type) = self.lookup(ids);
        code.push_str(&format!(
            "  @override\n  List<Map<String, {dart_type}>> get map => [\n"
        ));
        for name in &names {
            code.push_str(&format!("    {{'{name}': {name}}},\n"));
        }
        code.push_str("  ];\n");
        code.push_str(&by_name_lookup(&names, &dart_type, ByNameKind::Override));
        code.push_str("}\n");
        code
    }

    fn standard_classes(
        &self,
        class: &str,
        collection: &Collection,
        selected: Option<&[String]>,
    ) -> String {
        let has_modes = collection.modes.len() > 1;
        let mut code = String::new();
        for mode in select_modes(collection, selected) {
            let mode_class = format!(
                "App{}{class}",
                if has_modes {
                    format_class_name(&mode.name)
                } else {
                    String::new()
                }
            );
            code.push_str(&format!("final class {mode_class} {{\n"));
            code.push_str(&format!("  const {mode_class}._();\n\n"));

            let mut names = Vec::new();
            let mut types = Vec::new();
            for id in &collection.variable_ids {
                let Some(variable) = self.index.variable(id) else {
                    continue;
                };
                let name = self.identifier(variable);
                let raw = variable.values_by_mode.get(&mode.mode_id);
                let (field_type, value) = match raw {
                    Some(VariableValue::Alias(alias)) => match self.index.resolve_alias(alias) {
                        Some((value, resolved)) => (
                            dart_type(Some(resolved)),
                            format_value(Some(value), resolved),
                        ),
                        None => ("dynamic", template(None)),
                    },
                    Some(VariableValue::Color(color)) => ("Color", color_code(color)),
                    _ if variable.resolved_type == ResolvedType::Float => ("double", template(raw)),
                    _ => (
                        dart_type(Some(variable.resolved_type)),
                        format_value(raw, variable.resolved_type),
                    ),
                };
                let mode_suffix = if has_modes {
                    format!(", mode: {}", mode.name)
                } else {
                    String::new()
                };
                code.push_str(&format!(
                    "  /// Name: {}, value: {value}{mode_suffix}\n",
                    variable.name
                ));
                code.push_str(&format!("  static const {field_type} {name} = {value};\n"));
                if is_lookup_type(field_type) {
                    names.push(name);
                    types.push(field_type);
                }
            }

            let dart_type = common_dart_type(&types);
            let entries: Vec<String> = names.iter().map(|n| format!("{{'{n}': {n}}}")).collect();
            code.push_str(&format!(
                "\nstatic List<Map<String, {dart_type}>> get map => [\n    {}\n  ];\n",
                entries.join(",\n    ")
            ));
            code.push_str(&by_name_lookup(&names, &dart_type, ByNameKind::Static));
            code.push_str("}\n\n");
        }
        code
    }
}
