---
id: "TASK-034zBIYG4bskj0L6ueomFK"
type: "task"
status: "done"
title: "Add deterministic sibling-parallelism pin tests to runtime-pi"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "packages/runtime-pi/test/sibling-parallelism.test.ts"
      - type: "file"
        path: "packages/runtime-pi/test/no-execution-mode.test.ts"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-pi-sibling-tool-call-parallelism.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-pi-sibling-tool-call-parallelism.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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

# Add deterministic sibling-parallelism pin tests to runtime-pi

## Work

plan `docs/plans/implement-pi-sibling-tool-call-parallelism.md`（Phase A、
T1–T5）。pinned `pi-agent-core@0.85.1` が既に持つ sibling tool-call 並列
実行 semantics を決定的 test で contract 化する。新規並列実行機構の実装
はなく、Pi の agent loop をそのまま使うため **初回実行で green を期待する
regression 検出器**（TDD の red 対象ではない）。fail した場合は
discovery の前提が崩れた可能性があるため、実装を止めて design owner に
escalate する。

### 1. `packages/runtime-pi/test/sibling-parallelism.test.ts`（新規）

`test/session.test.ts` と同じ構成（`mkdtemp` + `realpath` workspace、
`ModelRuntime.create({ refreshOnCreate: false })` +
`runtime.registerNativeProvider(fake.provider)`、`createWorkerSession`
+ `session.prompt()`、`session.subscribe` / `session.messages` で観測）
で以下の 4 test を実装する。instrumented tool は `defineTool`
（`@earendil-works/pi-coding-agent`）+ `Type.Object({})`（`typebox`）
で自作し、`createSandboxedTools` ではなく `customTools` / `toolNames`
option で注入する。`ToolDefinition` の必須 field（`name` / `label` /
`description` / `parameters` / `execute`）を全て揃え、`toolNames`
option は allowlist として `["probe_1","probe_2","probe_3"]`（test 4
では `["write","edit"]`）を含めること。

1. **bounded-gate 並列実行（N=3）**: 3 個の instrumented
   `ToolDefinition`（`probe_1`..`probe_3`）。各 `execute` は「開始
   marker 記録 → 共有 gate で全 N 個の開始が揃うまで待機（約 1s の
   `Promise.race` deadline で reject）→ 成功 result を返す」。
   `parameters: Type.Object({})` は必須 — schema 不適合は
   `kind: "immediate"`（execute 未呼出）になり「直列実行」と誤診
   する。fake stream は `[fauxToolCall("probe_1",{}),
   fauxToolCall("probe_2",{}), fauxToolCall("probe_3",{})]` を持つ
   1 assistant message（`stopReason: "toolUse"`）→ 終了 text
   message を emit。`it(..., { timeout: 10_000 })` で N×deadline
   超を明示し assertion 由来の失敗が timeout に潰れないようにする。
   補助 assert: 全 `tool_execution_start` が最初の
   `tool_execution_end` より前に揃う（事前に marker 数 === N で全
   call が `kind: "prepared"` に到達したことを確認し、immediate
   発生は fixture 不備として区別）。「全 N 個同時 in-flight」要求は
   Pi の unbounded `Promise.all` を pin する意図的な厳格基準であり、
   将来 Pi が batch 並行度 cap を導入した場合は「少なくとも 2 個の
   overlap」への緩和を要する旨を test コメントに明記する
2. **result 順序（reverse-completion gate）**: sleep ではなく決定的
   gate で完了順を逆転させる — `probe_i`（i < N）の `execute` は
   `probe_{i+1}` の完了 marker を await し、`probe_N` は即成功
   （完了順が確定的に probe_N…probe_1、timer 不使用）。gate の
   await にも test 1 と同じ約 1s の `Promise.race` deadline を持た
   せ、`it(..., { timeout })` を明示する — 直列 regression 時に
   `probe_1` が unreachable marker を待ち続けて `session.prompt` が
   hang し vitest 既定 5s timeout まで掛かるのを防ぐ。
   `session.messages` 内の `role === "toolResult"` message（`pi-ai`
   の message role は camelCase `"toolResult"`。`"tool_result"` は
   extension event 名で混同しない）並びが元の call 順
   （`toolCallId`/`toolName` 対応）であることを assert。
   `tool_execution_end` event は完了順 emit のため assert 対象は
   `session.messages` のみ
