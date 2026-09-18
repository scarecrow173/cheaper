---
id: "TASK-034r05iHQeQpDpZOlmNJoM"
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
    - "docs/plans/apm-delegation-layer.md"
  implemented-by: []
  depends-on:
    - "docs/plans/apm-delegation-layer.md"
    - "docs/tasks/implement-pre-read-code-hook.md"
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

docs index README 群へ DISC-034r05iHy2gdnp9dPPZ28N/SPEC-034r05iHK8bA7gexEML9oZ/DESIGN-034r05iHy2gdnoFHbdllXV/PLAN-034r05iHE1FbkoW7f4XJXS/
TASK-034r05iHQeQpDpVUAMbEeR–0073 を登録し、README に APM skill/hook を追記。
doc audits と全 baseline を確認する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/apm-skills-and-pre-read-hook.md`
- `docs/specs/apm-delegation-skills-and-read-hook.md`

## Dependencies

- TASK-034r05iHQeQpDpVUAMbEeR、TASK-034r05iHQeQpDpWN0qICDI、TASK-034r05iHQeQpDpYxJZR1PT

## Affected Files

- `docs/*/README.md`、`README.md`

## Done When

- [ ] 各 index に新規 doc が登録される
- [ ] audit_docs / audit_adr が findings なし

## Verification

- [ ] `mise exec -- pnpm -r build && pnpm -r test && pnpm lint && pnpm typecheck` 全 green
