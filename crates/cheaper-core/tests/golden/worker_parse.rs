use cheaper_core::worker::parse::parse_worker_file;

use crate::common::*;

#[test]
fn parse_worker_file_cases() {
    let f = fixture("worker/parse.json");
    for case in cases(&f) {
        let name = case_name(case);
        let produced = match parse_worker_file(
            case["content"].as_str().unwrap(),
            case["sourcePath"].as_str().unwrap(),
        ) {
            Ok(parsed) => Ok(serde_json::json!({
                "frontmatter": parsed.frontmatter,
                "body": parsed.body,
            })),
            Err(e) => Err(cheaper_err(&e, &[])),
        };
        check(&name, &case["expected"], produced);
    }
}
