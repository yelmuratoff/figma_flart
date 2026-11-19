use std::fmt::Write;

use serde_json::Value;

/// Serializes `value` byte-for-byte as JavaScript's `JSON.stringify` does, so a
/// checksum computed inside Figma matches one computed here.
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, value);
    out
}

/// FNV-1a 32-bit hash, formatted as the collector formats it.
pub fn checksum(text: &str) -> String {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in text.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    format!("fnv1a32:{hash:08x}")
}

/// Formats a number as JavaScript's `Number.prototype.toString` does.
pub fn js_number(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if value == 0.0 {
        return "0".to_string();
    }
    let sign = if value < 0.0 { "-" } else { "" };
    let scientific = format!("{:e}", value.abs());
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("LowerExp output always contains an exponent");
    let exponent: i32 = exponent.parse().expect("LowerExp exponent is an integer");
    let digits = mantissa.replace('.', "");
    let k = digits.len() as i32;
    let n = exponent + 1;

    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        let (whole, fraction) = digits.split_at(n as usize);
        format!("{whole}.{fraction}")
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let e = n - 1;
        let e = if e >= 0 {
            format!("+{e}")
        } else {
            e.to_string()
        };
        let (first, rest) = digits.split_at(1);
        if rest.is_empty() {
            format!("{first}e{e}")
        } else {
            format!("{first}.{rest}e{e}")
        }
    };
    format!("{sign}{body}")
}

fn write_value(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        Value::Number(number) => {
            if let Some(int) = number.as_i64() {
                write!(out, "{int}").expect("writing to a String cannot fail");
            } else if let Some(int) = number.as_u64() {
                write!(out, "{int}").expect("writing to a String cannot fail");
            } else {
                out.push_str(&js_number(number.as_f64().unwrap_or(f64::NAN)));
            }
        }
        Value::String(text) => write_string(out, text),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_value(out, item);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            for (index, (key, item)) in map.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_string(out, key);
                out.push(':');
                write_value(out, item);
            }
            out.push('}');
        }
    }
}

fn write_string(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if u32::from(ch) < 0x20 => {
                write!(out, "\\u{:04x}", u32::from(ch)).expect("writing to a String cannot fail");
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_number_matches_javascript() {
        let cases: &[(f64, &str)] = &[
            (0.0, "0"),
            (-0.0, "0"),
            (20.0, "20"),
            (0.1, "0.1"),
            (-1.79943569378338e-7, "-1.79943569378338e-7"),
            (1e21, "1e+21"),
            (1.5e-7, "1.5e-7"),
            (0.000001, "0.000001"),
            (123456789012345680000.0, "123456789012345680000"),
            (0.10980392247438431, "0.10980392247438431"),
            (999.0, "999"),
            (-3.0, "-3"),
            (0.5, "0.5"),
            (1e-7, "1e-7"),
            (2.5e25, "2.5e+25"),
            (0.30000000000000004, "0.30000000000000004"),
        ];
        for (value, expected) in cases {
            assert_eq!(js_number(*value), *expected, "js_number({value:?})");
        }
    }

    #[test]
    fn checksum_matches_js_reference() {
        let data: Value = serde_json::from_str(
            r#"{"name":"Тёмный \"режим\"\n\t\u0001😀\\","n":[0.1,-1.79943569378338e-7,1e+21,20],"ok":true,"z":null}"#,
        )
        .unwrap();
        let canonical = canonical_json(&data);
        assert_eq!(
            canonical,
            "{\"name\":\"Тёмный \\\"режим\\\"\\n\\t\\u0001😀\\\\\",\"n\":[0.1,-1.79943569378338e-7,1e+21,20],\"ok\":true,\"z\":null}"
        );
        assert_eq!(checksum(&canonical), "fnv1a32:4ec11b42");
    }

    #[test]
    fn canonical_json_ignores_input_whitespace() {
        let compact: Value = serde_json::from_str(r#"{"a":[1,2.5],"b":"x"}"#).unwrap();
        let spaced: Value =
            serde_json::from_str("{\n  \"a\": [ 1, 2.5 ],\n  \"b\": \"x\"\n}").unwrap();
        assert_eq!(canonical_json(&compact), canonical_json(&spaced));
    }
}
