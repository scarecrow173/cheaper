//! `security` policy section(issue #164 — opt-in `security.execDenyPatterns`)。
//!
//! trusted layer(builtin + user-scope `policy.yaml`)のみが authoring
//! できる deterministic executable deny-list。#163 で mandatory 層から
//! 除去された旧 `command_denied_executable_re`(A9)を operator opt-in
//! の profile として再配置する — 宣言形(`SecurityPolicy` — serde
//! native・source string 保持)と compiled 形(`ExecDenyList`)を分離し、
//! compile 境界は `ExecDenyList` のみが持つ(`filesystem.deny_paths`
//! の resolved string 列 → `PathDenylist` 先例と同型)。
//!
//! match 規約: 各 pattern は `^(?:pattern)$` 相当の stem 完全一致・
//! `case_insensitive(true)` で個別 compile(per-pattern — 失敗 pattern
//! を error `details.pattern` へ帰属させるため合成 regex にしない)、
//! 配列は OR 評価。unanchored match は `sh` が `bash`/`sshd` へ
//! substring match する false positive(deny の誤発動 = 正当 grant の
//! 拒否)を生じるため許さない — substring 意図は `.*` で明示する。
//! 空集合(既定)は何も deny しない(#163 後の既定挙動不変)。
//!
//! deny の照合対象は `executable_stem`(declared spec・resolved
//! canonical path 双方に同一適用)。invalid regex は compile 境界で
//! `InvalidRuntimePolicy` に fail-closed する — warn-and-skip は deny
//! を silent に弱めるため不採用(ADR-034KLgkao8SgWWKIIsCqaB 確定)。

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::errors::{CheaperError, ErrorCode};

/// `security` section の JSON schema properties。`execDenyPatterns` は
/// `route.rewrite.approvalBypass` の `["array","null"]` nullable 先例
/// (schema.rs)に倣う — 明示 `null` ≡ 未指定 ≡ `[]`。`maxItems: 128`
/// は `command.grants` の `maxProperties: 128` と同じ bounding 方針、
/// `items` の bound は `bounded_string`(minLength 1・maxLength 4096)
/// に揃える。`uniqueItems` — OR 評価で重複は意味を持たないため
/// fail-loud。
pub fn security_policy_properties() -> Value {
    json!({
        "execDenyPatterns": {
            "type": ["array", "null"],
            "maxItems": 128,
            "uniqueItems": true,
            "items": {"type": "string", "minLength": 1, "maxLength": 4096},
        },
    })
}

/// `security` section の normalized 表現(trusted layer のみ authoring
/// 可能 — `command`/`route` と同型に project partial schema は
/// section 全体を除外する)。serde-native: pattern の **source string**
/// を保持し、compiled regex は載せない(`CommandGrant.argv` と同じ規約
/// — invocation parity check が declared-shape equality を見るため)。
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SecurityPolicy {
    #[serde(rename = "execDenyPatterns")]
    pub exec_deny_patterns: Vec<String>,
}

/// `resolveSecurityPolicy`: raw `security` section(`Option<&Value>`)を
/// resolved `SecurityPolicy` へ passthrough 解決する。`None`/`null`/
/// 非 object は `[]` 等価(`route` と同じ schema-gated 前提 — 値
/// domain は schema が保証済み、item は string のみ収集)。
/// **compile は行わない** — compile 境界は `ExecDenyList` のみ。
pub fn resolve_security_policy(input: Option<&Value>) -> Result<SecurityPolicy, CheaperError> {
    let exec_deny_patterns = input
        .and_then(Value::as_object)
        .and_then(|o| o.get("execDenyPatterns"))
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Ok(SecurityPolicy { exec_deny_patterns })
}

/// `security` section → compiled deny predicate の一回経路
/// (resolve + compile)。deny 注入点が raw JSON から直接 compiled
/// 形を得るための helper。
pub fn resolve_exec_deny_list(input: Option<&Value>) -> Result<ExecDenyList, CheaperError> {
    let security = resolve_security_policy(input)?;
    ExecDenyList::compile(&security.exec_deny_patterns)
}

