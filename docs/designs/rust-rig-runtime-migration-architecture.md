---
id: "DESIGN-034Bbq1qY8WFBdL26tUdBw"
type: "design"
status: "approved"
title: "Rust + Rig runtime migration architecture"
created: "2026-10-06"
updated: "2026-10-06"
owners: []
relations:
  source: []
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
    - "worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md"
    - "workspace-filesystem-boundary-sandboxed-tools.md"
    - "canonical-tool-runtime.md"
    - "pi-execution-engine.md"
    - "model-resolver.md"
    - "cli-contract-envelope-exit-codes.md"
    - "auth-command.md"
    - "devin-local-provider-plugin.md"
    - "delegation-usage-measurement-log.md"
    - "usage-savings-telemetry.md"
    - "sea-executable-build-pipeline-and-embedded-resource-resolution.md"
    - "version-pinned-launcher-acquisition.md"
    - "unified-version-source-of-truth.md"
    - "token-counter-contract.md"
    - "bounded-tool-scheduler.md"
    - "repair-abort-and-failure-normalization-hardening.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
    - "docs/adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md"
    - "docs/discovery/rust-rig-native-runtime-migration-for-cheaper-cli.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../research/rig-agent-runtime/migration-plan.md"
    - "../research/rig-agent-runtime/final-report.md"
    - "../research/pi-runtime-replacement/migration-comparison.md"
  defers: []
  deferred-by: []
---

# Rust + Rig runtime migration architecture

## Context

cheaper CLI の worker runtime を、Pi SDK(`@earendil-works/pi-ai` /
`pi-coding-agent` 0.85.1)+ TypeScript/Node.js SEA 構成から、Rust +
Rig(0xPlaygrounds/rig、`=0.43.x` pin)ベースのネイティブ実行ファイルへ全面
rewrite する(SPEC-034BbgvL9s41K7NflOddZ6 / ADR-034BbhP6hJV7DFtv7k7AGs)。
外部 contract(envelope `schemaVersion:1`・exit code 表・usage.jsonl・hook
4 target・config 書式・Release asset 名・SHA256SUMS・launcher sh/ps1)は
不変であり、変わるのは実装言語・ビルド層・配布バイナリの中身のみ。

移植元は `packages/core`(≈2.7k 行)・`packages/tool-runtime`(≈4.4k 行)・
`packages/runtime-pi`(Pi 依存部除く ≈2.0k 行 + devin provider ≈1.5k 行)・
`packages/workflow`(≈2.1k 行)・`packages/cli`(≈3.6k 行)、計 ≈16.4k 行。
Rig が担うのは provider 抽象と agent loop のみで、model catalog・credential
store・OAuth・transient retry・devin provider(ギャップ G1-G4)は cheaper
側の責務として新設計する。フェーズ構成・ギャップ G1-G8・differential
golden 戦略の詳細は `docs/research/rig-agent-runtime/migration-plan.md` に
準拠し、本書はその構造設計を確定する。

## Scope

### In Scope

- 新 cargo workspace(`rust/` または `packages-rs/`)の crate 分割と依存方向の
  確定。Rig 型は `cheaper-rig-adapter` 1 crate に隔離する。
- `cheaper-core` / `cheaper-tools` / `cheaper-runtime` / `cheaper-rig-adapter` /
  `cheaper-workflow` / `cheaper-cli` / `cheaper-provider-devin` の責務と公開
  境界。
- run オーケストレーション(capability 適用 → model 解決 → tool scope →
  Rig agent run → output 抽出 → repair → meta 構築)の Rust 設計。
  abort 分類(timeout/signal/maxTurns/maxToolCalls → PARTIAL_MUTATION 包装)・
  prompt grace settle は cheaper 側で保持する。
- 自前ギャップの設計: model catalog store(G1)・credential store + OAuth
  flow(G2)・transient retry 層(G3)・devin provider(G4)・`process` tool(G5)・
  resource/concurrency scheduler(G6)・fake provider seam(G7)・builtin
  resource 埋込(G8)。
- envelope serializer の byte 互換規則、usage.jsonl 新旧 record 互換、
  `meta.usage.costUsd` の pricing 層算出口径への変更。
- differential golden 検証基盤(共有 fake provider fixture + normalize 規則)。
- cargo release matrix(win-x64 / linux-x64 / linux-arm64)による配布切替。

### Out of Scope

- 機能追加・worker 追加・envelope schema 拡張。
- 配布 contract(GitHub Release + SHA256SUMS + launcher acquisition)の再設計。
- `packages/apm/` の構造変更(launcher・hooks.json・skills は無改変)。
- molo / Goose GDK へのピボット実装(不達時判断は plan の責務)。
- 初期 provider 範囲外の Pi builtin provider 移植、darwin-* target 追加
  (Deferred 参照)。
- `<userConfigDir>/pi/` 既存 credential の import(再 login 方針で割り切る)。

## Components and Boundaries

### crate 構成

