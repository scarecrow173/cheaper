---
id: "TASK-034z3zP5mtHjiKzurDeCDM"
type: "task"
status: "todo"
title: "Implement build-apm-bundle.mjs assembling executables into hooks tree"
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
    - "docs/plans/assemble-apm-executable-payload-with-platform-launchers.md"
  implemented-by: []
  depends-on:
    - "docs/plans/assemble-apm-executable-payload-with-platform-launchers.md"
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
    - "docs/test-specs/apm-executable-payload-layout-and-launcher-dispatch-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement build-apm-bundle.mjs assembling executables into hooks tree

## Work

`scripts/build-apm-bundle.mjs` を新規作成する。

- 既存 `scripts/lib/esbuild-bundle.mjs` helper を再利用して canonical
  JS bundle を生成し、`bundle-cli.mjs` と同じ APM tree 出力
  （`bin/cheaper.mjs`・`version.json`・`workers/`・`policy.yaml`）を
  行う
- `--exe-dir <dir>` を受け取り、`cheaper-<platform>-<arch>[.exe]`
  命名の executable を `cli/bin/<platform>-<arch>/cheaper[.exe]` に
  コピーする（POSIX target には実行 bit を付与）
- `--exe-dir` 未指定時は executable を配置しない（部分 bundle を
  許容）
- 最後に配置した artifact 一覧と size を出力する

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/assemble-apm-executable-payload-with-platform-launchers.md
- docs/designs/apm-executable-payload-layout-and-platform-launchers.md

## Dependencies

- `scripts/lib/esbuild-bundle.mjs`（Phase 1 で切り出し済み）

## Affected Files

- `scripts/build-apm-bundle.mjs`（新規）

## Done When

- [ ] <!-- completion criterion -->

## Verification

- [ ] <!-- command, test, review step, or observable behavior -->
