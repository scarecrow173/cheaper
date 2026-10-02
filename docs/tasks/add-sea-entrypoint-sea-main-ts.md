---
id: "TASK-034z1POhgJ49oVv2arDj6X"
type: "task"
status: "done"
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
    - "docs/plans/implement-node-26-sea-standalone-executable-proof-of-concept.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-node-26-sea-standalone-executable-proof-of-concept.md"
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

# Add SEA entrypoint sea-main.ts

## Work

`packages/cli/src/sea-main.ts` を新規追加する。`main.ts` と同様に
`registerBundledOAuthFlows()` を呼んだうえで、`runCli(argv, io,
{ builtinDir: import.meta.dirname })` に VFS root を渡す薄い
entrypoint。通常 Node 実行の `defaultBuiltinDir()` とは分岐させず、
`CliHooks.builtinDir` の既存 seam のみを使う（design 準拠）。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-node-26-sea-standalone-executable-proof-of-concept.md
- docs/designs/sea-executable-build-pipeline-and-embedded-resource-resolution.md

## Dependencies

- なし（version 統一タスクと論理的には関連するが実装上は独立）

## Affected Files

- `packages/cli/src/sea-main.ts`（新規）

## Done When

- [ ] `pnpm build` で `packages/cli/dist/sea-main.js` が出力される
- [ ] sea-main が `runCli` に `builtinDir: import.meta.dirname` を渡す

## Verification

- [ ] `pnpm build` exits 0 かつ `dist/sea-main.js` が生成される
- [ ] `version.json` を `dist/` に配置した状態で
      `node packages/cli/dist/sea-main.js --version` exits 0
      （builtinDir = dist/ の real FS で同一規則が動くことを確認）
