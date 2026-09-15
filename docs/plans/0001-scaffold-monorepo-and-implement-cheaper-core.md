---
id: "PLAN-0001"
type: "plan"
status: "completed"
title: "Scaffold monorepo and implement @cheaper/core"
created: "2026-09-15"
updated: "2026-09-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/handoff/2026-09-15-cheaper-v1-implementation-handoff.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/0001-cheaper-v1-worker-runtime-cli.md"
  implemented-by:
    - "../tasks/0001-scaffold-pnpm-monorepo-root-configuration.md"
    - "../tasks/0002-create-empty-cheaper-core-package-skeleton.md"
    - "../tasks/0003-implement-core-error-model-and-exit-code-table.md"
    - "../tasks/0004-implement-duration-parser.md"
    - "../tasks/0005-implement-tool-vocabulary-and-alias-normalization.md"
    - "../tasks/0006-implement-config-directory-resolution.md"
    - "../tasks/0007-implement-worker-frontmatter-parser.md"
    - "../tasks/0008-implement-worker-schema-reference-resolution-and-ajv-validation.md"
    - "../tasks/0009-implement-worker-definition-validation.md"
    - "../tasks/0010-implement-worker-resolver-with-origin-precedence.md"
    - "../tasks/0011-implement-runtime-policy-schemas.md"
    - "../tasks/0012-implement-runtime-policy-merge.md"
    - "../tasks/0013-implement-runtime-policy-loading-order.md"
    - "../tasks/0014-implement-runtime-policy-cli-restriction.md"
    - "../tasks/0015-implement-capability-intersection-checks.md"
    - "../tasks/0016-implement-workspace-path-resolution-with-symlink-policy.md"
    - "../tasks/0017-implement-model-tier-ordering.md"
    - "../tasks/0018-implement-models-yaml-config-schema-and-merge.md"
    - "../tasks/0019-implement-model-tier-classification.md"
    - "../tasks/0020-implement-model-candidate-resolution-and-fallback.md"
    - "../tasks/0021-export-core-public-api-and-run-full-verification.md"
  depends-on: []
  blocks:
    - "docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/0001-cheaper-v1-worker-runtime-cli.md"
    - "docs/designs/0001-worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md"
    - "docs/designs/0003-model-resolver.md"
    - "docs/designs/0008-repository-structure-toolchain.md"
    - "docs/adr/0002-worker-yaml-frontmatter-markdown.md"
    - "docs/adr/0003-worker-definition-runtime-policy-runtime-invariants-capability.md"
    - "docs/adr/0005-model-tier-model-resolver.md"
    - "docs/adr/0008-typescript-pnpm-monorepo.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Scaffold monorepo and implement @cheaper/core Implementation Plan

## Goal

pnpm monorepo を立て、Pi 非依存の `@cheaper/core` を DESIGN-0001/0003 のとおり実装し、単体テストで検証する。

## Scope

1 つの実装ストリーム（ルート scaffold + `@cheaper/core` の全モジュール）。境界は「`packages/core` と `packages/cli/policy.yaml` まで」。`runtime-pi`・`cli`・built-in workers は PLAN-0002/0003。

共通事項:

- 全コマンドは `mise exec -- pnpm ...` 経由。
- 実装は引継ぎ §41 の順序（core → runtime-pi → cli）で、Pi integration から書き始めない。

## Upstream Documents

- `docs/specs/0001-cheaper-v1-worker-runtime-cli.md`
- `docs/adr/0002-worker-yaml-frontmatter-markdown.md`
- `docs/adr/0003-worker-definition-runtime-policy-runtime-invariants-capability.md`
- `docs/adr/0005-model-tier-model-resolver.md`
- `docs/adr/0008-typescript-pnpm-monorepo.md`

## Design Inputs

- `docs/designs/0001-worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md`
- `docs/designs/0003-model-resolver.md`
- `docs/designs/0008-repository-structure-toolchain.md`
- `docs/designs/0002-workspace-filesystem-boundary-sandboxed-tools.md`（`workspace/paths.ts` の `resolveWorkspacePath` アルゴリズムのみ）

## File Map

