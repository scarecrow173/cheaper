---
id: "IMPL-034z3CDzdvjhrEshqx1zRg"
type: "impl"
status: "completed"
title: "implement build-apm-bundle"
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
    - "docs/tasks/implement-build-apm-bundle-mjs-assembling-executables-into-hooks-tree.md"
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

# implement build-apm-bundle

## Summary

APM tree 組み立てを `scripts/lib/apm-bundle.mjs` に集約し、
`build-apm-bundle.mjs` が bundle + platform executable を
`cli/bin/<target>/` に配置するようにした。

## Context

docs/plans/assemble-apm-executable-payload-with-platform-launchers.md task 2。bundle-cli.mjs の既存出力を壊さず
executable payload を追加する。

## Implementation

### Changes

- `scripts/lib/apm-bundle.mjs`（新規）: `buildApmCliTree()`
  が `bin/cheaper.mjs`・`version.json`・`workers/`・
  `policy.yaml` を生成。rmSync 対象を生成物のみに限定して
  `bin/<target>/` の executable を保持
- `scripts/bundle-cli.mjs`: 薄い wrapper 化（出力不変）
- `scripts/build-apm-bundle.mjs`（新規）: `buildApmCliTree()`
  + `--exe-dir` の `cheaper-<platform>-<arch>[.exe]` を
  `cli/bin/<target>/cheaper[.exe]` にコピー、POSIX に 755 付与、
  `--exe-dir` 省略時は executable なしで完走

### Decisions During Implementation

- `pnpm bundle`（bundle-cli）が executable dir を消さないよう
  rmSync の対象を生成物 4 点に限定した。これにより
  freshness diff（`git diff --exit-code`）が LFS pointer の
  削除誤検知を起こさない

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `pnpm bundle` → 既存 tracked file に diff なし（出力不変）
- `node scripts/build-apm-bundle.mjs --exe-dir dist/sea` →
  `cli/bin/win32-x64/cheaper.exe`（112.3 MiB）配置

## Risks

- `--exe-dir` に linux artifacts を混ぜた状態での動作は
  release artifact 入手時に初検証

## Follow-ups

- release workflow で artifact download →
  `build-apm-bundle --exe-dir` → APM publish の流れを将来 plan
  で接続可能
