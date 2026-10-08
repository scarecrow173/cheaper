//! JS-compatible canonical number formatter (`json/number.ts` has no
//! source file — TS relies on `JSON.stringify`). This module is the
//! single implementation of the design-doc numeric format rule:
//! integers without a decimal point, JS exponential notation at the
//! `1e21` / `1e-6` boundaries, `-0` → `0`, non-finite → `null`.
//!
//! Rust `{:e}` produces the shortest round-trip digits (same Grisu/Ryu
//! digits as V8); the formatter re-arranges them per ECMA-262
//! `Number::toString`.

use serde::Serialize;
use serde_json::value::RawValue;

/// `Number.prototype.toString` / `JSON.stringify` number formatting.
///
/// `NaN`/`±Infinity` format as `null` (`JSON.stringify` behavior).
pub fn format_number(x: f64) -> String {
    if !x.is_finite() {
        return "null".to_string();
    }
    if x == 0.0 {
        return "0".to_string(); // covers -0
    }
    let negative = x.is_sign_negative();
    // e.g. "1.2345e3" or "1e-6".
    let s = format!("{:e}", x.abs());
    let (mantissa, exp) = s.split_once('e').expect("LowerExp has exponent");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i64; // number of significant digits
    let n = exp.parse::<i64>().expect("LowerExp exponent") + 1; // digit count before the point in normalized form

    let mut out = String::new();
    if negative {
        out.push('-');
    }
    if k <= n && n <= 21 {
        // Integer: digits followed by (n - k) zeros.
        out.push_str(&digits);
        out.push_str(&"0".repeat((n - k) as usize));
    } else if 0 < n && n <= 21 {
        // Decimal point inside the digits.
        out.push_str(&digits[..n as usize]);
        out.push('.');
        out.push_str(&digits[n as usize..]);
    } else if -6 < n && n <= 0 {
        // 0.0000ddd
        out.push_str("0.");
        out.push_str(&"0".repeat((-n) as usize));
        out.push_str(&digits);
    } else {
        // Exponential: d[.rest]e±exp (sign always emitted).
        out.push_str(&digits[..1]);
        if k > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        let e = n - 1;
        if e >= 0 {
            out.push('+');
        }
        out.push_str(&e.to_string());
    }
    out
}

/// `serde::Serialize` wrapper emitting the canonical JS number text.
/// Uses `serde_json::value::RawValue` so the digits appear verbatim in
/// the JSON output (`serialize_with = js_number::serialize`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JsNumber(pub f64);

impl Serialize for JsNumber {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let raw =
            RawValue::from_string(format_number(self.0)).map_err(serde::ser::Error::custom)?;
        raw.serialize(serializer)
    }
}

/// `serialize_with` helper for `f64` fields in envelope/report structs.
pub fn serialize<S: serde::Serializer>(v: &f64, serializer: S) -> Result<S::Ok, S::Error> {
    JsNumber(*v).serialize(serializer)
}

/// `serialize_with` helper for `Option<f64>` fields.
pub fn serialize_opt<S: serde::Serializer>(
    v: &Option<f64>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match v {
        Some(f) => JsNumber(*f).serialize(serializer),
        None => serializer.serialize_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_have_no_decimal_point() {
        assert_eq!(format_number(1.0), "1");
        assert_eq!(format_number(42.0), "42");
        assert_eq!(format_number(-7.0), "-7");
        assert_eq!(format_number(1e20), "100000000000000000000");
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number(-0.0), "0");
    }

    #[test]
    fn decimals_use_shortest_digits() {
        assert_eq!(format_number(0.1), "0.1");
        assert_eq!(format_number(1.5), "1.5");
        assert_eq!(format_number(12.3456), "12.3456");
        assert_eq!(format_number(-0.25), "-0.25");
    }

    #[test]
    fn js_exponential_boundaries() {
        assert_eq!(format_number(1e21), "1e+21");
        assert_eq!(format_number(1.5e21), "1.5e+21");
        assert_eq!(format_number(9.99e20), "999000000000000000000");
        assert_eq!(format_number(1e-6), "0.000001");
        assert_eq!(format_number(9.9e-7), "9.9e-7");
        assert_eq!(format_number(1e-7), "1e-7");
        assert_eq!(format_number(-1e-7), "-1e-7");
        assert_eq!(format_number(5e-324), "5e-324");
        assert_eq!(
            format_number(1.7976931348623157e308),
            "1.7976931348623157e+308"
        );
    }

    #[test]
    fn non_finite_is_null() {
        assert_eq!(format_number(f64::NAN), "null");
        assert_eq!(format_number(f64::INFINITY), "null");
        assert_eq!(format_number(f64::NEG_INFINITY), "null");
    }

    #[test]
    fn serde_integration_emits_raw_number() {
        #[derive(serde::Serialize)]
        struct Doc {
            #[serde(serialize_with = "serialize")]
            v: f64,
            #[serde(serialize_with = "serialize_opt")]
            o: Option<f64>,
        }
        let s = serde_json::to_string(&Doc {
            v: 1.0,
            o: Some(1e21),
        })
        .unwrap();
        assert_eq!(s, r#"{"v":1,"o":1e+21}"#);
        let s = serde_json::to_string(&Doc {
            v: f64::NAN,
            o: None,
        })
        .unwrap();
        assert_eq!(s, r#"{"v":null,"o":null}"#);
    }
}
