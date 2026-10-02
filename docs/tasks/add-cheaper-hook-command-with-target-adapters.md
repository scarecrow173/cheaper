---
id: "TASK-034z52ABwxDsaQWun7VUDu"
type: "task"
status: "done"
title: "add cheaper hook command with target adapters"
created: "2026-10-02"
updated: "2026-10-02"
owners: []
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
    - "docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md"
  implemented-by:
    - "docs/impl/ir/add-cheaper-hook-command-with-target-adapters.md"
  depends-on:
    - "docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md"
    - "docs/tasks/port-routing-policy-js-to-packages-cli-src-hook-routing-policy-ts.md"
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
    - "docs/test-specs/cheaper-hook-subcommands-and-hooks-json-executable-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# add cheaper hook command with target adapters

## Work

`cheaper hook <event> --target <agent>` command と target 別
adapter（stdin normalize + verdict render）を実装する。

- `packages/cli/src/args.ts`: `options` に `target` を追加、
  `positionals[0] === "hook"` の受理、`hook` 以外への `--target`
  誤用ガード
- `packages/cli/src/hook/targets.ts`: windsurf（deny = stderr +
  exit 2）/ copilot（deny JSON + 常に exit 0、異形 payload への
  dual-schema allow 移植）/ pretooluse（claude・codex 共用
  `hookSpecificOutput` + 無言 exit 0）/ cursor（常に schema-valid
  `permission` JSON + exit 0、`file_path`/`command` key で両 event
  共有）
- `packages/cli/src/hook/hook.ts`: raw stdin 読み（`readTaskInput`
  の JSON object 強制を回避）→ normalize → `evaluate` → render。
  最深部 catch-all で全 error → target 適合 allow + exit 0
  （target 不明なら silent exit 0）。envelope・`recordUsage`・
  runCli 汎用 catch 経路に到達させない
- `cli.ts` の dispatch に `hook` を追加し、HELP に 1 行追記

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md
- docs/designs/cheaper-hook-subcommands-replacing-node-based-hook-adapters.md

## Dependencies

- docs/tasks/port-routing-policy-js-to-packages-cli-src-hook-routing-policy-ts.md

## Affected Files

- `packages/cli/src/args.ts`、`cli.ts`、`hook/` 配下
- `packages/apm/.apm/hooks/cli/bin/cheaper.mjs`（`pnpm bundle` 再生成）

## Done When

- [ ] `cheaper hook pre-read-code --target windsurf` 等の全 surface
      で stdin payload → verdict が動く
- [ ] parse 失敗・未知 `--target`・内部例外で非 0 exit を返さない
- [ ] copilot は常に exit 0、cursor は常に schema-valid JSON を返す
- [ ] `usage.jsonl` に hook 計測が記録されない
- [ ] `pnpm bundle` 再生成済み

## Verification

- [ ] `node packages/cli/bin/cheaper.js hook ...` で target 別
      fixture が正しい stdout/stderr/exit code を返す
- [ ] `mise exec -- pnpm -F @cheaper/cli test` 緑
