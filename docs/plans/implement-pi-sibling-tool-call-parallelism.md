---
id: "PLAN-034zBjFcH4nDUr72Vc1XJ3"
type: "plan"
status: "completed"
title: "Implement Pi sibling tool-call parallelism guidance and deterministic tests in runtime-pi"
test-spec-skip: "Behavior guarantees are pinned by vitest files inside the implementation tasks (sibling-parallelism.test.ts / run-prompt.test.ts / no-execution-mode.test.ts); the approved design's Verification Notes already define the guarantees, so no separate test-spec artifact is needed."
created: "2026-10-04"
updated: "2026-10-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/55"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "packages/apm/.apm/hooks/cli/bin/cheaper.mjs"
  implements:
    - "docs/specs/verify-and-leverage-pi-sibling-tool-call-parallelism-inside-workers.md"
  implemented-by:
    - "../tasks/add-deterministic-sibling-parallelism-pin-tests-to-runtime-pi.md"
    - "../tasks/add-same-turn-batching-guidance-to-buildsystemprompt-and-regenerate-bundle.md"
    - "../tasks/append-sibling-tool-call-parallelism-section-to-execution-engine-design-doc.md"
    - "../tasks/run-canonical-verification-and-close-out-issue-55-implementation.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/verify-and-leverage-pi-sibling-tool-call-parallelism-inside-workers.md"
    - "docs/designs/pi-sibling-tool-call-parallelism-inside-workers.md"
    - "docs/adr/pi-sibling-tool-call-parallelism-via-runtime-output-contract-guidance.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement Pi sibling tool-call parallelism guidance and deterministic tests in runtime-pi Implementation Plan

## Goal

issue #55 / SPEC-034zA513QKarQrr5SQtgp7 / DESIGN-034zAKcWuB8SuFkOePsubg の
実装: pinned `pi-agent-core@0.85.1` が既に持つ sibling tool-call 並列実行
semantics を決定的 test で contract 化し、`buildSystemPrompt` の
`## Output contract` に same-turn batching ガイダンスを追加して、
execution-engine 文書へ並列性の記述を追記する。

## Scope

`packages/runtime-pi` 内の 1 source file 変更 + 3 test file 追加と
`docs/designs/pi-execution-engine.md` への 1 セクション追記のみの
単一 implementation stream。新規並列実行機構は実装しない（Pi の
agent loop をそのまま使う）。分割は不要。

Non-goals（design の Out of Scope と一致）:

- 独自 agent loop・独自並列 executor の実装
- mutation tool への `executionMode: "sequential"` 付与（batch 全体の
  直列化で効果源を潰すため不採用）
- per-worker markdown（`packages/cli/workers/*.md`）へのガイダンス複写
  （表現ドリフト防止）
- CLI / stdout envelope contract の変更
- #57 ToolScheduler・#62 Workflow Engine の実装（導入余地を閉じない
  確認に留める）
- 実 model のガイダンス遵守度の実測（非決定的で AC 対象外）

## Upstream Documents

- `docs/specs/verify-and-leverage-pi-sibling-tool-call-parallelism-inside-workers.md`
  （SPEC-034zA513QKarQrr5SQtgp7、status: approved）
- `docs/adr/pi-sibling-tool-call-parallelism-via-runtime-output-contract-guidance.md`
  （ADR-034zA5nZuUdGLDMTGh2V6z、status: accepted。Option A = runtime
  output-contract guidance の決定）
- issue #55（trigger）:
  https://github.com/scarecrow173/cheaper/issues/55

## Design Inputs

- `docs/designs/pi-sibling-tool-call-parallelism-inside-workers.md`
  （DESIGN-034zAKcWuB8SuFkOePsubg、status: approved。追加する英文
  bullet の verbatim 文言・4 つの決定的 test 設計・文書化内容が定義
  済み。本 plan はこの文言と test 設計を再解釈せずそのまま実装する）

## File Map

