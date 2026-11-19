//! Name formatters ported from 1.1.1 `utils.ts`.

const DART_RESERVED_WORDS: &[&str] = &[
    "assert", "break", "case", "catch", "class", "const", "continue", "default", "do", "else",
    "enum", "extends", "false", "final", "finally", "for", "if", "in", "is", "new", "null",
    "rethrow", "return", "super", "switch", "this", "throw", "true", "try", "var", "void", "while",
    "with",
];

const GENERATED_MEMBER_NAMES: &[&str] = &[
    "map",
    "byName",
    "type",
    "copyWith",
    "lerp",
    "hashCode",
    "runtimeType",
    "toString",
    "noSuchMethod",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByNameKind {
    Static,
    Abstract,
    Override,
    Instance,
}

/// `name.replace(/[^a-zA-Z0-9]+(.)/g, (m, chr) => chr.toUpperCase())`.
fn camel_join(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_alphanumeric() {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let mut run_end = i;
        while run_end < chars.len() && !chars[run_end].is_ascii_alphanumeric() {
            run_end += 1;
        }
        if run_end < chars.len() {
            out.extend(chars[run_end].to_uppercase());
            i = run_end + 1;
        } else if run_end - i >= 2 {
            out.extend(chars[run_end - 1].to_uppercase());
            i = run_end;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn lower_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// `capitalize` and `capitalizeFirstLetter`.
pub fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn remove_spaces_and_digits(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_ascii_digit() && !ch.is_whitespace())
        .collect()
}

pub fn format_variable_name(name: &str) -> String {
    lower_first(&camel_join(name))
}

pub fn format_class_name(name: &str) -> String {
    remove_spaces_and_digits(&capitalize(&camel_join(name)))
}

/// `formatStyleName`, `formatColorName`, and `formatEffectStyleName`, which
/// differ only in the fallback prefix for an empty name.
pub fn format_style_name(name: &str, index: usize, fallback_prefix: &str) -> String {
    if name.is_empty() {
        return format!("{fallback_prefix}{index}");
    }
    let replaced: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == ' ' {
                ch
            } else {
                ' '
            }
        })
        .collect();
    replaced
        .split_whitespace()
        .enumerate()
        .map(|(position, word)| {
            if position == 0 {
                word.to_lowercase()
            } else {
                capitalize(word)
            }
        })
        .collect()
}

pub fn to_dart_identifier(name: &str, prefix: &str) -> String {
    let needs_prefix = name.is_empty()
        || name.starts_with(|ch: char| ch.is_ascii_digit())
        || DART_RESERVED_WORDS.contains(&name)
        || GENERATED_MEMBER_NAMES.contains(&name);
    if needs_prefix {
        format!("{prefix}{}", capitalize(name))
    } else {
        name.to_string()
    }
}

pub fn infer_font_weight(style: &str) -> u32 {
    match style {
        "Black" => 900,
        "Extra Bold" | "Heavy" => 800,
        "Bold" => 700,
        "Semi Bold" | "DemiBold" => 600,
        "Medium" => 500,
        "Light" => 300,
        "Extra Light" | "Ultra Light" => 200,
        "Thin" | "Hairline" => 100,
        _ => 400,
    }
}

pub fn infer_font_style(style: &str) -> &'static str {
    if style.contains("Italic") || style.contains("Oblique") {
        "FontStyle.italic"
    } else {
        "FontStyle.normal"
    }
}

pub fn text_decoration(decoration: &str) -> &'static str {
    match decoration.to_lowercase().as_str() {
        "underline" => "TextDecoration.underline",
        "overline" => "TextDecoration.overline",
        "line-through" => "TextDecoration.lineThrough",
        _ => "TextDecoration.none",
    }
}

pub fn nullable_dart_type(dart_type: &str) -> String {
    if dart_type == "dynamic" {
        dart_type.to_string()
    } else {
        format!("{dart_type}?")
    }
}

pub fn common_dart_type(dart_types: &[&str]) -> String {
    let mut unique: Vec<&str> = Vec::new();
    for dart_type in dart_types {
        if !unique.contains(dart_type) {
            unique.push(dart_type);
        }
    }
    match unique.as_slice() {
        [only] => only.to_string(),
        _ if unique.contains(&"dynamic") => "dynamic".to_string(),
        _ => "Object".to_string(),
    }
}

pub fn by_name_lookup(names: &[String], dart_type: &str, kind: ByNameKind) -> String {
    let signature = format!("{} byName(String $name)", nullable_dart_type(dart_type));
    if kind == ByNameKind::Abstract {
        return format!("  {signature};\n");
    }
    let prefix = match kind {
        ByNameKind::Static => "static ",
        ByNameKind::Override => "@override\n  ",
        ByNameKind::Instance | ByNameKind::Abstract => "",
    };
    let mut code = format!("\n  {prefix}{signature} => switch ($name) {{\n");
    for name in names {
        code.push_str(&format!("    '{name}' => {name},\n"));
    }
    code.push_str("    _ => null,\n  };\n");
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_javascript() {
        let cases: &[(&str, &str, &str, &str, &str)] = &[
            ("1", "1", "", "1", "size1"),
            (
                "text/strong",
                "textStrong",
                "TextStrong",
                "textStrong",
                "textStrong",
            ),
            ("a--b", "aB", "AB", "aB", "aB"),
            ("a-/", "a/", "A/", "a", "a/"),
            ("a-", "a-", "A-", "a", "a-"),
            ("-lead", "lead", "Lead", "lead", "lead"),
            ("цвет/фон", "н", "Н", "", "н"),
            ("Hub Light", "hubLight", "HubLight", "hubLight", "hubLight"),
            ("zero_dark", "zeroDark", "ZeroDark", "zeroDark", "zeroDark"),
            (
                "spacing/2xl",
                "spacing2xl",
                "Spacingxl",
                "spacing2xl",
                "spacing2xl",
            ),
            ("default", "default", "Default", "default", "sizeDefault"),
            ("", "", "", "textStyle7", "size"),
            ("Heading 1", "heading1", "Heading", "heading1", "heading1"),
            ("!!!", "!", "!", "", "!"),
        ];
        for (name, variable, class, style, ident) in cases {
            assert_eq!(format_variable_name(name), *variable, "variable {name:?}");
            assert_eq!(format_class_name(name), *class, "class {name:?}");
            assert_eq!(
                format_style_name(name, 7, "textStyle"),
                *style,
                "style {name:?}"
            );
            assert_eq!(
                to_dart_identifier(&format_variable_name(name), "size"),
                *ident,
                "ident {name:?}"
            );
        }
    }

    #[test]
    fn font_weight_matches_javascript() {
        let cases = [
            ("Black", 900),
            ("DemiBold", 600),
            ("Semi Bold", 600),
            ("Semi Bold Italic", 400),
            ("Thin", 100),
            ("Regular", 400),
        ];
        for (style, weight) in cases {
            assert_eq!(infer_font_weight(style), weight, "{style}");
        }
    }
}
