---
id: "TASK-034EekSpuRWpAF5iF3T316"
type: "task"
status: "done"
title: "T2: setup boundary abort code restore and shared reverse lookup"
created: "2026-10-11"
updated: "2026-10-12"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-abort-classification-at-setup-boundary.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-abort-classification-at-setup-boundary.md"
    - "docs/tasks/add-errorcode-from-str-reverse-lookup-in-cheaper-core.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/abort-classification-at-setup-boundary.md"
  references: []
  defers: []
  deferred-by: []
---

# T2: setup boundary abort code restore and shared reverse lookup

## Work

plan `docs/plans/implement-abort-classification-at-setup-boundary.md`
(PLAN-034Eek9Q2fFlfb2FzIBUtz)の T2。T1 の `ErrorCode::from_str` を
両 consumer に配線する: (1) `setup_abort_error` に `code()` 復元分岐
を追加して setup boundary の abort 分類を記録済み code へ復元、
(2) adapter の private table `error_code_from_str` を削除して共有
API へ切替。分類を pin する unit test と `hang_grace` determinism
確認まで含む。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-abort-classification-at-setup-boundary.md` —
  本 task が implement する plan。File Map「T2」が作業の正
- `docs/designs/abort-classification-at-setup-boundary.md`
  (DESIGN-034EehnT1xOPAee9dK22Kx)— 分岐順・message 形・adapter
  置換範囲の AUTHORITATIVE 定義(実装疑似コードあり)
- `docs/specs/abort-classification-at-setup-boundary.md`
  (SPEC-034EdS8ly2SqcS7cJ8hvfJ)— requirements・acceptance criteria

## Dependencies

- `docs/tasks/add-errorcode-from-str-reverse-lookup-in-cheaper-core.md`
  (T1)— `ErrorCode::from_str` が前提

## Affected Files

- `crates/cheaper-runtime/src/run/mod.rs`:
  - `setup_abort_error`(L937-951)に `error_object()` downcast の後・
    `INTERNAL_ERROR` fallback の前へ復元分岐を追加:
    `reason.code().and_then(ErrorCode::from_str)` が `Some(code)`
    なら `CheaperError::new(code, format!("{context} failed: {}",
    reason.message()))` を返す。message は fallback と同じ
    `"{context} failed:"` prefix を維持し、details は付けない
  - `#[cfg(test)] mod tests` に unit test を追加:
    - `RunAbortReason` 全種の分類 pin — `RunAbort::new()` に
      `request_abort(reason)` して `signal().reason().unwrap()`
      で実際の `abort_payload` 形 `AbortReason` を得て
      `setup_abort_error("tool scope ready", ...)` を呼び、
      Timeout→`Timeout`・Signal→`Aborted`・MaxTurns→
      `MaxTurnsExceeded`・MaxToolCalls→`MaxToolCallsExceeded` と
      message `"tool scope ready failed: worker run aborted:
      {reason}"` を検証
    - `error_object` 経路不変 — `AbortReason::from_error` で包んだ
      `CheaperError`(code + details 保持)が downcast rebuild で
      そのまま返る(`code()` 復元より優先)
    - 語彙外 code(`AbortReason::new("Error", "io failed",
      Some("ENOENT".to_string()))`)・code 無し(`AbortReason::msg`)が
      `INTERNAL_ERROR` + `"{context} failed: {message}"` を維持
    - `error_object` は存在するが `CheaperError` ではない residual
      — `AbortReason::from_error` で
      `cheaper_core::errors::node_io_error_with_code("open", "/x",
      "ENOENT", io::Error::new(...))` 等の `NodeIoError` を包んだ
      reason が downcast miss → `code()` 復元でも `None`
      (`"ENOENT"` は語彙外)となり `INTERNAL_ERROR` +
      `"{context} failed: {message}"` を維持。`from_error` は
      `code() == Some("ENOENT")` も返すため、この 1 ケースで
      「非 ErrorCode code」と「error object 非 CheaperError」の
      AC 両方を cover できる
- `crates/cheaper-rig-adapter/src/error.rs`:
  - `abort_reason_to_cheaper`(L204-212)の
    `reason.code().and_then(error_code_from_str)` を
    `reason.code().and_then(ErrorCode::from_str)` に置換
  - private fn `error_code_from_str`(L216-245)と直上の
    「cheaper-core に `FromStr` は無いので adapter 側で閉じて持つ」
    コメントを削除。`abort_reason_to_cheaper` の契約(`ABORTED`
    fallback・`"worker run aborted: {message}"` 形)は不変

## Done When

- [x] `RunAbortReason` 全種について `abort_payload` 経路が
      `setup_abort_error` で正しい `ErrorCode` に復元される unit
      test が pass
- [x] `error_object` 保持 `AbortReason` の分類挙動が不変である pin
      test が pass
- [x] 語彙外 code・code 無し・`error_object` 保持だが非
      `CheaperError`(例: `AbortReason::from_error(NodeIoError)`)
      の `AbortReason` が `INTERNAL_ERROR` + `"{context} failed:
      {message}"` を維持する pin test が pass
- [x] adapter の private `error_code_from_str` が除去され
      `ErrorCode::from_str` を使っている
- [x] envelope schema・exit code・`ERROR_TABLE` が無変更
- [x] `hang_grace` fixture が dev-seam 付き workspace test run で
      deterministic に pass(連続 3 回)

## Out of Scope

- `abort_reason_to_cheaper` の `ABORTED` fallback・message 形の変更
- code-less `AbortReason` の setup boundary fallback 見直し(現行
  `INTERNAL_ERROR` 維持 — spec の Deferred Design Concerns)
- abort semantics・`abort_payload` の形・`RunAbortReason::error_code`
  写像の変更

## Verification

- [x] `mise exec -- cargo test -p cheaper-runtime` exits 0
- [x] `mise exec -- cargo test -p cheaper-rig-adapter` exits 0
- [x] `git grep -n "error_code_from_str" -- crates/` が 0 件
- [x] `git grep -n "and_then(ErrorCode::from_str)" --
      crates/cheaper-runtime crates/cheaper-rig-adapter` が 2 件
      (両 consumer が共有 API を使用)
- [x] `mise exec -- cargo test --workspace --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam` を連続 3 回
      実行し全て exits 0(`hang_grace` の deterministic pass)
- [x] `mise run check` exits 0

## Closing Note

- `crates/cheaper-runtime/tests/fixtures/08-signal-abort.json` の
  `abort.after_ms` 40 → 120 引き上げ(`fb145df`)を本 task の作業に
  含めた。Affected Files 外だが review で in-scope-adjacent と判断
  された — fixture 09(`partial-mutation`、`after_ms: 120`)や CLI
  regression case(`run-signal-abort` 800ms・`run-hang-grace`
  500ms)と同じ race class(setup 区間への abort 着弾)であり、
  setup 着弾した abort には design 確定どおり details が付かない
  ため、code fix だけでは post-setup 着弾を前提に書かれた
  `errorDetails`/streamed meta の assert を満たせない。bump 後
  dev-seam 付き workspace run 3/3 で deterministic pass。詳細は
  `tmp/task-reports/abort-T2.md` の Scope note を参照。