依存方向は `cheaper-cli → cheaper-workflow → cheaper-runtime → {cheaper-rig-adapter, cheaper-tools} → cheaper-core` の一方向。`cheaper-provider-devin` は `cheaper-rig-adapter` に加え Rig crate 自体(`rig`/`rig-core`/`rig-agent`)にも直接依存する(Rig custom `Wire`/`Decoder`/`Transport` trait 実装のため — ADR の「Rig 型は adapter のみ」の決定に対する意図的な拡張であり、隔離原則の例外として `cheaper-provider-devin` を第 2 の Rig 接触面とする)。`cheaper-runtime` が feature 経由で組み込む方式は採用しない(runtime → devin の optional dep は devin → runtime の plugin trait 依存と合わせて Cargo が拒否するパッケージ循環になる)。plugin 登録は composition root である `cheaper-cli` が runtime+devin の両方に依存して行う。plugin contract(credential 永続化・catalog 登録・logout hook の trait 面)は `cheaper-runtime` が所有し、`cheaper-provider-devin` はその trait を実装する(逆向き依存を入れない)。

| crate | 移植元 | 責務 |
| --- | --- | --- |
| `cheaper-core` | `packages/core` | worker `.md` frontmatter+schema 解決・検証、policy.yaml schema/normalize/merge/restrict/capability、models.yaml・tier 分類・resolveModel/resolveExplicitModel・pricing snapshot・auto 分類、duration パース、`CheaperError` code→exitCode/retryable 表(ERROR_TABLE をそのまま写す)、workspace path canonicalize・symlink policy・denylist、o200k_base token 計測(`tiktoken-rs`)、resource manifest 型、tool vocabulary。Rig 非依存・tokio 非依存の純ロジック crate |
| `cheaper-tools` | `packages/tool-runtime` + `runtime-pi/tools` | canonical tool 8 種(read/grep/find/ls/write/edit/count_tokens/process)+ catalog(input/output JSON schema・resource declaration)、path boundary・denylist・write scope・resource claim、mutation journal、tool call budget・token meter、並行 scheduler(shared/exclusive・serialized/per-resource、DESIGN-034zWclUCjCIH7M27XEu4j の admission 設計を port)、invocation/lease scope、artifact store、`process` tool(minimal env・output cap・process-tree kill、G5)、line-framer/walker |
| `cheaper-rig-adapter` | `runtime-pi/session.ts` + `runtime-pi/model-runtime.ts` の Pi 接着部 | Rig 型を唯一参照する crate。`rig-agent` `AgentBuilder` + `.tool()` 登録 + `default_max_turns` で agent loop を構成し、`Tool`/`PortableTool` trait 実装で `cheaper-tools` の tool scope を Rig に expose(ToolContext で invocation scope 注入)。hook(`ModelTurnFinished`/`ToolCallDelta`)で turn/tool call を計測し、逐次集計を `WorkerUsage` へ写像(PromptResponse 非依存、§run 参照)。transient retry wrapper(G3、completion call 単位)を置く。dev-only fake seam(G7)は **catalog/auth/stream の 3 層に跨る**(fake model を catalog `getAvailable` に登録・keyless credential 解決・scripted stream の 3 点セットで `cheaper-fake` provider を構成。stream 接着は adapter、catalog/auth 注入点は `cheaper-runtime` の各 store 層の dev-only trait 実装) |
| `cheaper-runtime` | `runtime-pi/run.ts`・`output.ts`・`repair.ts`・`journal.ts`・`usage.ts`・`auth.ts`・`provider-plugin.ts`・`model-runtime.ts` | run オーケストレーション(`runWorker` 相当)、abort 分類と PARTIAL_MUTATION 包装、prompt grace settle、最終 assistant 抽出・code fence 除去・schema 検証、tool 無し repair session、UsageCollector 相当、model catalog store(G1)、credential store + OAuth flow(G2)、`WorkerRunMeta` 構築。Rig 型は使わず `cheaper-rig-adapter` が提供する trait 境界経由で session を取得する |
| `cheaper-provider-devin` | `runtime-pi/providers/devin` | Devin CLI 子プロセス委譲 auth/catalog + Connect-protocol vendored transport(wire/protobuf-lite framing・jwt・metadata・context-map、現行 stream.ts 656 行 port)+ Rig custom `Wire`/`Decoder`/`Transport` 接着(G4)。隔離 data dir(`<userConfigDir>/providers/devin/data`)を維持 |
| `cheaper-workflow` | `packages/workflow` | workflow YAML schema・compiler・DAG engine(job 並行・lease・executor port)・artifact store・bindings・resource inference・interface 投影。Rig 非依存。executor port は `cheaper-runtime` の runWorker 相当を呼ぶ |
| `cheaper-cli` | `packages/cli` | 引数パース・command dispatch(run 系 alias・count-tokens・models・usage・auth 3・workflow・hook)、envelope serializer、stdin input、`--format text`、usage.jsonl 記録・集計、hook 4 target adapter + routing policy(fail-open)、SIGINT/SIGTERM 中継、builtin resource 埋込(G8)、version 解決 |

