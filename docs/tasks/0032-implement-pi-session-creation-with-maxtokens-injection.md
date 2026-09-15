---
id: "TASK-0032"
type: "task"
status: "done"
title: "Implement Pi session creation with maxTokens injection"
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
    - "docs/tasks/0029-implement-tool-registry-mapping-vocabulary-to-pi-tools.md"
    - "docs/tasks/0030-implement-cheaperresourceloader-without-discovery.md"
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

# Implement Pi session creation with maxTokens injection

## Work

PLAN-0002 step 11。`createAgentSession` 設定（DESIGN-0004 の呼び出し形: cwd=workspace.real, thinkingLevel off, noTools builtin, tools allowlist, customTools, CheaperResourceLoader, inMemory SessionManager/SettingsManager, retry/compaction 設定）+ `session.agent.streamFunction` wrap で `options.maxTokens = min(policy.output.maxTokens, model.maxTokens)` を注入。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md`
- `docs/designs/0004-pi-execution-engine.md`

## Dependencies

- `docs/tasks/0029-implement-tool-registry-mapping-vocabulary-to-pi-tools.md` — 依存先（Dependency Graph）
- `docs/tasks/0030-implement-cheaperresourceloader-without-discovery.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/runtime-pi/src/session.ts`
- `packages/runtime-pi/test/session.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] 作成された session の tool 登録・settings・streamFunction wrap のテストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi test -- test/session.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- systemPrompt 文面は run（TASK-0037）側で Worker body + Output contract を組み立てる。