- `package.json` - private、`packageManager: pnpm@11`、scripts: `build`=`pnpm -r build`、`test`=`pnpm -r test`、`lint`=`biome check .`、`typecheck`=`tsc -b`
- `pnpm-workspace.yaml` - `packages/*`
- `tsconfig.base.json` - `module: NodeNext`、`target: ES2023`、`strict`、`exactOptionalPropertyTypes`、`noUncheckedIndexedAccess`
- `tsconfig.json` - 3 package への project references
- `biome.json` - lint/format 設定
- `vitest.workspace.ts` - 各 package の test 設定
- `.gitignore` - `dist/`、`node_modules/` を追加
- `packages/core/package.json` - name `@cheaper/core`、`type: module`、`exports: ./dist/index.js`、deps: `ajv`, `yaml`；devDeps: `typescript`, `vitest`, `@types/node`
- `packages/core/tsconfig.json` - base 継承
- `packages/core/src/errors.ts` - `CheaperError` + code→exitCode/retryable 対応表（DESIGN-0001 の表が唯一のソース）
- `packages/core/src/duration.ts` - `parseDuration`
- `packages/core/src/config/dirs.ts` - 設定ディレクトリ解決
- `packages/core/src/worker/parse.ts` - frontmatter/body 分離
- `packages/core/src/worker/schema.ts` - `SchemaRef` 解決 + Ajv 2020 `validateSchema`（input/output 両用）
- `packages/core/src/worker/validate.ts` - frontmatter 検証
- `packages/core/src/worker/resolve.ts` - `resolveWorker` + `--worker-file` 経路
- `packages/core/src/policy/schema.ts` - complete/partial 両スキーマ
- `packages/core/src/policy/load.ts` - §23 ロード順
- `packages/core/src/policy/merge.ts` - deep merge/scalar replace/array replace
- `packages/core/src/policy/restrict.ts` - CLI restriction（`--timeout` のみ）
- `packages/core/src/policy/capability.ts` - tools ⊄ allow / tier > maxTier 判定
- `packages/core/src/tools/vocabulary.ts` - `ToolName`・alias・正規化
- `packages/core/src/workspace/paths.ts` - `resolveWorkspacePath`（DESIGN-0002 アルゴリズム 1–5、win32 大小文字非依存比較）
- `packages/core/src/model/tier.ts` - tier 順序 `low < medium < high`
- `packages/core/src/model/config.ts` - `models.yaml` schema/merge
- `packages/core/src/model/classify.ts` - built-in 正規表現表 + cost heuristic。Pi 型に依存しない最小 shape `{ provider, id, cost?: {input, output}, maxTokens }` を core 側で定義
- `packages/core/src/model/resolve.ts` - 候補・順序・higher fallback・`POLICY_VIOLATION`/`MODEL_TIER_UNAVAILABLE`
- `packages/core/src/index.ts` - 公開 API export
- `packages/core/test/` - 各 src モジュール対応の `*.test.ts`。fixture は `packages/core/test/fixtures/`（Worker md / policy yaml / models yaml / symlink 用 tmp dir 作成ヘルパー）
- `packages/cli/policy.yaml` - built-in policy（引継ぎ §20 の canonical 値。core の load テストの built-in fixture として本 plan で作成。built-in workers は PLAN-0003）

## Implementation Sequence

- [ ] 1. ルート scaffold（package.json, workspace, tsconfig.base, biome.json, vitest.workspace）と `pnpm install`。依存追加は `pnpm add` で行い `minimum_release_age` 7d を守る。ignored build scripts 警告は DESIGN-0008 のとおり `pnpm.onlyBuiltDependencies` を空配列で明示して exit 0 にする
- [ ] 2. `packages/core` の空パッケージ + `tsc -b` + 空 vitest が pass
- [ ] 3. `errors.ts`（CheaperError + 対応表）
- [ ] 4. `duration.ts`（`^[1-9][0-9]*(ms|s|m|h)$` → ms）
- [ ] 5. `tools/vocabulary.ts`（alias 完全一致、`UNKNOWN_TOOL`、重複検出）
- [ ] 6. `config/dirs.ts`（`CHEAPER_CONFIG_DIR` > `XDG_CONFIG_HOME/cheaper` > `~/.config/cheaper`、project `.cheaper/`、builtinDir は引数注入）
- [ ] 7. `worker/parse.ts`
- [ ] 8. `worker/schema.ts`
- [ ] 9. `worker/validate.ts`
- [ ] 10. `worker/resolve.ts`
- [ ] 11. `policy/schema.ts`
- [ ] 12. `policy/merge.ts`
- [ ] 13. `policy/load.ts`
- [ ] 14. `policy/restrict.ts`
- [ ] 15. `policy/capability.ts`
- [ ] 16. `workspace/paths.ts`（テストは tmp dir に実 symlink。Windows で symlink 作成権限が無い場合は `it.skipIf` で理由をログし、CI/Unix で必ず実行することを Open Questions に記載）
- [ ] 17. `model/tier.ts`
- [ ] 18. `model/config.ts`
- [ ] 19. `model/classify.ts`
- [ ] 20. `model/resolve.ts`
- [ ] 21. `index.ts` 公開 API、`pnpm -r build && pnpm -r test && pnpm biome check .`