- `packages/runtime-pi/src/run.ts` — `buildSystemPrompt`（run.ts:111-125）
  を `export` 化し、`## Output contract` の `- Available tools:` bullet
  の後・`"Output schema:"` 行の前に 5 本の batching bullet を追加
  （`toolNames.length > 0` のときのみ emit。文言は design の code
  snippet を verbatim 転記）
- `packages/runtime-pi/test/sibling-parallelism.test.ts` — 新規。sibling
  並列 semantics の決定的 test 4 本（bounded-gate 並列実行・call 順
  result emit・`stopReason: "length"` 全員未実行 error・同一ファイル
  write→edit の `withFileMutationQueue` 直列化 pin）
- `packages/runtime-pi/test/run-prompt.test.ts` — 新規。export された
  `buildSystemPrompt` を function 単位で呼び、guidance key phrase の
  含有と `toolNames: []` での bullet 非出力を assert
- `packages/runtime-pi/test/no-execution-mode.test.ts` — 新規。
  `src/**/*.ts` に `toolExecution` / `executionMode` が存在しない
  ことを機械的に検査（comment/string literal strip 済み identifier
  一致）
- `docs/designs/pi-execution-engine.md` — 末尾に `## Sibling tool-call
  parallelism` セクション追記（attribution + `updated` frontmatter
  更新付き）
- `packages/apm/.apm/hooks/cli/bin/cheaper.mjs` — **generated**。
  `run.ts` の prompt 文言変更は bundled CLI に inline されるため、
  Phase B で bundle を再生成し commit に含める（`mise run check` の
  `git diff --exit-code -- packages/apm/.apm/hooks/cli` が
  regenerate 差分で fail するのを防ぐ）
- 変更なし（既存 seam を再利用）: `packages/runtime-pi/src/session.ts`
  （`WorkerSessionOptions.streamFn` / `customTools` / `toolNames`）、
  `packages/runtime-pi/src/tools/registry.ts`（`createSandboxedTools`、
  write/edit は `withFileMutationQueue` 継承）、
  `packages/runtime-pi/src/testsupport.ts` / `test/fixtures/fake-stream.ts`
  （`createFakePi` / `fauxAssistantMessage` / `fauxToolCall` re-export）

## Dependency Graph

| Step | Depends on | Blocks |
| ---- | ---------- | ------ |
| T1 sibling-parallelism.test.ts 作成 | — | T2 |
| T2 sibling test 実行・green 確認 | T1 | T5, T15 |
| T3 no-execution-mode.test.ts 作成 | — | T4 |
| T4 inspection test 実行・green 確認 | T3 | T5, T15 |
| T5 Phase A commit | T2, T4 | — |
| T6 buildSystemPrompt export 化 | — | T7 |
| T7 typecheck | T6 | T8 |
| T8 run-prompt.test.ts 作成 | T7 | T9 |
| T9 prompt test red 確認 | T8 | T10 |
| T10 guidance bullet 追加 | T9 | T11 |
| T11 runtime-pi suite green 確認 | T10 | T12, T15 |
| T12 bundle 再生成・diff 確認 | T11 | T13 |
| T13 Phase B commit | T12 | — |
| T14 execution-engine 文書追記 | — | T15 |
| T15 `mise run check` | T2, T4, T13, T14 | T16 |
| T16 Phase C commit | T15 | — |

Critical path: T6→T7→T8→T9→T10→T11→T12→T15。

並行可能 stream: Phase A（T1–T5）は `run.ts` を触らないため Phase B
（T6–T13）と並行可能。Phase C（T14）も独立。

TDD ordering 注記: Phase A の 2 test は Pi 既存挙動の pin であり初回
実行で green を期待する（red 確認ではなく regression 検出器）。prompt
test（T8–T9）は T6 の `buildSystemPrompt` export なしには import でき
ないため、red 確認は export 後に行う。test 作成は guidance 実装
（T10）より先で TDD 順序は維持される。

