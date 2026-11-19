use super::js::round;
use super::naming::{
    ByNameKind, by_name_lookup, format_class_name, format_style_name, infer_font_style,
    infer_font_weight, text_decoration,
};
use super::{HEADER, Index, Options, mode_name};
use crate::canonical::js_number;
use crate::snapshot::{
    LetterSpacingUnit, LineHeightUnit, Mode, Snapshot, TextStyle, VariableValue,
};

const MODE_FIELDS: [&str; 6] = [
    "fontSize",
    "lineHeight",
    "letterSpacing",
    "fontFamily",
    "fontStyle",
    "fontWeight",
];

const BLOCK_INDENT: &str = "        ";

struct Props {
    font_size: f64,
    font_style: &'static str,
    font_weight: u32,
    letter_spacing: f64,
    font_family: String,
    line_height: Option<f64>,
    decoration: &'static str,
}

pub fn modes(index: &Index, snapshot: &Snapshot) -> Option<Vec<Mode>> {
    snapshot.text_styles.iter().find_map(|style| {
        MODE_FIELDS
            .iter()
            .filter_map(|field| style.bound_variables.get(*field))
            .find_map(|alias| index.multi_mode_collection_of(alias))
    })
}

pub fn generate(
    index: &Index,
    snapshot: &Snapshot,
    options: Options,
    selected: Option<&[String]>,
) -> String {
    let styles = &snapshot.text_styles;
    if styles.is_empty() {
        return "No defined textstyles".to_string();
    }

    let mut code = HEADER.to_string();
    match selected {
        Some(mode_ids) => {
            let all_modes = modes(index, snapshot);
            let suffix_by_mode = mode_ids.len() > 1;
            for mode_id in mode_ids {
                let name = mode_name(all_modes.as_deref(), mode_id);
                let suffix = if suffix_by_mode {
                    format_class_name(&name)
                } else {
                    String::new()
                };
                let props: Vec<Props> = styles
                    .iter()
                    .map(|style| props_for_mode(index, style, mode_id))
                    .collect();
                code.push_str(&render(styles, &props, &suffix, options));
                code.push('\n');
            }
        }
        None => {
            let props: Vec<Props> = styles.iter().map(props).collect();
            code.push_str(&render(styles, &props, "", options));
        }
    }
    code
}

fn render(styles: &[TextStyle], props: &[Props], suffix: &str, options: Options) -> String {
    let names: Vec<String> = styles
        .iter()
        .enumerate()
        .map(|(position, style)| format_style_name(&style.name, position, "textStyle"))
        .collect();
    if options.theme_extensions {
        theme_extension(&names, props, suffix, options.include_font_name)
    } else {
        abstract_class(&names, props, suffix, options.include_font_name)
    }
}

fn theme_extension(names: &[String], props: &[Props], suffix: &str, font_name: bool) -> String {
    let class = format!("App{suffix}TextTheme");
    let mut code = format!("@immutable\nclass {class} extends ThemeExtension<{class}> {{\n");
    for name in names {
        code.push_str(&format!("  final TextStyle? {name};\n"));
    }
    code.push_str(&format!("\n   const {class}({{\n"));
    for name in names {
        code.push_str(&format!("    this.{name},\n"));
    }
    code.push_str("  });\n\n");
    code.push_str(&format!("  const {class}.fallback()\n      : this(\n"));
    for (name, props) in names.iter().zip(props) {
        code.push_str(&block(name, props, font_name));
    }
    code.push_str("      );\n\n");
    code.push_str(&format!("  @override\n  {class} copyWith({{\n"));
    for name in names {
        code.push_str(&format!("    TextStyle? {name},\n"));
    }
    code.push_str(&format!("  }}) {{\n    return {class}(\n"));
    for name in names {
        code.push_str(&format!("      {name}: {name} ?? this.{name},\n"));
    }
    code.push_str("    );\n  }\n\n");
    code.push_str(&format!(
        "  @override\n  {class} lerp({class}? other, double t) {{\n"
    ));
    code.push_str(&format!("    if (other is! {class}) return this;\n"));
    code.push_str(&format!("    return {class}(\n"));
    for name in names {
        code.push_str(&format!(
            "      {name}: TextStyle.lerp({name}, other.{name}, t),\n"
        ));
    }
    code.push_str("    );\n  }\n");
    code.push_str(&by_name_lookup(names, "TextStyle", ByNameKind::Instance));
    code.push_str("}\n");
    code
}

