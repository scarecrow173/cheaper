---
id: "TASK-0060"
type: "task"
status: "todo"
title: "Anchor policy timeout at runWorker start and amend DESIGN-0004"
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
    - "docs/tasks/0057-extract-promptwithabortgrace-helper-and-apply-to-main-run.md"
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

# Anchor policy timeout at runWorker start and amend DESIGN-0004

## Work

timeout タイマー開始を `runWorker` 先頭（`resolvePiModel` 前）へ移し、model 解決・session 構築を含む run 区間を policy timeout で計測する。`session.abort()` は helper の signal listener 経由なので `session` 未生成でも `runAbort` 経由で機能する。あわせて DESIGN-0004 の「CLI 入口で `setTimeout`」の記述を「agent execution timeout（runWorker 開始時点）」に改訂し、CLI 前段（stdin 読込・worker/policy 解決・runtime 初期化）は対象外であることを明記する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/adr/0010-normalize-repair-abort-grace-provider-failure-classification-and-agent-execution.md`
- `docs/designs/0010-repair-abort-and-failure-normalization-hardening.md`
- `docs/designs/0004-pi-execution-engine.md`

## Dependencies

- TASK-0057（`requestAbort`/`promptWithAbortGrace` の構造変更が前提）

## Affected Files

- `packages/runtime-pi/src/run.ts`
- `docs/designs/0004-pi-execution-engine.md`
- `packages/runtime-pi/test/run.test.ts`（model 解決・session 構築を含む timeout の確認）

## Done When

- [ ] timeout が `runWorker` 開始時点から計測される（model 解決・session 構築を含む）
- [ ] DESIGN-0004 が agent execution timeout として記述され、CLI 前段が対象外と明記される

## Verification

- [ ] `mise exec -- pnpm -F @cheaper/runtime-pi test` が green
- [ ] session 構築が遅延した場合に `TIMEOUT` になることを示すテストまたは確認
