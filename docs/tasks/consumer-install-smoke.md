---
id: "TASK-034r05iHQeQpDq2z00rDnn"
type: "task"
status: "done"
title: "Consumer install smoke e2e and skill updates"
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
    - "docs/tasks/cli-bundle-script-and-hook-argv.md"
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

# Consumer install smoke e2e and skill updates

## Work

local path dep consumer に `apm install --target windsurf` する e2e test
（`apm` 不在環境は skip）を `packages/cli/test/e2e/apm-install.test.ts`
に追加。deploy・`--version`・hook block を確認。cheaper-usage skill に
bundled path を追記し、docs index を更新する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/cli-bundle-layout.md`
- `docs/specs/apm-self-contained-cli-distribution.md`

## Dependencies

- TASK-034r05iHQeQpDq14tAA3rz

## Affected Files

- `packages/cli/test/e2e/apm-install.test.ts`
- `.apm/skills/cheaper-usage/SKILL.md`
- `docs/*/README.md`、`README.md`

## Done When

- [ ] consumer install smoke test が green（または env 不在で skip）
- [ ] skill/index が更新される

## Verification

- [ ] `mise exec -- pnpm -F cheaper test` が green
