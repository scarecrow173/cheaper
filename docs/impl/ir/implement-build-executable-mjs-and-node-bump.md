---
id: "IMPL-034z1SUivUVftC75FVZ3OG"
type: "impl"
status: "completed"
title: "Implement build-executable.mjs and Node bump"
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
    - "docs/tasks/implement-build-executable-mjs-and-bump-node-to-26-9.md"
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

# Implement build-executable.mjs and Node bump

## Summary

`scripts/build-executable.mjs` を新規実装し、Node を 26.8.2 →
26.10.0 に bump（`useVfs` は v26.9.0 で導入）。current host
（win32-x64）で `cheaper-windows-x64.exe`（112.3 MiB）を生成した。

## Context

docs/tasks/implement-build-executable-mjs-and-bump-node-to-26-9.md
（PLAN-034z1M6zCAdKb12eiCWZRm task 4）。

## Implementation

### Changes

- `scripts/build-executable.mjs`（新規）: `--target` / `--out-dir`
  option、esbuild helper 経由で `dist/sea-main.js` を bundle、
  `workers/`・`policy.yaml`・生成 `version.json` を assets に含む
  sea-config を生成し `node --build-sea` を実行。命名
  `cheaper-<platform>-<arch>[.exe]`（win32→windows ラベル変換）、
  size を stdout 報告
- `mise.lock`: node 26.8.2 → 26.10.0

### Decisions During Implementation

- **postject は不要**: Node 26.10.0 の `--build-sea` は blob 注入まで
  内蔵し最終 executable を `output` に直接書く（最小実験で確認）。
  plan の open question が解消し、依存追加は発生しなかった
- `disableExperimentalSEAWarning: true` を設定（ExperimentalWarning が
  stderr を汚染しないよう。envelope 契約を守る）
- cross-target は host との一致を要求して fail-closed にした
  （`executable` フィールド経由の cross build は Phase 2）
- assets の key は VFS root 相対（`workers/bulk-reader.md` 等）で
  `join(import.meta.dirname, key)` で読めることを最小実験で確認済み

## Related Experiments

### Adopted

- 最小 SEA probe（hello.mjs + data.txt asset）: `--build-sea` が
  注入済み exe を直接出力、`import.meta.dirname` が VFS root
  （Windows では `\\.\nul\vfs\0`）を指し、`node:fs` read が効き
  write が EROFS になることを確認

### Rejected

なし

## Validation

- `node --version` → v26.10.0
- `node scripts/build-executable.mjs --target win32-x64` exits 0、
  `dist/sea/cheaper-windows-x64.exe` 112.3 MiB 生成

## Risks

- `useVfs` は Stability 1.1（Active development）。Node minor の挙動
  変化は CI smoke で検知する方針（design 記録済み）

## Follow-ups

- Phase 2 で cross-target matrix + GitHub Release 接続
  （`executable` フィールドに target node binary を渡す設計）
