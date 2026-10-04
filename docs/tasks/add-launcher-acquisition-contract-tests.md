---
id: "TASK-034zQTNEJMgP0EgzXJbGtM"
type: "task"
status: "done"
title: "Add launcher acquisition contract tests"
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

# Launcher acquisition contract tests
## Work
1. temp環境に全user cache変数を隔離するHTTP stub/PEまたはsh fixtureを作成する。
2. sibling/hit/miss、各fail-closed、concurrency、hook stream/argv/fallbackのtestsを追加する。
3. focused commandで未実装acquisitionを原因とするredを記録する。
## Classification
normal-plan-task
## Files
packages/cli/test/e2e/launcher-acquisition.test.ts と同e2e fixture。
## Done When
harnessが動作し、現実装に対してcache miss/hit契約がredであることを確認する。
## Verification
- [x] mise exec -- pnpm exec vitest run packages/cli/test/e2e/launcher-acquisition.test.ts の失敗がacquisition未実装を示す（exit1を期待）。
## Scope
launcherの実装は後続task。依存追加無し。
