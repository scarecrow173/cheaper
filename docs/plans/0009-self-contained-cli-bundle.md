---
id: "PLAN-0009"
type: "plan"
status: "completed"
test-spec-skip: "Verified by package unit/e2e test suites and CI; no dedicated test-spec doc authored."
title: "Self-contained CLI bundle distribution"
created: "2026-09-16"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../designs/0014-cli-bundle-layout.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/0014-cli-bundle-layout.md"
    - "../specs/0009-apm-self-contained-cli-distribution.md"
  implemented-by: []
  depends-on:
    - "../plans/0008-apm-delegation-layer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/0014-cli-bundle-layout.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Self-contained CLI bundle distribution plan

## Goal

`pnpm bundle` で `.apm/hooks/cli/` を生成し、hook が bundled CLI を呼び、
consumer install で `.windsurf/hooks/cheaper/cli/` へ deploy される状態を
e2e で保証する。

## Upstream Documents

- SPEC-0009 / DESIGN-0014 / DISC-0006

## File Map

| File | Change |
|---|---|
| `scripts/bundle-cli.mjs` | 新規 |
| `package.json`（root） | `bundle` script 追加 |
| `.apm/hooks/cli/**` | 生成物（commit） |
| `.apm/hooks/hooks.json` | command に CLI argv 追加 |
| `.apm/hooks/pre-read-code.js` | argv[2] 解決追加 |
| `.apm/skills/cheaper-usage/SKILL.md` | bundled path 記載 |
| `packages/cli/test/e2e/apm-install.test.ts` | 新規（apm 不在時 skip） |
| `docs/*/README.md` | index 更新 |

## Task Breakdown

- TASK-0074 bundle script + generated cli/ + hook argv resolution
- TASK-0075 consumer install smoke e2e + skill/index/README updates
- TASK-0076 full verification + commit

## Verification

- `mise exec -- pnpm bundle` 再生成可能
- `mise exec -- pnpm -r build && pnpm -r test && pnpm lint && pnpm typecheck`
- e2e: install → deploy → version → hook block

## Self-Review

- [x] File paths and terminology consistent with DESIGN-0014.

## Implementation Handoff

bundle は commit する生成物。script 実行後に test を回す。
