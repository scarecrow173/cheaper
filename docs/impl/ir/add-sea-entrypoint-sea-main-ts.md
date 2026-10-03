---
id: "IMPL-034z1STQxtFa8POTFVCwbd"
type: "impl"
status: "completed"
title: "Add SEA entrypoint sea-main.ts"
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
    - "docs/tasks/add-sea-entrypoint-sea-main-ts.md"
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

# Add SEA entrypoint sea-main.ts

## Summary

`packages/cli/src/sea-main.ts` を追加。`registerBundledOAuthFlows()` を
呼んだうえで `runCli(argv, io, { builtinDir: import.meta.dirname })`
に VFS root を渡す薄い entrypoint。

## Context

docs/tasks/add-sea-entrypoint-sea-main-ts.md（PLAN-034z1M6zCAdKb12eiCWZRm
task 3）。DESIGN-034z1EfqYaEBfyNvqZyFSD の「`CliHooks.builtinDir` seam
のみを使う」方針に従う。

## Implementation

### Changes

- `packages/cli/src/sea-main.ts`（新規）: main.ts と同じ構成で、
  hooks に `builtinDir: import.meta.dirname` を渡す

### Decisions During Implementation

- `defaultBuiltinDir()` と分岐させず builtinDir を明示注入する形に
  した。`useVfs` では injected main が VFS root で実行され
  `import.meta.dirname` が VFS root を指すため

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `pnpm build` exits 0、`dist/sea-main.js` 生成
- `version.json` を `dist/` に staging して
  `node dist/sea-main.js --version` → `1.0.0`（real FS で同一規則を確認）

## Risks

- `import.meta.dirname` が SEA VFS 内で期待どおり VFS root を指すかは
  PoC smoke（TASK-034z1PTkni5iNJPysyfxRN）で初確定する

## Follow-ups

- `build-executable.mjs` がこの entry を bundle して SEA 化する
  （TASK-034z1PSPnrFA3o7z3IYIUy）
