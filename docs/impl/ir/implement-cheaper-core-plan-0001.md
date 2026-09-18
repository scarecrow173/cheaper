---
id: "IMPL-034r05iHy2gdnpgYK6Sj6D"
type: "impl"
status: "completed"
title: "Implement @cheaper/core (PLAN-034r05iHy2gdnpHMZZk4ZU)"
created: "2026-09-15"
updated: "2026-09-15"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "package.json"
      - type: "file"
        path: "pnpm-workspace.yaml"
      - type: "file"
        path: "tsconfig.base.json"
      - type: "file"
        path: "tsconfig.json"
      - type: "file"
        path: "biome.json"
      - type: "file"
        path: "vitest.workspace.ts"
      - type: "dir"
        path: "packages/core/"
      - type: "file"
        path: "packages/cli/policy.yaml"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/scaffold-pnpm-monorepo-root-configuration.md"
    - "docs/tasks/create-empty-cheaper-core-package-skeleton.md"
    - "docs/tasks/implement-core-error-model-and-exit-code-table.md"
    - "docs/tasks/implement-duration-parser.md"
    - "docs/tasks/implement-tool-vocabulary-and-alias-normalization.md"
    - "docs/tasks/implement-config-directory-resolution.md"
    - "docs/tasks/implement-worker-frontmatter-parser.md"
    - "docs/tasks/implement-worker-schema-reference-resolution-and-ajv-validation.md"
    - "docs/tasks/implement-worker-definition-validation.md"
    - "docs/tasks/implement-worker-resolver-with-origin-precedence.md"
    - "docs/tasks/implement-runtime-policy-schemas.md"
    - "docs/tasks/implement-runtime-policy-merge.md"
    - "docs/tasks/implement-runtime-policy-loading-order.md"
    - "docs/tasks/implement-runtime-policy-cli-restriction.md"
    - "docs/tasks/implement-capability-intersection-checks.md"
    - "docs/tasks/implement-workspace-path-resolution-with-symlink-policy.md"
    - "docs/tasks/implement-model-tier-ordering.md"
    - "docs/tasks/implement-models-yaml-config-schema-and-merge.md"
    - "docs/tasks/implement-model-tier-classification.md"
    - "docs/tasks/implement-model-candidate-resolution-and-fallback.md"
    - "docs/tasks/export-core-public-api-and-run-full-verification.md"
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

# Implement @cheaper/core (PLAN-034r05iHy2gdnpHMZZk4ZU)

## Summary

PLAN-034r05iHy2gdnpHMZZk4ZU（docs/plans/0001）: pnpm monorepo scaffold と @cheaper/core の全モジュール実装。対象 task: TASK-034r05iHK8bA7gC9ZdvYF9..0021。

## Context

PLAN-034r05iHy2gdnpHMZZk4ZU / DESIGN-034r05iHrZUE6WrdtQIOmE, DESIGN-034r05iHrZUE6Wu1cMdeJ1, DESIGN-034r05iHrZUE6Wzyjd1NS8 / SPEC-034r05iHK8bA7g59V25ltC / ADR-034r05iHlWosDpIq9VUsl7,0003,0005,0008 に従う。task ごとの判断・検証証跡は本 IR に追記する。

## Implementation

### Changes

