---
id: "SPEC-034EdS8ly2SqcS7cJ8hvfJ"
type: "spec"
status: "implemented"
title: "Restore recorded abort code classification at setup boundaries"
created: "2026-10-11"
updated: "2026-10-12"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/94"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/centralize-the-abort-code-to-errorcode-reverse-lookup-in-cheaper-core.md"
    - "../adr/normalize-repair-abort-grace-provider-failure-classification-and-agent-execution.md"
    - "./repair-and-abort-hardening-telemetry-consistency-and-test-hermeticity.md"
    - "../designs/cli-contract-envelope-exit-codes.md"
  refines:
    - "../adr/normalize-repair-abort-grace-provider-failure-classification-and-agent-execution.md"
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Restore recorded abort code classification at setup boundaries

## What Should Be Built

run pipeline の setup boundary(`scope.ready()` 等の tool scope 構築区間)で
`AbortReason` が throw されたとき、`AbortReason.code()` に記録済みの分類
code(`ABORTED`/`TIMEOUT`/`MAX_TURNS_EXCEEDED`/`MAX_TOOL_CALLS_EXCEEDED`)を
`ErrorCode` へ復元し、envelope の error code に反映する。

現状は非対称: prompt/stream 経路では adapter の `abort_reason_to_cheaper`
(`crates/cheaper-rig-adapter/src/error.rs:204-212`)が `code()` を
`error_code_from_str` で逆引きして分類を復元するのに対し、setup 経路の
`setup_abort_error`(`crates/cheaper-runtime/src/run/mod.rs:937-951`)は
`error_object()` の downcast だけを見る。`RunAbortReason::abort_payload`
(`crates/cheaper-runtime/src/run/abort.rs:69-75`)が生成する payload は
code を保持するが error object を持たない(`AbortReason::new` 経由)ため、
downcast は `None` となり `INTERNAL_ERROR` fallback(例:
`"tool scope ready failed: worker run aborted: signal"`)へ落ちる。

あわせて code→`ErrorCode` の逆引き写像を 1 箇所の authoritative 実装へ
集約し、adapter と runtime の双方が共有する。配置先は
ADR-034EdSlppBqVgIegIeGXgH の決定に従う。

## Why Now

issue #94: abort が tool-scope setup 区間に着弾すると `hang_grace`
fixture が期待する `ABORTED` ではなく `INTERNAL_ERROR` を返して flake
する。dev-seam 付きの feature-unified workspace test run ではほぼ確定的に
失敗し、`mise run check` の信頼性を毀損する。

abort 分類は envelope `error.code`・exit code・retry 判断に直結する外部
contract であり、stream 経路と setup 経路の分類非対称は同じ abort に
対して到達区間によって結果が変わる invariant 違反である。

## Users and Value

- 親 Coding Agent: setup 区間での abort でも envelope `error.code` が
  記録済み分類(`ABORTED`/`TIMEOUT`/`MAX_TURNS_EXCEEDED`/
  `MAX_TOOL_CALLS_EXCEEDED`)を返し、retryable 判定・exit code が
  stream 経路と一致する。
- contributor / CI: `hang_grace` fixture が deterministic になり、
  `cargo test --workspace` の結果を信頼できる。

## Scope

### In Scope

- `setup_abort_error`(`crates/cheaper-runtime/src/run/mod.rs`)に
  `reason.code()` → `ErrorCode` の復元分岐を追加する。
  優先順序は `error_object()` downcast → `code()` 復元 →
  `INTERNAL_ERROR` fallback の後段追加とし、既存経路の挙動は変えない。
- code→`ErrorCode` 逆引き写像の単一化(配置は ADR の決定に従い、
  adapter・runtime 双方から到達可能な leaf crate — cheaper-core の
  `ErrorCode` 逆引き API を想定)。
- adapter の private table `error_code_from_str`
  (`crates/cheaper-rig-adapter/src/error.rs:216-245`)を共有実装への
  呼び出しに置き換える。
- setup-phase abort 分類を `RunAbortReason` 全種で pin する unit test。

### Out of Scope

- abort semantics 自体の変更(reason 先勝ち記録、abort grace、signal
  配線、`abort_payload` の形)。
- `hang_grace` 以外の flake の調査・修正。
- retry policy・`ERROR_TABLE`(exit code/retryable)の変更。
- envelope schema(`schemaVersion`)・exit code 表の変更。
- `code()` を持たない `AbortReason` の setup boundary fallback 挙動の
  見直し(現行 `INTERNAL_ERROR` を維持 — Deferred Design Concerns
  参照)。

## Requirements

