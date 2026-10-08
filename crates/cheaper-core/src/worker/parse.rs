//! Port of `packages/core/src/worker/parse.ts`.
//!
//! Line-wise frontmatter extraction (`---` open/close), YAML 1.2 frontmatter
//! parse via [`crate::yaml::parse_yaml`] (eemeli `yaml` replacement —
//! YAML 1.2 core schema keeps `y`/`on`/… as strings), nonempty body
//! requirement.

use serde_json::{json, Map, Value};

use crate::errors::{CheaperError, ErrorCode};

/// Port of `ParsedWorkerFile`.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedWorkerFile {
    pub frontmatter: Value,
    pub body: String,
}

fn invalid(source_path: &str, reason: &str, cause: Option<crate::yaml::YamlError>) -> CheaperError {
    let mut details = Map::new();
    details.insert("sourcePath".to_string(), json!(source_path));
    details.insert("reason".to_string(), json!(reason));
    let err = CheaperError::new(
        ErrorCode::InvalidWorkerDefinition,
        format!("invalid worker definition: {reason}"),
    )
    .with_details(details);
    match cause {
        Some(e) => err.with_cause(e),
        None => err,
    }
}

/// `parseWorkerFile`.
pub fn parse_worker_file(
    content: &str,
    source_path: &str,
) -> Result<ParsedWorkerFile, CheaperError> {
    // `content.split(/\r?\n/)`: the `\r` is consumed as part of the
    // `\r?\n` delimiter, so it is stripped from every segment followed
    // by a `\n` — i.e. all but the final segment. Literal `"---\r"`
    // (no newline) therefore keeps its `\r` and the first line fails
    // the `---` frontmatter-open check (verified in Node).
    let mut lines: Vec<&str> = content.split('\n').collect();
    let last_idx = lines.len() - 1;
    for (i, l) in lines.iter_mut().enumerate() {
        if i != last_idx {
            *l = l.strip_suffix('\r').unwrap_or(l);
        }
    }
    if lines.first() != Some(&"---") {
        return Err(invalid(source_path, "frontmatter-not-found", None));
    }
    let close = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, l)| crate::js::js_trim(l) == "---")
        .map(|(i, _)| i);
    let Some(close) = close else {
        return Err(invalid(source_path, "frontmatter-not-closed", None));
    };
    let frontmatter: Value = match crate::yaml::parse_yaml(&lines[1..close].join("\n")) {
        Ok(v) => v,
        Err(e) => return Err(invalid(source_path, "frontmatter-yaml-error", Some(e))),
    };
    match &frontmatter {
        Value::Object(_) => {}
        _ => return Err(invalid(source_path, "frontmatter-not-object", None)),
    }
    let body = crate::js::js_trim(&lines[close + 1..].join("\n")).to_string();
    if body.is_empty() {
        return Err(invalid(source_path, "body-empty", None));
    }
    Ok(ParsedWorkerFile { frontmatter, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reason(e: &CheaperError) -> &str {
        e.details().unwrap()["reason"].as_str().unwrap()
    }

    #[test]
    fn parses_frontmatter_and_body() {
        let content = "---\nname: w\ndescription: d\n---\n\nDo things.\n";
        let parsed = parse_worker_file(content, "w.md").unwrap();
        assert_eq!(parsed.frontmatter["name"], json!("w"));
        assert_eq!(parsed.body, "Do things.");
    }

    #[test]
    fn rejects_missing_frontmatter() {
        let e = parse_worker_file("no front", "w.md").unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidWorkerDefinition);
        assert_eq!(reason(&e), "frontmatter-not-found");
    }

    #[test]
    fn rejects_unclosed_frontmatter() {
        let e = parse_worker_file("---\nname: w\n", "w.md").unwrap_err();
        assert_eq!(reason(&e), "frontmatter-not-closed");
    }

    #[test]
    fn rejects_bad_yaml() {
        let e = parse_worker_file("---\nname: [unclosed\n---\nbody\n", "w.md").unwrap_err();
        assert_eq!(reason(&e), "frontmatter-yaml-error");
    }

    #[test]
    fn rejects_non_object_frontmatter() {
        for fm in ["- a\n- b", "just a string", ""] {
            let content = format!("---\n{fm}\n---\nbody\n");
            let e = parse_worker_file(&content, "w.md").unwrap_err();
            assert_eq!(reason(&e), "frontmatter-not-object", "{fm:?}");
        }
    }

    #[test]
    fn rejects_empty_body() {
        let e = parse_worker_file("---\nname: w\n---\n  \n", "w.md").unwrap_err();
        assert_eq!(reason(&e), "body-empty");
    }

    #[test]
    fn feff_is_js_whitespace_for_delimiter_and_body() {
        // JS `trim` strips U+FEFF (Rust `trim` does not): a close
        // delimiter padded with FEFF still counts, and an FEFF-only body
        // is empty.
        let parsed = parse_worker_file("---\nname: w\n---\u{FEFF}\nbody\n", "w.md").unwrap();
        assert_eq!(parsed.body, "body");
        let e = parse_worker_file("---\nname: w\n---\n\u{FEFF}\n", "w.md").unwrap_err();
        assert_eq!(reason(&e), "body-empty");
    }

    #[test]
    fn trailing_cr_before_eof_is_kept_like_js_split() {
        // `split(/\r?\n/)` only consumes `\r` when it precedes `\n`:
        // literal `"---\r"` keeps the `\r` in line 0 → not the `---`
        // opener (probed in Node).
        let e = parse_worker_file("---\r", "w.md").unwrap_err();
        assert_eq!(reason(&e), "frontmatter-not-found");
        // A mid-line `\r\n` is still consumed normally.
        let parsed = parse_worker_file("---\r\nname: w\r\n---\r\nbody\r\n", "w.md").unwrap();
        assert_eq!(parsed.frontmatter["name"], json!("w"));
        // A lone `\r` inside the last segment is preserved either way.
        let e = parse_worker_file("---\nname: w\n---\nbody\r", "w.md").unwrap();
        assert_eq!(e.body, "body");
    }

    #[test]
    fn yaml_12_scalars_stay_strings() {
        // YAML 1.2 core schema: `y`/`on`/`off` are plain strings, not
        // booleans (serde_norway resolved them as YAML 1.1 booleans).
        let parsed = parse_worker_file("---\nname: y\nflag: on\n---\nbody\n", "w.md").unwrap();
        assert_eq!(parsed.frontmatter["name"], json!("y"));
        assert_eq!(parsed.frontmatter["flag"], json!("on"));
        // `on:` key stays a string, and `true`/`false` remain booleans.
        let parsed =
            parse_worker_file("---\nname: w\non: 1\nbool: true\n---\nbody\n", "w.md").unwrap();
        assert!(parsed.frontmatter.get("on").is_some());
        assert_eq!(parsed.frontmatter["bool"], json!(true));
    }
}
