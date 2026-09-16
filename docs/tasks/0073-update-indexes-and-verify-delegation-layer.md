---
id: "TASK-0073"
type: "task"
status: "done"
title: "Update indexes and verify delegation layer"
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
    - "docs/plans/0008-apm-delegation-layer.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0008-apm-delegation-layer.md"
    - "docs/tasks/0072-implement-pre-read-code-hook.md"
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

# Update indexes and verify delegation layer

## Work

docs index README 群へ DISC-0005/SPEC-0008/DESIGN-0013/PLAN-0008/
TASK-0070–0073 を登録し、README に APM skill/hook を追記。
doc audits と全 baseline を確認する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/0013-apm-skills-and-pre-read-hook.md`
- `docs/specs/0008-apm-delegation-skills-and-read-hook.md`

## Dependencies

- TASK-0070、TASK-0071、TASK-0072

## Affected Files

- `docs/*/README.md`、`README.md`

## Done When

- [ ] 各 index に新規 doc が登録される
- [ ] audit_docs / audit_adr が findings なし

## Verification

- [ ] `mise exec -- pnpm -r build && pnpm -r test && pnpm lint && pnpm typecheck` 全 green