- `reason.error_object()` が `CheaperError` へ downcast できる経路は
  挙動不変(code + message + details の rebuild をそのまま返す)。
- `error_object` が無く `code()` が既知の `ErrorCode` 文字列の場合、
  その code を復元した `CheaperError` を構成する。復元は
  `reason.code()` の文字列のみを key とし、`name() == "CheaperError"`
  のような payload name 照合は要求しない(adapter の
  `abort_reason_to_cheaper` と同じ規則)。現時点で ErrorCode 語彙の
  code を生成するのは describeError と `abort_payload` のみであり、
  この規則は安全である。
- `code()` が無い、または既知の `ErrorCode` 文字列でない場合は、
  現行の `INTERNAL_ERROR` fallback(`"{context} failed: {message}"`)を
  維持する。
- 逆引き写像は 1 箇所の authoritative 実装とし、adapter・runtime の
  双方がそれを使う。`AbortReason.code()` は CheaperError code 以外の
  describeError code(Node errno `"ENOENT"` 等)も運ぶため、逆引きは
  未知文字列で `None` を返す寛容な parse であること。
- 依存方向 `cli → {workflow, provider} → runtime → {rig-adapter, tools}
  → core` を壊さない。実際の edge は
  `cheaper-workflow → {core, tools}`(runtime の sibling であり、
  cli が runtime executor を注入する)、
  `cheaper-provider → {runtime, rig-adapter, core}`、
  `cheaper-runtime → {core, tools, rig-adapter}` であり、
  workflow は runtime に依存しない。
- `cheaper-core` は Rig/Tokio 依存を持ち込まないままとする。
- 新規 dependency は追加しない。

## Acceptance Criteria

- [x] `RunAbortReason` 全種(`Timeout`/`Signal`/`MaxTurns`/`MaxToolCalls`)
  について、setup boundary で abort が着弾した場合に
  `TIMEOUT`/`ABORTED`/`MAX_TURNS_EXCEEDED`/`MAX_TOOL_CALLS_EXCEEDED`
  が復元される unit test がある。
- [x] `hang_grace` fixture が dev-seam 付き feature-unified workspace
  test run で deterministic に pass する。
- [x] `error_object` を持つ `AbortReason` の分類挙動が不変であることを
  示す test がある。
- [x] `ErrorCode` 語彙外の code 文字列(Node errno `"ENOENT"` 等)を
  運ぶ `AbortReason`、または error object が存在するが
  `CheaperError` ではない `AbortReason` が、依然として
  `INTERNAL_ERROR` と `"{context} failed: {message}"` の形を維持して
  返されることを pin する test がある。
- [x] code→`ErrorCode` 写像が 1 箇所のみに存在し、adapter 側の
  private table が除去されている。
- [x] envelope schema・exit code・`ERROR_TABLE` が不変。
- [x] `mise run check` が全工程 green。

## Deferred Design Concerns

- `code()` を持たない `AbortReason` の setup boundary fallback(現行
  `INTERNAL_ERROR`) | code-less abort(DOMException 型 payload 等)を
  `ABORTED` 寄りに分類すべきかの検討は、本修正で観測された症状
  (code 保持 payload)の外 | 実害が観測された時点で再検討 |
  稀な code-less abort が `INTERNAL_ERROR` として残留する可能性。

## Sources and Relations

- issue #94: https://github.com/scarecrow173/cheaper/issues/94 —
  `hang_grace` fixture の `INTERNAL_ERROR` vs `ABORTED` flake。
- 根本原因検証:
  - `RunAbortReason::abort_payload`
    (`crates/cheaper-runtime/src/run/abort.rs:69-75`)は code を保持し
    error object を持たない `AbortReason` を生成する。
  - `setup_abort_error`(`crates/cheaper-runtime/src/run/mod.rs:937-951`)
    は `error_object()` downcast のみを見て `INTERNAL_ERROR` へ落とし、
    `reason.code()` を無視する。call site は `scope.ready()`
    (run/mod.rs:664-668)。
  - adapter の `abort_reason_to_cheaper`
    (`crates/cheaper-rig-adapter/src/error.rs:204-212`)は
    `error_code_from_str` で code を復元する — 非対称の原因。
- 写像配置の決定: ADR-034EdSlppBqVgIegIeGXgH。
- 関連系譜: SPEC-034r05iHK8bA7g9KcsCqKj(repair/abort hardening)、
  ADR-034r05iHrZUE6W2tpzbgKv(abort grace・失敗分類の正規化)、
  DESIGN cli-contract-envelope-exit-codes(envelope/exit code 不変条件)。