- ルート scaffold（TASK-034r05iHK8bA7gC9ZdvYF9）: `package.json`, `pnpm-workspace.yaml`, `tsconfig.base.json`, `tsconfig.json`（references は core のみ。runtime-pi/cli は PLAN-034r05iHy2gdnpIrX0mTIb/0003 で追加）, `biome.json`, `vitest.workspace.ts`
- `@cheaper/core` skeleton（TASK-034r05iHK8bA7gD3wnFQ3l）: `packages/core/{package.json,tsconfig.json,src/index.ts,test/}`、deps `ajv@8.20.0` / `yaml@2.9.1`
- `src/errors.ts`（TASK-034r05iHK8bA7gEdDgIUDn）: `ERROR_TABLE`（DESIGN-034r05iHrZUE6WrdtQIOmE エラー表の全 24 code）, `CheaperError`, `isCheaperError`
- `src/duration.ts`（TASK-034r05iHK8bA7gFkeCQ9Xu）: `DURATION_PATTERN` + `parseDuration`（RangeError）
- `src/tools/vocabulary.ts`（TASK-034r05iHK8bA7gGxgfMRs7）: `TOOL_NAMES`, `TOOL_ALIASES`, `normalizeToolNames`
- `src/config/dirs.ts`（TASK-034r05iHK8bA7gHSXhJFHL）: `resolveConfigDirs`（CHEAPER_CONFIG_DIR > XDG > ~/.config）
- `src/model/tier.ts`（TASK-034r05iHK8bA7gVCcwDtMW）: `TIERS`, `compareTier`, `isTier`, `nextTier`
- `src/worker/parse.ts`（TASK-034r05iHK8bA7gJlZFs4r8）: frontmatter `---` ブロック + YAML parse + body 必須
- `src/worker/schema.ts`（TASK-034r05iHK8bA7gKN1oXRb8）: `parseSchemaRef`/`loadSchema`/`compileSchema`（Ajv2020 strict+allErrors、2020-12 のみ）
- `src/worker/validate.ts`（TASK-034r05iHK8bA7gLHezkOrX）: `WorkerDefinition`, `validateWorkerDefinition`, `validateWorkerInput`
- `src/worker/resolve.ts`（TASK-034r05iHK8bA7gN2lJtm5b）: `resolveWorker`（project→user→builtin、invalid 即 throw）, `resolveWorkerFile`（explicit）
- `src/policy/schema.ts`（TASK-034r05iHK8bA7gNEcUV7hq）: `RuntimePolicy`, `POLICY_COMPLETE_SCHEMA`/`POLICY_PARTIAL_SCHEMA`（Ajv2020 strict+allErrors、additionalProperties false 全階層、complete は全 field + `version:1` 必須、partial は `version` のみ必須）, `validatePolicySchema`, `normalizePolicy`（timeout→timeoutMs、tools.allow 語彙検証・alias 正規化を INVALID_RUNTIME_POLICY で wrap）
- `src/policy/merge.ts`（TASK-034r05iHK8bA7gPbLk4c5Q）: `mergePolicyLayers`（object deep merge / scalar replace / array 全体 replace、非破壊）
- `src/policy/load.ts`（TASK-034r05iHK8bA7gQBQpBmLM）: `loadPolicy`（§23: built-in complete → user partial → merge → project partial → merge → effective complete → normalize）
- `packages/cli/policy.yaml`（TASK-034r05iHK8bA7gQBQpBmLM）: built-in canonical policy（引継ぎ §20 の値）
- `src/policy/restrict.ts`（TASK-034r05iHK8bA7gRZ6ARuGy）: `restrictPolicy`（`--timeout` のみ。超過は POLICY_VIOLATION details {requested, policy}）
- `src/policy/capability.ts`（TASK-034r05iHK8bA7gTkcKo0xf）: `checkCapabilities`（missing tools / tier 超過 → POLICY_VIOLATION、effective tools = worker.tools）
- `src/workspace/paths.ts`（TASK-034r05iHK8bA7gUBy3ygQD）: `canonicalizeWorkspaceRoot`, `resolveWorkspacePath`, `WorkspaceEscapeError`, `SymlinkPolicy`
- `src/model/config.ts`（TASK-034r05iHK8bA7gX5ANC9Qn）: `ModelConfig`, `MODELS_CONFIG_SCHEMA`, `loadModelConfig`（merge に `mergePolicyLayers` 共有）
- `src/model/classify.ts`（TASK-034r05iHK8bA7gYsTuUS4s）: `ModelInfo`, `classifyModel`（overrides > built-in 表 > cost heuristic > medium）
- `src/model/resolve.ts`（TASK-034r05iHK8bA7gYVLOiMzO）: `ResolvedModel`, `resolveModel`
- `src/index.ts`（TASK-034r05iHK8bA7h0AqAB1Yr）: 全公開 API export。`test/no-pi-deps.test.ts`（`@earendil-works/` 非依存 assert）、`test/index.test.ts`（export smoke）
- `test/fixtures/tmp.ts`: `mkdtemp`/`rm` ヘルパー

### Decisions During Implementation

