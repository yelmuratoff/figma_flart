use super::js::{to_hex, to_hex_floor};
use super::naming::{format_class_name, format_style_name};
use super::{HEADER, Index, mode_name};
use crate::snapshot::{Mode, Paint, PaintKind, PaintStyle, Snapshot, VariableValue};

pub fn modes(index: &Index, snapshot: &Snapshot) -> Option<Vec<Mode>> {
    snapshot.paint_styles.iter().find_map(|style| {
        let alias = style.paints.first()?.bound_variables.get("color")?;
        index.multi_mode_collection_of(alias)
    })
}

pub fn generate(index: &Index, snapshot: &Snapshot, selected: Option<&[String]>) -> String {
    let styles = &snapshot.paint_styles;
    if styles.is_empty() {
        return "No defined colors".to_string();
    }
    if styles.iter().any(|style| style.paints.is_empty()) {
        return String::new();
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
                code.push_str(&class(index, styles, &suffix, Some(mode_id)));
                code.push('\n');
            }
        }
        None => code.push_str(&class(index, styles, "", None)),
    }
    code
}

fn class(index: &Index, styles: &[PaintStyle], suffix: &str, mode_id: Option<&str>) -> String {
    let mut code = format!("abstract class App{suffix}Colors {{\n");
    for (position, style) in styles.iter().enumerate() {
        let name = format_style_name(&style.name, position, "color");
        let paint = &style.paints[0];
        match paint.kind {
            PaintKind::Solid => {
                let base = paint.color.unwrap_or_default();
                let (r, g, b, opacity) = match mode_id.and_then(|id| mode_color(index, paint, id)) {
                    Some((r, g, b, a)) => (r, g, b, a),
                    None => (base.r, base.g, base.b, opacity_or_one(paint)),
                };
                code.push_str(&format!(
                    "  static const Color {name} = Color(0x{}{}{}{});\n\n",
                    to_hex_floor(opacity),
                    to_hex_floor(r),
                    to_hex_floor(g),
                    to_hex_floor(b)
                ));
            }
            PaintKind::GradientLinear => {
                let stops: Vec<String> = paint
                    .gradient_stops
                    .iter()
                    .flatten()
                    .map(|stop| {
                        let color = stop.color;
                        format!(
                            "Color(0x{}{}{}{})",
                            to_hex(color.a.unwrap_or(1.0)),
                            to_hex(color.r),
                            to_hex(color.g),
                            to_hex(color.b)
                        )
                    })
                    .collect();
                code.push_str(&format!(
                    "  static const {name} = LinearGradient(colors: [{}]);\n\n",
                    stops.join(", ")
                ));
            }
            PaintKind::Other => {}
        }
    }
    code.push_str("}\n");
    code
}

/// `paint.opacity || 1`: a missing or zero opacity becomes 1.
fn opacity_or_one(paint: &Paint) -> f64 {
    paint
        .opacity
        .filter(|opacity| *opacity != 0.0)
        .unwrap_or(1.0)
}

fn mode_color(index: &Index, paint: &Paint, mode_id: &str) -> Option<(f64, f64, f64, f64)> {
    let variable = index.variable(&paint.bound_variables.get("color")?.id)?;
    match index.value_in_mode(variable, mode_id)? {
        (VariableValue::Color(color), _) => {
            Some((color.r, color.g, color.b, color.a.unwrap_or(1.0)))
        }
        _ => None,
    }
}
