---
id: "IMPL-0002"
type: "impl"
status: "completed"
title: "Implement @cheaper/runtime-pi (PLAN-0002)"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "dir"
        path: "packages/runtime-pi/"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/0022-scaffold-runtime-pi-package-with-pi-import-smoke-test.md"
    - "docs/tasks/0023-implement-mutation-journal.md"
    - "docs/tasks/0024-implement-boundary-checked-tool-operations-with-atomic-writes.md"
    - "docs/tasks/0025-implement-sandboxed-directory-walker.md"
    - "docs/tasks/0026-implement-sandboxed-grep-tool.md"
    - "docs/tasks/0027-implement-sandboxed-find-tool.md"
    - "docs/tasks/0028-implement-tool-call-budget-wrapper.md"
    - "docs/tasks/0029-implement-tool-registry-mapping-vocabulary-to-pi-tools.md"
    - "docs/tasks/0030-implement-cheaperresourceloader-without-discovery.md"
    - "docs/tasks/0031-add-scripted-fake-stream-test-fixture.md"
    - "docs/tasks/0032-implement-pi-session-creation-with-maxtokens-injection.md"
    - "docs/tasks/0033-implement-final-output-extraction-and-schema-validation.md"
    - "docs/tasks/0034-implement-output-repair-session.md"
    - "docs/tasks/0035-implement-usage-aggregation-from-message-end-events.md"
    - "docs/tasks/0036-implement-modelruntime-adapter-to-core-model-shape.md"
    - "docs/tasks/0037-implement-runworker-orchestration-with-limits-and-abort-classification.md"
    - "docs/tasks/0038-export-runtime-pi-public-api-and-run-full-verification.md"
  implemented-by: []
  depends-on: []
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement @cheaper/runtime-pi (PLAN-0002)

## Summary

PLAN-0002（docs/plans/0002）: `@cheaper/runtime-pi` パッケージ。Pi SDK 統合層（session・制限・abort 分類・出力抽出・repair・usage）と sandboxed tools（boundary operations・walker・grep/find・budget・registry）を実装する。対象 task: TASK-0022..0038。

## Context

PLAN-0002 / DESIGN-0002, DESIGN-0003, DESIGN-0004, DESIGN-0005 / SPEC-0001 / ADR-0001,0006,0007 に従う。task ごとの判断・検証証跡は本 IR に追記する。

## Implementation

### Changes

`packages/runtime-pi` 新設（TASK-0022..0038 全て完了）:

