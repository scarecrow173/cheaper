---
id: "TASK-034z1PNNPwye2uvwy1eAG8"
type: "task"
status: "todo"
title: "Unify packageVersion resolution on builtinDir"
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
  verified-by:
    - "docs/test-specs/embedded-builtin-resource-resolution-across-distribution-forms.md"
  references: []
  defers: []
  deferred-by: []
---

# Unify packageVersion resolution on builtinDir

## Work

`packages/cli/src/version.ts` の `packageVersion()` を `builtinDir` 基準の
`version.json` 解決に揃え、`cli.ts` の呼び出し側から builtinDir を渡す
（TSPEC-034z1P09dg86lO9uB31cUi が要求する 3 配布形態の統一規則）。
TDD: builtinDir 指定で version.json を解決する unit test を先に追加し
失敗を確認してから実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-node-26-sea-standalone-executable-proof-of-concept.md
- docs/designs/sea-executable-build-pipeline-and-embedded-resource-resolution.md
- docs/test-specs/embedded-builtin-resource-resolution-across-distribution-forms.md

## Dependencies

- なし（先頭タスクと独立）

## Affected Files

- `packages/cli/src/version.ts`（変更）
- `packages/cli/src/cli.ts`（変更）
- `packages/cli/test/`（version 解決テスト追加）

## Done When

- [ ] `packageVersion` が builtinDir 基準で `version.json` を解決する
- [ ] 通常実行・APM bundle 実行での `--version` 出力が変わらない

## Verification

- [ ] `pnpm test` exits 0（新規 version 解決テストを含む）
- [ ] `node packages/apm/.apm/hooks/cli/bin/cheaper.mjs --version` が
      package version を出力する