/// compiled executable deny predicate — operator 宣言 pattern 集合。
/// 空集合(既定)は何も deny しない。`Default` = 空。
#[derive(Debug, Clone, Default)]
pub struct ExecDenyList {
    patterns: Vec<regex::Regex>,
}

impl ExecDenyList {
    /// 各 pattern を `^(?:pattern)$` 相当の stem 完全一致・
    /// `case_insensitive(true)` で**個別** compile(失敗 pattern を
    /// error `details.pattern` に載せる attribution のため 1 本の
    /// 合成 regex にしない)。配列は OR 評価。compile 不能な
    /// pattern は `InvalidRuntimePolicy` で fail-closed
    /// (`ArgPattern::compile` の compile 時拒否と同型)。
    pub fn compile(patterns: &[String]) -> Result<Self, CheaperError> {
        let mut compiled = Vec::with_capacity(patterns.len());
        for pattern in patterns {
            let re = regex::RegexBuilder::new(&format!("^(?:{pattern})$"))
                .case_insensitive(true)
                .build()
                .map_err(|e| {
                    security_invalid("invalid execDenyPatterns regex")
                        .with_details(Map::from_iter([("pattern".to_string(), json!(pattern))]))
                        .with_cause(e)
                })?;
            compiled.push(re);
        }
        Ok(Self { patterns: compiled })
    }

    /// declared/resolved executable 文字列を `executable_stem` で
    /// 正規化した stem へ OR match。空集合は常に false。
    pub fn is_denied(&self, executable: &str) -> bool {
        let stem = executable_stem(executable);
        self.patterns.iter().any(|re| re.is_match(&stem))
    }

    /// deny 無効化の判定用(predicate 注入の有無ではなく集合の空で
    /// 表す — `exec_deny.is_empty()` が既定 no-op の同値条件)。
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
}

/// basename 正規化 — `/`・`\` 区切りの最終要素 → 小文字化 →
/// `.exe`/`.com` suffix を 1 回 strip → stem。declared spec と
/// resolved canonical path の両方に同一適用(旧
/// `is_denied_command_executable` の規約を引き継ぐ)。
pub fn executable_stem(executable: &str) -> String {
    let name = executable
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .to_lowercase();
    name.strip_suffix(".exe")
        .or_else(|| name.strip_suffix(".com"))
        .unwrap_or(&name)
        .to_string()
}