## Implementation Sequence

### Phase A: 決定的 pin test（Pi 既存挙動の固定。初回 green 期待）

- [ ] T1. `packages/runtime-pi/test/sibling-parallelism.test.ts` を新規作成し、
      `session.test.ts` と同じ構成（`mkdtemp` + `realpath` workspace、
      `ModelRuntime.create({ refreshOnCreate: false })` +
      `runtime.registerNativeProvider(fake.provider)`、
      `createWorkerSession` + `session.prompt()`、`session.subscribe` /
      `session.messages` での観測）で以下の 4 test を実装する。
      instrumented tool は `defineTool`（`@earendil-works/pi-coding-agent`）
      + `Type.Object({})`（`typebox`）で自作し、`createSandboxedTools`
      ではなく `customTools`/`toolNames` option で注入する。
      `ToolDefinition` の必須 field（`name` / `label` / `description` /
      `parameters` / `execute`）を全て揃え、`toolNames` option は
      allowlist として機能するため `["probe_1","probe_2","probe_3"]`
      （test 4 では `["write","edit"]`）を含めること:
      1. **bounded-gate 並列実行（N=3）**: 3 個の instrumented
         `ToolDefinition`（`probe_1`..`probe_3`）を作る。各 `execute` は
         「開始 marker 記録 → 共有 gate で全 N 個の開始が揃うまで待機
         （約 1s の `Promise.race` deadline で reject）→ 成功 result を
         返す」。`parameters: Type.Object({})` は必須 — schema 不適合は
         `kind: "immediate"`（execute 未呼出）になり「直列実行」と
         誤診する。fake stream は `[fauxToolCall("probe_1",{}),
         fauxToolCall("probe_2",{}), fauxToolCall("probe_3",{})]` を持つ
         1 assistant message（`stopReason: "toolUse"`）→ 終了 text
         message を emit。`it(..., { timeout: 10_000 })` で N×deadline
         超を明示し assertion 由来の失敗が timeout に潰れないようにする。
         補助 assert: 全 `tool_execution_start` が最初の
         `tool_execution_end` より前に揃う（事前に marker 数 === N で
         全 call が `kind: "prepared"` に到達したことを確認し、
         immediate 発生は fixture 不備として区別）。「全 N 個同時
         in-flight」要求は Pi の unbounded `Promise.all` を pin する
         意図的な厳格基準であり、将来 Pi が batch 並行度 cap を導入した
         場合は「少なくとも 2 個の overlap」への緩和を要する旨を test
         コメントに明記する
      2. **result 順序（reverse-completion gate）**: sleep ではなく
         決定的 gate で完了順を逆転させる — `probe_i`（i < N）の
         `execute` は `probe_{i+1}` の完了 marker を await し、
         `probe_N` は即成功（完了順が確定的に probe_N…probe_1、
         timer 不使用）。gate の await にも test 1 と同じ約 1s の
         `Promise.race` deadline を持たせ、`it(..., { timeout })` を
         明示する — 直列 regression 時に `probe_1` が unreachable
         marker を待ち続けて session.prompt が hang し vitest 既定
         5s timeout まで掛かるのを防ぐ。`session.messages` 内の
         `role === "toolResult"` message（`pi-ai` の message role は
         camelCase `"toolResult"`。`"tool_result"` は extension event
         名で混同しない）並びが元の call 順（`toolCallId`/`toolName`
         対応）であることを assert する。`tool_execution_end` event
         は完了順 emit のため assert 対象は `session.messages` のみ
      3. **`stopReason: "length"`**: sibling batch を持つ assistant
         message を `{ stopReason: "length" }` で emit し、続けて終了
         text message。全 instrumented tool の `execute` が一度も
         呼ばれないことと、全 tool result（`role === "toolResult"`
         message / `tool_execution_end` の `isError`）が error で
         あることを assert する
      4. **同一ファイル write→edit 直列化**（`withFileMutationQueue`
         継承 pin）: `createSandboxedTools({ names: ["write","edit"],
         root, symlinkPolicy, journal, maxToolCalls, onLimit })` の実
         tool を使い、sibling batch `[write("f.txt","hello"),
         edit("f.txt",{edits:[{oldText:"hello",newText:"world"}]})]` を
         emit。両 call 成功 + 最終内容 `"world"` を assert する。
         **検出の非対称性を test コメントに明記する**: PASS 経路は
         決定的だが、queue regression 時は write の `writeFile` が
         edit の `access` より先に完了すれば false-PASS し得る
         （regression 検出側は timing 依存）。強化選択肢として
         `createWriteOperations`/`createEditOperations` seam 経由の
         op 呼出順記録（`write.writeFile` 完了が `edit.access` 開始に
         先行することの assert）をコメントに併記する