### 境界ルール

- Rig 型(`rig::` / `rig_core::` / `rig_agent::`)の `use` は
  `cheaper-rig-adapter` と `cheaper-provider-devin` のみに許可。他 crate が
  Rig 型を直接参照することを lint/レビューで禁止(0.x breaking change の
  影響半径を adapter に限定、現行 `createPiToolHandlers`→`toPiTool` と同じ
  adapter 思想)。
- tokio 依存は `cheaper-rig-adapter`・`cheaper-runtime`・`cheaper-workflow`
  (DAG job 並行)・`cheaper-cli`(entry 層)・`cheaper-provider-devin` に限定。
  `cheaper-core` は同期純ロジック。`cheaper-tools` は process tool の
  spawn/timeout/kill と scheduler の並行制御を含むため tokio 依存を許可する
  (executor 抽象化は行わない — YAGNI)。
- abort 分類・PARTIAL_MUTATION・meta 構築・envelope 意味は全て cheaper 側
  crate の責務。Rig の `PromptError` は adapter 境界で `CheaperError` 系に
  写像し、上位に漏らさない。
- 未知・初期スコープ外の provider api 種別は catalog 層で明示 error
  (`INVALID_MODEL_CONFIG` 系)を返し、silent fallback しない。

## Data and Control Flow

### run オーケストレーション(`cheaper-runtime::run_worker`)

現行 `runtime-pi/src/run.ts` の構造をそのまま移植する:

1. `check_capabilities`(worker × policy)で effective tool 集合を確定。
2. `run_abort: CancellationToken` + `tokio::time::timeout(
   policy.execution.timeout_ms)` + 外部 signal で abort trigger を一元化。
   `abort_reason: Option<AbortReason>` を先勝ちで記録(timeout/signal/
   maxTurns/maxToolCalls → TIMEOUT/ABORTED/MAX_TURNS_EXCEEDED/
   MAX_TOOL_CALLS_EXCEEDED)。
3. `cheaper-tools` で invocation runtime + worker scope を生成(journal・
   budget・denylist・writeScope・resourceManifest・`maxToolCalls` onLimit →
   `requestAbort("maxToolCalls")`)。
4. catalog 層で model 解決(tier または `--model <provider>/<id>` explicit。
   exclude・availability・`policy.model.maxTier` を enforce)。
5. `cheaper-rig-adapter` で agent を構成: `AgentBuilder` に system prompt
   (現行 `buildSystemPrompt` の output contract・batching guidance を文字列
   として完全再現)+ sandboxed tools + `default_max_turns` + `maxTokens`。
   hook で turn 終了を検知し、`turns >= maxTurns` かつ最終 stop が tool
   継続(現行 `stopReason === "toolUse"` 相当)のときのみ `maxTurns` abort。
6. prompt 実行。usage/turns/toolCalls の収集源は **`PromptResponse` ではなく
   hook ベースの逐次集計を唯一の正とする**。`PromptResponse`/
   `completion_calls` は成功時にしか存在せず、abort・provider error・
   `MaxTurnsError`・`PromptCancelled` では usage を持たない error が返るため、
   accumulator が以下の hook 面で逐次加算する(**Phase 0 PoC 実測で確定**):
   - `turns`: `on_model_turn_finished`(accepted turn のみ発火。
     invalid-call retry で巻き戻された attempt は発火しない — cheaper の
     「受諾 turn」のみ計上する規則と整合)
   - `usage`: `on_outcome` の completion outcome(`CompletionResponse.usage`)
     で per-call 加算 + **`on_run_settled` の
     `SettledOutcome::Response(&PromptResponse).usage`(rollback 含む run 総量)
     へ reconcile(max)**。巻き戻された attempt は `on_outcome` も
     `on_model_turn_finished` も発火しない(rig-agent `settle_model_turn`
     前段で `TurnRetried`→break)ため、reconcile 無しでは billed usage が
     欠落する。**error 終了時(`SettledOutcome::Error`)は `PromptResponse`
     を持たず rollback 分 usage は捕捉不能** — error 経路では per-call
     加算分のみが残ることを許容とする(usage-bearing error record の
     欠落ではなく、rollback 試行分の billing 差分のみ)
   - `toolCalls`/`rejectedToolCalls`: `on_outcome` の tool outcome。
     `Ok(ToolResult)` でも **`is_skipped()`(denied dispatch)は
     `rejectedToolCalls`**、`Ok` 非 skipped は `toolCalls`、`Err` outcome
     (dispatch 失敗)は `rejectedToolCalls`。`Ok` 判定だけでは denied
     tool が `toolCalls` に誤計上される(PoC レビュー指摘)
   - `repair` session・transient retry も同一 accumulator に合算する
     (現行 `session.subscribe` + shared collector と同型)。
   `transientRetries` は G3 retry wrapper が per-completion-call 単位で
   計上する(§retry 参照)。`PromptResponse` は `on_run_settled` reconcile
   と取得時 sanity check に限定する。
