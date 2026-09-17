---
id: "TASK-0104"
type: "task"
status: "todo"
title: "Add version check and set scripts"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
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
    - "docs/plans/0019-unified-version-source-of-truth.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0019-unified-version-source-of-truth.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0024-unified-version-source-of-truth.md"
    - "docs/specs/0019-unified-version-source-of-truth.md"
    - "docs/adr/0022-cli-package-json-as-canonical-version-with-check-enforcement.md"
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

# Add version check and set scripts

## Work

DESIGN-0024 どおり、version 一致検証 `scripts/check-version.mjs` と
一括 bump `scripts/set-version.mjs` を作成する。

### 1. `scripts/check-version.mjs`（新規）

- canonical = `packages/cli/package.json` の `version`。
- 一致対象から version を抽出して canonical と比較:
  - `packages/core/package.json`、`packages/runtime-pi/package.json`:
    JSON parse の `version` field。
  - `packages/apm/apm.yml`: `^version:\s*"?([^"\s]+)"?\s*$` の行抽出
    （quote 有無両対応、YAML parser 不使用）。
  - `packages/apm/.apm/hooks/cli/version.json`: JSON parse。
- 全一致 → `version check OK: <v>` + exit 0。
- 不一致 → file ごとの値一覧を出して exit 1。
- `apm.yml`（root、`cheaper-dev`）は対象外。
- 既存 script 慣例に合わせ、header コメントで目的・対象・対象外を
  記す（日本語可。`bundle-cli.mjs` の style に揃える）。

### 2. `scripts/set-version.mjs <X.Y.Z>`（新規）

- 引数を `/^\d+\.\d+\.\d+$/` で検証（不正なら usage + exit 1）。
- `packages/{cli,core,runtime-pi}/package.json` の `version` を
  上書き（JSON parse → 上書き → 2-space indent + 末尾 newline で
  書戻し）。
- `packages/apm/apm.yml` の `version:` 行のみ置換。
- 終了時に `node scripts/bundle-cli.mjs` 実行を促すメッセージを出す。
- bump 後に `check-version.mjs` を呼んで一致を確認する。

### 3. 手動注入検証

- 対象 file の 1 つを一時的にずらし `check-version.mjs` が exit 1 と
  file 名を出すことを確認。確認後に戻す。
- `set-version.mjs` に不正引数を渡して usage + exit 1 を確認。

## Acceptance

- [ ] 全宣言が一致する状態で `check-version.mjs` が exit 0。
- [ ] 宣言をずらすと exit 1 + 不一致 file が列挙される。
- [ ] `set-version.mjs <semver>` が対象 file を一括更新し、不正引数で
      exit 1 になる。
- [ ] `pnpm lint`（biome の対象に入る場合）が pass する。
