---
id: "TASK-0031"
type: "task"
status: "done"
title: "Add scripted fake stream test fixture"
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

# Add scripted fake stream test fixture

## Work

PLAN-0002 step 10。scripted fake stream: `StreamFn` として `AssistantMessageEventStream` を返し、スクリプト化した AssistantMessage 列（text のみ / toolcall 含む / 継続待ち）を emit。`options.signal` の abort を honor し `{type:"error", reason:"aborted"}` で終了する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md`
- `docs/designs/0004-pi-execution-engine.md`

## Dependencies

- `docs/tasks/0022-scaffold-runtime-pi-package-with-pi-import-smoke-test.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/runtime-pi/test/fixtures/fake-stream.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] fake stream が script どおりイベント列を返し signal abort で終了するテストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi test -- test/fixtures` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- run への組み込みは TASK-0037。
