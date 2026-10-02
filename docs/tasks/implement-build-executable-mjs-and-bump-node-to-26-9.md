---
id: "TASK-034z1PSPnrFA3o7z3IYIUy"
type: "task"
status: "todo"
title: "Implement build-executable.mjs and bump Node to >=26.9"
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
    - "docs/tasks/extract-shared-esbuild-bundle-helper-from-bundle-cli-mjs.md"
    - "docs/tasks/unify-packageversion-resolution-on-builtindir.md"
    - "docs/tasks/add-sea-entrypoint-sea-main-ts.md"
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

# Implement build-executable.mjs and bump Node to >=26.9

## Work

1. `useVfs` が入った Node ≥26.9.0 に toolchain を更新する
   （`mise.toml` は `node = "26"` のまま `mise.lock` を更新。
   `minimum_release_age = "7d"` で直近 release は弾かれる点に注意）
2. `scripts/build-executable.mjs` を新規実装する:
   - option: `--target <platform>-<arch>`（default: current host）と
     `--out-dir`（default: `dist/sea/`）
   - shared helper（`scripts/lib/esbuild-bundle.mjs`）で
     `dist/sea-main.js` から canonical bundle を生成
   - `sea-config.json` を生成: `{ "main": <bundle>,
     "mainFormat": "module", "useVfs": true, "useSnapshot": false,
     "useCodeCache": false, "assets": { workers/, policy.yaml,
     version.json を含む } }`
   - `node --build-sea` で注入（postject が必要な場合は依存追加を
     ユーザー確認してから対応）
   - artifact 命名 `cheaper-<platform>-<arch>[.exe]`、size を stdout に
     報告

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-node-26-sea-standalone-executable-proof-of-concept.md
- docs/designs/sea-executable-build-pipeline-and-embedded-resource-resolution.md

## Dependencies

- extract-shared-esbuild-bundle-helper-from-bundle-cli-mjs.md
- unify-packageversion-resolution-on-builtindir.md
- add-sea-entrypoint-sea-main-ts.md

## Affected Files

- `scripts/build-executable.mjs`（新規）
- `mise.lock`（Node ≥26.9.0）
- `package.json`（`build:sea` script 追加を検討）
- `dist/sea/`（生成出力）

## Done When

- [ ] `node scripts/build-executable.mjs --target win32-x64` が
      `dist/sea/cheaper-windows-x64.exe` を生成し size を報告する
- [ ] sea-config が design の初期値（`mainFormat: module` /
      `useVfs: true` / snapshot・codeCache false）を使う

## Verification

- [ ] `node --version` が v26.9.0 以上を報告する
- [ ] `node scripts/build-executable.mjs --target win32-x64` exits 0 かつ
      `dist/sea/cheaper-windows-x64.exe` が存在する
