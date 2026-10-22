---
id: "TASK-03404zg7SVQPn5JeZ0lwDD"
type: "task"
status: "done"
title: "Make exec_infra_failure_returns_error deterministic via error code and message"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/157"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/fix-route-test-flake-via-test-side-determinism.md"
  implemented-by:
    - "docs/impl/ir/fix-route-test-determinism-for-ubuntu-ci-flake-issue-157.md"
  depends-on:
    - "docs/plans/fix-route-test-flake-via-test-side-determinism.md"
  blocks:
    - "docs/tasks/verify-route-test-determinism-and-mise-check.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/fix-route-test-flake-via-test-side-determinism.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/deterministic-route-test-assertions.md"
  references:
    - "docs/designs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
  defers: []
  deferred-by: []
---

# Make exec_infra_failure_returns_error deterministic via error code and message

## Work

`crates/cheaper-cli/tests/integration/route_run.rs` の
`exec_infra_failure_returns_error`(:599-637)で、実行到達証明を
`marker.exists()`(child が kill 前に marker を書けたかの timing 依存
観測)から、`run_route` の戻り `Err` の code + message assert へ置き換える
(design D1)。fixture 配線は変更しない。

手順:

1. file 先頭の use 群に
   `use cheaper_core::errors::{CheaperError, ErrorCode};` を追加する
   (先例: `tests/integration/envelope.rs:10`)。
2. `assert!(run_route(&mut f, &token).await.is_err());`(:627)と
   `marker.exists()` assert(:628-631)を、design D1 の確定形へ置き換える:

   ```rust
   let err = run_route(&mut f, &token)
       .await
       .err()
       .expect("route must fail: grant timeout is 1ms");
   assert_eq!(
       err.downcast_ref::<CheaperError>().map(CheaperError::code),
       Some(ErrorCode::ToolExecutionFailure),
       "job 失敗は engine が outcome.error を ToolExecutionFailure に collapse する決定的 infra-failure code",
   );
   assert!(
       err.to_string().contains("routed command did not complete"),
       "captured()==None && outcome.error.is_some() 分岐固有の message — workflow が exec job を失敗まで実行した証拠: {err}",
   );
   ```

   - `.err().expect(...)` を使う — `unwrap_err()` は `RouteRunOutput:
     Debug` を要求するため使わない(design Components and Boundaries)。
   - `ErrorCode::Timeout` の assert は使わない — engine が job 失敗を
     `ToolExecutionFailure` に collapse するため `Err` 側へ届かず、全
     platform で決定的 fail になる(design D1 代替案)。

維持するもの(変更禁止):

- `timeout: '1ms'`(:603)、policy.yaml の `CHEAPER_ROUTE_CHILD_MODE: io`
  → `sleep` 書換え(:604-613)、`reject_fx` 呼出し、payload token 生成。
- `marker` 変数自体 — `reject_fx` signature が `&marker` を要求し
  `CHEAPER_ROUTE_CHILD_MARKER` は `revalidation_failures_reject_...` 側と
  共有する fixture 構築物のため、assert 対象から外すだけで配線は残す
  (design D5)。
- usage log に `"route"` record が書かれないことの assert(:632-636)。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/fix-route-test-flake-via-test-side-determinism.md (T1 slice)
- docs/designs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md (D1)

## Dependencies

- docs/plans/fix-route-test-flake-via-test-side-determinism.md

## Affected Files

- crates/cheaper-cli/tests/integration/route_run.rs

## Done When

- [x] `run_route` の `Err` を `.err().expect(...)` で取り出し
      `code() == ErrorCode::ToolExecutionFailure` を assert する
- [x] `err.to_string().contains("routed command did not complete")` を
      assert する
- [x] `marker.exists()` 系の timing 依存 assertion が残っていない
- [x] `timeout: '1ms'`・`is_err()` 相当の失敗検証・marker/reject_fx 配線・
      usage log assert が維持されている

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_run::exec_infra_failure_returns_error -- --exact` exits 0
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_run::` exits 0(他 route_run test の非 regress)
- [x] `mise exec -- cargo fmt --check` exits 0

## Done

`route_run.rs` に `use cheaper_core::errors::{CheaperError, ErrorCode};` を
追加し、`exec_infra_failure_returns_error` の `marker.exists()` assert を
`.err().expect(...)` + `code() == ErrorCode::ToolExecutionFailure` +
message `"routed command did not complete"` の assert へ置き換えた。
fixture 配線(`timeout: '1ms'`・reject_fx・marker・policy.yaml 書換え)と
usage log assert は不変。
