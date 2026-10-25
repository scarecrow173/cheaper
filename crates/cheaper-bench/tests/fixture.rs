//! `tests/fixture.rs` — 生成 fixture JSON の `RunFixture::parse`
//! round-trip・step 語彙・profile 生成・emit generator の pin
//! (design §2.5・§1.4・TSPEC Fixture 生成節)。
//!
//! spawn 測定は一切行わない — `cargo test` 混入禁止(TSPEC boundary
//! invariant)に従い、純粋関数 + 生成 file の parse 検証のみ。

use std::collections::BTreeSet;

use cheaper_bench::fixture::{
    fixture_document, profile_fixture, seeded_emit_bytes, write_fixture, ProfileOptions,
};
use cheaper_bench::manifest::FakeStream;
use cheaper_runtime::dev::fixture::RunFixture;
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// profile 駆動生成(v1 buildFakeStreamFixture parity)
// ---------------------------------------------------------------------------

#[test]
fn profile_fixture_parses_via_run_fixture() {
    let doc = fixture_document(None).unwrap();
    let text = serde_json::to_string(&doc).unwrap();
    // round-trip — canonical schema への anchor。
    RunFixture::parse(&text).unwrap();
}

#[test]
fn profiles_produce_one_session_each() {
    // session 結合規則 — worker job N 件には session N 件
    // (create_session 1 回ごとに sessions[] を 1 件消費)。
    let doc = profile_fixture(&ProfileOptions {
        profiles: vec!["compact".into(), "large".into(), "compact".into()],
        delay_ms: 0,
        tool_call_path: None,
    });
    assert_eq!(doc["sessions"].as_array().unwrap().len(), 3);
    fixture_document(Some(&FakeStream::Profiles(
        cheaper_bench::manifest::ProfileSpec {
            profiles: Some(vec!["compact".into(), "large".into()]),
            delay_ms: None,
            tool_call_path: None,
        },
    )))
    .unwrap();
}

#[test]
fn tool_call_path_only_injects_into_session_zero() {
    // v1 parity — tool_call は queue 先頭の 1 call のみ(session 0 先頭)。
    let doc = profile_fixture(&ProfileOptions {
        profiles: vec!["compact".into(), "compact".into()],
        delay_ms: 0,
        tool_call_path: Some("a.txt".into()),
    });
    let sessions = doc["sessions"].as_array().unwrap();
    let first = &sessions[0]["steps"][0];
    assert_eq!(
        first["tool_calls"],
        json!([{"name": "read", "input": {"path": "a.txt"}}])
    );
    assert_eq!(sessions[0]["steps"][1]["turn"]["tool_calls"], json!(true));
    assert!(sessions[1]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s.get("tool_calls").is_none()));
}

#[test]
fn delay_step_follows_tool_calls() {
    // v1 parity — delay は profile response にのみ掛かるため
    // `tool_calls` step より後に置く。
    let doc = profile_fixture(&ProfileOptions {
        profiles: vec!["compact".into()],
        delay_ms: 25,
        tool_call_path: Some("a.txt".into()),
    });
    let steps = doc["sessions"][0]["steps"].as_array().unwrap();
    let keys: Vec<&str> = steps
        .iter()
        .map(|s| s.as_object().unwrap().keys().next().unwrap().as_str())
        .collect();
    assert_eq!(
        keys,
        vec![
            "tool_calls",
            "turn",
            "delay_ms",
            "usage_delta",
            "turn",
            "finish"
        ]
    );
}

