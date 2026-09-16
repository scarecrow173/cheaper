---
id: "TASK-0068"
type: "task"
status: "done"
title: "Add count-tokens CLI command"
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
    - "docs/plans/0007-canonical-token-counter.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/0066-implement-core-token-counter.md"
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

# Add count-tokens CLI command

## Work

`parseCliArgs` に `count-tokens` command（`{ kind: "count-tokens", path, cwd, format, quiet }`）を追加し、`cli.ts` で
`canonicalizeWorkspaceRoot` → `loadPolicy`（effective `filesystem.symlinks`）→
`countFileTokens` → success/error envelope を出力する。
model runtime・worker 解決・stdin 読込には触れない。HELP を更新し、
args・main・e2e test を追加する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/0012-token-counter-contract.md`

## Dependencies

- TASK-0066

## Affected Files

- `packages/cli/src/args.ts`
- `packages/cli/src/cli.ts`
- `packages/cli/test/args.test.ts`
- `packages/cli/test/main.test.ts` / `test/e2e/cli.test.ts`

## Done When

- [ ] `cheaper count-tokens <path>` が envelope JSON（`result` に count）を返す
- [ ] model runtime・worker resolution・stdin に触れない
- [ ] `--input`/`--worker-file`/`--timeout` が拒否される

## Verification

- [ ] `mise exec -- pnpm -F cheaper test` が green
