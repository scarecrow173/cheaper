//! RunFixture JSON 生成(design §2.5・ADR 確定方式)+ emit child の
//! seeded 高エントロピ byte 生成器(§1.4)。
//!
//! `benchmarks/fixtures/fake-worker-stream.mjs` の Rust port:
//! `serde_json::json!`/template 組立て → **書込み前に必ず
//! `cheaper_runtime::dev::fixture::RunFixture::parse` で round-trip
//! 検証** → file write → `CHEAPER_TEST_FAKE_STREAM` path 設定用の
//! path を返す。`dev::fixture` 型は Deserialize-only のため、generator
//! は canonical schema へ parse で anchor される(JS 平行実装の
//! drift 解消)— 無効 step・型違反は benchmark 実行前に fail-fast。
//!
//! session 結合規則(§2.5): `FakeSessionFactory` は `create_session`
//! 1 回ごとに `sessions[]` を宣言順で 1 件消費する — worker job N 件
//! には session N 件を profiles 側が供給する(不足分は "no scripted
//! response" で失敗する v1 parity)。

use std::path::{Path, PathBuf};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_runtime::dev::fixture::RunFixture;
use serde_json::{json, Value};

use crate::manifest::{FakeStream, ProfileSpec};

/// `usage_delta` の scripted 固定値(v1 `PROFILE_USAGE` port)。
/// 実測ではなく deterministic な宣言値 — workflow/direct 両 arm が同じ
/// script を replay するため comparison 意味論(両 arm 同値 → delta 0)
/// は変わらない。
fn profile_usage(profile: &str) -> Value {
    match profile {
        "compact" => json!({"input": 1000, "output": 30}),
        "large" => json!({"input": 1000, "output": 300}),
        _ => json!({"input": 1000, "output": 50}),
    }
}

/// 旧 fake が返していた bulk-reader 応答 payload(output schema 適合 —
/// `summary`/`findings`/`relevantFiles`/`suggestedReads`/`openQuestions`
/// の 5 required key)。v1 `workerResponseText` port。
pub fn worker_response_text(profile: &str, index: usize) -> String {
    let large = profile == "large";
    let body = json!({
        "summary": if large {
            format!("seeded compact summary {index}")
        } else {
            "seeded compact summary".to_string()
        },
        "findings": if large {
            json!([{
                "title": format!("fixture finding {index}"),
                "detail": "x".repeat(1000),
                "confidence": "high",
                "locations": [{"path": "README.md", "lines": {"start": 1, "end": 1}}],
            }])
        } else {
            json!([])
        },
        "relevantFiles": [],
        "suggestedReads": [],
        "openQuestions": [],
    });
    serde_json::to_string(&body).expect("worker response text serializes")
}

/// profile 駆動 session 生成の解決済み入力(manifest `ProfileSpec`・
/// per-invocation override の正規化後形)。
#[derive(Debug, Clone)]
pub struct ProfileOptions {
    /// session 1 件ごとの profile 名(session 数 = profiles 数)。
    pub profiles: Vec<String>,
    /// 各 session の `delay_ms` step(profile response にのみ掛かる —
    /// `tool_calls` step より後に置く v1 parity)。
    pub delay_ms: u64,
    /// 指定時は session 0 先頭に `tool_calls` step を挿入
    /// (旧 `CHEAPER_BENCH_TOOL_CALL_PATH` — queue 先頭の 1 call のみ)。
    pub tool_call_path: Option<String>,
}

impl Default for ProfileOptions {
    fn default() -> Self {
        Self {
            profiles: vec!["compact".to_string()],
            delay_ms: 0,
            tool_call_path: None,
        }
    }
}

impl From<&ProfileSpec> for ProfileOptions {
    fn from(spec: &ProfileSpec) -> Self {
        let defaults = ProfileOptions::default();
        Self {
            profiles: spec
                .profiles
                .clone()
                .filter(|p| !p.is_empty())
                .unwrap_or(defaults.profiles),
            delay_ms: spec.delay_ms.unwrap_or(0.0).max(0.0) as u64,
            tool_call_path: spec.tool_call_path.clone(),
        }
    }
}

fn session_steps(profile: &str, index: usize, opts: &ProfileOptions) -> Vec<Value> {
    let mut steps = Vec::new();
    if index == 0 {
        if let Some(path) = &opts.tool_call_path {
            steps.push(json!({
                "tool_calls": [{"name": "read", "input": {"path": path}}]
            }));
            steps.push(json!({"turn": {"tool_calls": true}}));
        }
    }
    if opts.delay_ms > 0 {
        steps.push(json!({"delay_ms": opts.delay_ms}));
    }
    steps.push(json!({"usage_delta": profile_usage(profile)}));
    steps.push(json!({"turn": {}}));
    steps.push(json!({"finish": {"text": worker_response_text(profile, index)}}));
    steps
}

