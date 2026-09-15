---
id: "TASK-0030"
type: "task"
status: "todo"
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
    - "docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
    - "docs/tasks/0022-scaffold-runtime-pi-package-with-pi-import-smoke-test.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement CheaperResourceLoader without discovery

## Work

PLAN-0002 step 9。`CheaperResourceLoader`: 11 メソッド全実装。extensions/skills/prompts/themes/agentsFiles は空、`getExtensions` は `{extensions: [], errors: [], runtime: createExtensionRuntime()}`、systemPrompt は注入された固定文字列、`getSystemPromptSource()` は undefined、`getAppendSystemPromptSources()` は []、`extendResources`/`reload` は no-op。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md`
- `docs/designs/0004-pi-execution-engine.md`

## Dependencies

- `docs/tasks/0022-scaffold-runtime-pi-package-with-pi-import-smoke-test.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/runtime-pi/src/resource-loader.ts`
- `packages/runtime-pi/test/resource-loader.test.ts`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] Pi の ResourceLoader 型を満たし discovery を行わないことのテストが green

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/runtime-pi test -- test/resource-loader.test.ts` exits 0
- [ ] `mise exec -- pnpm biome check .` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- systemPrompt の内容構成は TASK-0032/0037。
