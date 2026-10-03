---
id: "TASK-034zBJ55ACvdlADYOtaKKc"
type: "task"
status: "done"
title: "Add same-turn batching guidance to buildSystemPrompt and regenerate bundle"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "packages/runtime-pi/test/run-prompt.test.ts"
    modified:
      - type: "file"
        path: "packages/runtime-pi/src/run.ts"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "packages/apm/.apm/hooks/cli/bin/cheaper.mjs"
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

# Add same-turn batching guidance to buildSystemPrompt and regenerate bundle

## Work

plan `docs/plans/implement-pi-sibling-tool-call-parallelism.md`（Phase B、
T6–T13）。`buildSystemPrompt` の `## Output contract` に same-turn
batching ガイダンス 5 bullet を追加し、bundled CLI を再生成する。
TDD 順序: export → test 作成 → red 確認 → 実装 → green → bundle 再生成。

### 1. `buildSystemPrompt` の export 化（T6–T7）

`packages/runtime-pi/src/run.ts` の `buildSystemPrompt`（run.ts:111）を
`export function` に変更する（signature・挙動は不変）。変更後に
`mise exec -- pnpm --filter @cheaper/runtime-pi build`（`tsc -b`）で
typecheck が通ることを確認する。

### 2. `packages/runtime-pi/test/run-prompt.test.ts`（新規、T8–T9）

`WorkerDefinition` fixture（`buildSystemPrompt` が読むのは
`worker.instructions` と `worker.outputSchema.schema` のみだが、型完備の
fixture は `test/run.test.ts` の `worker()` helper と同形 —
`compileSchema` 使用 — で構築する）で `buildSystemPrompt` を
`../src/run.js` から直接呼び（`session.systemPrompt` 経由ではなく
function 単位）、以下を assert する:

- (a) `toolNames` 非空のとき、same-turn 発行（"same assistant turn"）、
  serial 化禁止、polling 禁止 + timeout/lifecycle の runtime 管理、
  unsafe mutation（副作用依存 read↔mutation pair と同一対象への複数
  mutation を含む）、output token limit（cap 超過で全 sibling 未実行）、
  contract 不変 の各 key phrase が出力に含まれること
- (b) `toolNames: []` のとき batching bullet が一切 emit されず
  `"Available tools: none"` と batching 指示が同居しないこと

test 作成後に `mise exec -- pnpm --filter @cheaper/runtime-pi exec vitest run test/run-prompt.test.ts`
を実行し、guidance 含有 assert が fail（red）することを確認する
（`toolNames: []` assert は現行でも pass し得る）。

### 3. guidance bullet 追加（T10–T11）

`run.ts` の `buildSystemPrompt` に、DESIGN-034zAKcWuB8SuFkOePsubg の
「Data and Control Flow → runtime 指示拡張」に定義された 5 本の英文
bullet を **verbatim** で追加する。挿入位置は `- Available tools:`
bullet の後・`"Output schema:"` 行の前で、`...(toolNames.length > 0
? [...] : [])` による条件 emit とする（design の code snippet をその
まま転記。文言の再起草はしない）。追加後に
`mise exec -- pnpm --filter @cheaper/runtime-pi test` を実行し、
`run-prompt.test.ts` が green になり、runtime-pi suite 全体（既存
envelope/contract test 含む）が無変更で pass することを確認する。

### 4. bundle 再生成（T12）

`mise exec -- pnpm typecheck`（root の `tsc -b` で全 package の dist を
再生成）と `mise exec -- pnpm bundle`（`node scripts/bundle-cli.mjs`）
を実行して bundled CLI を再生成し、
`packages/apm/.apm/hooks/cli/bin/cheaper.mjs` の diff に新 guidance
bullet が含まれることを確認する（`mise run check` の
`git diff --exit-code -- packages/apm/.apm/hooks/cli` freshness gate
は regenerate 済みの bundle を要求する）。

### 5. lint 確認 + commit（T13）

`mise exec -- pnpm exec biome check packages/runtime-pi` が clean で
あることを確認し、Phase B の変更（export + 5 bullet +
run-prompt.test.ts + 再生成 `cheaper.mjs`）を 1 論理変更として
commit する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-pi-sibling-tool-call-parallelism.md`
- `docs/designs/pi-sibling-tool-call-parallelism-inside-workers.md`
  （5 本の英文 bullet が verbatim 定義済み。文言の再起草はしない）
- `docs/adr/pi-sibling-tool-call-parallelism-via-runtime-output-contract-guidance.md`
  （Option A = runtime output-contract guidance の決定）

## Dependencies

- なし（`buildSystemPrompt` の export は package 内部の additive 変更。
  Phase A / Phase C の task と並行可能）

## Affected Files

- `packages/runtime-pi/src/run.ts` — `buildSystemPrompt` の export 化 +
  `## Output contract` への 5 bullet 追加
- `packages/runtime-pi/test/run-prompt.test.ts`（新規）
- `packages/apm/.apm/hooks/cli/bin/cheaper.mjs` — **generated**（prompt
  文言変更が bundled CLI に inline されるため再生成して commit に含める）

## Done When

- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi test` が exit 0
      （新 test green + 既存 suite 無変更 pass）
- [x] `packages/apm/.apm/hooks/cli/bin/cheaper.mjs` が新 guidance を
      含んで再生成され、`git diff --exit-code -- packages/apm/.apm/hooks/cli`
      が exit 0（regen 差分が commit 済みで working tree に残らない）
- [x] Phase B の変更（export + 5 bullet + test + 再生成 bundle）が
      1 つの論理変更として commit されている

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi exec vitest run test/run-prompt.test.ts` exits 0
- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi test` exits 0
- [x] `git diff --exit-code -- packages/apm/.apm/hooks/cli` exits 0
- [x] `mise exec -- pnpm exec biome check packages/runtime-pi` exits 0