- [ ] T2. `mise exec -- pnpm --filter @cheaper/runtime-pi exec vitest run test/sibling-parallelism.test.ts`
      を実行し 4 test 全 pass を確認する。**初回 green を期待**（Pi 既存
      挙動の pin であり TDD の red 対象ではない）。fail した場合は
      discovery の前提が崩れた可能性があるため実装を止めて design
      owner に escalate する
- [ ] T3. `packages/runtime-pi/test/no-execution-mode.test.ts` を新規作成する。
      `packages/runtime-pi/src/**/*.ts` を glob 走査し、comment
      （`//`・`/* */`）と string literal を strip した上で
      `toolExecution` / `executionMode` identifier が出現しないことを
      assert する（簡易 strip、または property-assignment pattern
      `toolExecution\s*:|executionMode\s*:` で十分）。`src/testsupport.ts`
      も走査対象。「identifier の完全な不存在」を要求するのは #57 余地
      確保より強い意図的な厳格さであり、将来 legitimate な sequential
      tool を追加する場合は test の緩和を要する旨をコメントに明記する
- [ ] T4. `mise exec -- pnpm --filter @cheaper/runtime-pi exec vitest run test/no-execution-mode.test.ts`
      を実行し pass を確認する（現状 src に設定が無いため初回 green
      期待）
- [ ] T5. `mise exec -- pnpm exec biome check packages/runtime-pi` で
      lint/format が clean であることを確認し、Phase A の 2 test file を
      commit する（deterministic pin 追加の 1 論理変更）

### Phase B: prompt guidance（TDD: export → test → red → 実装 → green → bundle 再生成）

- [ ] T6. `packages/runtime-pi/src/run.ts` の `buildSystemPrompt`
      （run.ts:111）を `export function` に変更する（signature・挙動は
      不変）
- [ ] T7. `mise exec -- pnpm --filter @cheaper/runtime-pi build`
      （`tsc -b`）で typecheck が通ることを確認する
- [ ] T8. `packages/runtime-pi/test/run-prompt.test.ts` を新規作成する。
      `WorkerDefinition` fixture（`buildSystemPrompt` が読むのは
      `worker.instructions` と `worker.outputSchema.schema` のみだが、
      型完備の fixture は `test/run.test.ts` の `worker()` helper と
      同形 — `compileSchema` 使用 — で構築する）で `buildSystemPrompt` を
      `../src/run.js` から直接呼び（`session.systemPrompt` 経由ではなく
      function 単位）、以下を assert する:
      (a) `toolNames` 非空のとき、same-turn 発行（"same assistant
      turn"）、serial 化禁止、polling 禁止 + timeout/lifecycle の
      runtime 管理、unsafe mutation（副作用依存 read↔mutation pair と
      同一対象への複数 mutation を含む）、output token limit（cap
      超過で全 sibling 未実行）、contract 不変 の各 key phrase が出力
      に含まれること
      (b) `toolNames: []` のとき batching bullet が一切 emit されず
      `"Available tools: none"` と batching 指示が同居しないこと
