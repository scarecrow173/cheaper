---
id: "TASK-034EDQ2U9suHgZ8yJxDtPt"
type: "task"
status: "done"
title: "T03: cheaper-cli error/signal helpers and runtime entry"
created: "2026-10-12"
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
    - "docs/plans/execute-post-feature-structural-refactoring.md"
  implemented-by: []
  depends-on:
    - "docs/plans/execute-post-feature-structural-refactoring.md"
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
    - "docs/test-specs/post-feature-structural-refactoring-parity.md"
  references: []
  defers: []
  deferred-by: []
---

# T03: cheaper-cli error/signal helpers and runtime entry

## Work

cli crate の重複を集約する（spec Phase 0 cli 項目）。

- `invalid_request` 共通 helper 化 + `--timeout` error helper 共通化
  （`errors.rs` 新規 or `cli.rs` 内集約）— `workflow_run.rs` の
  details 欠落を `run.rs` と同じ形に揃える（意図的例外）
- `wait_os_signal`/`SignalRelay` を `signal.rs` 新規 module へ集約し
  run/workflow で共有（auth の thread 型 signal は結合しない）
- `Runtime::create` + devin plugin 登録の単一入口化（run.rs・auth.rs・
  usage_report.rs の 3 箇所）
- `count.rs` の `BoxError` 重複を cli.rs/errors.rs 側定義に一本化

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-feature-structural-refactoring.md`
- `docs/specs/post-feature-structural-refactoring-scope.md`
- `docs/designs/post-feature-structural-refactoring.md`

## Dependencies

- なし

## Affected Files

- `crates/cheaper-cli/src/{cli.rs,errors.rs,signal.rs,run.rs,workflow_run.rs,
  auth.rs,usage_report.rs,count.rs,models.rs}`

## Done When

- [ ] helper 群が一箇所から参照される
- [ ] `workflow run --timeout` が `details.timeout` を emit する
  （run.rs と同一 shape の意図的例外）
- [ ] それ以外の envelope/error/exit code に差分なし

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` が pass
