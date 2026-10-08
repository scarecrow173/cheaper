use cheaper_core::token::count::{
    count_file_tokens, count_text_tokens, CountFileTokensOptions, CountTokensError,
};

use crate::common::*;

#[test]
fn token_cases() {
    let f = fixture("token/count.json");
    for case in cases(&f) {
        let name = case_name(case);
        let op = case["op"].as_str().unwrap();
        let produced = match op {
            "countText" => match count_text_tokens(case["text"].as_str().unwrap()) {
                Ok(tokens) => Ok(serde_json::json!({ "tokens": tokens })),
                // `cap` on the js-tiktoken plain Error → `{kind:"other"}`.
                Err(e) => Err(serde_json::json!({
                    "kind": "other",
                    "name": "Error",
                    "message": e.to_string(),
                })),
            },
            "countFile" => {
                let Some(c) = case_dir(case) else {
                    panic!("{name}: case dir setup failed");
                };
                let input = unscrub(&case["input"], &c.needles);
                match count_file_tokens(&CountFileTokensOptions {
                    root: &c.root,
                    path: input.as_str().unwrap(),
                    symlink_policy: symlink_policy(&case["policy"]),
                    max_bytes: case["maxBytes"].as_u64(),
                }) {
                    Ok(r) => Ok(serde_json::json!({
                        "path": r.path,
                        "tokens": r.tokens,
                        "bytes": r.bytes,
                    })),
                    Err(CountTokensError::Escape(e)) => Err(escape_err(&e, &c.needles)),
                    Err(CountTokensError::TooLarge(e)) => Err(serde_json::json!({
                        "kind": "tooLarge",
                        "path": e.path(),
                        "bytes": e.bytes(),
                        "maxBytes": e.max_bytes(),
                    })),
                    Err(CountTokensError::SpecialToken(e)) => Err(serde_json::json!({
                        "kind": "other",
                        "name": "Error",
                        "message": e.to_string(),
                    })),
                    Err(CountTokensError::Io(_)) => Err(serde_json::json!({"kind": "io"})),
                }
            }
            other => panic!("unknown op {other} in {name}"),
        };
        check(&name, &case["expected"], produced);
    }
}