- [ ] T9. `mise exec -- pnpm --filter @cheaper/runtime-pi exec vitest run test/run-prompt.test.ts`
      を実行し、guidance 含有 assert が fail（red）することを確認する
      （`toolNames: []` assert は現行でも pass し得る）
- [ ] T10. `run.ts` の `buildSystemPrompt` に、DESIGN-034zAKcWuB8SuFkOePsubg
      の「Data and Control Flow → runtime 指示拡張」に定義された 5 本
      の英文 bullet を **verbatim** で追加する。挿入位置は
      `- Available tools:` bullet の後・`"Output schema:"` 行の前で、
      `...(toolNames.length > 0 ? [...] : [])` による条件 emit とする
      （design の code snippet をそのまま転記。文言の再起草はしない）
- [ ] T11. `mise exec -- pnpm --filter @cheaper/runtime-pi test` を実行し、
      `run-prompt.test.ts` が green になり、runtime-pi suite 全体
      （既存 envelope/contract test 含む）が無変更で pass することを
      確認する
- [ ] T12. `mise exec -- pnpm typecheck`（root の `tsc -b` で全 package
      の dist を再生成）と `mise exec -- pnpm bundle`
      （`node scripts/bundle-cli.mjs`）を実行して bundled CLI を再生成
      し、`packages/apm/.apm/hooks/cli/bin/cheaper.mjs` の diff に新
      guidance bullet が含まれることを確認する（`mise run check` の
      `git diff --exit-code -- packages/apm/.apm/hooks/cli` freshness
      gate は regenerate 済みの bundle を要求する）
- [ ] T13. `mise exec -- pnpm exec biome check packages/runtime-pi` で
      lint/format が clean であることを確認し、Phase B の変更（export +
      5 bullet + run-prompt.test.ts + 再生成 `cheaper.mjs`）を 1 論理
      変更として commit する

### Phase C: 文書化

- [ ] T14. `docs/designs/pi-execution-engine.md` 本文末尾に
      `## Sibling tool-call parallelism` セクションを追記し、front
      matter の `updated` を当日日付に更新する。
      design の文書化 subsection に従い以下を含める:
      pinned `pi-agent-core@0.85.1` の `toolExecution: "parallel"` 既定
      を cheaper はそのまま利用する（`toolExecution`/`executionMode`
      は一切設定しない）、parallel path semantics（`Promise.all`
      overlap 実行、`tool_result` は call 順 emit、`stopReason: "length"`
      で sibling 全員未実行 error、同一ファイル write/edit は
      `withFileMutationQueue` で call 順直列化）、節約源が **Worker
      turn 数と増大 input context の反復送信の削減**であり並列化は
      token 単価を変えない旨、`output.maxTokens` cap との相互作用
      （cap 超過 turn は全 sibling 未実行になるため batch 粒度を cap
      に収める）。approved doc への後追記である旨と、DESIGN-034zAKcWuB8SuFkOePsubg /
      SPEC-034zA513QKarQrr5SQtgp7 / issue #55 への参照を attribution
      として明記する

### Phase D: 全体検証

- [ ] T15. `mise run check` を実行し、全 step（`pnpm lint`、
      `check-version.mjs`、`check-doc-ids.mjs`、`pnpm typecheck`、
      `pnpm test`、bundle freshness の `git diff --exit-code`）が
      exit 0 で pass することを確認する
- [ ] T16. Phase C の doc 追記を commit する

## Task Breakdown

Phase A「決定的 pin test」（T1–T5）、Phase B「prompt guidance」
（T6–T13）、Phase C「文書化」（T14）、Phase D「全体検証」
（T15–T16）の 4 phase・16 step。Phase A は `run.ts` を触らず Phase B
と並行可能、Phase C は任意時点で実施可能。commit は Phase 境界の 3
commit（pins / prompt guidance + 再生成 bundle / doc）を目安とする。

## Dependencies and Risks

