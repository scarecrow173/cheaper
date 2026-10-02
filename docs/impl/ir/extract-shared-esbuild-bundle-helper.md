---
id: "IMPL-034z1ST2V3s2UsM4Mi2S87"
type: "impl"
status: "completed"
title: "Extract shared esbuild bundle helper"
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
    - "docs/tasks/extract-shared-esbuild-bundle-helper-from-bundle-cli-mjs.md"
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

# Extract shared esbuild bundle helper

## Summary

`scripts/bundle-cli.mjs` の esbuild 呼び出し部を
`scripts/lib/esbuild-bundle.mjs` の `buildEsmBundle()` に切り出し、
APM bundle 出力を byte 一致で維持したまま SEA build からも再利用可能
にした。

## Context

docs/tasks/extract-shared-esbuild-bundle-helper-from-bundle-cli-mjs.md
（PLAN-034z1M6zCAdKb12eiCWZRm の task 1）に従う純粋 refactor。

## Implementation

### Changes

- `scripts/lib/esbuild-bundle.mjs`（新規）: `repoRoot()` と
  `buildEsmBundle({ entry, outfile })` を export。esbuild JS launcher の
  spawn・createRequire banner 前置・shebang 保持・出力 dir 作成を内包
- `scripts/bundle-cli.mjs`: esbuild 部分を `buildEsmBundle` 呼び出しに
  置換。version.json・workers/・policy.yaml の copy は変更なし

### Decisions During Implementation

- entry/outfile は絶対パスで受け取り helper 内で `path.relative` に
  変換する形にし、caller 側の path 組み立てを単純化した

## Related Experiments

### Adopted

なし（機械的 refactor のため Experiment Log は作成しない）

### Rejected

なし

## Validation

- `pnpm bundle` exits 0、`bundled cheaper CLI -> packages\apm\.apm\hooks\cli (12.2 MiB)`
- HEAD 版 `bundle-cli.mjs` の再生成物と新 helper の出力が sha256
  `c839dae8…` で byte 一致（committed bundle との差分は staleness であり
  更新済み）
- commit: 4b70ba3

## Risks

- なし（出力不変を実測で確認済み）

## Follow-ups

- `build-executable.mjs` が同 helper で `dist/sea-main.js` を bundle
  する（TASK-034z1PSPnrFA3o7z3IYIUy）