- `pnpm.onlyBuiltDependencies: []` は pnpm 11 が `package.json` の `pnpm` フィールドを読まないため `pnpm-workspace.yaml` の `onlyBuiltDependencies: []` に配置（警告メッセージに従う）。
- `.gitignore` の `dist/`, `node_modules/`, `*.tsbuildinfo` は既存 rules が既にカバーしていたため追記なし。
- `tsconfig.base.json` に `esModuleInterop: true` を追加（ajv CJS import のため必須）。
- `ajv/dist/2020.js` は NodeNext で default import が namespace 型になるため `Ajv2020Ns.default` を経由（runtime は `module.exports = Ajv2020` で実体一致を確認済み）。
- `parseWorkerFile` の先頭行 `---` は完全一致（trim しない）。行頭空白付きは reject。
- `resolveWorker` の searched は `<dir>/workers/<name>.md` の 3 path。
- `vitest run --passWithNoTests` を core の test script に使用（空パッケージ時点で green にするため）。
- `RuntimePolicy.tools.allow` の型は `ToolName[]` とした（指示 shape は `string[]` だが「ToolName[] を検証時に保証」の意図に沿い、かつ `string[]` へ代入可能なため後方互換）。
- `normalizePolicy` の tools.allow は「normalizeToolNames 相当」として alias 正規化・未知名拒否・正規化後の重複拒否を実施し、エラーは `INVALID_RUNTIME_POLICY`（details.tool / details.reason）で wrap。policy 側は空配列 valid のため non-empty 検査は適用しない。
- policy file 形式の `timeout` は schema 側でも `DURATION_PATTERN` を pattern として検証し、`normalizePolicy` でも `parseDuration` の RangeError を `INVALID_RUNTIME_POLICY` に包む二重防御。
- `resolveWorkspacePath`: `display` は real（canonical）側からの workspace 相対とした（`within-workspace` で inside symlink を辿った場合、論理 path ではなく実 path が表示される）。NUL・`..`・絶対 escape・dangling symlink はすべて `WorkspaceEscapeError`（CheaperError ではなく tool 層で error result 化される専用 Error、input を保持）。
- `~` は resolveWorkspacePath では展開しない（Pi 側 resolveToCwd が行う）。literal `~/x` は workspace 内の `~` ディレクトリとして扱われることをテストで固定。
- win32 大小文字非依存は `process.platform` 判定ではなく root/candidate 比較の `toLowerCase` で portable に実装（`isInsideRoot`）。
- **DESIGN-034r05iHrZUE6Wu1cMdeJ1 表の dead rule を発見（verbatim 転記で実装、要判断）**: low パターン `mini` が "gemini" の substring ("ge**mini**") に match し、評価順 high→low→medium のため medium の `gemini-.*-pro` は到達不能。`gemini-2.5-pro` は medium ではなく low に分類される。表を逐語転記した結果であり、意図と異なる場合は DESIGN-034r05iHrZUE6Wu1cMdeJ1 の修正が必要（例: `mini` → `\bmini\b`）。テストは verbatim 挙動を assert。
- `resolveModel` の同一 preference pattern 内の順序は unspecified のため、cost 昇順→provider→id の同 comparator を適用して total order を保証。
- models.yaml の `overrides` key は `<provider>/<modelId>` 形式の pattern 検証を入れず loose にした（design は形式を規定するが key 検証を要求しない。未マッチ key は no-op）。
- symlink テストは本機（Windows）で symlink 作成が成功したため skip なしで全実行。skip 経路は `ctx.skip()`（`it.skipIf` は宣言時評価のため不可）+ `console.warn` で理由出力。
- `canonicalizeWorkspaceRoot` は存在確認のみ（isDirectory 検査は design に無く追加しない）。

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- TASK-034r05iHK8bA7gC9ZdvYF9/0002: `mise exec -- pnpm install` exit 0、`pnpm -r build` exit 0、`pnpm -r test` exit 0（passWithNoTests）、`pnpm biome check .` exit 0、`pnpm tsc -b` exit 0
- TASK-034r05iHK8bA7gEdDgIUDn/0004/0005/0006/0017: テスト先行（5 ファイル import fail → red 確認）→ 実装 → `pnpm -F @cheaper/core test` 71 tests pass。出力 `tmp/test-output/tasks-0003-0006-0017.txt`
- TASK-034r05iHK8bA7gJlZFs4r8/0008/0009/0010: テスト先行（4 ファイル import fail → red）→ 実装 → `pnpm -F @cheaper/core test` 113 tests pass。出力 `tmp/test-output/tasks-0007-0010.txt`
- TASK-034r05iHK8bA7gNEcUV7hq..0016, 0018..0021: 各 task テスト先行（import fail → red 確認）→ 実装 → green → `pnpm biome check .` + `pnpm tsc -b` pass を逐次確認。最終 `pnpm -r build`/`pnpm -r test`（225 tests pass, 20 files）/`pnpm biome check .`/`pnpm tsc -b` すべて exit 0。出力 `tmp/test-output/tasks-0011-0021.txt`
- Windows symlink: 本機で symlink 作成成功（dir/file/junction いずれも可）のため skip なし。`test/workspace/paths.test.ts` 16 tests 全実行
- 共通: `pnpm tsc -b` 0 errors、`pnpm biome check .` errors 0（warning/info も解消）

## Risks

- `vitest.workspace.ts` は vitest 5 で動作確認済み（`pnpm -F` フィルタ経由の実行は package 内 vitest で workspace file 未使用）。root からの workspace 実行は TASK-034r05iHK8bA7h0AqAB1Yr 以降に確認。

## Follow-ups

- PLAN-034r05iHy2gdnpHMZZk4ZU の全 21 task 完了。残りは PLAN-034r05iHy2gdnpIrX0mTIb（runtime-pi）以降。
- DESIGN-034r05iHrZUE6Wu1cMdeJ1 の low パターン `mini` が "gemini" 部分文字列に match し medium の `gemini-.*-pro` を dead rule にしている件は、要 design 修正判断（上記 Decisions 参照）。