7. `extract_output` 相当(最終 assistant text → fence 除去 → JSON parse →
   `jsonschema` crate で validate)。失敗時は repair session へ。
8. abort が記録されていれば reason 分類で throw し、journal 非空なら
   PARTIAL_MUTATION で包む(cause に元 error)。`WorkerRunError` 相当の
   「meta snapshot を運ぶ error」型を `cheaper-runtime` に置き、CLI は
   `run_meta` から error envelope の meta を埋める(取得不能 field は null)。
   **error/abort/maxTurns/INVALID_OUTPUT の全経路で meta snapshot は
   hook 集計 accumulator から構築する**(PromptResponse 非依存、§step 6)。
   これにより usage-bearing error record(`meta.usage`+`meta.model` を持つ
   error が usage.jsonl に記録される現行契約)が維持される。
9. `build_meta` 相当で `WorkerRunMeta` 全 field(model{provider,id,tier,
   selection,tierSource,effectiveCost,...}・usage・turns・toolCalls・
   rejectedToolCalls・repairs・transientRetries・changes・execution・
   savings{sourceTokens,returnedTokens,avoidedContextTokens,
   directlyComparable})を構築。
10. `meta.usage.costUsd` は provider 応答値ではなく cheaper pricing 層
    (pricing snapshot × token 数)で算出する。`usage.jsonl` 新旧 record の
    口径混在は `docs/guides/usage.md` に明記する(spec 要求)。

### Rig 適用の意味論固定(review で確定した規則)

以下は planning 前に固定する Rig 適用ルール。実装者の自由解釈を禁止する。

- **maxTurns の主従**: Rig の `default_max_turns` は model-call budget であり
  cheaper の「最終 stop が tool 継続のときのみ abort」意味と異なる。
  Rig 側には十分大きな値(または未設定)を渡し、**maxTurns 判定は cheaper 側
  hook のみが行う**。`MaxTurnsError` が表面化した場合は `MAX_TURNS_EXCEEDED`
  ではなく内部 guard 到達として扱う(契約: cheaper 分類が正)。
- **invalid/unknown tool call**: Rig の `InvalidToolCallAction` は `Retry`
  (corrective feedback で turn を巻き戻して loop 継続、現行 Pi の
  `kind:"immediate"` error tool result と同型)を選択する。docs.rs の enum
  意味論は `Retry{feedback}`=feedback で巻戻し、`Repair{tool_name}`=tool 名の
  rename+再検証であり、意図する挙動は `Retry`。`Fail` は契約破壊のため禁止。
  `Repair`/`Skip`/`Stop` も契約を変えるため禁止。
  **PoC 実測で判明した必須設定**: `Retry` が有効になるには run 単位の
  `max_invalid_tool_call_retries(n)` budget(既定 0、未設定だと `Retry`
  が黙って `Fail` に潰される)を必ず設定する。runner 側の knob であり
  hook 返却だけでは不十分。
  `tool_concurrency` は 1 を明示設定し、並行 dispatch は導入しない(現行の
  sibling 並行意味は scheduler 側で保持)。計数写像: invalid call は tool
  runtime に到達しないため `toolCalls`/`rejectedToolCalls` のどちらにも
  計上しない(現行 accounting は runtime 実行到達時のみ `toolCalls++`、
  boundary/limit 拒否のみ `rejectedToolCalls++`)。
- **transient retry(G3)の粒度**: retry wrapper は **completion call 単位**
  (transport/model-call 層)に限定し、run 全体・tool 実行を包まない
  (非冪等 mutation の二重実行を防止)。429/5xx/network timeout を
  transient と分類し、retry 実施ごとに `transientRetries` を計上。
  Rig provider error の HTTP status/body 露出(`provider_response_status`
  等)を PoC で確認し、露出が弱い場合は error 文字列分類に退避する旨を
  go 条件に含める。
- **run surface**: streaming run(`stream()` 系)を採用する。
  `cancel()`(abort)と delta hook は streaming 面の API であり、
  non-streaming では発火しない。adapter は streaming run のみを expose する。

### structured output repair

- main loop には `output_schema` を適用しない(tool call 抑制を避ける現行
  設計を維持)。`Extractor` ではなく、現行 `repair.ts` と同型の自前 loop:
  tool 無し completion session + `response_format: json_schema`(または
  provider 別 `output_mode`)+ schema error フィードバック retry。
- 各 attempt は新規 session で、前回 raw output + `jsonschema` crate の
  error(`instancePath` + message)を repair user prompt に埋め込む。
  `policy.output.repairAttempts`・`transientRetries`・run 全体の
  `run_abort` signal・abort grace(既定 5000ms)を現行通り伝える。
- **repair session の usage/turns/transientRetries は main run の
  accumulator に合算する**(現行 `repair.ts` が同一 collector に subscribe
  する構造と同型)。repair 呼出し分を meta.usage/turns に含める規則を
  維持しないと differential golden の usage 厳密比較が fail する。
