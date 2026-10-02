---
id: "TASK-034z1PGTvhoaHhdTUgPahz"
type: "task"
status: "todo"
title: "Extract shared esbuild bundle helper from bundle-cli.mjs"
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

# Extract shared esbuild bundle helper from bundle-cli.mjs

## Work

`scripts/bundle-cli.mjs` 内の esbuild 呼び出し（esbuild JS launcher の
`spawnSync`・`createRequire` banner 前置・shebang 保持）を
`scripts/lib/esbuild-bundle.mjs` の export function に切り出し、
`scripts/bundle-cli.mjs` はそれを呼んで既存どおり APM tree
（`cheaper.mjs`・`version.json`・`workers/`・`policy.yaml`）を出力する。
挙動は不変の純粋 refactor。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-node-26-sea-standalone-executable-proof-of-concept.md
- docs/designs/sea-executable-build-pipeline-and-embedded-resource-resolution.md

## Dependencies

- なし（先頭タスク）

## Affected Files

- `scripts/lib/esbuild-bundle.mjs`（新規）
- `scripts/bundle-cli.mjs`（変更）

## Done When

- [ ] `pnpm bundle` が `packages/apm/.apm/hooks/cli/` を従来どおり生成する
- [ ] bundle 出力内容がリファクタ前と一致する

## Verification

- [ ] `pnpm bundle` exits 0
- [ ] `git diff packages/apm/.apm/hooks/cli/` が空（生成物は committed
      artifact のため byte 一致が確認できる）
