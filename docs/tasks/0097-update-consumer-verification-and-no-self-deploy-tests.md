---
id: "TASK-0097"
type: "task"
status: "todo"
title: "Update consumer e2e fixtures to packages/apm and add no-self-deploy verification"
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
    - "docs/tasks/0096-repoint-bundle-pipeline-and-ci-freshness.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0021-dev-apm-project-and-distributable-package-layout.md"
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

# TASK-0097: Update consumer verification and no-self-deploy tests

## Work

DESIGN-0021「Test Topology」に従い、consumer e2e を新 package path
へ追従させ、root development install が cheaper own primitive を
deploy しないことを検証する。

### 1. `packages/cli/test/e2e/apm-fixture.ts`

`makeConsumerLayout` が repo `.apm/` を一時 `pkg/` へコピーして
最小 `apm.yml` を自前生成している現行を、実 distribution package
`packages/apm/`（`apm.yml` + `.apm/`）をそのままコピーする形へ
変更する。package path は fixture 内の定数一箇所に集約し、他の
test からも参照できるようにする。

### 2. `packages/cli/test/e2e/pre-read-hook.test.ts`

- `hookScript` を `packages/apm/.apm/hooks/pre-read-code.js` へ
- `bundledCli` を `packages/apm/.apm/hooks/cli/bin/cheaper.mjs` へ

repo 内 path は fixture が公開する定数（または同等の helper）から
導出し、二重管理を避ける。

### 3. 他 e2e の期待値追従

`apm-install.test.ts`・`apm-install-targets.test.ts` 等で repo
`.apm/` を直接参照している箇所があれば fixture 経由へ寄せる。
deploy 結果の期待値（`.windsurf/` 等への skill/hook deploy）は
不変のはずで、差分があれば挙動変化として原因を確認する。

### 4. no-self-deploy 検証の追加

DESIGN-0021 の 2 層検証を追加する:

- **静的検証（unit）**: `packages/cli/test/` または適切な既存
  test file に、repo root に `.apm/` が存在しないこと、
  `packages/apm/{apm.yml,.apm/hooks,.apm/skills}` が存在すること、
  root `apm.yml` が自己参照 dependency・`includes` を持たない
  ことを assert するテストを追加する。
- **挙動検証（e2e）**: dev-project fixture（root `apm.yml` と同形の
  `.apm/` なし project、remote dep を解決しない最小 manifest）を
  一時 dir に作り `apm install` し、target dir に cheaper-*
  skill/hook が deploy されないことを確認する。`apm` CLI 非存在
  環境では既存の skip 判定（`apmAvailable()` 相当）で skip する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0016-dev-apm-and-distributable-package-separation.md`
- `docs/designs/0021-dev-apm-project-and-distributable-package-layout.md`

## Dependencies

- `docs/tasks/0096-repoint-bundle-pipeline-and-ci-freshness.md`
  （bundled CLI が `packages/apm/.apm/hooks/cli` にある前提）

## Affected Files

- `packages/cli/test/e2e/apm-fixture.ts`
- `packages/cli/test/e2e/pre-read-hook.test.ts`
- `packages/cli/test/e2e/apm-install.test.ts`
- `packages/cli/test/e2e/apm-install-targets.test.ts`
- no-self-deploy 検証の追加先 test file

## Done When

- [ ] 全 e2e が repo `.apm/` ではなく `packages/apm/` を参照する
- [ ] repo 内 `.apm/` path の二重管理がなく fixture 定数から導出
- [ ] root `.apm/` 不存在と package layout の静的検証がある
- [ ] dev-project fixture の no-self-deploy e2e がある（apm 不在
      環境では skip）

## Verification

- [ ] `mise exec -- pnpm --filter cheaper test` が exit 0
- [ ] `mise exec -- pnpm lint` / typecheck が exit 0
