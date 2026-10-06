use cheaper_core::json::number::format_number;

use crate::common::*;

#[test]
fn js_number_format() {
    let f = fixture("number-format.json");
    for case in cases(&f) {
        let input = &case["input"];
        let x = match input {
            serde_json::Value::Number(n) => n.as_f64().unwrap(),
            serde_json::Value::String(s) => match s.as_str() {
                "Infinity" => f64::INFINITY,
                "-Infinity" => f64::NEG_INFINITY,
                "NaN" => f64::NAN,
                _ => panic!("unknown f64 literal {s}"),
            },
            _ => panic!("bad input {input}"),
        };
        let name = format!("format_number({input})");
        let got = format_number(x);
        check(&name, &case["expected"], Ok(serde_json::json!(got)));
    }
}
