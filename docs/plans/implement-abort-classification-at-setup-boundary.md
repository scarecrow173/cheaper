---
id: "PLAN-034Eek9Q2fFlfb2FzIBUtz"
type: "plan"
status: "approved"
title: "Implement abort classification at setup boundary"
created: "2026-10-11"
updated: "2026-10-11"
owners: []
relations:
  source:
    - "docs/designs/abort-classification-at-setup-boundary.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/abort-classification-at-setup-boundary.md"
    - "docs/specs/abort-classification-at-setup-boundary.md"
  implemented-by:
    - "docs/tasks/add-errorcode-from-str-reverse-lookup-in-cheaper-core.md"
    - "docs/tasks/restore-setup-boundary-abort-classification.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/centralize-the-abort-code-to-errorcode-reverse-lookup-in-cheaper-core.md"
    - "docs/designs/cli-contract-envelope-exit-codes.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/abort-classification-at-setup-boundary.md"
    - "docs/designs/abort-classification-at-setup-boundary.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/abort-classification-at-setup-boundary.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement abort classification at setup boundary Implementation Plan

## Goal

SPEC-034EdS8ly2SqcS7cJ8hvfJ(issue #94)の fix:
cheaper-core に `ErrorCode::from_str` 逆引きを追加し、
`setup_abort_error` に `code()` 復元分岐を挟んで setup boundary の
abort 分類を記録済み code へ復元し、adapter の private table を
共有 API へ置き換える。

## Scope

単一 implementation stream。core API → consumer 配線の直列 2 task
(T1 → T2)で、同一 plan 内で完結する ~50 行規模の修正。
spec の Out of Scope(abort semantics・retry policy・`ERROR_TABLE`・
envelope schema・code-less abort の fallback 見直し)は全 task で
対象外。

## Upstream Documents

- `docs/specs/abort-classification-at-setup-boundary.md`(approved、
  SPEC-034EdS8ly2SqcS7cJ8hvfJ)— scope・requirements・
  acceptance criteria
- `docs/adr/centralize-the-abort-code-to-errorcode-reverse-lookup-in-cheaper-core.md`
  (accepted、ADR-034EdSlppBqVgIegIeGXgH)— 逆引きの配置(option A:
  cheaper-core・語彙単一 source・寛容 `Option` parse)

## Design Inputs

- `docs/designs/abort-classification-at-setup-boundary.md`
  (DESIGN-034EehnT1xOPAee9dK22Kx)— AUTHORITATIVE。API 形状
  (inherent `from_str` → `Option`、trait 非採用)・
  `setup_abort_error` の分岐順・message 形・adapter 置換範囲は
  全て design に従う
- `docs/designs/cli-contract-envelope-exit-codes.md` — envelope・
  exit code 不変条件の参照元

## File Map

### T1: `ErrorCode::from_str` 逆引き API(cheaper-core)

- `crates/cheaper-core/src/errors.rs` — `impl ErrorCode` に
  `pub fn from_str(s: &str) -> Option<Self>` を `as_str` 隣に追加。
  実装は `ERROR_TABLE` entry 列挙 + `as_str()` 完全一致照合のみ
  (第二の match table を作らない)。`#[cfg(test)]` に
  `from_str` round-trip(全変種)+ 語彙外/小文字/空文字列 → `None`
  の unit test を追加

### T2: setup boundary 復元 + adapter 共有化

- `crates/cheaper-runtime/src/run/mod.rs` — `setup_abort_error`
  (L937-951)に `error_object()` downcast の後・`INTERNAL_ERROR`
  fallback の前へ `reason.code().and_then(ErrorCode::from_str)` の
  復元分岐を追加(message は `"{context} failed: {reason.message()}"`
  維持)。`#[cfg(test)]` に全 `RunAbortReason` の
  `abort_payload` 経路分類 pin test・`error_object` 不変 test・
  語彙外 code/code 無し → `INTERNAL_ERROR` pin test を追加
- `crates/cheaper-rig-adapter/src/error.rs` —
  `abort_reason_to_cheaper`(L204-212)内の
  `error_code_from_str` 呼出しを `ErrorCode::from_str` に置換し、
  private fn `error_code_from_str`(L216-245)と直上コメントを削除。
  `abort_reason_to_cheaper` の契約(`ABORTED` fallback・message 形)
  は不変

## Implementation Sequence

- [ ] T1 `ErrorCode::from_str` + core unit test
- [ ] T2 `setup_abort_error` 復元分岐 + adapter private table 除去 +
      runtime pin test + determinism 確認

T1 は独立に検証可能(`cargo test -p cheaper-core`)。T2 は T1 の API
を consumer するため直列。T2 完了時に全 verification を実行する。

## Dependencies and Risks

| Step | Depends on | Blocks |
| --- | --- | --- |
| T1 core from_str | — | T2 |
| T2 rewiring | T1 | — |

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | `from_str` を trait impl や第二 table で実装し語彙が drift | low | med | design で形状確定済み(ERROR_TABLE × as_str 照合のみ)、round-trip test が語彙追加時の追随を強制 |
| 2 | 分岐順序を誤り `error_object` 経路の挙動が変わる | low | med | 後段追加のみ・`error_object` 不変 pin test・downcast 優先は spec requirement |
| 3 | adapter の契約(`ABORTED` fallback・message 形)を誤って変更 | low | med | 共有するのは写像 table のみ — 変換関数の変更は call site 1 行 + table 削除に限定 |

## Task Breakdown

- `docs/tasks/add-errorcode-from-str-reverse-lookup-in-cheaper-core.md`
  (T1)
- `docs/tasks/restore-setup-boundary-abort-classification.md`(T2)

## Verification

- [ ] `mise exec -- cargo test -p cheaper-core` exits 0(from_str
      round-trip・語彙外 → `None` を含む)
- [ ] `mise exec -- cargo test -p cheaper-runtime` exits 0
      (setup_abort_error 分類 pin test を含む)
- [ ] `mise exec -- cargo test -p cheaper-rig-adapter` exits 0
- [ ] `git grep -n "error_code_from_str" -- crates/` が 0 件
- [ ] `git grep -n "and_then(ErrorCode::from_str)" --
      crates/cheaper-runtime crates/cheaper-rig-adapter` がちょうど
      2 件(runtime `setup_abort_error` + adapter
      `abort_reason_to_cheaper` の両 consumer が共有 API を使用)
- [ ] `mise exec -- cargo test --workspace --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam` を連続 3 回
      実行し `hang_grace`(fixtures_replay_all)が毎回 deterministic
      に pass する
- [ ] `mise run check` exits 0

## Open Questions / Gaps

- なし(API 形状・分岐順・message 形は design で確定済み)

## Self-Review

- [x] Every spec requirement maps to a task(逆引き単一化 → T1、
      setup 復元 + adapter 置換 + pin test → T2)
- [x] No placeholders remain
- [x] File paths, relations, status values, and terminology are
      consistent

## Implementation Handoff

- Recommended mode: inline execution(2 task・~50 行の直列修正で
  委譲の価値が薄い)
- Independent work streams: none(T1 → T2 直列)
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix.

## Review Handoff

- 上流 doc: spec(approved)+ ADR(accepted)+ design で API 形状・
  分岐順・message 形は確定済み。Open Questions は無い
- task 分解の基準: core の語彙所有(T1)と consumer 配線(T2)の
  責務境界。runtime と adapter の編集は同じ合成規則
  (`code().and_then(ErrorCode::from_str)`)の適用であり T2 に集約
- `hang_grace` determinism は 3 回連続 workspace test run で確認
  し、`mise run check` green を最終 gate とする
