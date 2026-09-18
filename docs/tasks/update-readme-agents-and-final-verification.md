---
id: "TASK-034r05iHQeQpDqvqdt4mXS"
type: "task"
status: "done"
title: "Update README/AGENTS for dev/distribution split and run final verification"
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
    - "docs/plans/dev-apm-and-distributable-package-separation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/dev-apm-and-distributable-package-separation.md"
    - "docs/tasks/update-consumer-verification-and-no-self-deploy-tests.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/dev-apm-project-and-distributable-package-layout.md"
    - "docs/specs/separate-dev-apm-project-from-distributable-package.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/separate-dev-apm-project-from-distributable-package.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-034r05iHQeQpDqvqdt4mXS: Update docs and run final verification

## Work

PLAN-034r05iHE1Fbkp5YZSKJM2 の仕上げ task。development/distribution 分離を
README・AGENTS.md に反映し、verification matrix 全項目を実行して
status を同期する。

### 1. `README.md`

- `.apm/` を配布物として説明している記述を `packages/apm/` に
  追従させ、root project が development-only、consumer へは
  `packages/apm` を install する分離構造であることを明記する。
- consumer の install 参照が `packages/apm` sub-path（virtual
  package）になることを記載する。大掛かりな README 再構成は
  Issue #15 の scope なので必要最小限に留める。

### 2. `AGENTS.md`

セットアップ節に distribution package（`packages/apm/`）の存在を
最小限反映する。bundle 生成・freshness の対象 path 言及があれば
新 path に合わせる。

### 3. 最終検証

```bash
mise exec -- pnpm -r build
mise exec -- pnpm lint
mise exec -- pnpm typecheck
mise exec -- pnpm test
mise exec -- node scripts/bundle-cli.mjs
git diff --exit-code -- packages/apm/.apm/hooks/cli
node .claude/skills/doc-status/scripts/audit_docs.js --type all
```

- `pnpm test` には TASK-034r05iHQeQpDqtGttOcbh の no-self-deploy 検証と consumer
  e2e を含む。
- 実 provider / CLI / hook の検証は従来通り一時 consumer
  workspace でのみ行い、repo 内 bundle を直接叩く検証は
  `pre-read-hook.test.ts` の既存形（hook script + bundled CLI の
  direct invoke、self-install ではない）を維持する。

### 4. status 同期

- SPEC-034r05iHK8bA7go4BQtGNS → `implemented`
- PLAN-034r05iHE1Fbkp5YZSKJM2 → `completed`
- TASK-034r05iHQeQpDqrbmtswfZ〜0098 → `done`
- 各 index（README.md）の status を同期

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/dev-apm-and-distributable-package-separation.md`
- `docs/designs/dev-apm-project-and-distributable-package-layout.md`
- `docs/specs/separate-dev-apm-project-from-distributable-package.md`

## Dependencies

- `docs/tasks/update-consumer-verification-and-no-self-deploy-tests.md`
  （consumer verification の安定化が前提）

## Affected Files

- `README.md`
- `AGENTS.md`
- `docs/specs/0016-*.md`・`docs/plans/0016-*.md`・`docs/tasks/0095..0098-*.md`
  の status と各 index

## Done When

- [x] README が `packages/apm` への install 参照と責務分離を説明
- [x] AGENTS.md が新構造に整合
- [x] 上記検証コマンドが全て exit 0 で結果が記録されている
- [x] SPEC-034r05iHK8bA7go4BQtGNS/PLAN-034r05iHE1Fbkp5YZSKJM2/TASK-034r05iHQeQpDqrbmtswfZ..0098 と各 index の status が
      同期されている

## Verification

- [x] `mise exec -- pnpm -r build` / `pnpm lint` / `pnpm typecheck`
      / `pnpm test` が exit 0
- [x] bundle 再生成後 `git diff --exit-code --
      packages/apm/.apm/hooks/cli` が clean
- [x] `audit_docs.js --type all` が no findings

## Verification results

2026-09-18 実施（Windows / node v26.8.2 / pnpm 11.0.0）。

| Command | Result |
|---------|--------|
| `mise exec -- pnpm -r build` | exit 0（core / runtime-pi / cli tsc -b 全 Done） |
| `mise exec -- pnpm lint` | exit 0（biome check、141 files / warnings のみ） |
| `mise exec -- pnpm typecheck` | exit 0（tsc -b） |
| `mise exec -- pnpm --filter cheaper exec tsc --noEmit -p .` | exit 0 |
| `mise exec -- pnpm --filter cheaper test` | exit 0（14 files / 131 tests pass） |
| `mise exec -- node scripts/bundle-cli.mjs` | exit 0 → `packages/apm/.apm/hooks/cli`（12.2 MiB） |
| `git diff --exit-code -- packages/apm/.apm/hooks/cli` | exit 0（freshness clean） |
| `node .claude/skills/doc-status/scripts/audit_docs.js --type all` | exit 0（185 files / no findings） |

test 内訳の要点:

- `test/e2e/apm-package-layout.test.ts`（5 tests）: root `.apm/` 不存在・
  `packages/apm` package layout・dev-only root manifest の静的検証と、
  `.apm/` を持たない dev-project fixture への `apm install` が
  cheaper-* skill/hook を deploy しない実測（apm CLI 存在環境で実行、
  約 6.7s）。
- `test/e2e/apm-install*.test.ts`: consumer workspace へ実
  `packages/apm` package を local path dep として install し、全
  target（windsurf/copilot/codex/claude/cursor/agent-skills）で
  skills/hook bundle/bundled CLI の deploy と動作を確認。
- `pre-read-code.js` は `packages/**` lint scope 入りに伴い biome
  format を適用（挙動変更なし）。