fn abstract_class(names: &[String], props: &[Props], suffix: &str, font_name: bool) -> String {
    let mut code = format!("abstract class App{suffix}TextStyles {{\n");
    for (name, props) in names.iter().zip(props) {
        code.push_str(&static_style(name, props, font_name));
    }
    code.push_str("  \n  static List<Map<String, TextStyle>> get map => [\n");
    for name in names {
        code.push_str(&format!("    {{'{name}': {name}}},\n"));
    }
    code.push_str("  ];\n");
    code.push_str(&by_name_lookup(names, "TextStyle", ByNameKind::Static));
    code.push_str("}\n");
    code
}

fn static_style(name: &str, props: &Props, font_name: bool) -> String {
    let mut code = format!("  static const TextStyle {name} = TextStyle(\n");
    code.push_str(&body(props, font_name, "    "));
    code.push_str("  );\n\n");
    code
}

fn block(name: &str, props: &Props, font_name: bool) -> String {
    let mut code = format!("{BLOCK_INDENT}{name}: const TextStyle(\n");
    code.push_str(&body(props, font_name, &format!("{BLOCK_INDENT}  ")));
    code.push_str(&format!("{BLOCK_INDENT}),\n"));
    code
}

fn body(props: &Props, font_name: bool, indent: &str) -> String {
    let mut code = format!("{indent}fontSize: {},\n", js_number(props.font_size));
    code.push_str(&format!(
        "{indent}fontWeight: FontWeight.w{},\n",
        props.font_weight
    ));
    if font_name {
        code.push_str(&format!("{indent}fontFamily: '{}',\n", props.font_family));
    }
    if let Some(line_height) = props.line_height {
        let height = round((line_height / props.font_size) * 100.0) / 100.0;
        code.push_str(&format!("{indent}height: {},\n", js_number(height)));
    }
    if props.letter_spacing != 0.0 {
        code.push_str(&format!(
            "{indent}letterSpacing: {},\n",
            js_number(props.letter_spacing)
        ));
    }
    code.push_str(&format!("{indent}fontStyle: {},\n", props.font_style));
    code.push_str(&format!("{indent}decoration: {},\n", props.decoration));
    code
}

fn letter_spacing(style: &TextStyle, font_size: f64) -> f64 {
    let spacing = style.letter_spacing;
    match spacing.unit {
        LetterSpacingUnit::Pixels => spacing.value,
        LetterSpacingUnit::Percent => (spacing.value * font_size) / 100.0,
    }
}

fn style_line_height(style: &TextStyle) -> Option<f64> {
    match style.line_height.unit {
        LineHeightUnit::Auto => None,
        _ => style.line_height.value,
    }
}

fn props(style: &TextStyle) -> Props {
    Props {
        font_size: style.font_size,
        font_style: infer_font_style(&style.font_name.style),
        font_weight: infer_font_weight(&style.font_name.style),
        letter_spacing: letter_spacing(style, style.font_size),
        font_family: style.font_name.family.clone(),
        line_height: style_line_height(style),
        decoration: text_decoration(&style.text_decoration),
    }
}

fn props_for_mode(index: &Index, style: &TextStyle, mode_id: &str) -> Props {
    let bound = |field: &str| -> Option<Option<&VariableValue>> {
        let alias = style.bound_variables.get(field)?;
        let variable = index.variable(&alias.id)?;
        Some(
            index
                .value_in_mode(variable, mode_id)
                .map(|(value, _)| value),
        )
    };
    let number = |value: Option<&VariableValue>| match value {
        Some(VariableValue::Number(number)) => Some(*number),
        _ => None,
    };

    let font_size = bound("fontSize")
        .and_then(number)
        .unwrap_or(style.font_size);
    let line_height = match bound("lineHeight") {
        Some(value) => number(value),
        None => style_line_height(style),
    };
    let letter_spacing = match bound("letterSpacing") {
        Some(value) => number(value).unwrap_or(0.0),
        None => letter_spacing(style, font_size),
    };
    let font_family = match bound("fontFamily").flatten() {
        Some(VariableValue::Text(family)) => family.clone(),
        _ => style.font_name.family.clone(),
    };
    Props {
        font_size,
        font_style: infer_font_style(&style.font_name.style),
        font_weight: infer_font_weight(&style.font_name.style),
        letter_spacing,
        font_family,
        line_height,
        decoration: text_decoration(&style.text_decoration),
    }
}