### Dependencies

- 実装依存: なし（`streamFn`・`customTools`・`createFakePi` 等の seam は
  全て既存。新規依存 package の追加なし — design の NFC）
- `pi-agent-core@0.85.1` は transitive 依存で pin は lockfile 解決結果に
  依存する

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| - | ---- | ---------- | ------ | ---------- |
| 1 | data-loss: なし。本番 data 変換・migration はなく、file 書込み test は mkdtemp 配下に閉じる | — | — | — |
| 2 | breaking-change: `## Output contract` への 5 bullet 追加で全 worker の system prompt 文言が変わり model 挙動に影響し得る（envelope / 最終 JSON contract / exit code は不変） | med | low | bullet 5 本目が contract 不変を明示。T11/T15 で既存 envelope・contract test が無変更 pass することを確認 |
| 8 | bundle freshness: prompt 文言変更が `cheaper.mjs` に inline されるため、再生成を忘れると `mise run check` の `git diff --exit-code -- packages/apm/.apm/hooks/cli` が fail | med | low | T12 で再生成 + diff 確認を必須化し、Phase B commit（T13）に generated bundle を同梱 |
| 3 | external dependency: `pi-agent-core` / `pi-ai` の version bump で sibling 並列 semantics・faux provider 再現度が drift する可能性 | med | med | sibling-parallelism.test.ts が drift 検出器として機能するよう設計済み（design の fake-stream 忠実度 risk と一致）。Pi bump 時は同 test の再検証で検知 |
| 4 | bounded-gate test の厳格さ: 「全 N 個同時 in-flight」要求は Pi unbounded `Promise.all` を pin。将来 Pi が batch 並行度 cap を導入すると false-fail | low | low | design 指針どおり「少なくとも 2 個の overlap」への緩和条件を test コメントに明記（T1） |
| 5 | write→edit 直列化 test の検出非対称: queue regression 時も timing 次第で false-PASS し得る | low | med | 非対称性を test コメントに明記し、injected ops による op 順序 assert の強化選択肢を併記（T1） |
| 6 | instrumented tool の `kind: "immediate"` 誤診: `parameters` schema 不適合で `execute` 未到達 → 直列実行の誤判定 | low | low | `Type.Object({})` 必須条件を T1・test コメントに明記。補助 assert 前に marker 数 === N で全 call の `prepared` 到達を確認 |
| 7 | ガイダンスの実効性は非決定的: model が same-turn 発行に従わなくても機能誤りではない（turn 削減効果が出ないだけ） | high | low | AC 対象外（design Deferred）。`meta.turns` / `usage.jsonl` run record / `cheaper usage` 集計で後から観測 |

## Verification

| Step | Verification command or criteria | Pass condition |
| ---- | -------------------------------- | -------------- |
| T1–T2 | `mise exec -- pnpm --filter @cheaper/runtime-pi exec vitest run test/sibling-parallelism.test.ts` | exit 0、4 test 全 pass（初回 green） |
| T3–T4 | `mise exec -- pnpm --filter @cheaper/runtime-pi exec vitest run test/no-execution-mode.test.ts` | exit 0（`src/` に `toolExecution`/`executionMode` 不在） |
| T6–T7 | `mise exec -- pnpm --filter @cheaper/runtime-pi build` | `tsc -b` exit 0 |
| T8–T9 | `mise exec -- pnpm --filter @cheaper/runtime-pi exec vitest run test/run-prompt.test.ts` | guidance 含有 assert が fail（red 確認済み。`toolNames: []` assert は pass し得る） |
| T10–T11 | `mise exec -- pnpm --filter @cheaper/runtime-pi test` | exit 0（`vitest run --passWithNoTests`）。新 test green + 既存 suite 無変更 pass |
| T12 | `mise exec -- pnpm typecheck && mise exec -- pnpm bundle` | `cheaper.mjs` の diff に新 guidance bullet が含まれる |
| T14 | manual review: `docs/designs/pi-execution-engine.md` | `## Sibling tool-call parallelism` セクションが存在し、節約源（turn 数・context 反復削減、token 単価不変）・cap 相互作用・attribution・`updated` 更新が明記される |
| T15 | `mise run check` | lint・version・doc-ids・typecheck・test・bundle freshness の全 step exit 0 |
| T5/T13/T16 | `mise exec -- pnpm exec biome check packages/runtime-pi` + `git status` / `git log` | lint clean で各 commit が 1 つの論理変更のみを含む |

