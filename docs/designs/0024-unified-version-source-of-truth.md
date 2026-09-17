---
id: "DESIGN-0024"
type: "design"
status: "approved"
title: "Unified version source of truth"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0019-unified-version-source-of-truth.md"
    - "../adr/0022-cli-package-json-as-canonical-version-with-check-enforcement.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/0019-unified-version-source-of-truth.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./0014-cli-bundle-layout.md"
    - "./0021-dev-apm-project-and-distributable-package-layout.md"
    - "./0022-repo-owned-setup-check-entrypoints.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/0022-cli-package-json-as-canonical-version-with-check-enforcement.md"
  derived-by:
    - "../plans/0019-unified-version-source-of-truth.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Unified version source of truth

## Context

SPEC-0019 / ADR-0022 の実装設計。canonical version source を
`packages/cli/package.json` に定め、全 version 宣言の一致を
`scripts/check-version.mjs` で検証し（`mise run check` 組み込み）、
`scripts/set-version.mjs` で一括 bump する。初期値は `1.0.0` に統一。

## Scope

### In Scope

- 新規 `scripts/check-version.mjs`（一致検証）
- 新規 `scripts/set-version.mjs`（一括 bump + bundle 再生成の指示）
- `packages/{cli,core,runtime-pi}/package.json` の version を `1.0.0` へ
- `mise.toml` の `check` task に `node scripts/check-version.mjs` を追加
- `docs/guides/release.md`（version bump + release 手順）
- `AGENTS.md` / `README.md` への最小限の pointer（必要箇所のみ）
- bundle 再生成（`version.json` が `1.0.0` になる）

### Out of Scope

- root `apm.yml` の version（dev project 識別子、対象外 — 文書で明記）
- `--version` の読み取り経路変更（既存 package.json → version.json
  fallback のまま、値が正しくなるだけ）
- git tag / GitHub Release 自動化
- `pnpm version` との統合

## Design

### 一致対象と canonical

```
canonical: packages/cli/package.json  version
一致対象:  packages/core/package.json
          packages/runtime-pi/package.json
          packages/apm/apm.yml                    (version: 行)
生成物照合: packages/apm/.apm/hooks/cli/version.json
対象外:    apm.yml（cheaper-dev、独立 version）
```

### `scripts/check-version.mjs`

- `packages/cli/package.json` の `version` を canonical として読む。
- 各一致対象 file から version を抽出し、canonical と比較:
  - `package.json` 系: JSON parse して `version` field。
  - `packages/apm/apm.yml`: `version:` 行を正規表現で抽出
    （`^version:\s*"?([^"\s]+)"?\s*$` — YAML parser を持ち込まない）。
  - `version.json`: JSON parse。
- 全て一致すれば `version check OK: <v>` を出して exit 0。
- 不一致があれば file ごとの値を一覧して exit 1。

### `scripts/set-version.mjs <X.Y.Z>`

- `X.Y.Z` を semver 形式で検証（`/^\d+\.\d+\.\d+$/`）。
- 一致対象の全 file（cli・core・runtime-pi package.json、
  `packages/apm/apm.yml`）の version を書き換える。
  - `package.json`: JSON parse → `version` 上書き → 整形維持で書戻し
    （既存の 2-space indent + 末尾 newline を保つ）。
  - `apm.yml`: `version:` 行のみ正規表現で置換（他行を触らない）。
- 終了時に「`node scripts/bundle-cli.mjs` を実行して version.json を
  再生成し、git diff で配布物を確認せよ」と指示を出す。
- bump 後の検証は `check-version.mjs` を呼んで一致を確認する。

### `mise.toml` check task への組み込み

`check` の `run` 列に `node scripts/check-version.mjs` を追加する。
位置は `pnpm lint` の後（安価な検証を先にまとめる）。

```toml
[tasks.check]
run = [
  "pnpm lint",
  "node scripts/check-version.mjs",
  "pnpm typecheck",
  "pnpm test",
  "node scripts/bundle-cli.mjs",
  "git diff --exit-code -- packages/apm/.apm/hooks/cli",
]
```

CI は `mise run check` を既に呼ぶため変更不要（SPEC-0017 の構造で
自動的に version check が走る）。

### 初期値 `1.0.0` への統一

`packages/{cli,core,runtime-pi}/package.json` の `version` を
`0.0.0` → `1.0.0` に変更し、`node scripts/bundle-cli.mjs` で
`version.json` を `1.0.0` に再生成する。`packages/apm/apm.yml` は
既に `1.0.0` のため変更なし。

### release 手順（docs/guides/release.md）

1. `node scripts/set-version.mjs <X.Y.Z>` で bump
2. `node scripts/bundle-cli.mjs` で version.json 再生成
3. `mise run check` で全検証（version check 含む）
4. 変更を commit（`release:` または `chore:` prefix）
5. `cheaper --version` / bundled CLI で値を目視確認

## Test Design

- check script の検証は手動注入で確認: 一時的に宣言をずらして
  `check-version.mjs` が exit 1 になることを確認（test file は
  追加しない — script 自体が検証器であり、self-test は過剰）。
- `--version` 出力は cli test 既存経路があれば確認、なければ
  手動で `node packages/cli/bin/cheaper.js --version`。
- `version.json` の値は bundle 後の `git diff` / check script で確認。

## Verification

- `node scripts/check-version.mjs` が pass。
- 宣言をずらした場合に exit 1。
- `mise run check` 全 pass（version check 込み）。
- `pnpm typecheck` / `pnpm lint` / test 全件。
