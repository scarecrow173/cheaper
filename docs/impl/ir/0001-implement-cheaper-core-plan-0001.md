---
id: "IMPL-0001"
type: "impl"
status: "in-progress"
title: "Implement @cheaper/core (PLAN-0001)"
created: "2026-09-15"
updated: "2026-09-15"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - "package.json"
      - "pnpm-workspace.yaml"
      - "tsconfig.base.json"
      - "tsconfig.json"
      - "biome.json"
      - "vitest.workspace.ts"
      - "packages/core/"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/0001-scaffold-pnpm-monorepo-root-configuration.md"
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

# Implement @cheaper/core (PLAN-0001)

## Summary

PLAN-0001（docs/plans/0001）: pnpm monorepo scaffold と @cheaper/core の全モジュール実装。対象 task: TASK-0001..0021。

## Context

PLAN-0001 / DESIGN-0001, DESIGN-0003, DESIGN-0008 / SPEC-0001 / ADR-0002,0003,0005,0008 に従う。task ごとの判断・検証証跡は本 IR に追記する。

## Implementation

### Changes

- ルート scaffold（TASK-0001）: `package.json`, `pnpm-workspace.yaml`, `tsconfig.base.json`, `tsconfig.json`（references は core のみ。runtime-pi/cli は PLAN-0002/0003 で追加）, `biome.json`, `vitest.workspace.ts`
- `@cheaper/core` skeleton（TASK-0002）: `packages/core/{package.json,tsconfig.json,src/index.ts,test/}`、deps `ajv@8.20.0` / `yaml@2.9.1`
- `src/errors.ts`（TASK-0003）: `ERROR_TABLE`（DESIGN-0001 エラー表の全 24 code）, `CheaperError`, `isCheaperError`
- `src/duration.ts`（TASK-0004）: `DURATION_PATTERN` + `parseDuration`（RangeError）
- `src/tools/vocabulary.ts`（TASK-0005）: `TOOL_NAMES`, `TOOL_ALIASES`, `normalizeToolNames`
- `src/config/dirs.ts`（TASK-0006）: `resolveConfigDirs`（CHEAPER_CONFIG_DIR > XDG > ~/.config）
- `src/model/tier.ts`（TASK-0017）: `TIERS`, `compareTier`, `isTier`, `nextTier`
- `src/worker/parse.ts`（TASK-0007）: frontmatter `---` ブロック + YAML parse + body 必須
- `src/worker/schema.ts`（TASK-0008）: `parseSchemaRef`/`loadSchema`/`compileSchema`（Ajv2020 strict+allErrors、2020-12 のみ）
- `src/worker/validate.ts`（TASK-0009）: `WorkerDefinition`, `validateWorkerDefinition`, `validateWorkerInput`
- `src/worker/resolve.ts`（TASK-0010）: `resolveWorker`（project→user→builtin、invalid 即 throw）, `resolveWorkerFile`（explicit）
- `test/fixtures/tmp.ts`: `mkdtemp`/`rm` ヘルパー

### Decisions During Implementation

- `pnpm.onlyBuiltDependencies: []` は pnpm 11 が `package.json` の `pnpm` フィールドを読まないため `pnpm-workspace.yaml` の `onlyBuiltDependencies: []` に配置（警告メッセージに従う）。
- `.gitignore` の `dist/`, `node_modules/`, `*.tsbuildinfo` は既存 rules が既にカバーしていたため追記なし。
- `tsconfig.base.json` に `esModuleInterop: true` を追加（ajv CJS import のため必須）。
- `ajv/dist/2020.js` は NodeNext で default import が namespace 型になるため `Ajv2020Ns.default` を経由（runtime は `module.exports = Ajv2020` で実体一致を確認済み）。
- `parseWorkerFile` の先頭行 `---` は完全一致（trim しない）。行頭空白付きは reject。
- `resolveWorker` の searched は `<dir>/workers/<name>.md` の 3 path。
- `vitest run --passWithNoTests` を core の test script に使用（空パッケージ時点で green にするため）。

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- TASK-0001/0002: `mise exec -- pnpm install` exit 0、`pnpm -r build` exit 0、`pnpm -r test` exit 0（passWithNoTests）、`pnpm biome check .` exit 0、`pnpm tsc -b` exit 0
- TASK-0003/0004/0005/0006/0017: テスト先行（5 ファイル import fail → red 確認）→ 実装 → `pnpm -F @cheaper/core test` 71 tests pass。出力 `tmp/test-output/tasks-0003-0006-0017.txt`
- TASK-0007/0008/0009/0010: テスト先行（4 ファイル import fail → red）→ 実装 → `pnpm -F @cheaper/core test` 113 tests pass。出力 `tmp/test-output/tasks-0007-0010.txt`
- 共通: `pnpm tsc -b` 0 errors、`pnpm biome check .` errors 0（warning/info も解消）

## Risks

- `vitest.workspace.ts` は vitest 5 で動作確認済み（`pnpm -F` フィルタ経由の実行は package 内 vitest で workspace file 未使用）。root からの workspace 実行は TASK-0021 以降に確認。

## Follow-ups

- TASK-0011..0021（policy/*, workspace/paths, model/config|classify|resolve, index export）が残り。
