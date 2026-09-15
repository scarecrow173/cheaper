---
id: "TASK-0057"
type: "task"
status: "todo"
title: "Extract promptWithAbortGrace helper and apply to main run"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
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
    - "docs/plans/0005-harden-repair-abort-grace-and-provider-failure-normalization.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0005-harden-repair-abort-grace-and-provider-failure-normalization.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Extract promptWithAbortGrace helper and apply to main run

## Work

`session.prompt()` と abort grace（signal abort 後 `graceMs` で待機打ち切り）の race を `promptWithAbortGrace` として新規モジュールに抽出し、`run.ts` の main prompt を置き換える。`requestAbort` は `abortReason` 記録と `runAbort.abort()` のみに縮退し、`session.abort()`・grace timer 起動は helper 側の signal listener へ移す。grace timer は prompt settle 時に `clearTimeout` して event loop を保持しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/0010-repair-abort-and-failure-normalization-hardening.md`
- `docs/specs/0005-repair-and-abort-hardening-telemetry-consistency-and-test-hermeticity.md`
- `docs/adr/0010-normalize-repair-abort-grace-provider-failure-classification-and-agent-execution.md`

## Dependencies

- なし（本チェーンの先頭タスク）

## Affected Files

- `packages/runtime-pi/src/prompt-grace.ts`（新規）
- `packages/runtime-pi/src/run.ts`
- `packages/runtime-pi/test/run.test.ts`（abort 経路の回帰 + grace timer 解放の確認）

## Done When

- [ ] main prompt が `promptWithAbortGrace` 経由で実行され、abort 後 `graceMs` で `grace` 解決する
- [ ] `requestAbort` が `session.abort()`/`setTimeout(resolveGrace)` を持たない
- [ ] prompt settle 後に grace timer が残らない（`clearTimeout` または `unref`）

## Verification

- [ ] `mise exec -- pnpm -F @cheaper/runtime-pi test` が green
- [ ] abort 系の既存テスト（timeout/signal/limit）が変更なしで通る
