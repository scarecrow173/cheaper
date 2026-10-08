---
id: "ADR-034r05iHfSMW9SGV4dpcy2"
type: "adr"
status: "superseded"
title: "Agent Runtime として Pi SDK (@earendil-works/pi-coding-agent) を採用する"
created: "2026-09-15"
updated: "2026-10-08"
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
  implements:
    - "../specs/cheaper-v1-worker-runtime-cli.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by:
    - "migrate-the-cheaper-cli-runtime-to-rust-and-rig.md"
  related:
    - "workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
    - "model-tier-model-resolver.md"
    - "wrapper-measured-deterministic-metadata.md"
    - "typescript-pnpm-monorepo.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/cheaper-v1-pi-sdk.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 1. Agent Runtime として Pi SDK (@earendil-works/pi-coding-agent) を採用する

## Context and Problem Statement

cheaper は「安価なモデル上で宣言的 Worker を実行する Worker Runtime」を作る。agent loop・provider 抽象・認証・tool calling・usage 収集・abort の基盤が必要であり、それを自前実装するか既存基盤を使うかを決める必要がある（引継ぎ §2.2）。

## Decision Drivers

* 引継ぎ §2.2: Runtime は Pi SDK に固定、複数 Runtime Adapter にしない、独自 Agent Loop は原則実装しない
* provider abstraction / OAuth・API auth / subscription access / model registry / agent loop / tool calling / coding tools / usage / abort を再利用したい
* YAGNI（§43）: 不要な抽象化・インフラを追加しない

## Considered Options

* (1) Pi SDK `createAgentSession` を in-process で利用する（`@earendil-works/pi-coding-agent`）
* (2) `@earendil-works/pi-ai` 上に独自 agent loop を実装する
* (3) 複数 runtime adapter 抽象を設計する
* (4) `pi --mode rpc` で別プロセス JSON-RPC 連携する

## Decision Outcome

Chosen option: "(1) Pi SDK `createAgentSession` を in-process で利用する", because provider 抽象・認証・agent loop・tool calling・usage・abort を再利用でき、独自 Agent Loop を実装しないという §2.2 の方針に合致し、プロセス管理も不要だから。

### Consequences

* Good, because agent loop・retry・compaction・usage 収集を Pi に任せられ、cheaper は Worker resolution / policy / sandboxed tools / envelope に集中できる。
* Good, because Pi が対応する OAuth/サブスク provider（ChatGPT Codex, Claude Pro/Max, GitHub Copilot, xAI, OpenRouter, Radius）と API key provider をそのまま利用できる。
* Bad, because Pi SDK のバージョン・内部イベント仕様への依存が生まれる（`@earendil-works` スコープの更新に追随が必要）。
* Bad, because `pi-tui` 等 TUI 依存も install される（依存の重さ）。

### Confirmation

* 依存パッケージが `@earendil-works/pi-coding-agent`（0.85.x 系）であり `@mariozechner/*` でないことを lockfile で確認。
* session が `SessionManager.inMemory()` / `SettingsManager.inMemory()` / 自作 `ResourceLoader` で作られ、`~/.pi/agent` や `cwd` の自動 discovery が無効であることをコードレビューとテストで確認。

## Implementation Plan

* Affected paths: `packages/runtime-pi/`（予定 path、ADR-034r05iHlWosDpP8BzaVZ8 参照）— Pi SDK 統合・session 作成・イベント監視・abort
* Patterns to follow: Pi SDK 公式 examples/sdk/12-full-control.ts の「no discovery」構成（自作 `ResourceLoader` で extensions/skills/AGENTS.md/prompts の discovery を一切行わない）
* Patterns to avoid: 独自 agent loop の実装、runtime adapter 抽象、`DefaultResourceLoader` の既定 discovery をそのまま使うこと、`bash`/`powershell`/`network` 系 tool の有効化
* Dependencies or configuration: `@earendil-works/pi-coding-agent`（`@earendil-works` スコープ固定、`@mariozechner/*` は 0.73.1 で凍結済み）、`@earendil-works/pi-ai`（`getModel`/`Model` 型/`InMemoryCredentialStore`）、`typebox`（custom tool スキーマ）。依存追加は別途承認制

制約（確定事項）:

