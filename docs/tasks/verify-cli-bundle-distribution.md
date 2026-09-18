---
id: "TASK-034r05iHQeQpDq2OwMGpOI"
type: "task"
status: "done"
title: "Verify CLI bundle distribution"
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
    - "docs/plans/self-contained-cli-bundle.md"
  implemented-by: []
  depends-on:
    - "docs/plans/self-contained-cli-bundle.md"
    - "docs/tasks/consumer-install-smoke.md"
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

# Verify CLI bundle distribution

## Work

全 baseline（build/test/lint/typecheck）と doc audits を確認し、
Phase 4 を commit する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/cli-bundle-layout.md`
- `docs/specs/apm-self-contained-cli-distribution.md`

## Dependencies

- TASK-034r05iHQeQpDq14tAA3rz、TASK-034r05iHQeQpDq2z00rDnn

## Affected Files

- なし（検証のみ）

## Done When

- [ ] 全 baseline green
- [ ] audits findings なし

## Verification

- [ ] `mise exec -- pnpm -r build && pnpm -r test && pnpm lint && pnpm typecheck` 全 green
