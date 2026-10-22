---
id: "ADR-034KoUAHJ1vI8BbyJoL0Q9"
type: "adr"
status: "accepted"
title: "Resolve route test flake via test-side determinism"
created: "2026-10-22"
updated: "2026-10-22"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/ubuntu-ci-route-test-flake.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/ubuntu-ci-route-test-flake.md"
  derived-by:
    - "../designs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Resolve route test flake via test-side determinism

## Context and Problem Statement

issue #157 の 2 件の route 系 test が ubuntu CI で断続失敗する。原因は
本番実装ではなく test が timing(`timeout: '1ms'` vs 子 process の
spawn→marker 書込み)と `fs::read_dir` 列挙順(mtime 秒 tie の
stable-sort tie-break)に暗黙依存していること。system architecture 上の
判断(新規 pattern・依存・境界の変更)は不要で、scope は test file のみ。
この ADR は修正方針の選択根拠を最小限に記録する。

## Considered Options

* Option A: test 側で determinism を確保する — sweep test は対象 file に
  `set_modified` で一意な mtime 順序を与え、infra-failure test は
  `marker.exists()` の代わりに error の `code() ==
  ErrorCode::ToolExecutionFailure` + message `"routed command did
  not complete"` を検証する(engine が job 失敗を
  `ToolExecutionFailure` に collapse するため `Err` の code からは
  `Timeout` は検証できない。message は `finish` の
  `captured() == None && outcome.error.is_some()` 分岐に固有の
  post-admission 証拠)。
* Option B: timing 余裕を確率的に広げる — grant `timeout` を数百 ms〜
  秒に緩めて marker 書込みが kill に先勝つことを期待する(issue 本文の
  対応案)。
* Option C: 実装側に deterministic tie-break を入れる —
  `sweep_with` の sort key を (mtime 秒, path) にして削除順を
  platform 非依存にする。

## Decision Outcome

Chosen option: "Option A", because 違反している invariant は「test の
assertion が制御下にない外部状態(timing・readdir 順)を観測する」こと
であり、責務境界は test 側にある。Option B は race を残したまま window
を広げるだけで理論上の flake が残り、test 時間も伸びる。Option C は
production filename(`<unix-ms>-<pid>-<hex>.log`)で mtime tie がほぼ
起きない領域への変更で、本件の主原因(test 構築)を直さないため YAGNI —
将来必要になれば別途検討する(spec Deferred Design Concerns)。

## Implementation Plan

* Affected paths:
  `crates/cheaper-cli/tests/integration/route_run.rs`
  (`exec_infra_failure_returns_error`)、
  `crates/cheaper-cli/src/route/log_store.rs`
  (`sweep_enforces_ttl_count_size_and_skips_tmp`)
* Constraints: 本番コード変更なし。`timeout: '1ms'`・30s sleep
  fixture・sweep の TTL/count/size semantics・`.tmp` 除外検証は維持。
  error は `route::run::finish` が包み直す
  `CheaperError::new(error.code /* ToolExecutionFailure */,
  "routed command did not complete")` を
  `downcast_ref::<CheaperError>()` で取り出し、
  `code() == ErrorCode::ToolExecutionFailure` と message
  `contains("routed command did not complete")` で検証する。job の
  diagnostic code(`Timeout` 等)は engine が `outcome.error` を
  `ToolExecutionFailure("workflow job failed")` に collapse する
  (engine.rs:1172-1188)ため `Err` 側へ伝播しない — `Timeout`
  code の assert は全 platform で決定的に fail するため使わない。

## Verification

* [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_run::exec_infra_failure_returns_error -- --exact` が連続実行で安定して pass する
* [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam sweep_enforces` が pass する
* [ ] ubuntu CI の `mise run check` が pass する
