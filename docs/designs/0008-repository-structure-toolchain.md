---
id: "DESIGN-0008"
type: "design"
status: "draft"
title: "リポジトリ構成とツールチェーン"
created: "2026-09-15"
updated: "2026-09-15"
owners: ["scarecrow173"]
relations:
  source:
    - "../handoff/2026-09-15-cheaper-v1-implementation-handoff.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "0001-worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md"
    - "0004-pi-execution-engine.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/0008-typescript-pnpm-monorepo.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# リポジトリ構成とツールチェーン

## Context

ADR-0008 の 3 パッケージ monorepo を具体的な layout・依存・ビルド/検証設定に落とす。依存方向は `cli → runtime-pi → core`、`@cheaper/core` は Pi 系を持たない。

## Scope

### In Scope

- ルート設定（pnpm workspace・tsconfig.base・Biome・Vitest workspace）
- 3 パッケージの package.json・`exports`・`bin`
- ディレクトリ構造と依存パッケージ一覧
- 検証コマンド

### Out of Scope

- publish/release フロー（V1 は private workspace）
- CI 設定

## Components and Boundaries

### ルート

- `package.json`（private, `packageManager: pnpm@11`）
- `pnpm-workspace.yaml`（`packages/*`）
- `tsconfig.base.json`（`module: NodeNext`, `target: ES2023`, `strict`, `exactOptionalPropertyTypes`, `noUncheckedIndexedAccess`）
- `biome.json`、`vitest.workspace.ts`
- 各 package は `tsc -b`（project references）、ESM only、`exports` で `dist/index.js`

### パッケージ

- `@cheaper/core`: 型・Worker parser/validator/resolver・policy・tool vocabulary・path security・model tier 分類・error model。**Pi 系依存なし**（`ajv`, `yaml` のみ）
- `@cheaper/runtime-pi`: Pi SDK 統合・sandboxed tools・実行・計測
- `cheaper`（cli）: コマンド・envelope・exit code・built-in workers/schemas/policy.yaml 同梱。`bin: { cheaper: "./bin/cheaper.js" }`

### ディレクトリ

```text
packages/core/src/{errors.ts, duration.ts, config/dirs.ts,
  worker/{parse,validate,schema,resolve}.ts,
  policy/{schema,load,merge,restrict}.ts,
  tools/vocabulary.ts, workspace/paths.ts,
  model/{tier,classify,config,resolve}.ts, index.ts}
packages/runtime-pi/src/{resource-loader.ts,
  tools/{operations,grep,find,walker,budget,registry}.ts,
  journal.ts, session.ts, output.ts, repair.ts, usage.ts, run.ts, index.ts}
packages/cli/src/{main.ts, args.ts, envelope.ts, text-format.ts}
packages/cli/bin/cheaper.js
packages/cli/workers/{bulk-reader.md, cheap-writer.md, schemas/*.json}
packages/cli/policy.yaml
```

### 依存（ユーザー承認済み）

- runtime: `@earendil-works/pi-coding-agent@0.85.1`, `@earendil-works/pi-ai@0.85.1`, `typebox`, `ajv`, `yaml`, `diff`
- dev: `typescript`, `vitest`, `@types/node`, `@biomejs/biome`
- すべて `minimum_release_age = 7d` を満たすバージョン

## Data and Control Flow

- build: `pnpm -r build`（`tsc -b` で project references 順）
- test: `pnpm -r test`（vitest。core は純粋単体、runtime-pi は fake stream + 実 filesystem の tmp dir で Security/Execution/cheap-writer カテゴリ、cli は envelope/exit code と `--help`）
- lint/format: `pnpm biome check .`（いずれも `mise exec --` 経由）

## Non-Functional Constraints

- Node 26 / pnpm 11（mise.toml）、ESM only
- `minimum_release_age = 7d` 遵守、依存追加は別途承認制（ADR-0008）

## Risks and Trade-offs

- pnpm の ignored build scripts（esbuild/@google/genai/protobufjs）で `pnpm add` が exit 1 になることがプローブで確認済み。install 手順に `pnpm approve-builds` または `pnpm.onlyBuiltDependencies` の扱いを設計に残す
- `@earendil-works/pi-coding-agent` は `@earendil-works/pi-tui` を含む重い依存（probe node_modules 143MB）。runtime-pi に閉じ込め、core/cli が直接参照しない構成で影響を局所化

## Deferred Design Concerns

- publish/リリース配布 | V1 は private workspace | 外部配布要求 | bin エントリの `node` shebang・pack 内容を要検証
- ignored build scripts の明示的承認ポリシー | 現状 install で警告 | CI 導入時 | ネイティブ/wasm 依存の再現性リスク

## Verification Notes

- [ ] `pnpm -r build` / `pnpm -r test` / `pnpm biome check .` が全 pass
- [ ] `@cheaper/core` の package.json が `@earendil-works/*` を含まない
- [ ] `cheaper` bin が `node` で直接起動し `--help`/`--version` が動く
- [ ] built-in `workers/*.md` と `schemas/*.json` が `packages/cli` に同梱される
