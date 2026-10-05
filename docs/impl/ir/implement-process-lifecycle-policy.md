---
id: "IMPL-034AGqZzIjQvvNqijsHb1N"
type: "impl"
status: "in-progress"
title: "Implement process lifecycle policy"
created: "2026-10-05"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - { type: "file", path: "packages/core/src/policy/process.ts" }
      - { type: "file", path: "packages/core/src/policy/schema.ts" }
      - { type: "file", path: "packages/core/src/policy/merge.ts" }
      - { type: "file", path: "packages/core/test/policy/process.test.ts" }
      - { type: "file", path: "packages/core/test/policy/load.test.ts" }
      - { type: "file", path: "packages/cli/policy.yaml" }
      - { type: "file", path: "packages/apm/.apm/hooks/cli/policy.yaml" }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/implement-process-lifecycle-policy.md"
  implemented-by: []
  depends-on: []
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement process lifecycle policy

## Summary

P slice の policy/default/validation/restrict を実装した。optional raw lifecycle fields と default-complete ResolvedProcessPolicy を追加し、trusted replace → defaults → project-min を維持する。全 issue の review/verification は後続 R に残る。

## Context

TASK-034AGiBmLrlXnGsS5oLlL8 / approved PLAN-034AEwZy3GcNygp87aXrL7。coordinator direct plan verdict の P2（Pi focused scope 拡大）と P3（first-cause order / commit checkpoints）は plan に反映済み。新 active Dispatch の到着までは direct user-owned continuation とし、settled lifecycle IDs は使わない。

## Implementation

### Changes

core policy test に defaults、positive bounded duration、cap、per-task rejection、trusted/project layering と配布 assets の保証を追加した。meaningful RED の後、resolver/schema semantic boundary、merge の4新 restrict fields、許可済み policy.yaml ×2 を変更した。canonical load test の完全一致 snapshot に新 defaults を追加した。

### Decisions During Implementation

source 確認: loadPolicy は trusted merge 後に resolveProcessPolicy を呼び project restriction 前に既定を補完する。merge の restrict-concurrency は既存三つの数値 cap のみ。duration は shared parseDuration を使い、overflow は process config boundary で拒否する。partial policy の省略は保持し、project-min は元の duration 表現を選択する。

## Related Experiments

### Adopted

なし。承認済み設計を実行するため exp は作らない。

### Rejected

なし。

## Validation

planning-to-task-graph → task-graph-to-implementation、Task DAG issues=[]、runnable は P のみ。

- RED: 新 policy suite exit 1。さらに `mise exec -- pnpm exec vitest run packages/core/test/policy/process.test.ts -t 'normalizes legacy|snapshots heartbeat'` は2 failed / 47 deselected、既定4 field の欠落と valid override が unknown field として拒否されることを確認。
- GREEN: `mise exec -- pnpm exec vitest run packages/core/test/policy/process.test.ts` は49 passed / 1 file、1.37s。
- broad policy regression: canonical load snapshot の旧 expected により1 failed / 108 passed。observed diff は新4 defaultsのみ。完全一致 assertion を維持して expected を更新。
- GREEN: `mise exec -- pnpm exec vitest run packages/core/test/policy` は109 passed / 7 files、1.58s。
- `mise exec -- pnpm --filter @cheaper/core build` exit 0（tsc -b）。
- `mise exec -- pnpm exec biome check --write`（P の4変更TS file）exit 0。
- final root lint/typecheck/build/CLI focused/implementation review は R で実行する。baseline 43 tests は issue acceptance と混同しない。

## Risks

raw/schema/default/merge の一部だけ変更すると project restriction を bypass する。trusted/default/project 全層を同じ focused test で確認する。新 deps/manifest は変更しない。

## Follow-ups

P の GREEN と commit 後に E、次に R。canonical check deferred to coordinator merge run。