## Rollback Strategy

不可逆 step はない（migration・公開 API 変更・data 変換なし。
`buildSystemPrompt` の export 化は package 内部の additive 変更）。
全変更は `git revert` で戻せる。

- Trigger: prompt 文言変更が既存 contract test を壊す、または guidance
  による worker 挙動悪化が観測される
- Procedure: `git revert <Phase B commit>` — bullet 追加は text 差し替え
  のみで即 revert 可能。Phase A の test file は独立のため revert 不要
  （pin として残してよい）。Phase C の doc 追記も独立 revert 可能
- Verification: revert 後に `mise exec -- pnpm --filter @cheaper/runtime-pi test`
  が pass すること

## Open Questions / Gaps

- prompt test の所在: design は `test/run-prompt.test.ts` 新設 または
  `run.test.ts` 内 describe 追加を許容する。本 plan は新規 file
  `run-prompt.test.ts` を採用（`buildSystemPrompt` 単位検査で session
  が不要なため）
- `pi-execution-engine.md` への追記位置: design は「追記セクション」と
  のみ指定。approved doc の既存構造（Context→…→Verification Notes）を
  維持するため本文末尾への `## Sibling tool-call parallelism` 追加と
  した
- upstream chain の status: SPEC-034zA513QKarQrr5SQtgp7 は briefing
  review の ACCEPT 裁定に基づき `approved`、ADR-034zA5nZuUdGLDMTGh2V6z
  は `accepted` 済み。design gate も充足
- `stopReason: "length"` の faux-provider 経路は repo 内の既存 test で
  未使用の経路（`pi-ai` faux.d.ts の stopReason union には含まれる）。
  T2 で fail した場合は escalation clause が発動する前提

## Self-Review

- [x] Every spec requirement maps to a task（並列実行→T1、call 順
      result→T1、stopReason length→T1、guidance 各文言→T8/T10、同一
      file 直列化→T1、contract/envelope 不変→T11/T15、execution-mode
      非設定の code 検査→T3、execution-engine 文書→T14）
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（総量が小さく critical path 単一。
  Phase A の test 作成と Phase B の prompt 変更は並行可能だが、分割
  dispatch の overhead が見合わない）
- Independent work streams: Phase A（T1–T5）と Phase C（T13）は Phase B
  （T6–T12）と並行可能
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix.

## Review Handoff

- 前提: DESIGN-034zAKcWuB8SuFkOePsubg が approved であり、5 本の英文
  bullet・4 test の設計・文書化内容が verbatim 定義済み。実装者は文言
  を再起草しない
- 前提: `session.ts` の `streamFn`/`customTools`/`toolNames` seam、
  `testsupport.ts` の `createFakePi`/`fauxAssistantMessage`/`fauxToolCall`、
  `registry.ts` の `createSandboxedTools` が現行のまま利用可能
- 着目点: upstream chain は spec `approved` / ADR `accepted` / design
  `approved` で揃っている（briefing review の ACCEPT 裁定に基づく）
- 着目点: bounded-gate test の「全 N 個同時 in-flight」は意図的に厳しい
  pin であり、将来の Pi batch cap 導入時に緩和が必要になる可能性を
  test コメントで担保済みか確認すること
- 着目点: write→edit test の検出非対称性（PASS 経路は決定的、
  regression 検出は timing 依存）を plan 通りコメントに残したか
