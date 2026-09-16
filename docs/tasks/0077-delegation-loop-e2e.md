---
id: "TASK-0077"
type: "task"
status: "done"
title: "Delegation loop e2e (pre_read_code block to bundled bulk-read)"
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
    - "docs/plans/0009-self-contained-cli-bundle.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/0075-consumer-install-smoke.md"
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

# Delegation loop e2e (pre_read_code block to bundled bulk-read)

## Work

consumer install 後の deploy 物だけで、parent の委譲 loop 全体を検証する:

1. `pre_read_code` hook が large file の read を exit 2 + guidance で block
2. parent が bundled CLI で `bulk-read` を実行（fake stream fixture、
   import 不要の array form で consumer 内に生成）
3. envelope が圧縮結果（summary/findings、file 本文非含有）を返し、
   `meta.model` が fake provider を示す

`packages/cli/test/e2e/delegation-loop.test.ts` と共有 helper
`apm-fixture.ts` を追加。実 Windsurf IDE 上での hook 発火は同一 command
contract により代替される（手動実機検証は後続課題）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/0008-apm-delegation-skills-and-read-hook.md`
- `docs/specs/0009-apm-self-contained-cli-distribution.md`

## Dependencies

- TASK-0075

## Affected Files

- `packages/cli/test/e2e/apm-fixture.ts`（新規）
- `packages/cli/test/e2e/delegation-loop.test.ts`（新規）
- `packages/cli/test/e2e/apm-install.test.ts`（fixture 共有化）

## Done When

- [x] block → bulk-read → compressed result の loop が e2e green
- [x] file 本文が parent へ返らないことが assert される

## Verification

- [x] `mise exec -- pnpm -F cheaper test` が green