* `SessionManager.inMemory()`、`SettingsManager.inMemory()` を使い、Pi の設定・session 永続化に依存しない
* 完全自作 `ResourceLoader` を渡し、extensions/skills/AGENTS.md/prompts の discovery を一切行わない
* `noTools: "builtin"` 相当で Pi 組み込みツールを無効化し、cheaper 提供の sandboxed tool のみ登録する（ADR-034r05iHlWosDpO699OwYF）
* maxTurns / maxToolCalls は SDK 側に設定がないため、`turn_end` / `tool_execution_start` イベントを数えて `session.abort()` する
* usage は `turn_end` の `message.usage`（`input/output/cacheRead/cacheWrite/totalTokens/cost`）から取得する
* model 一覧は `ModelRuntime.getAvailable()` から取得する
* 引継ぎ §2.2「独自 Agent Loop を実装しない」を明記する

## Verification

* [ ] `@earendil-works/pi-coding-agent` を依存に持ち、`createAgentSession` 経由で session を作成していることをコードレビューで確認
* [ ] 自作 `ResourceLoader` が全 getter で空/固定値を返し、`~/.pi/agent` 配下の extensions/skills/AGENTS.md が読み込まれないことを統合テストで確認
* [ ] `turn_end`/`tool_execution_start` のカウントで policy の maxTurns/maxToolCalls 超過時に `session.abort()` が呼ばれることをテストで確認
* [ ] `message.usage` から token/cost が result メタデータへ載ることをテストで確認

## Pros and Cons

### Option 1: Pi SDK `createAgentSession` in-process

* Good, because Pi の全基盤（provider/auth/agent loop/tools/usage/abort）を再利用できる。
* Good, because 型安全で同プロセス内の細かい制御（イベント監視・abort）ができる。
* Bad, because Node プロセス内実行でプロセス分離がなく、Pi の内部変更に追随する必要がある。

### Option 2: pi-ai 上に独自 agent loop

* Good, because ツール実行・turn 制御を完全に掌握できる。
* Bad, because §2.2「独自 Agent Loop を実装しない」に反し、retry/compaction/streaming の再実装コストが大きい。

### Option 3: 複数 runtime adapter 抽象

* Good, because 将来別 runtime へ差し替えやすい。
* Bad, because §2.2 が複数 adapter を明示的に否定し、YAGNI 違反。現時点で実装する対象が1つしかない抽象は有害。

### Option 4: `pi --mode rpc` 別プロセス

* Good, because プロセス分離され言語非依存。
* Bad, because 起動・終了・エラー処理・JSON-RPC フレーミングの管理コストが増え、型安全性も失われる。one-shot CLI モデル（ADR-034r05iHlWosDpKs4eBT59）では過剰。

## More Information

* Pi SDK docs: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/sdk.md
* Full-control example（no discovery）: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/examples/sdk/12-full-control.ts
* パッケージ: `@earendil-works/pi-coding-agent` 0.85.1（npm、2026-09-05 更新）。rename 経緯は DISC-034r05iHy2gdnp377SlYbj 参照
* 引継ぎ §2.2

## 更新履歴

### 2026-09-15: 実機プローブ結果に基づく補足

* maxTurns/maxToolCalls の強制方法を具体化: `AgentSession`/`AgentOptions` には maxTurns 相当が存在しないため、登録する全 `ToolDefinition` の `execute` を wrap して tool 実行前に budget 判定し、超過時は `session.abort()`。maxTurns は `turn_end` イベントを数えて同様に abort する。`Agent.beforeToolCall`/`shouldStopAfterTurn` hook は `AgentSession` が extension runner 経由で上書きするため直接代入は使わず、tool wrap + イベント監視の方式を採る。
* retry は Pi `settings.retry`（`SettingsManager.inMemory({retry:{enabled,maxRetries}})`）に委ね、`auto_retry_start` イベント数を計測する。cheaper 独自 retry は持たない。
* 実測: `createAgentSession({noTools:"builtin", tools:["<custom>"], customTools})` で組み込み tool が一切登録されず custom tool のみが登録されることを確認（0.85.1）。`noTools:"builtin"` 単独でも customTools は残る。
* 実測: `session.abort()` 後も `prompt()` は resolve し、`agent_end`/`agent_settled` が発火する。abort 理由の分類は cheaper 側が記録する。
* `output.maxTokens` は session オプションに存在しないため `session.agent.streamFunction` を wrap して注入する。`streamFunction === streamSimple` の内部チェックは要約系 auth（`_getSummarizationRequestAuth`）のみにあり、`compaction.enabled:false` で無効化する。
