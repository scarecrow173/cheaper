---
id: "TASK-0098"
type: "task"
status: "todo"
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
    - "docs/plans/0016-dev-apm-and-distributable-package-separation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0016-dev-apm-and-distributable-package-separation.md"
    - "docs/tasks/0097-update-consumer-verification-and-no-self-deploy-tests.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0021-dev-apm-project-and-distributable-package-layout.md"
    - "docs/specs/0016-separate-dev-apm-project-from-distributable-package.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/0016-separate-dev-apm-project-from-distributable-package.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-0098: Update docs and run final verification

## Work

PLAN-0016 の仕上げ task。development/distribution 分離を
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

- `pnpm test` には TASK-0097 の no-self-deploy 検証と consumer
  e2e を含む。
- 実 provider / CLI / hook の検証は従来通り一時 consumer
  workspace でのみ行い、repo 内 bundle を直接叩く検証は
  `pre-read-hook.test.ts` の既存形（hook script + bundled CLI の
  direct invoke、self-install ではない）を維持する。

### 4. status 同期

- SPEC-0016 → `implemented`
- PLAN-0016 → `completed`
- TASK-0095〜0098 → `done`
- 各 index（README.md）の status を同期

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0016-dev-apm-and-distributable-package-separation.md`
- `docs/designs/0021-dev-apm-project-and-distributable-package-layout.md`
- `docs/specs/0016-separate-dev-apm-project-from-distributable-package.md`

## Dependencies

- `docs/tasks/0097-update-consumer-verification-and-no-self-deploy-tests.md`
  （consumer verification の安定化が前提）

## Affected Files

- `README.md`
- `AGENTS.md`
- `docs/specs/0016-*.md`・`docs/plans/0016-*.md`・`docs/tasks/0095..0098-*.md`
  の status と各 index

## Done When

- [ ] README が `packages/apm` への install 参照と責務分離を説明
- [ ] AGENTS.md が新構造に整合
- [ ] 上記検証コマンドが全て exit 0 で結果が記録されている
- [ ] SPEC-0016/PLAN-0016/TASK-0095..0098 と各 index の status が
      同期されている

## Verification

- [ ] `mise exec -- pnpm -r build` / `pnpm lint` / `pnpm typecheck`
      / `pnpm test` が exit 0
- [ ] bundle 再生成後 `git diff --exit-code --
      packages/apm/.apm/hooks/cli` が clean
- [ ] `audit_docs.js --type all` が no findings
