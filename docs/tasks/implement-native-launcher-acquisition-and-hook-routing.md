---
id: "TASK-034zQTSDQA908MNdv5Qe8v"
type: "task"
status: "in-progress"
title: "Implement native launcher acquisition and hook routing"
created: "2026-10-04"
updated: "2026-10-04"
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
    - "../plans/implement-version-pinned-launcher-acquisition.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-version-pinned-launcher-acquisition.md"
    - "../tasks/add-launcher-acquisition-contract-tests.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/version-pinned-launcher-integrity-and-stream-preservation.md"
  references: []
  defers: []
  deferred-by: []
---

# Native launcher acquisition and hook routing
## Work
1. sh/pwshのsibling優先、固定version/cache pair/manifest/hash/probe/lock/atomic/cleanupを実装する。
2. hooks.jsonを子PowerShell launcher経由に統一し既存fallbackを保全する。
3. focused testsをgreenにしstdin/argv/exitとfail closedを確認する。
## Classification
normal-plan-task
## Files
packages/apm/.apm/hooks/cheaper、cheaper.ps1、hooks.json と新規e2eの修正。
## Done When
全対象runnerのfocused testsがpass。cacheはhook dir外、latest無し、通常失敗partial無し。
## Verification
- [ ] mise exec -- pnpm exec vitest run packages/cli/test/e2e/launcher-acquisition.test.ts exits 0。
## Scope
LFS payload/生成物/workflow/scripts/hot fileを変更しない。