3. **`stopReason: "length"`**: sibling batch を持つ assistant
   message を `{ stopReason: "length" }` で emit し、続けて終了
   text message。全 instrumented tool の `execute` が一度も呼ばれ
   ないことと、全 tool result（`role === "toolResult"` message /
   `tool_execution_end` の `isError`）が error であることを assert
4. **同一ファイル write→edit 直列化**（`withFileMutationQueue` 継承
   pin）: `createSandboxedTools({ names: ["write","edit"], root,
   symlinkPolicy, journal, maxToolCalls, onLimit })` の実 tool を使い、
   sibling batch `[write("f.txt","hello"),
   edit("f.txt",{edits:[{oldText:"hello",newText:"world"}]})]` を
   emit。両 call 成功 + 最終内容 `"world"` を assert。**検出の非対称
   性を test コメントに明記する**: PASS 経路は決定的だが、queue
   regression 時は write の `writeFile` が edit の `access` より先に
   完了すれば false-PASS し得る（regression 検出側は timing 依存）。
   強化選択肢として `createWriteOperations`/`createEditOperations`
   seam 経由の op 呼出順記録（`write.writeFile` 完了が `edit.access`
   開始に先行することの assert）をコメントに併記する

### 2. `packages/runtime-pi/test/no-execution-mode.test.ts`（新規）

`packages/runtime-pi/src/**/*.ts` を glob 走査し、comment（`//`・
`/* */`）と string literal を strip した上で `toolExecution` /
`executionMode` identifier が出現しないことを assert する（簡易
strip、または property-assignment pattern `toolExecution\s*:` /
`executionMode\s*:` で十分）。`src/testsupport.ts` も走査対象。
「identifier の完全な不存在」を要求するのは #57 余地確保より強い
意図的な厳格さであり、将来 legitimate な sequential tool を追加する
場合は test の緩和を要する旨をコメントに明記する。現状 src に設定が
無いため初回 green 期待。

### 3. lint 確認 + commit

`mise exec -- pnpm exec biome check packages/runtime-pi` が clean で
あることを確認し、Phase A の 2 test file を deterministic pin 追加の
1 論理変更として commit する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-pi-sibling-tool-call-parallelism.md`
- `docs/designs/pi-sibling-tool-call-parallelism-inside-workers.md`
  （4 test の設計が verbatim 定義済み。再解釈しない）
- `docs/specs/verify-and-leverage-pi-sibling-tool-call-parallelism-inside-workers.md`

## Dependencies

- なし（`streamFn`・`customTools`・`createFakePi`・
  `fauxAssistantMessage`/`fauxToolCall`・`createSandboxedTools` 等の
  seam は全て既存。新規依存 package の追加なし）
- plan の並行可能 stream どおり `run.ts` を触らないため、Phase B の
  task（`docs/tasks/add-same-turn-batching-guidance-to-buildsystemprompt-and-regenerate-bundle.md`）
  と並行可能

## Affected Files

- `packages/runtime-pi/test/sibling-parallelism.test.ts`（新規）
- `packages/runtime-pi/test/no-execution-mode.test.ts`（新規）
- 変更なし（既存 seam を再利用）:
  `packages/runtime-pi/src/session.ts`、
  `packages/runtime-pi/src/tools/registry.ts`、
  `packages/runtime-pi/src/testsupport.ts`、
  `packages/runtime-pi/test/fixtures/fake-stream.ts`

## Done When

- [x] `packages/runtime-pi/test/sibling-parallelism.test.ts` と
      `packages/runtime-pi/test/no-execution-mode.test.ts` の両方が
      存在する
- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi exec vitest run test/sibling-parallelism.test.ts test/no-execution-mode.test.ts`
      が exit 0（sibling-parallelism の 4 test 全 pass、初回 green）
- [x] `mise exec -- pnpm exec biome check packages/runtime-pi` が clean
- [ ] 2 test file が 1 つの論理変更として commit されている

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi exec vitest run test/sibling-parallelism.test.ts test/no-execution-mode.test.ts` exits 0
- [x] `mise exec -- pnpm exec biome check packages/runtime-pi` exits 0