/// `command_invalid` と同型の error helper —
/// `"invalid security policy: {reason}"` + `InvalidRuntimePolicy`。
fn security_invalid(reason: &str) -> CheaperError {
    CheaperError::new(
        ErrorCode::InvalidRuntimePolicy,
        format!("invalid security policy: {reason}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_stem_normalizes_basename() {
        // `/`・`\` 区切りの最終要素 → 小文字化 → `.exe`/`.com`
        // suffix を 1 回 strip。
        assert_eq!(executable_stem("python3"), "python3");
        assert_eq!(executable_stem("/usr/bin/Python3.EXE"), "python3");
        assert_eq!(executable_stem("dir\\tool.com"), "tool");
        assert_eq!(executable_stem("C:\\Tools\\Git.EXE"), "git");
        // suffix strip は 1 回のみ — `.exe.com` は `.exe` を残す。
        assert_eq!(executable_stem("foo.exe.com"), "foo.exe");
        // mixed-case suffix も小文字化後に strip される。
        assert_eq!(executable_stem("Runner.Exe"), "runner");
        // separator を含まない bare name・空文字はそのまま。
        assert_eq!(executable_stem("sh"), "sh");
        assert_eq!(executable_stem(""), "");
    }

    #[test]
    fn deny_list_matches_stem_exactly_case_insensitive() {
        let deny =
            ExecDenyList::compile(&["python.*".to_string(), "ssh".to_string(), "tar".to_string()])
                .unwrap();
        // stem 完全一致 + case-insensitive — basename 正規化経由。
        assert!(deny.is_denied("python3"));
        assert!(deny.is_denied("PYTHON.EXE"));
        assert!(deny.is_denied("/usr/bin/python3.13"));
        assert!(deny.is_denied("C:\\tools\\SSH.EXE"));
        assert!(deny.is_denied("tar"));
        // `sh`/`ssh` pattern は `bash`/`sshd` に substring match しない
        // (`^(?:pattern)$` anchor — substring 意図は `.*` で明示)。
        let sh = ExecDenyList::compile(&["sh".to_string()]).unwrap();
        assert!(sh.is_denied("sh"));
        assert!(!sh.is_denied("bash"));
        assert!(!sh.is_denied("sshd"));
        assert!(!sh.is_denied("git-shell"));
        // 非 match は受理方向。
        assert!(!deny.is_denied("git"));
        assert!(!deny.is_denied("cargo"));
    }

    #[test]
    fn deny_list_pattern_case_is_insensitive_too() {
        // mixed-case 宣言も小文字化 stem + case-insensitive compile の
        // 二重正規化で match する(ADR の「pattern も case-insensitive
        // 評価」)。
        let deny = ExecDenyList::compile(&["Python".to_string()]).unwrap();
        assert!(deny.is_denied("python"));
        assert!(deny.is_denied("PYTHON"));
        // OR 評価 — いずれかの pattern が match すれば denied。
        let deny = ExecDenyList::compile(&["a".to_string(), "b".to_string()]).unwrap();
        assert!(deny.is_denied("a"));
        assert!(deny.is_denied("b"));
        assert!(!deny.is_denied("c"));
    }

    #[test]
    fn empty_deny_list_is_noop() {
        let deny = ExecDenyList::default();
        assert!(deny.is_empty());
        // 空集合は何も deny しない(既定 no-op — #163 後の既定挙動)。
        for exe in ["python", "ssh", "tar", "sh", "env", "ld-linux.so"] {
            assert!(!deny.is_denied(exe), "{exe}");
        }
        // compile 済みの空集合も同じ。
        let deny = ExecDenyList::compile(&[]).unwrap();
        assert!(deny.is_empty());
        assert!(!deny.is_denied("python"));
    }

    #[test]
    fn invalid_regex_fails_closed_with_pattern_attribution() {
        // invalid regex は compile 境界で `InvalidRuntimePolicy` —
        // warn-and-skip しない(deny を silent に弱めない)。
        let e = ExecDenyList::compile(&["ok".to_string(), "(".to_string()]).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
        assert_eq!(
            e.message(),
            "invalid security policy: invalid execDenyPatterns regex"
        );
        // 失敗した source string を `details.pattern` に帰属。
        assert_eq!(e.details().unwrap()["pattern"], json!("("));
    }

    #[test]
    fn resolve_security_policy_defaults_to_empty() {
        // `None`/`null`/非 object は `[]` 等価(schema-gated 前提の
        // passthrough — `route` resolve と同型)。
        for input in [None, Some(&Value::Null), Some(&json!("x")), Some(&json!(1))] {
            let p = resolve_security_policy(input).unwrap();
            assert!(p.exec_deny_patterns.is_empty());
        }
        for v in [
            json!({}),
            json!({"execDenyPatterns": null}),
            json!({"execDenyPatterns": []}),
        ] {
            let p = resolve_security_policy(Some(&v)).unwrap();
            assert!(p.exec_deny_patterns.is_empty());
        }
        let p = resolve_security_policy(Some(&json!({
            "execDenyPatterns": ["python.*", "ssh"]
        })))
        .unwrap();
        assert_eq!(
            p.exec_deny_patterns,
            vec!["python.*".to_string(), "ssh".to_string()]
        );
        // serde-native round-trip — parity check は declared-shape
        // equality を見る(`CommandGrant.argv` と同じ規約)。
        let serialized = serde_json::to_value(&p).unwrap();
        assert_eq!(p, resolve_security_policy(Some(&serialized)).unwrap());
    }

    #[test]
    fn resolve_exec_deny_list_compiles_in_one_pass() {
        let deny = resolve_exec_deny_list(Some(&json!({
            "execDenyPatterns": ["deny-me"]
        })))
        .unwrap();
        assert!(deny.is_denied("deny-me"));
        assert!(!deny.is_denied("allow-me"));
        // invalid regex も本経路で `InvalidRuntimePolicy` に
        // fail-closed する。
        let e = resolve_exec_deny_list(Some(&json!({
            "execDenyPatterns": ["("]
        })))
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
    }
}
