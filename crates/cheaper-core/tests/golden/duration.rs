use cheaper_core::duration::parse_duration;

use crate::common::*;

#[test]
fn parse_duration_cases() {
    let f = fixture("duration.json");
    for case in cases(&f) {
        let name = case_name(case);
        let input = case["input"].as_str().unwrap();
        let produced = match parse_duration(input) {
            Ok(ms) => Ok(serde_json::json!({"ms": js_num(ms as f64)})),
            Err(e) => Err(serde_json::json!({"kind": "range", "message": e.to_string()})),
        };
        // The `rustOverflow` case records the documented divergence (Risk
        // #8): TS returns a finite f64, the port rejects overflow.
        if case["rustOverflow"].as_bool().unwrap_or(false) {
            assert!(
                produced.is_err(),
                "{name}: expected overflow rejection (TS keeps {expected})",
                expected = case["expected"]["ok"]["ms"]
            );
            continue;
        }
        // TS records `ms` as a JS number: above 2^53 its decimal text
        // (`9223372036854776000`) differs from the exact u64 the port
        // computes (`9223372036854775808`) but round-trips to the same
        // f64. Canonicalize the expected side through `js_num` too so
        // both halves compare in the same number representation.
        let mut expected = case["expected"].clone();
        if let Some((slot, f)) = expected
            .get_mut("ok")
            .and_then(|ok| ok.get_mut("ms"))
            .and_then(|v| v.as_f64().map(|f| (v, f)))
        {
            *slot = js_num(f);
        }
        check(&name, &expected, produced);
    }
}
