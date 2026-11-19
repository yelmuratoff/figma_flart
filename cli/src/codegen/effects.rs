use super::js::to_hex;
use super::naming::{format_class_name, format_style_name};
use super::{HEADER, Index, mode_name};
use crate::canonical::js_number;
use crate::snapshot::{
    Effect, EffectKind, EffectStyle, Mode, Offset, Rgba, Snapshot, VariableValue,
};

pub fn modes(index: &Index, snapshot: &Snapshot) -> Option<Vec<Mode>> {
    snapshot
        .effect_styles
        .iter()
        .flat_map(|style| &style.effects)
        .filter_map(|effect| effect.bound_variables.as_ref())
        .flat_map(|bound| bound.values())
        .find_map(|alias| index.multi_mode_collection_of(alias))
}

pub fn generate(index: &Index, snapshot: &Snapshot, selected: Option<&[String]>) -> String {
    let styles = &snapshot.effect_styles;
    if styles.is_empty() {
        return "No defined effect styles".to_string();
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

fn class(index: &Index, styles: &[EffectStyle], suffix: &str, mode_id: Option<&str>) -> String {
    let mut code = format!("abstract class App{suffix}EffectStyles {{\n");
    for (position, style) in styles.iter().enumerate() {
        let style_name = format_style_name(&style.name, position, "effectStyle");
        for (effect_position, effect) in style.effects.iter().enumerate() {
            let name = format!("{style_name}Effect{effect_position}");
            match effect.kind {
                EffectKind::DropShadow | EffectKind::InnerShadow => {
                    code.push_str(&shadow(index, &name, effect, mode_id));
                }
                EffectKind::LayerBlur | EffectKind::BackgroundBlur => {
                    let radius = match mode_id {
                        Some(id) => js_number(number(index, effect, "radius", id, effect.radius)),
                        None => js_number(effect.radius),
                    };
                    code.push_str(&format!(
                        "  static const double {name}BlurRadius = {radius};\n\n"
                    ));
                }
                EffectKind::Other => {}
            }
        }
    }
    code.push_str("}\n");
    code
}

fn shadow(index: &Index, name: &str, effect: &Effect, mode_id: Option<&str>) -> String {
    let base_color = effect.color.unwrap_or_default();
    let base_offset = effect.offset.unwrap_or(Offset { x: 0.0, y: 0.0 });
    let (color, offset, radius, spread) = match (mode_id, &effect.bound_variables) {
        (Some(id), Some(_)) => (
            mode_color(index, effect, id).unwrap_or(base_color),
            Offset {
                x: number(index, effect, "offsetX", id, base_offset.x),
                y: number(index, effect, "offsetY", id, base_offset.y),
            },
            js_number(number(index, effect, "radius", id, effect.radius)),
            js_number(number(
                index,
                effect,
                "spread",
                id,
                effect.spread.unwrap_or(0.0),
            )),
        ),
        _ => (
            base_color,
            base_offset,
            js_number(effect.radius),
            effect
                .spread
                .map_or_else(|| "undefined".to_string(), js_number),
        ),
    };
    let alpha = color.a.unwrap_or(1.0);
    format!(
        "  static const BoxShadow {name} = BoxShadow(\n    color: Color(0x{}{}{}{}),\n    offset: Offset({}, {}),\n    blurRadius: {radius},\n    spreadRadius: {spread},\n  );\n\n",
        to_hex(alpha),
        to_hex(color.r),
        to_hex(color.g),
        to_hex(color.b),
        js_number(offset.x),
        js_number(offset.y),
    )
}

fn number(index: &Index, effect: &Effect, field: &str, mode_id: &str, fallback: f64) -> f64 {
    bound_value(index, effect, field, mode_id)
        .and_then(|value| match value {
            VariableValue::Number(number) => Some(*number),
            _ => None,
        })
        .unwrap_or(fallback)
}

fn mode_color(index: &Index, effect: &Effect, mode_id: &str) -> Option<Rgba> {
    match bound_value(index, effect, "color", mode_id)? {
        VariableValue::Color(color) => Some(Rgba {
            a: Some(color.a.unwrap_or(1.0)),
            ..*color
        }),
        _ => None,
    }
}

fn bound_value<'a>(
    index: &Index<'a>,
    effect: &Effect,
    field: &str,
    mode_id: &str,
) -> Option<&'a VariableValue> {
    let alias = effect.bound_variables.as_ref()?.get(field)?;
    let variable = index.variable(&alias.id)?;
    index
        .value_in_mode(variable, mode_id)
        .map(|(value, _)| value)
}
