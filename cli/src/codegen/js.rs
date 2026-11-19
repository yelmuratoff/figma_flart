//! JavaScript semantics the 1.1.1 generators relied on, reproduced so the
//! generated Dart stays byte-identical.

use crate::canonical::js_number;
use crate::snapshot::VariableValue;

/// `Math.round`: halves round toward positive infinity.
pub fn round(value: f64) -> f64 {
    let floor = value.floor();
    if value - floor >= 0.5 {
        floor + 1.0
    } else {
        floor
    }
}

/// `padStart(Math.round(channel * 255).toString(16), 2, "0")`.
pub fn to_hex(channel: f64) -> String {
    hex_byte(round(channel * 255.0))
}

/// `padStart(Math.floor(channel * 255).toString(16), 2, "0")`.
pub fn to_hex_floor(channel: f64) -> String {
    hex_byte((channel * 255.0).floor())
}

fn hex_byte(value: f64) -> String {
    format!("{:02x}", value as i64)
}

/// `${value}` for a variable value as JavaScript interpolates it.
pub fn template(value: Option<&VariableValue>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(VariableValue::Number(number)) => js_number(*number),
        Some(VariableValue::Bool(flag)) => flag.to_string(),
        Some(VariableValue::Text(text)) => text.clone(),
        Some(VariableValue::Color(_) | VariableValue::Alias(_)) => "[object Object]".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_hex_matches_javascript() {
        let cases: &[(f64, &str)] = &[
            (0.0, "00"),
            (0.5, "80"),
            (0.07000000029802322, "12"),
            (0.10980392247438431, "1c"),
            (1.0, "ff"),
            (0.0019607843, "00"),
            (0.9980392157, "ff"),
        ];
        for (channel, expected) in cases {
            assert_eq!(to_hex(*channel), *expected, "to_hex({channel})");
        }
    }

    #[test]
    fn round_ties_go_up() {
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(round(0.49999999999999994), 0.0);
    }
}
