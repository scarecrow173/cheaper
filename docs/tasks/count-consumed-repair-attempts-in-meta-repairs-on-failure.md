---
id: "TASK-034r05iHQeQpDpIa7OmcYd"
type: "task"
status: "done"
title: "Count consumed repair attempts in meta.repairs on failure"
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
    - "docs/plans/harden-repair-abort-grace-and-provider-failure-normalization.md"
  implemented-by: []
  depends-on:
    - "docs/plans/harden-repair-abort-grace-and-provider-failure-normalization.md"
    - "docs/tasks/apply-abort-grace-and-provider-failure-normalization-to-repair.md"
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

# Count consumed repair attempts in meta.repairs on failure

## Work

`repairOutput` に `onAttempt?: (attempt: number) => void` を追加し、各 attempt 開始時に呼び出す。`run.ts` は `onAttempt` で `repairs` を更新し、attempts 使い切り（`INVALID_OUTPUT`）・repair 中 abort・途中失敗でも `meta.repairs` が消費回数を示すようにする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/repair-and-abort-hardening-telemetry-consistency-and-test-hermeticity.md`
- `docs/designs/repair-abort-and-failure-normalization-hardening.md`

## Dependencies

- TASK-034r05iHQeQpDpHsFsVyxq（repair.ts の変更が前提）

## Affected Files

- `packages/runtime-pi/src/repair.ts`
- `packages/runtime-pi/src/run.ts`
- `packages/runtime-pi/test/repair.test.ts` / `run.test.ts`（失敗時 `meta.repairs` の検証）

## Done When

- [ ] repair attempts 使い切りで `INVALID_OUTPUT` になった場合 `meta.repairs` が消費回数を示す
- [ ] repair 中に abort/timeout した場合も `meta.repairs` が消費回数を示す
- [ ] 成功時は従来どおり `meta.repairs` が成功までの attempt 数を示す

## Verification

- [ ] `mise exec -- pnpm -F @cheaper/runtime-pi test` が green
- [ ] 失敗時 `meta.repairs` のテストが追加されている
