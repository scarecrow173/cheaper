//! `route::payload` — rewrite payload の base64url codec(design
//! §RoutePayload、R18-R20)。encode/decode・schema 検証・
//! allowlist env 絞り込みの唯一の owner — hook rewrite 側と
//! `route run` 側で codec を複製しない(B#6/R20)。
//!
//! env 値は payload の一時的な transport にのみ存在し、route
//! log・usage record・workflow input へは持ち出さない(R32)。

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use cheaper_core::errors::{CheaperError, ErrorCode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// payload の wire schema(R19)。`env` は route `passthroughEnv`
/// で allowlist された ambient capture のみを載せる — env-prefix
/// (`KEY=VALUE cmd`)の値は transport せず、`route run` が
/// `payload.command` から再導出する(C3)。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoutePayload {
    /// 原 shell command(全体 — env-prefix 含む)。
    pub command: String,
    /// `command_name(argv[0])` の executable 名。
    pub executable: String,
    /// env-prefix 除く argv 列。
    pub argv: Vec<String>,
    /// post-prefix args(`argv[len(match.command)..]`)。
    pub args: Vec<String>,
    /// 原 command の意図 cwd — `cd <dir>` prefix unwrap では
    /// canonical 済みの移動先、それ以外は request cwd。
    /// `route run` は workspace root 内部(同一を含む)であることを
    /// 再検証した上で routed child の exec cwd として使う。
    pub cwd: String,
    /// ambient env の allowlist 部分集合。
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

/// encode 用の入力 — caller が採取済みの値を渡し、env の
/// allowlist 絞り込みはこの層が担う(構造保証を codec owner に
/// 集約するため)。
pub struct EncodeInput {
    pub command: String,
    pub executable: String,
    pub argv: Vec<String>,
    pub args: Vec<String>,
    pub cwd: String,
    /// ambient env の候補全体 — `passthrough_env` の名前のみ残る。
    pub ambient_env: BTreeMap<String, String>,
    /// route `passthroughEnv` 宣言(allowlist)。
    pub passthrough_env: Vec<String>,
}

/// ambient env を route `passthroughEnv` の名前集合へ絞る
/// (R18 — allowlist のみ transport)。
pub fn filter_passthrough_env(
    ambient: &BTreeMap<String, String>,
    passthrough_env: &[String],
) -> BTreeMap<String, String> {
    ambient
        .iter()
        .filter(|(k, _)| passthrough_env.iter().any(|p| p == *k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// `RoutePayload` → JSON → unpadded base64url の単一 token
/// (R20 — cmd/PowerShell/bash で quoting 差が出ないよう opaque
/// 1 token 化)。
pub fn encode(input: EncodeInput) -> Result<String, CheaperError> {
    let payload = RoutePayload {
        command: input.command,
        executable: input.executable,
        argv: input.argv,
        args: input.args,
        cwd: input.cwd,
        env: filter_passthrough_env(&input.ambient_env, &input.passthrough_env),
    };
    let bytes = serde_json::to_vec(&payload).map_err(|e| {
        CheaperError::new(ErrorCode::InternalError, "route payload encode failed").with_details(
            serde_json::json!({"error": e.to_string()})
                .as_object()
                .unwrap()
                .clone(),
        )
    })?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

/// base64url token → JSON → schema 検証。decode 失敗・schema 違反は
/// `InvalidRequest` で fail-closed(`route run` は再 resolve して
/// allowlist/env を再検証する — ここでは transport の形だけを
/// 保証する)。
pub fn decode(token: &str) -> Result<RoutePayload, CheaperError> {
    let invalid = |msg: &str, detail: serde_json::Value| {
        CheaperError::new(
            ErrorCode::InvalidRequest,
            format!("invalid route payload: {msg}"),
        )
        .with_details(detail.as_object().unwrap().clone())
    };
    let bytes = URL_SAFE_NO_PAD.decode(token).map_err(|e| {
        invalid(
            "base64url decode failed",
            serde_json::json!({"error": e.to_string()}),
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|e| {
        invalid(
            "schema mismatch",
            serde_json::json!({"error": e.to_string()}),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> EncodeInput {
        EncodeInput {
            command: "CI=1 cargo test --workspace".to_string(),
            executable: "cargo".to_string(),
            argv: vec!["cargo".into(), "test".into(), "--workspace".into()],
            args: vec!["--workspace".into()],
            cwd: "/work".to_string(),
            ambient_env: BTreeMap::from([
                ("ALLOWED".to_string(), "v1".to_string()),
                ("SECRET".to_string(), "leak".to_string()),
            ]),
            passthrough_env: vec!["ALLOWED".to_string()],
        }
    }

    #[test]
    fn round_trip_and_allowlist() {
        let token = encode(input()).unwrap();
        // unpadded base64url — `=`・`+`・`/` を含まない opaque token。
        assert!(!token.contains(['=', '+', '/']));
        let decoded = decode(&token).unwrap();
        assert_eq!(decoded.command, "CI=1 cargo test --workspace");
        assert_eq!(decoded.executable, "cargo");
        assert_eq!(decoded.argv, vec!["cargo", "test", "--workspace"]);
        assert_eq!(decoded.args, vec!["--workspace"]);
        assert_eq!(decoded.cwd, "/work");
        // allowlist のみ transport — SECRET は落ちる(R18/R32)。
        assert_eq!(
            decoded.env,
            BTreeMap::from([("ALLOWED".to_string(), "v1".to_string())])
        );
    }

    #[test]
    fn decode_rejects_malformed() {
        assert!(decode("!!!not-base64!!!").is_err());
        // valid base64url だが schema 違反。
        let bad = URL_SAFE_NO_PAD.encode(b"{\"command\":1}");
        assert!(decode(&bad).is_err());
    }
}