- attempts 使い切りは `INVALID_OUTPUT`(details: errors 先頭 20 件・
  rawOutput 先頭 2000 文字)。repair 中の abort は reason 分類を優先する。
- Ajv → `jsonschema` で message 文言が変わるため、envelope
  `details.errors[]` の契約は `instancePath` のみとし、message 本文・件数・
  code は非契約とする(spec §parity 検証規則が正。`code` field は現行
  `{instancePath,message}` のみのため追加しない)。

### tool 移植の固定ルール(review で確定)

- **grep の JS RegExp 方言は tool contract**(`pattern` schema が
  "JavaScript regular expression" と広告)。`regex` crate は lookahead/
  backreference 非対応で方言非互換のため不採用。**`regress` crate(JS 方言
  互換の backtracking engine)を第一候補**とし、ReDoS 隔離は現行の
  worker_thread+`terminate()` 相当を **eval 用の subprocess 分離**
  (helper process spawn + timeout kill + output byte cap)で再現する
  — Rust thread は安全に kill できないため。per-file/total eval budget・
  deadline・heap cap の意味は `limits.ts` から port。方言互換が regress で
  満たせない場合は「JS 方言 subset への制限」を spec/contract 変更として
  正式に起案する(暗黙の契約破壊を禁止)。
- **Windows path canonicalize 規則**(cheaper-core): `std::fs::canonicalize`
  は verbatim prefix(`\\?\`)を返すため `dunce::canonicalize` 相当で除去。
  `..` は Rust `Path` が正規化しないため component 単位の lexical resolve
  を自前実装。不存在 tail は「最深の存在する祖先を canonicalize + tail
  連結」の現行規則を再現。deny/writeScope/display/`root` field の文字列
  形式を TS 実装の byte 表現に合わせる(Windows golden fixture 必須)。
- **`process` tool の port 対象は列挙を完全化**: minimal env・output byte
  cap・process-tree kill に加え、per-task `timeout`・`heartbeat`・
  `killGrace`・task 単位 resource declaration・Windows PID-reuse 防止
  (creation timestamp 照合)を含む。`node` task の `process.execPath`
  fast-path は Rust binary に等価物が無いため **`node` 解釈は
  "node を PATH 解決せず明示 error"(POLICY_VIOLATION 系)に置き換え**、
  差分を contract 変更として spec に追記する。

### provider/auth/catalog(G1/G2/G4)

- **model catalog store(G1)**: bundled 静的 snapshot + user `models.json`
  (OpenAI-compat custom provider・`samplingParams` merge・`reasoning`/
  `maxTokens` metadata)を統合した catalog を `cheaper-runtime` に新設。
  user `models.json` の新配置は `<userConfigDir>/models.json`、形式は
  Pi `models.json` 互換(同 schema で読み、unknown key は無視)。
  `getAvailable` 相当(認証済み model のみ候補化)を担い、tier/explicit
  解決はこの catalog 上で動く。`<userConfigDir>/pi/` の
  `models.json`/`models-store.json` は読まない。
  **online refresh(models.dev fetch)は現行 `allowModelNetwork` が全 call
  site で未供給のため実質未使用の機構であり、v1 では実装しない**
  (offline 既定 + bundled/user file のみ。`PI_OFFLINE` 相当の opt-in fetch は
  Deferred)。
- **credential store + OAuth(G2)**: `<userConfigDir>` 配下の新 credential
  store(新規 file。`auth.json` 相当だが Pi 形式非互換)に永続化。
  `auth login/status/logout` の CLI contract を維持。api_key + env var +
  devin CLI 委譲を先に成立させ、OAuth(browser auth_url + device code、
  対象は github-copilot を含む利用実態のある flow)は provider ごとの独立
  sub-task として追加する。secret は stdout に出さない。Pi state は移行せず
  再 login を要求し、`auth status`/error で「旧 store は参照しない」旨を
  判別可能にする。
- **devin provider(G4)**: `cheaper-provider-devin` で Connect wire 656 行を
  port(protobuf-lite framing + gzip + JWT cache + context map)し、Rig
  custom `Wire`/`Decoder`/`Transport` に接着。Devin CLI 子プロセス
  (`auth login/status/logout`、`models list`)+ 隔離 data dir は contract
  通り維持。wire 互換は実 Devin CLI + endpoint でのみ検証可能なため、
  CI 外の手動 smoke として切り分ける。

### envelope / CLI 互換(`cheaper-cli`)

- `schemaVersion: 1`、success/utility/error/workflow の 4 形態を serde で
  厳密 serialize。stdout は `JSON.stringify(env) + "\n"` 相当(envelope 1
  JSON + 末尾改行 1 つのみ)。runtime/core/tools は stdout に書かない。
- **数値フォーマット規則**: serde_json の f64 出力(`1.0`・指数表現・`-0`)
  は `JSON.stringify` と byte 非互換。envelope/usage.jsonl/report の数値
  field(`costUsd`・`effectiveCost`・`effectiveCostPercentile`・
  `outputPerInput`・`estimatedSavingsUsd` 等)は JS 互換の canonical number
  formatter(整値は小数点なし・非有限値は `null`)で serialize する。
  実装手段は adapter でなく `cheaper-core` の serializer helper とする。
- **panic 境界**: Rust panic と tokio `JoinError` は envelope contract の
  一部とする。run/command 境界で `catch_unwind`(または task JoinError)を
  `INTERNAL_ERROR`(exit 70)error envelope に写像し、panic hook で stdout
  への panic 出力を抑止する。`cheaper hook` 経路では panic 含む全 error を
  target 適合の allow/exit 0 に変換する(fail-open 不変条件)。
  panic による envelope 無し終了(exit 101)は contract 違反とみなす。
- null 意味論: error envelope の meta は取得不能 field を `null` とし、
  `details`/`cause`/`savings` 等の optional field は「`undefined` なら key
  自体を省略」する現行 `errorEnvelope` の規則を再現(serde の
  `skip_serializing_if` で制御)。
- exit code は `ERROR_TABLE`(INVALID_REQUEST=2 … INTERNAL_ERROR=70)を
  そのまま写す。`cause` は CheaperError のときのみ `{code,message}`。
- `cheaper hook <event> --target <agent>` の 4 target adapter 出力と
  fail-open 契約(envelope 外の別 contract、copilot は非 0 を返さない)を
  byte 互換で port。
- `usage.jsonl` は `<userConfigDir>/usage.jsonl` へ 1 行 JSONL best-effort
  append。record 形(`kind:"run"|"count"`)を維持し、旧 schema record
  (`savings`/`turns`/`toolCalls` 欠落を許容)を `cheaper usage` が読める
  serde 構造(Option field 化)にする。
- builtin resource(workers/*.md・schemas/*.json・policy.yaml・version.json)
  は `include_dir!` でバイナリ埋込し、user/project override の探索順序
  (`CHEAPER_CONFIG_DIR` → `XDG_CONFIG_HOME/cheaper` → `~/.config/cheaper`、
  project `.cheaper/`)は不変。

### 配布

- `cargo build --release` + `lto` + `opt-level="s"` + strip で
  `x86_64-pc-windows-msvc` / linux-x64 / linux-arm64 の 3 target(Linux は
  musl static を検討)。asset 名(`cheaper-linux-x64`/`cheaper-linux-arm64`/
  `cheaper-windows-x64.exe`)・SHA256SUMS 生成/検証(`scripts/lib/
  release-artifacts.mjs` を流用)・`--version` probe は不変。launcher
  (sh/ps1)は無改修。
- `.github/workflows/build-executables.yml` を cargo matrix build に置換、
  `mise.toml` に Rust toolchain を追加。
- version canonical source を Cargo.toml workspace version へ移すか
  `packages/cli/package.json` を残すかは Deferred(下記)。`--version` 出力
  契約自体は不変。

### 段階計画との対応

設計上の依存関係として migration-plan のフェーズに対応付ける:

| Phase | 設計上の依存 |
| --- | --- |
| 0 PoC(go/no-go) | `cheaper-rig-adapter` の縮小版のみ(hook 粒度・repair 動線・custom Wire/Decoder 見通し・≤20MB 実測)。候補中立資産は作らない |
| 1 core | `cheaper-core` 完成。Rig 非依存で先行可能 |
| 2 tool-runtime | `cheaper-tools` 完成。`cheaper-core` のみ依存 |
| 3 runtime | `cheaper-rig-adapter` + `cheaper-runtime` + `cheaper-provider-devin`。G1/G2/G3/G4 を内含 |
| 4 CLI/workflow | `cheaper-cli` + `cheaper-workflow`。envelope 厳密互換・hook・usage.jsonl |
| 5 配布切替 | cargo matrix・asset 名不変・SHA256SUMS・launcher smoke |
| 6 TS 撤去 | `packages/*` TS 実装と Pi 依存の撤去、docs 更新 |

並走期間中は TS 版を CI で完全に生かし、Phase 0 通過後は TS 側への新機能
追加を凍結する。

## Non-Functional Constraints

- release binary(strip 後)≤ **20MB** を受け入れ閾値、**5-15MB** を目標
  (Phase 0 で最初に実測)。`rig`/`rig-core`/`rig-agent` は `=0.43.x` pin、
  transport は `rig-reqwest` opt-in、TLS は rustls 前提。
- upgrade は MIGRATING.md 手順の計画運用(四半期周期)とし、pin 更新を
  無計画に行わない。
- stdout 契約: envelope 1 JSON + 末尾改行のみ。診断・secret・auth 情報は
  stderr/永続 store に限定し stdout に出さない。
- workspace 外アクセス不許可・requested tools は vocabulary のみ・policy
  超過拒否・abort 時 rollback なし・未認証 model を選択しない、の runtime
  invariants は対応する Rust crate で同一責務を持たせる(overview.md の
  invariant 表を写す)。
- `cheaper-core` の JSON Schema 検証は `jsonschema` crate(draft 2020-12)、
  token 計測は `tiktoken-rs`(o200k_base)に置換。worker/policy/models.yaml
  の YAML は `serde_yml` or `serde_norway`。

## Risks and Trade-offs

| リスク | 緩和 |
| --- | --- |
| Rig 0.x breaking change 頻発 | `=0.43.x` pin + adapter 1 crate 隔離 + 計画 upgrade |
| Rig docs 陳腐化(custom provider ガイド等) | docs.rs API reference + source を正とし、Phase 0 で custom Wire/Decoder を実際に書いて見積確度を上げる |
| auth/OAuth(G2)の見えないコスト | api_key + env + devin CLI 委譲を先に成立させ、OAuth は provider 毎に独立 sub-task 化。pi-ai の flow 実装(MIT)を spec 源として参照 |
| devin wire 互換(G4)が実 endpoint 依存 | PoC 期に設計検証(go 条件 d)。移植後も TS 版 devin を残して differential 比較。最悪 devin を初期リリースから外す切り下げ判断は事前合意対象 |
| Ajv → jsonschema で schema error message が変わる | `instancePath` のみを契約、message 本文・件数・code は非契約と明文化(spec 準拠) |
| abort 分類・PARTIAL_MUTATION・transientRetries 等の silent regression | differential golden を必須 gate とし `meta` 全 field を比較対象に含める |
| Windows process tool / env / path 差分 | Phase 2 で Windows runner 実検証を完了条件に。TS 実装の Windows 分岐を仕様化してから port |
| 並走期間の TS/Rust 二重保守 | Phase 0 通過後の TS 側新機能凍結。修正は差分のある層のみ |
| `maxToolCalls` / `maxTurns` の abort 意味を Rig hook で再現できない | `rig-agent` hook 表現力が不足と判明した場合のみ `rig-core` の Model/streaming に退避し自前 loop に切替(ADR の fallback 設計) |

## Deferred Design Concerns

- darwin-* target の Release 追加 | 現行未提供で移行の完了条件ではない |
  Rust 化後に launcher の darwin acquisition 分岐解放とセットで再検討 |
  macOS 利用者が sibling executable 前提のまま残る。
- version canonical source(package.json → Cargo.toml 移行の是非) |
  `check-version.mjs`/`set-version.mjs`/`version.json`/`apm.yml` の整合経路に
  波及 | 配布切替(Phase 5)までに決定 | version 宣言不一致で
  `mise run check` が通らない。
- `TOKENIZER_IMPLEMENTATION` 表記(tiktoken-rs の実装名を出すか互換表示か) |
  `details.tokenizer` の consumer 影響が不明。normalize 規則で version は
  除外済み | CLI 移植時に決定 | 親 agent が identity 文字列に依存していれば
  破壊的変更。
- APM bundle 同梱 `cheaper.mjs`(JS fallback)の廃止判断 | offline preseed UX
  と `pnpm bundle` 検証経路の再設計を伴う | Phase 6 時に判断 | Node 前提
  経路が残り移行目的が半減する。
- workflow engine 移植のフェーズ位置(Phase 4 同居 vs 独立) | Rig 非依存で
  後回し可能 | plan 策定時にタスク分割として決定 | `workflow run/interface`
  が TS 版に残る期間が延びる。
- models.dev catalog の online refresh 範囲(bundled 静的 + user `models.json`
  のみに縮小するか、`PI_OFFLINE` 相当 policy まで移植するか) | 実装量と
  offline 契約のトレードオフ | runtime 移植フェーズの設計時 | catalog 鮮度の
  運用ルールが曖昧なまま残る。
- OAuth 対象 provider の初期 scope(openai-codex/anthropic/github-copilot/
  openrouter/kimi-coding/radius の全移植 vs 段階導入) | flow 毎の仕様確認
  コスト | Phase 3 の sub-task 分割時 | 対象外 provider は再 login 不可期間
  が生じる。
- devin provider の初期同梱可否 | wire 互換の検証手段が実 endpoint 依存 |
  Phase 3 の devin 移植着手時 | 切り下げ時は `auth status` で unsupported
  表示等の UX が必要。
- Rig PoC 不達時の Goose GDK ピボット | 本設計は Rig 前提 | Phase 0
  go/no-go ゲート | ピボット時は adapter 境界設計のみ流用可能で Rig 固有
  記述の見直しが必要。

## Review Findings(non-blocking、planning への申し送り)

review-protocol(Reviewer A/B → Validator → Judge)で CONFIRMED された blocking
issue 10 件は本文へ反映済み。以下の non-blocking 指摘は plan/task 分割時に
吸収する:

- ISS-11: Rig `default_max_turns` には十分大きな値を渡し、cheaper hook を正とする(意味論固定節に反映済み。残課題: 具体値の選定は実装時)
- ISS-14(残): models.json の merge 規則詳細(builtin 上書き可否等)は runtime 移植時に fixture で固定
- ISS-15: schema error item の field 構成は `{instancePath,message}` が現行。`code` を追加するなら新契約として spec 側で明示
- ISS-18: 実 provider 限定の negative differential(totalTokens 未報告 fallback・cache token 差・429 分類)は Phase 3 以降の検証手順に追加
- ISS-19: (a) `auth status` の `result.providers[]` 再現規則、(b) credential/catalog file 非存在時の `models`/`run` 動作(build smoke の `models` exit 0 を維持)、(c) YAML 方言差(serde_yml 優先、serde_norway は 1.1 boolean 差に注意)、(d) `savings.returnedTokens` 計算は key 順序保持の serialize(`preserve_order` or 自前)で行う
- ISS-20: tokenizer identity は `result.tokenizer`/`details.tokenizer` の両経路を除外済み。`implementation` 表記の最終決定は Deferred のまま
- ISS-21: CI matrix workflow に release binary サイズ計測ゲート(≤20MB assert)を追加(Phase 5 タスク)
- ISS-22: `cheaper usage --compare-model` の catalog 参照経路(runtime catalog→user pricing.json→bundled snapshot)と usage.jsonl の additive-only 拡張制約を CLI 移植時に明記
- ISS-23: OAuth Verification の対象(github-copilot のみ vs ADR の openai-codex/anthropic 含む)は Deferred の OAuth scope 決定時に整合
- ISS-24: process tool の列挙完全化は反映済み(timeout/heartbeat/killGrace/PID reuse/`node` 明示 error)

## Verification Notes

- [ ] Phase 0 go 条件 5 項目(≤20MB 実測・`WorkerRunMeta` 粒度・repair 動線・custom `Wire`/`Decoder` 見通し・3 OS ビルド)の実測確認。**追加で以下を go 条件に含める**: (a) hook 経由で error/abort 経路の usage/turns snapshot が取れること、(b) `InvalidToolCallAction::Retry` で unknown tool call が error feedback → loop 継続すること、(c) Rig error surface から HTTP status を分類できること、(d) streaming run の `cancel()` で in-flight abort が reason 分類可能なこと、(e) `panic` boundary(catch_unwind→INTERNAL_ERROR)が機能すること
- [ ] `Cargo.toml` の `rig` 系依存が `=0.43.x` pin で、Rig 型参照が `cheaper-rig-adapter`(+ `cheaper-provider-devin`)に限定されることの code review
- [ ] fake/scripted provider fixture による TS/Rust binary 間 differential golden 20-40 case が normalize 規則内で全パス。**fixture DSL を先行定義する**: 応答 item 列に加え、遅延(delay ms)・stream 途中 error(HTTP status/retryable 分類可能な形)・hang-until-abort・abort 応答・usage delta を表現できる scripted provider schema。fake provider は catalog 登録(`cheaper-fake/fake-model` を getAvailable に露出)・keyless credential・scripted stream の 3 層セットとして注入する(§adapter seam)
- [ ] normalize 規則(除外: `meta.durationMs`・`meta.execution.operationMs`/`activeMs`/`overlapMs`/`queueMs`(wall-clock 由来の非決定 field。`execution` の決定的 field `operations`/`byState`/`byTool`/`maxConcurrency` は比較対象)・`usage.costUsd`・`result.tokenizer.*`・`details.tokenizer.*`・`error.message` 本文(Node `err.message` と Rust `io::Error` Display の系統的差異を吸収。`error.code`/`exitCode` は厳密比較)・stderr 診断文言・JSON キー順・schema error の message 文言・件数・code / 厳密比較: `usage` token field(`input`/`output`/cache 系)・`turns`/`toolCalls`/`rejectedToolCalls`/`repairs`/`transientRetries`/`changes`/`savings`/`result`)の fixture 先行定義。`costUsd` は TS=provider 応答値 vs Rust=pricing 層値で系統的に異なるため除外(spec 側の「meta 全 field 比較」は normalize 適用後の意味と解釈し、spec の正規化規則記述を本規則に揃える)
- [ ] envelope `schemaVersion:1` byte 互換・exit code 表・hook 4 target byte 一致・usage.jsonl 新旧 record 互換を e2e/contract test で確認
- [ ] `meta.usage.costUsd` が pricing 層算出であること、新旧 record の口径混在が usage guide に記載されること
- [ ] `cheaper auth login github-copilot`(OAuth)→ run まで成功し、credential が新 store に永続化・`<userConfigDir>/pi/` を参照しないこと
- [ ] 同一 asset 名・SHA256SUMS・`--version` probe で Release が組め、無改修 launcher が Rust binary を取得・検証・exec できること
- [ ] Ollama 実 provider コンテナで built-in worker 3 種が成功 envelope を返す Rust e2e が通ること
