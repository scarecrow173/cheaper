---
id: "PLAN-0016"
type: "plan"
status: "completed"
test-spec-skip: "Verified by package unit/e2e test suites and CI; no dedicated test-spec doc authored."
title: "Development APM project / distributable package separation"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../designs/0021-dev-apm-project-and-distributable-package-layout.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/0021-dev-apm-project-and-distributable-package-layout.md"
    - "../specs/0016-separate-dev-apm-project-from-distributable-package.md"
  implemented-by:
    - "../tasks/0095-move-apm-primitives-to-distributable-package.md"
    - "../tasks/0096-repoint-bundle-pipeline-and-ci-freshness.md"
    - "../tasks/0097-update-consumer-verification-and-no-self-deploy-tests.md"
    - "../tasks/0098-update-readme-agents-and-final-verification.md"
  depends-on:
    - "../plans/0013-cross-agent-distribution-verification.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/0016-separate-dev-apm-project-from-distributable-package.md"
    - "../adr/0019-separate-dev-apm-project-via-nested-distributable-package.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/0021-dev-apm-project-and-distributable-package-layout.md"
  derived-by: []
  verifies:
    - "../specs/0016-separate-dev-apm-project-from-distributable-package.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# PLAN-0016: Development APM project / distributable package separation

## Goal

SPEC-0016 / ADR-0019 / DESIGN-0021 に従い、root `.apm/` の配布
primitive を `packages/apm/` へ移し、root `apm.yml` を
development-only manifest にする。bundle pipeline・CI・e2e を新
path に追従させ、consumer 側の deploy 結果を維持する。

## Scope

単一 implementation stream。移動対象は `.apm/` tree・`apm.yml` ×2・
`scripts/bundle-cli.mjs`・CI・e2e fixture・README/AGENTS.md に限定
される。4 task（TASK-0095〜0098）で完結するため plan 分割はしない。

### In Scope

- `git mv .apm packages/apm/.apm` と `packages/apm/apm.yml` 新設、
  root `apm.yml` の dev-only 化（`name: cheaper-dev`、`includes` 削除）。
- `scripts/bundle-cli.mjs` の `outDir` 変更と bundle 再生成、CI
  freshness path 追従。
- e2e fixture の `packages/apm` 参照化、hook/bundle path 追従、
  no-self-deploy 検証（静的 + e2e）の追加。
- README / AGENTS.md の development/distribution 分離の記述更新。

### Out of Scope

- worktree lifecycle・`mise run setup`/`mise run check`・CI の
  canonical entrypoint 化（Issue #7）。
- version source 統一（Issue #11）、per-target hook adapter（Issue #10）。
- 旧 layout の compatibility shim、generated target artifacts の
  Git 管理化。

## File Map

| File | Change |
| --- | --- |
| `.apm/` → `packages/apm/.apm/` | `git mv` で全 primitive 移動。 |
| `packages/apm/apm.yml` | 新規（`name: cheaper`、`version: "1.0.0"` 等の最小 manifest）。 |
| `apm.yml` | `name: cheaper-dev`、`includes` 削除。 |
| `scripts/bundle-cli.mjs` | `outDir` を `packages/apm/.apm/hooks/cli` へ。 |
| `.github/workflows/ci.yml` | freshness check path を `packages/apm/.apm/hooks/cli` へ。 |
| `packages/cli/test/e2e/apm-fixture.ts` | `packages/apm` をコピーする形へ更新。 |
| `packages/cli/test/e2e/pre-read-hook.test.ts` | `hookScript`/`bundledCli` path 追従。 |
| `packages/cli/test/` | no-self-deploy の静的検証 + e2e を追加。 |
| `README.md` | `.apm/` 記述を `packages/apm/` + 責務分離へ更新。 |
| `AGENTS.md` | distribution package の存在を最小限反映。 |

## Task Breakdown

1. **TASK-0095**: distribution package 分離。`git mv`、
   `packages/apm/apm.yml`、root `apm.yml` の dev-only 化。
2. **TASK-0096**: bundle pipeline 追従。`bundle-cli.mjs` の `outDir`
   変更、`pnpm bundle` 再生成、CI freshness path 更新。
3. **TASK-0097**: consumer-side verification。fixture を `packages/apm`
   コピーへ更新、hook/bundle path 追従、no-self-deploy 検証を追加。
4. **TASK-0098**: docs + final verification。README/AGENTS.md 更新、
   lint/typecheck/test/freshness 実行、SPEC-0016/PLAN-0016 status 同期。

## Dependency Graph

```text
TASK-0095 (structure)
   └─ TASK-0096 (bundle pipeline)   ※ .apm 移動後の path が前提
        └─ TASK-0097 (consumer verification)  ※ package path が前提
             └─ TASK-0098 (docs + final verification)
```

## Risk Register

| Risk | Mitigation |
| --- | --- |
| 移動で `hooks.json` 内の deploy 相対 path が壊れる | hook bundle 内相対 path は deploy 構造内で閉じており repo path 非依存。e2e で deploy 結果を検証。 |
| e2e が新 layout で失敗する | fixture は実 `packages/apm` をコピーし deploy 結果の期待値は不変。差分があれば test が検出する。 |
| `pnpm` が `packages/apm` を workspace package と誤認する | `package.json` を置かない。pnpm glob は manifest 必須のため対象外。 |
| freshness check が旧 path を見て常時 pass になる | CI の `git diff` 対象を新 path に変更し、TASK-0096 で実際に再生成して確認。 |

## Rollback Strategy

`git mv` 系の変更のため、revert は `git revert` または `.apm/` の
逆移動で戻せる。generated bundle は source から再生成可能で、互換
shim を残さない方針のため rollback 手順も単純な逆操作でよい。

## Verification Matrix

| Acceptance Criterion (SPEC-0016) | Verification |
| --- | --- |
| root `apm.yml` が development-only | manifest の静的検証（自己参照 dep・`.apm/` 不存在）。 |
| root に distributable `.apm/` がない | unit 静的検証で `!existsSync(root/.apm)`。 |
| nested distributable package の存在 | `packages/apm/{apm.yml,.apm/}` の存在 assert。 |
| root install で skills/hooks が deploy されない | dev-project fixture の `apm install` e2e。 |
| consumer install で skills/hooks/CLI が deploy される | 既存 apm-install e2e（fixture を実 package 参照に更新）。 |
| bundled CLI が package 内に生成され hook から解決 | bundle 再生成 + hook e2e の `bundledCli` 実行。 |
| freshness check が新 path を検証 | CI step と `git diff` 対象 path の確認。 |
| test が self-install に依存しない | 全 e2e が temp consumer workspace 経由であること。 |
| lint/typecheck/test 通過 | `pnpm lint` / `pnpm typecheck` / `pnpm test`。 |
| README/AGENTS.md が責務分離を説明 | 文書更新 diff の確認。 |
