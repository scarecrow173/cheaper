---
id: "TASK-0058"
type: "task"
status: "todo"
title: "Apply abort grace and provider failure normalization to repair"
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

# Apply abort grace and provider failure normalization to repair

## Work

`repair.ts` の各 attempt で `promptWithAbortGrace` を使い、signal abort 後 `graceMs` で待機を打ち切る。`unlink`（signal→`session.abort()`）は helper に吸収させる。repair の `session.prompt()` rejection は `PROVIDER_FAILURE`、session が `stopReason === "error"` で resolve した場合も `PROVIDER_FAILURE` に正規化する。`lastAssistantStop` を共有モジュールへ移して repair からも使う。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/0010-repair-abort-and-failure-normalization-hardening.md`
- `docs/adr/0010-normalize-repair-abort-grace-provider-failure-classification-and-agent-execution.md`

## Dependencies

- TASK-0057（`promptWithAbortGrace` helper が前提）

## Affected Files

- `packages/runtime-pi/src/repair.ts`
- `packages/runtime-pi/src/run.ts`（`lastAssistantStop` の移動元）
- `packages/runtime-pi/src/output.ts` または共有モジュール（`lastAssistantStop` 移動先）
- `packages/runtime-pi/test/repair.test.ts`（abort grace・`PROVIDER_FAILURE` 分類）

## Done When

- [ ] repair の `session.prompt()` が abort signal 受信後 `graceMs` で待機打ち切りされる
- [ ] repair の prompt rejection が `INTERNAL_ERROR` ではなく `PROVIDER_FAILURE` になる
- [ ] repair session が `stopReason === "error"` で resolve した場合 `INVALID_OUTPUT` ではなく `PROVIDER_FAILURE` になる
- [ ] `lastAssistantStop` が一箇所に定義され main/repair で共有される

## Verification

- [ ] `mise exec -- pnpm -F @cheaper/runtime-pi test` が green
- [ ] signal を無視する streamFn で repair 中 abort → grace 打ち切りのテストが追加されている
