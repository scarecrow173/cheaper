---
id: "PLAN-0010"
type: "plan"
status: "completed"
test-spec-skip: "Verified by package unit/e2e test suites and CI; no dedicated test-spec doc authored."
title: "Write-scope enforcement and privacy deny list"
created: "2026-09-16"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../designs/0015-write-scope-and-privacy-deny.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/0015-write-scope-and-privacy-deny.md"
  implemented-by:
    - "../tasks/0079-core-deny-matcher-and-write-scope-field.md"
    - "../tasks/0080-runtime-scope-and-deny-enforcement.md"
    - "../tasks/0081-writer-write-scope-and-e2e-verification.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/0015-write-scope-and-privacy-deny.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# PLAN-0010: Write-scope enforcement and privacy deny list

## Scope

SPEC-0010 の mechanism を実装する。`cheaper.writeScope` frontmatter field、
built-in + policy `filesystem.denyPaths`、tool 層での強制。

## File map

- `packages/core/src/workspace/deny-paths.ts` (new) — pattern matcher + defaults
- `packages/core/src/worker/validate.ts` — `cheaper.writeScope` 受理
- `packages/core/src/policy/schema.ts` — `filesystem.denyPaths`
- `packages/core/src/index.ts` — export
- `packages/cli/policy.yaml` — `denyPaths: []`
- `packages/runtime-pi/src/run.ts` — scope 抽出・plumb
- `packages/runtime-pi/src/tools/operations.ts` — deny + scope check
- `packages/runtime-pi/src/tools/walker.ts` — deny filter + denied message
- `packages/runtime-pi/src/tools/registry.ts` — opts plumbing
- `packages/runtime-pi/src/tools/count-tokens.ts` — deny check
- `packages/cli/workers/{code-writer,doc-writer}.md` — `writeScope: targets`
- 各 test

## Tasks

1. TASK-0079 core: matcher・frontmatter・policy schema・exports・unit test
2. TASK-0080 runtime-pi: operations/walker/registry/run への強制・unit test
3. TASK-0081 workers + e2e: frontmatter 宣言、fake-stream e2e、検証・index

## Risks

- mkdir の祖先許可を誤ると write tool が常に失敗する → e2e で create 系を
  必ず検証する
- deny filter が通常 file に誤爆しないよう pattern は unit test で網羅

## Rollback

`writeScope`/deny ともに additive。worker frontmatter の 1 行除去で
scope 強制は無効化できる。denyPaths は built-in default のみ残る。

## Verification

- core/runtime-pi/cli tests + `pnpm -r build` + `pnpm lint` + `pnpm typecheck`
- fake-stream e2e: targets 外 write が tool error + rejectedToolCalls 計上
- e2e: `.env` read が denied になり walk 結果に出ない
