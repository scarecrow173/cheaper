//! Centralized YAML document parsing.
//!
//! TS `packages/core` parses YAML with eemeli `yaml`, which follows the
//! YAML 1.2 core schema: only `true`/`false` are booleans, so `y`, `n`,
//! `yes`, `no`, `on`, `off` stay plain strings (as scalars and as keys).
//! `serde_yml` 0.0.13 is a deprecated shim over `noyalib` that provides
//! that YAML 1.2 resolution; `serde_norway` resolves the same scalars as
//! YAML 1.1 booleans and diverges from the TS contract. Approved dep per
//! the task doc (serde_yml or serde_norway). Migrate to noyalib directly
//! when dep policy allows.
//!
//! Every `serde_yml` item is `#[deprecated]` (the crate is a shim), so
//! the allow is scoped to this module — keep all YAML parsing here, and
//! pass [`YamlError`] (never the deprecated `serde_yml::Error`) across
//! module boundaries.
#![allow(deprecated)]

use serde_json::Value;

/// `serde_yml::Error` wrapper: keeps the deprecated shim type behind the
/// module boundary so callers need no `#[allow(deprecated)]`.
#[derive(Debug)]
pub struct YamlError(serde_yml::Error);

impl std::fmt::Display for YamlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for YamlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

/// Parse a YAML document to `serde_json::Value` (`yaml.parse` —
/// `parseYaml`) with YAML 1.2 core-schema scalar resolution. An empty or
/// comment-only document yields `Value::Null`, matching the TS
/// `parseYaml("") === null` behavior relied on by the
/// `frontmatter-not-object` check.
pub fn parse_yaml(text: &str) -> Result<Value, YamlError> {
    serde_yml::from_str(text).map_err(YamlError)
}

/// Serialize a `serde_json::Value` to a YAML document (`yaml.stringify`
/// 相当)。`cheaper policy grant` の write path が user `policy.yaml`
/// 保存に使う(design §(g))。rewrite は comment・key 順・整形を保存
/// しない(accepted caveat — ADR-034FlTS3BaiV3WKz8bX1Xv)。
pub fn stringify_yaml(value: &Value) -> Result<String, YamlError> {
    serde_yml::to_string(value).map_err(YamlError)
}
