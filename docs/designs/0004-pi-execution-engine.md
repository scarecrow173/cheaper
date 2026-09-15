---
id: "DESIGN-0004"
type: "design"
status: "approved"
title: "Pi 実行エンジン: session・制限・abort・retry・出力抽出・repair・usage"
created: "2026-09-15"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source:
    - "../handoff/2026-09-15-cheaper-v1-implementation-handoff.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "0001-worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md"
    - "0002-workspace-filesystem-boundary-sandboxed-tools.md"
    - "0003-model-resolver.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/0001-cheaper-v1-worker-runtime-cli.md"
    - "../adr/0001-agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md"
    - "../adr/0003-worker-definition-runtime-policy-runtime-invariants-capability.md"
  derived-by:
    - "../plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Pi 実行エンジン: session・制限・abort・retry・出力抽出・repair・usage

## Context

ADR-0001 の in-process Pi SDK 統合を実装レベルに落とす。実機プローブ（tmp/pi-probe、@earendil-works/*@0.85.1、Node 26.7.0）で確定した事実:

- `createAgentSession({ noTools: "builtin", tools: ["echo"], customTools })` で custom tool のみが登録される（`noTools:"builtin"` 単独でも customTools は残る）
- `session.abort()` 後も `prompt()` は **resolve** する（reject しない）。`agent_end`/`agent_settled` は発火する。分類は abort 理由の自前記録に依存する
- abort は stream が `options.signal` を尊重する場合に有効。signal を無視する stream では `abort()` 自体が完了を待ち続け得る（実機で確認）
- `session.agent.streamFunction` は public field で、代入した `StreamFn` が実際に呼ばれる（fake で検証済み）
- `streamFunction === streamSimple` の参照一致チェックは `_getSummarizationRequestAuth`（要約系リクエストの auth 取得）のみに存在。wrap すると要約系が lenient 経路になるが、`compaction.enabled: false` で要約は走らないため実害なし

## Scope

### In Scope

- session 生成と `CheaperResourceLoader`
- system prompt / user prompt 構成
- maxToolCalls / maxTurns / timeout / signal による制限強制と abort 分類
- transient retry（Pi `settings.retry` への委譲）
- `output.maxTokens` 強制（streamFunction wrap）
- 最終出力抽出・schema 検証・repair session
- usage/turns/toolCalls/repairs の計測

### Out of Scope

- envelope 組み立て・exit code 出力（DESIGN-0005）
- tool 実装と filesystem boundary（DESIGN-0002）
- model 選択（DESIGN-0003）

## Components and Boundaries

- `session.ts`: `createAgentSession` 呼び出しと制限 wrap の設置
- `resource-loader.ts`: `CheaperResourceLoader`（全 11 メソッド。extensions/skills/prompts/themes/agentsFiles は空、`getExtensions` は `{extensions: [], errors: [], runtime: createExtensionRuntime()}`、systemPrompt は固定文字列、`extendResources`/`reload` は no-op。`getSystemPromptSource()` は `undefined`、`getAppendSystemPromptSources()` は `[]` を返す）
- `output.ts`: 最終 assistant text 抽出・フェンス除去・`JSON.parse`・Ajv validate
- `repair.ts`: repair session 生成と repairAttempts ループ
- `usage.ts`: `message_end` の `message.usage` 逐次集計
- `run.ts`: `runWorker(deps)` オーケストレーション。`deps.stream?: StreamFn` を受け、`session.agent.streamFunction` へ注入する seam を持つ（テスト用 fake stream の注入点）

Session 生成:

```ts
createAgentSession({
  cwd: workspace.real,
  modelRuntime,
  model,
  thinkingLevel: "off",
  noTools: "builtin",
  tools: effectiveToolNames,          // allowlist に custom tool 名を含める
  customTools,
  resourceLoader: new CheaperResourceLoader(systemPrompt),
  sessionManager: SessionManager.inMemory(),
  settingsManager: SettingsManager.inMemory({
    compaction: { enabled: false },
    retry: { enabled: transientAttempts > 0, maxRetries: transientAttempts },
  }),
});
```

## Data and Control Flow

### prompt 構成

- system prompt = Worker Markdown body + runtime 付加の「Output contract」節（output schema JSON、「最終メッセージは JSON オブジェクトのみ、コードフェンス禁止」、「path は workspace 相対 POSIX」、利用可能 tool 名）
- user prompt = `Task input (JSON):` + 入力 JSON 整形。model/tier/provider はプロンプトに含めない

### 制限の強制（SDK に maxTurns/maxToolCalls が無いため cheaper 側で実施）

- maxToolCalls: 登録する全 ToolDefinition の `execute` を wrap。実行前に `count >= maxToolCalls` なら `limit = "maxToolCalls"` を記録し `session.abort()`、error result を返す。それ以外は count++ して委譲
- maxTurns: `turn_end` を数え、`turns >= maxTurns` かつその turn の assistant message に toolCall block が含まれる（続行しようとしている）場合 `limit = "maxTurns"` を記録し `abort()`
- timeout: CLI 入口で `setTimeout(effectiveTimeoutMs)` → `reason = "timeout"` を記録し abort。provider retry・repair も同じタイマー内
- SIGINT/SIGTERM: `reason = "signal"` → abort
- abort 猶予: `abort()` 呼び出し後 5 秒以内に `prompt()` が resolve しなければ待機を打ち切り、その時点の journal/usage で同じ分類経路に入る（stream が `options.signal` を無視した場合のハング防止。runtime-pi が `Promise.race` で実装し、cli はこれに依存して必ず envelope を出して exit する）
- abort の集約: runWorker 内部 AbortController に timeout/外部 signal/limit を集約し、repair session へも同 signal を渡す（repair 中の timeout/abort で repair の prompt が settle し、repair 解決後にも abort 分類を再確認して success を返さない）
- abort 後の分類: `prompt()` は abort 後も resolve する（実測）ため、resolve/reject どちらでも同一の分類経路に入る。reason に応じて `TIMEOUT` / `ABORTED` / `MAX_TURNS_EXCEEDED` / `MAX_TOOL_CALLS_EXCEEDED`。journal が非空なら `PARTIAL_MUTATION` で包み、`error.cause` に元エラー、`meta.changes` に変更一覧

### transient retry

- Pi の auto-retry（`settings.retry`）に委ね、`auto_retry_start` イベント数を `meta.transientRetries` に計上。cheaper 独自 retry は持たない。Worker 全体の再実行はしない

### output.maxTokens

- `effective = min(policy.output.maxTokens, model.maxTokens)`
- `session.agent.streamFunction` を wrap し、`options.maxTokens = effective` を注入してから元の `streamSimple` を呼ぶ方式を採用。実機で wrap した streamFunction が呼ばれることを確認済み。`=== streamSimple` チェックは要約系 auth のみに影響し、`compaction.enabled: false` で無効化済み

### 出力抽出と repair

- `session.messages` の最後の assistant message の text block 連結 → 先頭末尾の ```` ``` ```` フェンス除去 → `JSON.parse` → Ajv validate
- repair（output-only）: 失敗時 `repairAttempts` 回まで**新規 session**（`noTools: "all"`、customTools なし、同 model、systemPrompt = 「以下の schema に適合する JSON のみを返せ」+ schema）に対し、user prompt = 前回 raw output + validation errors を投げる。tool が物理的に存在しないため「repair has no tools」が構造的に成立。尽きたら `INVALID_OUTPUT`（details: Ajv errors 先頭 20 件、rawOutput 先頭 2000 文字）

### usage

- `message_end`（assistant）の `message.usage` を逐次加算 → `meta.usage = {input, output, cacheRead, cacheWrite, totalTokens, costUsd}`
- `meta.turns` = `turn_end` 数、`meta.toolCalls` = wrap カウンタ、`meta.repairs`、`meta.transientRetries`（`auto_retry_start` 数）、`meta.rejectedToolCalls`（boundary 拒否・budget 超過の error result 数）、`meta.durationMs`（CLI 入口から）
- 終了時 `session.dispose()`

## Non-Functional Constraints

- stdout には一切書かない（envelope は cli が組み立てる）
- abort の伝播は stream の `options.signal` 尊重に依存する。Pi 公式 provider は尊重する前提で、fake stream はテスト側で signal を honor する実装にする

## Risks and Trade-offs

- `streamFunction` wrap は SDK の public field だが内部参照チェックとの結合がある。要約系が将来 enabled になった場合は auth 経路を再確認する必要がある（Deferred に記載）
- repair を別 session にすることで transcript 分離と no-tools 保証を得る代わりに、1 回の追加 provider 呼び出しコストが掛かる
- maxTurns 判定は「その turn に toolCall が含まれる」場合のみ abort する。toolCall 無しの最終回答で `maxTurns` ちょうど到達は正常終了とする

## Deferred Design Concerns

- `session.agent.streamFunction` wrap が SDK 内部で要約系以外に影響するかの継続確認 | 0.85.1 では要約 auth のみ | SDK upgrade 時 | wrap が将来の内部最適化と衝突する可能性
- 要約/branch summary の有効化 | V1 では `compaction.enabled=false` 固定 | 長時間 Worker で context overflow が問題になった時点 | overflow 時の実行失敗リスクが残る

## Verification Notes

- [ ] Execution: timeout で `TIMEOUT` exit 7、SIGINT/SIGTERM で `ABORTED` exit 8
- [ ] Execution: maxTurns/maxToolCalls で `MAX_TURNS_EXCEEDED`/`MAX_TOOL_CALLS_EXCEEDED` exit 6、abort 後も `agent_end` が発火し `prompt()` が resolve する経路で分類される
- [ ] Execution: journal 非空の abort で `PARTIAL_MUTATION` exit 9、`cause` に元エラー
- [ ] Execution: repair は tool なし session で `repairAttempts` 回まで、尽きれば `INVALID_OUTPUT` exit 6
- [ ] Execution: `auto_retry_start` が `meta.transientRetries` に計上され、cheaper 独自 retry が走らない
- [ ] `stream` seam 経由の scripted fake stream で上記全てを再現できる