- `src/journal.ts` — mutation journal（before 記録・finalize で diffLines 行差分、差分なし除外）
- `src/tools/operations.ts` — Read/Write/Edit/Ls operations。全 path を `resolveWorkspacePath` 経由、escape は `Access denied: path is outside the workspace` に統一変換。write/edit は同 dir `.<name>.cheaper-tmp-<random>` 一時ファイル + rename の原子書き込み、journal 連携
- `src/tools/walker.ts` — 再帰 walker（.git/node_modules/>1MB/8KiB NUL binary skip、ancestor chain で symlink cycle 防止、symlink alias 経由の重複訪問は許可）
- `src/tools/grep.ts` / `find.ts` — defineTool + TypeBox、spawn 不使用（`vi.spyOn(child_process,"spawn")` 未呼出 assert 済み）
- `src/tools/budget.ts` — `wrapToolBudget` で execute を wrap。上限到達時は `{tools, budget:{toolCalls,rejectedToolCalls,limitHit}}` の budget 面に計上し onLimit 発火
- `src/tools/registry.ts` — ToolName → ToolDefinition。read/write/edit/ls は Pi `createXxxToolDefinition` に boundary ops 注入、grep/find は自前
- `src/resource-loader.ts` — `CheaperResourceLoader`（extensions/skills/prompts/themes/agentsFiles 全て空、systemPrompt = worker instructions + output contract、reload/extendResources no-op）
- `src/session.ts` — `createWorkerSession`（cwd=workspace.real、thinkingLevel=off、noTools、tools allowlist、customTools、inMemory SessionManager/SettingsManager、compaction off、retry=transientRetries）。`session.agent.streamFunction` wrap で `options.maxTokens = min(policy.output.maxTokens, model.maxTokens)` 注入。`deps.streamFn` seam
- `src/output.ts` — 最終 assistant text 連結 → ```` ``` ```` 除去 → JSON.parse → Ajv validate。失敗時 `{reason, rawOutput, errors}`
- `src/repair.ts` — tool 無し新規 session で repair。尽きれば INVALID_OUTPUT（errors 先頭20・rawOutput 先頭2000）
- `src/usage.ts` — `UsageCollector`（message_end usage 加算、turn_end→turns、auto_retry_start→transientRetries）
- `src/model-runtime.ts` — `ModelCatalog` 構造型 + `resolvePiModel`（getAvailable→ModelInfo→core resolveModel→getModel 実体解決）
- `src/run.ts` — `runWorker(deps)`。capability check→model 解決→journal/budget tools→session→prompt→分類（TIMEOUT/ABORTED/MAX_TURNS_EXCEEDED/MAX_TOOL_CALLS_EXCEEDED、journal 非空なら PARTIAL_MUTATION+cause）→repair→meta 集計
- `src/index.ts` — public API export
- `test/fixtures/fake-stream.ts` — pi-ai `createFauxCore`+`createProvider` ベースの scripted fake stream seam

### Decisions During Implementation

- **fake stream seam は pi-ai 同梱 faux provider を利用**。`createFauxCore` が scripted `AssistantMessage` queue・`options.signal` abort・callCount・factory step（context/options 参照可）を提供するため、イベントプロトコルの自前再実装より確実。fixture は薄い wrap（呼び出し記録 + `createProvider` による keyless auth Provider 構築）。
- **`session.prompt()` は auth 検証する**（`hasConfiguredAuth`/`checkAuth`）。テストは `ModelRuntime.create({refreshOnCreate:false})` + `registerNativeProvider(fake.provider)` + `refresh({allowNetwork:false})` で keyless provider を登録し通過。`getAvailable()` は refresh 後の snapshot に乗る。
- **`StreamFn` は pi-agent-core 由来**で runtime-pi の直接依存ではないため、構造上同型の `StreamFnLike` を session.ts でローカル定義（依存追加を回避）。`AgentSessionEvent` は pi-coding-agent から import 可能。
- **`noTools:"all"` を `createWorkerSession` のオプション化**（repair session 共用。既定 "builtin"）。
- **PARTIAL_MUTATION の適用範囲は abort 系分類（TIMEOUT/ABORTED/MAX_*）のみ**。INVALID_OUTPUT/PROVIDER_FAILURE には包まない（DESIGN-0004「abort 後の分類」箇条書きのスコープに従う）。repair 中に abort した場合は abort 分類を優先。
- **maxTurns 判定は turn_end で `message.stopReason === "toolUse"` の時のみ** abort 記録（stop/error で終わる最終 turn で誤記録しないため）。
- **abort 猶予 5s は `deps.abortGraceMs` で注入可能**（テスト seam。既定 5000）。
- **budget 戻り値は `{tools, budget}` 形**（counter は `.budget` 面。tools 配列と計上状態を分離）。
- **`createXxxToolDefinition` の戻り値は `defineTool()` で wrap** して `& AnyToolDefinition` 交差を得る（runtime identity、default generic の `ToolDefinition[]` へ代入可能にする型上の措置）。
- **`ResourceExtensionPaths`/`ResourceLoaderReloadOptions` は index 未 export** のため `Parameters<ResourceLoader["..."]>` で参照。

## Related Experiments

### Adopted

- pi-ai `faux` provider（`createFauxCore`/`createProvider`/`fauxAssistantMessage`/`fauxToolCall`）: scripted stream・abort・error・toolcall・exhaustion error を全て提供。session/run テストで実 provider 不要に。

### Rejected

- 自前 AssistantMessageEventStream fixture: faux が同等機能を型安全に提供するため不採用。
- `@earendil-works/pi-agent-core` の直接依存追加: `StreamFnLike` 構造型と `AgentSessionEvent` で代替できたため不採用。

## Validation

全て `mise exec --` 経由。出力は `tmp/test-output/`。

- `pnpm --filter @cheaper/runtime-pi test`: 17 files / 125 tests 全 pass（test-all.txt, TEST_EXIT=0）
- `pnpm -r build`（tsc -b）: core + runtime-pi Done（build-all.txt, BUILD_EXIT=0）
- `pnpm -r test`: core 225 + runtime-pi 125 = 350 tests 全 pass（test-all.txt）
- `pnpm biome check .`: exit 0（error 0・warning 0・info 1）（biome.txt）
- `pnpm tsc -b`: exit 0（tsc.txt）
- spawn 未呼出: grep/find テスト内 `vi.spyOn(child_process, "spawn")` assert green
- fake stream: script イベント列・signal abort（事前/途中）・calls 記録の 9 テスト green
- run: TIMEOUT/ABORTED/MAX_TURNS_EXCEEDED/MAX_TOOL_CALLS_EXCEEDED/PARTIAL_MUTATION(cause)/repair/INVALID_OUTPUT/PROVIDER_FAILURE/POLICY_VIOLATION/transientRetries(=1, Pi auto-retry 実測) の 12 テスト green

## Risks

- `session.agent.streamFunction` wrap は SDK 内部で要約系 auth（`_getSummarizationRequestAuth` の `=== streamSimple` 参照一致）に影響するが、`compaction.enabled:false` で要約系は走らない。SDK upgrade 時に再確認要（DESIGN-0004 追跡事項と同じ）。
- abort 後に prompt() が settle しない経路では 5s 猶予後に分類へ進むため、in-flight tool 実行の完了は保証しない（journal.finalize は disk 状態を見るため実害は限定的）。
- `pnpm install` 時に `ERR_PNPM_IGNORED_BUILDS`（@google/genai, esbuild, protobufjs の build script 抑止）が発生。`onlyBuiltDependencies: []` 設定済みでも exit 1。機能影響は確認されていないが環境依存の可能性。

## Follow-ups

- PLAN-0003: CLI/envelope。`runWorker` の meta に `durationMs` を CLI 側で付与する。
- 実 provider での smoke は手動手順として Review Handoff に記載（認証無し環境では実行不可）。