#[test]
fn finish_text_is_bulk_reader_schema_conformant() {
    // bulk-reader output schema の 5 required key — schema 不適合
    // finish は INVALID_OUTPUT で計測を壊すため pin する。
    let doc = profile_fixture(&ProfileOptions::default());
    let text = doc["sessions"][0]["steps"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()["finish"]["text"]
        .as_str()
        .unwrap()
        .to_string();
    let parsed: Value = serde_json::from_str(&text).unwrap();
    for key in [
        "summary",
        "findings",
        "relevantFiles",
        "suggestedReads",
        "openQuestions",
    ] {
        assert!(parsed.get(key).is_some(), "missing key: {key}");
    }
}

// ---------------------------------------------------------------------------
// `{fixture:{…}}` passthrough — DSL 全 step 語彙(失敗系含む)
// ---------------------------------------------------------------------------

#[test]
fn fixture_passthrough_accepts_full_step_vocabulary() {
    // error/hang/hang_until_abort/abort_response/panic 等の失敗系を
    // 含む DSL 全 step が parallel vocabulary なしで書ける
    // (TSPEC — `sessions[].steps` の全節を manifest が直接書く)。
    let spec = FakeStream::Fixture {
        fixture: json!({
            "name": "failure-matrix",
            "sessions": [
                {"steps": [
                    {"emit": {"text_delta": "chunk"}},
                    {"delay_ms": 1},
                    {"tool_calls": [{"name": "read", "input": {"path": "a.txt"}}]},
                    {"usage_delta": {"input": 10, "output": 5}},
                    {"turn": {"tool_calls": true}},
                    {"error": {"status": 429, "retryable": true, "message": "rate"}},
                    {"panic": "boom"},
                    "hang_until_abort",
                    {"finish": "done"}
                ]},
                {"steps": ["hang", "abort_response", {"hang": {}}, {"abort_response": {}}]}
            ],
            "policy": {"timeout_ms": 5000},
            "abort": {"after_ms": 40},
            "expect": {"status": "error", "errorCode": "PROVIDER_FAILURE"}
        }),
    };
    let doc = fixture_document(Some(&spec)).unwrap();
    RunFixture::parse(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(doc["sessions"].as_array().unwrap().len(), 2);
}

#[test]
fn invalid_step_fails_fast_at_generation() {
    // 無効 step・型違反は benchmark 実行前の生成時 fail-fast
    // (RunFixture::parse round-trip anchor — JS 平行実装 drift 解消)。
    for fixture in [
        json!({"sessions": [{"steps": [{"bogus_step": true}]}]}),
        json!({"sessions": [{"steps": [{"delay_ms": "soon"}]}]}),
        json!({"sessions": "not-an-array"}),
        json!({"sessions": [{"steps": [42]}]}),
    ] {
        let spec = FakeStream::Fixture { fixture };
        assert!(
            fixture_document(Some(&spec)).is_err(),
            "expected fail-fast: {spec:?}"
        );
    }
}

#[test]
fn unsupported_literal_fails_fast() {
    let spec = FakeStream::Literal("bogus".to_string());
    assert!(fixture_document(Some(&spec)).is_err());
}

#[test]
fn write_fixture_round_trips_from_disk() {
    // 書込み → file からの再 parse — CHEAPER_TEST_FAKE_STREAM seam が
    // 読むのは file なので disk 経路でも anchor を確認する。
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.json");
    write_fixture(&path, None).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let fixture = RunFixture::parse(&text).unwrap();
    assert_eq!(fixture.sessions.len(), 1);
}

// ---------------------------------------------------------------------------
// emit child generator(§1.4 — deterministic 高エントロピ pin)
// ---------------------------------------------------------------------------

#[test]
fn emit_bytes_are_deterministic_and_exact_length() {
    for len in [0usize, 1, 94, 4096, 32768] {
        let a = seeded_emit_bytes(len);
        let b = seeded_emit_bytes(len);
        assert_eq!(a, b, "deterministic for len={len}");
        assert_eq!(a.len(), len);
    }
}

#[test]
fn emit_bytes_are_printable_high_variety() {
    // `'L'.repeat(n)` 型の低エントロピ反復禁止 — 94 文字 alphabet の
    // 高 variety を pin(condense `outputTokens > tokenThreshold` 評価の
    // byte→token 密度の決定下界)。
    let out = seeded_emit_bytes(4096);
    assert!(out.iter().all(|b| (b'!'..=b'~').contains(b)));
    let distinct: BTreeSet<u8> = out.iter().copied().collect();
    assert!(
        distinct.len() >= 90,
        "low-variety output: {} distinct of 94",
        distinct.len()
    );
    // 先頭 94 byte が alphabet 順単調列でないことも pin(退化 seed の
    // 回帰検出)。
    let head: Vec<u8> = out[..94].to_vec();
    assert_ne!(head, (b'!'..=b'~').collect::<Vec<u8>>());
}
