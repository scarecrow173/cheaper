---
id: "TASK-034AGiBmLrlXnGsS5oLlL8"
type: "task"
status: "done"
title: "Implement process lifecycle policy"
created: "2026-10-05"
updated: "2026-10-05"
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
    - "../plans/implement-managed-process-lifecycle-hardening.md"
  implemented-by: ["../impl/ir/implement-process-lifecycle-policy.md"]
  depends-on:
    - "../plans/implement-managed-process-lifecycle-hardening.md"
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
    - "../test-specs/managed-process-bounded-lifecycle-guarantees.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement process lifecycle policy

## Work

top-level process lifecycle policy の raw/resolved type、shared defaults/ceiling、semantic validation と trusted/default/project-min 合成を一つの core boundary に実装する。

- [x] task の in-progress IR を作成する。
- [x] defaults/validation/layering/asset の回帰 test を記述する。
- [x] focused test の意味のある RED を確認する。
- [x] resolver/schema/merge と許可済み policy.yaml ×2 defaults を実装する。
- [x] focused test の GREEN を確認する。
- [x] policy logical change を commit する。

## Classification

- `normal-plan-task`

## Upstream Documents

- approved plan `../plans/implement-managed-process-lifecycle-hardening.md`、approved spec/accepted ADR/approved design を参照。
- test-spec `../test-specs/managed-process-bounded-lifecycle-guarantees.md` の policy rule を実行可能な test で保護する。

## Dependencies

- plan approval 済み（closed msg_6d66657cbe77 の coordinator direct verdict）。前提は既存 core policy。後続 E と R を block する。

## Affected Files

- `packages/core/src/policy/process.ts`, `schema.ts`, `merge.ts`, 必要な型整合のみ `load.ts`。
- `packages/core/test/policy/process.test.ts`, `packages/core/test/policy/load.test.ts` の canonical default snapshot。
- 承認済み `packages/cli/policy.yaml`, `packages/apm/.apm/hooks/cli/policy.yaml` の新 defaults。
- manifest/依存/root scripts/bundled CLI は変更しない。

## Done When

- [x] legacy defaults と snapshot、4 new restrict keys の project-min、trusted replace、positive Node-bounded duration、1..16MiB cap、per-task override 拒否を検証した。
- [x] 上記範囲を越える変更がなく、IR に RED/GREEN evidence を記録した。

## Verification

- [x] `mise exec -- pnpm exec vitest run packages/core/test/policy/process.test.ts` exits 0。
- [x] `git diff --check` exits 0。
