---
id: "TASK-0072"
type: "task"
status: "done"
title: "Implement pre_read_code hook and e2e test"
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
    - "docs/tasks/0071-create-worker-delegation-skills.md"
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

# Implement pre_read_code hook and e2e test

## Work

`.apm/hooks/hooks.json` と `.apm/hooks/pre-read-code.js` を実装。
zero-dep CommonJS、stdin JSON → stat → `cheaper count-tokens` →
threshold 超過で exit 2 + stderr guidance、他は exit 0（fail-open）。
`packages/cli/test/e2e/pre-read-hook.test.ts` で各 exit path を test。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/0013-apm-skills-and-pre-read-hook.md`
- `docs/specs/0008-apm-delegation-skills-and-read-hook.md`

## Dependencies

- TASK-0071（hook message が skill 名を参照するため）

## Affected Files

- `.apm/hooks/hooks.json`、`.apm/hooks/pre-read-code.js`（新規）
- `packages/cli/test/e2e/pre-read-hook.test.ts`（新規）

## Done When

- [ ] large→exit 2 + stderr guidance、small/missing/dir/bad-stdin→exit 0
- [ ] CHEAPER_CLI / CHEAPER_READ_TOKEN_THRESHOLD の env が効く

## Verification

- [ ] `mise exec -- pnpm -F cheaper test` が green