/// v1 `buildFakeStreamFixture` port — profile 1 件ごとに session を生成
/// (`create_session` 1 回ごとに `sessions[]` を 1 件消費する seam 規則
/// に整合)。
pub fn profile_fixture(opts: &ProfileOptions) -> Value {
    let sessions: Vec<Value> = opts
        .profiles
        .iter()
        .enumerate()
        .map(|(index, profile)| json!({"steps": session_steps(profile, index, opts)}))
        .collect();
    json!({
        "name": "cheaper-benchmark-fake-stream",
        "description": "benchmark harness 生成の scripted session。session 数 = responseProfiles 数",
        "sessions": sessions,
    })
}

/// manifest `fakeStream`/`armFakeStream` 指定を fixture document へ解決
/// し、`RunFixture::parse` round-trip で canonical schema へ anchor した
/// JSON を返す(§2.5 — 書込み前検証の pure 部)。
///
/// - `"auto"`(既定)/ `{profiles, delayMs, toolCallPath}` — profile 駆動
///   session 生成。
/// - `{fixture: {…}}` — RunFixture DSL の部分 object を passthrough
///   (`sessions[].steps` の error/hang/panic/abort 等の全節を manifest
///   が直接書ける — parallel vocabulary なし)。部分 object の merge は
///   `RunFixture` 側の `#[serde(default)]` が担う。
///
/// 無効 step・型違反・未知の literal 指定は `Err`(fail-fast)。
pub fn fixture_document(spec: Option<&FakeStream>) -> Result<Value, CheaperError> {
    let doc = match spec {
        None | Some(FakeStream::Literal(_)) => {
            // `"auto"` 以外の literal は既知語彙でない — domain 検証は
            // T2 の manifest validation が担うが、生成側も黙って既定値へ
            // 畳まない(literal 自体を解釈する境界で fail-fast)。
            match spec {
                Some(FakeStream::Literal(lit)) if lit != "auto" => {
                    return Err(bench_error(format!(
                        "unsupported fakeStream literal: {lit:?} (expected \"auto\")"
                    )));
                }
                _ => profile_fixture(&ProfileOptions::default()),
            }
        }
        Some(FakeStream::Profiles(spec)) => profile_fixture(&ProfileOptions::from(spec)),
        Some(FakeStream::Fixture { fixture }) => fixture.clone(),
    };
    // 生成 JSON は必ず `RunFixture::parse` へ通す — 無効 step・型違反は
    // benchmark 実行前に fail-fast(§2.5)。
    let text = serde_json::to_string(&doc)
        .map_err(|e| bench_error(format!("fixture document serialize failed: {e}")))?;
    RunFixture::parse(&text)
        .map_err(|e| bench_error(format!("generated fixture failed RunFixture::parse: {e}")))?;
    Ok(doc)
}

/// fixture JSON を生成して `path` へ書き、書いた path を返す
/// (`CHEAPER_TEST_FAKE_STREAM` 設定用)。`RunFixture::parse`
/// round-trip を通さない document は一切 file へ書かない。
pub fn write_fixture(path: &Path, spec: Option<&FakeStream>) -> Result<PathBuf, CheaperError> {
    let doc = fixture_document(spec)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            bench_error(format!(
                "fixture dir create failed {}: {e}",
                parent.display()
            ))
        })?;
    }
    let mut text = serde_json::to_string_pretty(&doc)
        .map_err(|e| bench_error(format!("fixture serialize failed: {e}")))?;
    text.push('\n');
    std::fs::write(path, &text)
        .map_err(|e| bench_error(format!("fixture write failed {}: {e}", path.display())))?;
    Ok(path.to_path_buf())
}

pub(crate) fn bench_error(message: impl Into<String>) -> CheaperError {
    CheaperError::new(ErrorCode::InvalidRequest, message.into())
}

// ---------------------------------------------------------------------------
// emit child の seeded 高エントロピ byte 生成器(§1.4)
// ---------------------------------------------------------------------------

/// 固定 seed(xorshift64* の任意非零 state — 値自体に意味はなく
/// 「seed 固定で deterministic」だけが contract)。
const EMIT_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// `'!'`(0x21)–`'~'`(0x7E)の印字可能非空白 ASCII 94 文字 —
/// 1 byte = 1 UTF-8 char で `len` がそのまま byte 数になる。
const EMIT_ALPHABET_LEN: u64 = 94;

/// xorshift64* — 外部 RNG crate を持ち込まない最小の deterministic
/// PRNG(workspace dep の `fastrand`/`uuid` は seedable ではなく本用途
/// 向きでないため、~10 行の inline 実装を選ぶ — KISS)。
struct SeededRng(u64);

impl SeededRng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

/// `emit` mode の stdout 内容 — `len` byte の deterministic 高エントロピ
/// 印字可能列を返す。固定 seed のため同じ `len` には常に同じ列。
///
/// `'L'.repeat(n)` 型の低エントロピ反復は o200k BPE で高圧縮され
/// token 数が byte 数を大きく下回るため禁止 — condense
/// `when: outputTokens > tokenThreshold` 評価(実 BPE 計数)の決定性
/// を、byte→token 密度の既知下界で担保する(§1.4)。
pub fn seeded_emit_bytes(len: usize) -> Vec<u8> {
    let mut rng = SeededRng(EMIT_SEED);
    (0..len)
        .map(|_| b'!' + (rng.next_u64() % EMIT_ALPHABET_LEN) as u8)
        .collect()
}
