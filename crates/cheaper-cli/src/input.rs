//! Port of `packages/cli/src/input.ts` — task input を `--input` file
//! または stdin から読む(DESIGN-0005)。

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::js::js_trim;
use serde_json::{Map, Value};

use crate::cli::CliStdin;
use crate::errors::invalid_request;

/// `readTaskInput`(DESIGN-0005)。
/// - stdin が TTY のときは待たずに INVALID_REQUEST
/// - 空 stdin → INVALID_REQUEST("stdin was empty")
/// - `--input <file>` の read 失敗 → INVALID_REQUEST + details{input,cause}
/// - JSON object 必須(array/scalar/null も拒否)
///
/// stdin の bytes は UTF-8 lossy 解釈(Node `Buffer.toString("utf8")` 相当)。
pub fn read_task_input(
    input: &str,
    stdin: &mut dyn CliStdin,
) -> Result<Map<String, Value>, CheaperError> {
    let text: String = if input == "-" {
        if stdin.is_tty() {
            return Err(invalid_request(
                "no task input: stdin is a TTY and --input was not given",
                None,
            ));
        }
        let bytes = stdin.read_all().map_err(|e| {
            // TS は stdin 読取り error を握らない — そのまま上がると
            // INTERNAL_ERROR になる。ここでも同じ帰結にする。
            CheaperError::new(ErrorCode::InternalError, e.to_string()).with_cause(e)
        })?;
        let t = String::from_utf8_lossy(&bytes).into_owned();
        if js_trim(&t).is_empty() {
            return Err(invalid_request("no task input: stdin was empty", None));
        }
        t
    } else {
        match std::fs::read(input) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(e) => {
                return Err(invalid_request(
                    format!("cannot read input file: {input}"),
                    Some(Map::from_iter([
                        ("input".to_string(), Value::from(input)),
                        // TS は details.cause に error message 文字列を入れる
                        // (Error object ではない)。
                        ("cause".to_string(), Value::from(e.to_string())),
                    ])),
                ));
            }
        }
    };

    let parsed: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => {
            return Err(invalid_request(
                "task input is not valid JSON",
                Some(Map::from_iter([(
                    "reason".to_string(),
                    Value::from("json-parse"),
                )])),
            ));
        }
    };
    match parsed {
        Value::Object(m) => Ok(m),
        _ => Err(invalid_request(
            "task input must be a JSON object",
            Some(Map::from_iter([(
                "reason".to_string(),
                Value::from("not-object"),
            )])),
        )),
    }
}