## Dependency Graph

| ステップ | 依存先 | 並列可能 |
| --- | --- | --- |
| 1 ルート scaffold | なし | - |
| 2 core 空パッケージ | 1 | - |
| 3 errors | 2 | 4,5,6,17 と並列 |
| 4 duration | 2 | 3,5,6,17 と並列 |
| 5 vocabulary | 2 | 3,4,6,17 と並列 |
| 6 dirs | 2 | 3,4,5,17 と並列 |
| 7 parse | 3 | - |
| 8 schema | 3,7 | - |
| 9 validate | 5,8 | - |
| 10 resolve | 6,9 | - |
| 11 policy/schema | 3,4 | 16 と並列 |
| 12 policy/merge | 11 | - |
| 13 policy/load | 12 | - |
| 14 policy/restrict | 13 | - |
| 15 policy/capability | 13 | - |
| 16 workspace/paths | 3 | 11–15 と並列 |
| 17 tier | 2 | 3–6 と並列 |
| 18 model/config | 17 | - |
| 19 classify | 17 | - |
| 20 model/resolve | 18,19 | - |
| 21 index + 全検証 | 10,15,16,20 | - |

critical path: 1→2→3→7→8→9→10→21

## Task Granularity

各「実装単位」は必ず次の 5 ステップに分解する: (1) 失敗するテストを書く (2) 対象テストを実行して red を確認 (3) 最小実装 (4) 再実行で green (5) `pnpm biome check` と `tsc -b` が pass。task-doc 化するときはこの単位を 1 task にする。

## Dependencies and Risks

### Risk Register

| リスク | 発生確率/影響 | 対策 |
| --- | --- | --- |
| Windows での symlink 作成権限不足で workspace/paths テストが実行不能 | 中/中 | `it.skipIf` で理由をログし CI/Unix で必ず実行 |
| `path.matchesGlob` の Node 26 での安定性 | 低/中 | core では使用しない。runtime-pi の find で使用し fallback 不要と確認済み |
| 外部依存 availability: `ajv` v8 の `ajv/dist/2020` export path が存在しない可能性 | 低/高 | install 直後に import smoke test |
| breaking-change | なし | 新規コードのみ |
| data-loss | なし | filesystem 書き込みは tmp fixture のみ |

## Rollback Strategy

ファイルシステム外・DB・公開 API・マイグレーション無し。irreversible step は無い。rollback = 該当コミットの `git revert`。検証は `pnpm -r build && pnpm -r test` が green であること。

## Verification

### Verification Matrix

