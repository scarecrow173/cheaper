use cheaper_core::errors::{CheaperError, ErrorCode, ERROR_TABLE};

use crate::common::*;

#[test]
fn error_table_matches() {
    let f = fixture("errors.json");
    let table = f["table"].as_object().unwrap();
    // Every recorded code must exist with identical exit/retryable flags.
    let rust_codes: std::collections::BTreeSet<_> =
        ERROR_TABLE.iter().map(|(c, _)| c.as_str()).collect();
    let fixture_codes: std::collections::BTreeSet<_> = table.keys().map(String::as_str).collect();
    assert_eq!(rust_codes, fixture_codes, "code set parity");
    for (code, entry) in table {
        let c = ERROR_TABLE
            .iter()
            .map(|(c, _)| *c)
            .find(|c| c.as_str() == code)
            .unwrap_or_else(|| panic!("code {code}"));
        assert_eq!(
            c.exit_code(),
            entry["exitCode"].as_u64().unwrap() as u8,
            "{code}"
        );
        assert_eq!(
            c.retryable(),
            entry["retryable"].as_bool().unwrap(),
            "{code}"
        );
    }
}

#[test]
fn cheaper_error_case() {
    let f = fixture("errors.json");
    for case in cases(&f) {
        let name = case_name(case);
        match name.as_str() {
            n if n.starts_with("CheaperError carries") => {
                let e = CheaperError::new(ErrorCode::Timeout, "timed out");
                let got = serde_json::json!({
                    "code": e.code().as_str(),
                    "exitCode": e.exit_code(),
                    "retryable": e.retryable(),
                    "message": e.message(),
                });
                check(&name, &case["expected"], Ok(got));
            }
            n if n.starts_with("isCheaperError") => {
                let want = &case["expected"]["ok"];
                let err: Box<dyn std::error::Error> =
                    Box::new(CheaperError::new(ErrorCode::Aborted, "x"));
                let got = serde_json::json!([
                    cheaper_core::errors::is_cheaper_error(err.as_ref()),
                    cheaper_core::errors::is_cheaper_error(&std::io::Error::other("x")),
                    false,
                    false,
                ]);
                check(&name, &case["expected"], Ok(got));
                let _ = want;
            }
            _ => panic!("unknown case {name}"),
        }
    }
}
