---
id: "TASK-034r05iHK8bA7hco9mHiho"
type: "task"
status: "done"
title: "Implement CheaperResourceLoader without discovery"
created: "2026-09-15"
updated: "2026-09-15"
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
    - "docs/plans/implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
    - "docs/tasks/scaffold-runtime-pi-package-with-pi-import-smoke-test.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement CheaperResourceLoader without discovery

## Work

PLAN-034r05iHy2gdnpIrX0mTIb step 9。`CheaperResourceLoader`: 11 メソッド全実装。extensions/skills/prompts/themes/agentsFiles は空、`getExtensions` は `{extensions: [], errors: [], runtime: createExtensionRuntime()}`、systemPrompt は注入された固定文字列、`getSystemPromptSource()` は undefined、`getAppendSystemPromptSources()` は []、`extendResources`/`reload` は no-op。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md`
- `docs/designs/pi-execution-engine.md`

## Dependencies

- `docs/tasks/scaffold-runtime-pi-package-with-pi-import-smoke-test.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/runtime-pi/src/resource-loader.ts`
- `packages/runtime-pi/test/resource-loader.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] Pi の ResourceLoader 型を満たし discovery を行わないことのテストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi test -- test/resource-loader.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- systemPrompt の内容構成は TASK-034r05iHK8bA7he1WXwcBs/0037。