| 対象（SPEC-0001 受け入れ条件） | 検証 | pass 条件 |
| --- | --- | --- |
| Worker resolution: project invalid → fallback 無し `INVALID_WORKER_DEFINITION` exit 3 | `pnpm --filter @cheaper/core test -- worker/resolve` | test green |
| Worker resolution: project → user → builtin 優先順位 | 同上 | test green |
| Worker resolution: `--worker-file` 独立経路・stem 一致・named resolution 非実施 | 同上 | test green |
| Worker resolution: 未検出名 → `WORKER_NOT_FOUND` | 同上 | test green |
| Worker resolution: name 規則違反 → `INVALID_WORKER_NAME` | 同上 | test green |
| Worker resolution: stem 不一致 → `WORKER_NAME_MISMATCH` | 同上 | test green |
| Worker validation: frontmatter 欠落/YAML エラー/body 空 → `INVALID_WORKER_DEFINITION` | `pnpm --filter @cheaper/core test -- worker/parse worker/validate` | test green |
| Worker validation: `cheaper:` strict・top-level 未知 field ignore | 同上 | test green |
| Worker validation: alias 完全一致・未知名 `UNKNOWN_TOOL`・正規化後重複 | `pnpm --filter @cheaper/core test -- tools/vocabulary worker/validate` | test green |
| Worker validation: tools non-empty | 同上 | test green |
| Worker validation: schema file/inline exactly one・`file` 基準解決・拡張子制限 | `pnpm --filter @cheaper/core test -- worker/schema` | test green |
| Worker validation: draft 2020-12 以外/コンパイル失敗 → `INVALID_*_SCHEMA` | 同上 | test green |
| Worker validation: schema 欠落 → `*_SCHEMA_NOT_FOUND` | 同上 | test green |
| Worker validation: input schema 不一致 → `INVALID_INPUT` exit 2 | 同上 | test green |
| Runtime Policy: §23 ロード順 | `pnpm --filter @cheaper/core test -- policy/load` | test green |
| Runtime Policy: deep merge / scalar replace / array replace | `pnpm --filter @cheaper/core test -- policy/merge` | test green |
| Runtime Policy: complete/partial schema・additionalProperties false 全階層 | `pnpm --filter @cheaper/core test -- policy/schema` | test green |
| Runtime Policy: 未知 field → `INVALID_RUNTIME_POLICY` | 同上 | test green |
| Runtime Policy: `--timeout` のみ許可・`--timeout > policy.timeout` → `POLICY_VIOLATION` | `pnpm --filter @cheaper/core test -- policy/restrict` | test green |
| Runtime Policy: duration 文字列 → `timeoutMs` 正規化 | `pnpm --filter @cheaper/core test -- duration` | test green |
| capability: tools ⊄ allow → `POLICY_VIOLATION`（欠けた tool 名を details に） | `pnpm --filter @cheaper/core test -- policy/capability` | test green |
| capability: tier > maxTier → `POLICY_VIOLATION` | 同上 | test green |
| Security: `../`・絶対 path・`~` → ESCAPE | `pnpm --filter @cheaper/core test -- workspace/paths` | test green |
| Security: symlink `deny` 拒否・`within-workspace` で外は拒否・内は許可 | 同上（tmp dir 実 symlink） | test green |
| Security: 不存在 file への create 許容・最深祖先 realpath が root 配下 | 同上 | test green |
| model: 分類優先順位 overrides > built-in > cost > medium | `pnpm --filter @cheaper/core test -- model/classify` | test green |
| model: `requested > maxTier` → `POLICY_VIOLATION` | `pnpm --filter @cheaper/core test -- model/resolve` | test green |
| model: `tierFallback: higher` で段階的上昇・無ければ `MODEL_TIER_UNAVAILABLE` | 同上 | test green |
| model: preferences パターン順・cost 昇順・provider/id の決定論的順序 | 同上 | test green |
| model: `models.yaml` 未知 field → `INVALID_MODEL_CONFIG` | `pnpm --filter @cheaper/core test -- model/config` | test green |
| error model: 全 code が対応表に存在し exitCode/retryable 一貫 | `pnpm --filter @cheaper/core test -- errors` | test green |
| `@cheaper/core` が `@earendil-works` を含まない | `node -e` で package.json を assert するテスト | test green |
| 全体 | `pnpm -r build && pnpm -r test && pnpm biome check .` | 全 green |

## Task Breakdown

Implementation Sequence の各番号がそのまま 1 task（内部は Task Granularity の 5 ステップ）。並列可能ブロックは Dependency Graph の「並列可能」列どおり。

## Open Questions / Gaps

- Windows での symlink テスト実行可否（権限が無い場合 `it.skipIf` + CI/Unix 必須）
- user config dir の `~` 解決は `os.homedir()` を使う（決定済み・記載のみ）

## Self-Review

- [x] Every spec requirement maps to a task.（SPEC-0001 の Worker resolution/validation/Runtime Policy/Security/Execution のうち core 範囲を Verification Matrix に全対応付け）
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: delegated / subagent-capable 実装を推奨
- Independent work streams: Dependency Graph の並列可能行（{3,4,5,6,17}、{11–15} と 16、{18,19}）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 仮定: `pnpm.onlyBuiltDependencies: []` をルート package.json に書くことで ignored build scripts の exit 1 を回避する（DESIGN-0008）
- レビュー重点: `worker/schema.ts` の Ajv 2020 draft 検証と `policy/capability.ts` の details 内容が DESIGN-0001 と一致するか
- 最重要 upstream: DESIGN-0001（error 対応表）、SPEC-0001 受け入れ条件
