---
id: "IMPL-034Dk4nQ7wXzVbNmY2hJrT"
type: "impl"
status: "in-progress"
title: "Phase 3: port runtime adapter providers"
created: "2026-10-08"
updated: "2026-10-08"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "directory"
        path: "crates/cheaper-runtime/"
      - type: "directory"
        path: "crates/cheaper-rig-adapter/"
      - type: "file"
        path: "docs/impl/ir/phase-3-port-runtime-adapter-providers.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/phase-3-port-runtime-adapter-providers.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/phase-1-port-cheaper-core.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/rust-rig-runtime-migration-architecture.md"
  derived-by: []
  verifies:
    - "docs/test-specs/rust-migration-auth-catalog.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Phase 3: port runtime adapter providers

## Summary

PLAN-034BepZr0oX3bWeqVXNk91(`docs/plans/phase-3-port-runtime-adapter-providers.md`)
の slice 実装 IR。本 file は slice 毎に section を追記する。

### Slice T2 — catalog store + credential store + auth service + plugin contract

`crates/cheaper-runtime` を新設し、`packages/runtime-pi` の
model catalog / credential / auth / plugin contract 層を Rust に移植した
(plan step 2-8 相当)。Rig・HTTP client(`reqwest`)・CLI wiring は
含まず、Rig 型は runtime に一切入れない(T1 adapter が持つ境界)。

module map:

- `src/catalog/mod.rs` — `CatalogStore`。bundled + plugin + user
  `models.json` の 3 層 compose(`applyModelsJson` → `applyExtension` →
  `modelOverrides` の pi-coding-agent `provider-composer.ts` port)。
  `get_available`/`get_model`/`get_supported_model` を公開し、未知・
  スコープ外 api 種別は `INVALID_MODEL_CONFIG` で明示 error。
  `ProviderEntry` に provider 単位の auth metadata(env 規則・configured
  apiKey・headers・oauth 宣言・plugin hook)を集約する。
- `src/catalog/schema.rs` — Pi `models.json` 互換 schema。BOM・
  `//` comment・trailing comma を剥がし draft 2020-12 JSON Schema で
  validate 後 deserialize(`ModelsConfigSchema` port、unknown key 許容)。
- `src/catalog/bundled.rs` — pi-ai `dist/providers/data/` から
  39 provider data file + `.manifest.json` を `include_str!` で byte
  同一 embed + `BUILTIN_PROVIDERS` 静的 metadata 表(40 provider、
  `builtinProviders()` 宣言順。data file を持たないのは radius のみ)。
- `src/auth/store.rs` — `CredentialStore`。
  `<userConfigDir>/credentials.json`(version+providers の新規 schema、
  Pi `auth.json` 非互換)に temp-file+rename で atomic write。
  `<userConfigDir>/pi/`・`PI_CODING_AGENT_DIR`・`.cheaper/` は一切読まない。
- `src/auth/api_key.rs` — `resolve-config-value.ts` port
  (`$VAR`/`${VAR}`/`$$`/`$!`/`!command` template 解決 + cache 付き
  command 実行)と `composeApiKeyAuth` の resolve/check port
  (`resolve_api_key_auth`/`check_api_key_auth`/`resolve_headers_or_throw`)。
  `ProviderEnv` trait で env/fileExists/homeDir を注入可能にした。
- `src/auth/service.rs` — `AuthService`。`status`(`providers[]`+
  `credentials[]`)/`login`/`logout`/`logout_all`/`get_auth`(
  `resolveProviderAuth` port)+ `AuthInteraction`/`OAuthFlow` seam。
  `ProviderAuthInfo`/`LoginResult`/`LogoutResult` は credential 値を
  含まない shape。
- `src/plugin.rs` — `CheaperProviderPlugin` contract(
  `register(registry, context)`)、`ProviderRegistry`、
  `PluginProviderRegistration`、`PluginAuth`(login/to_auth/check/
  refresh/logout)、`PluginCatalog`(refresh_models)、
  `ProviderPluginContext`(user_config_dir + env)。
- `src/runtime.rs` — `Runtime` composition point
  (`createModelRuntime` 相当の offline 版)。catalog + auth + plugin
  registry + oauth flow registry を束ね、`get_available` の認証
  predicate wiring と `resolve_model_auth` を提供する。
- `src/run.rs`/`repair.rs`/`meta.rs` — T3/T4 stub(module 宣言のみ)。

### Slice T1 — Rig adapter(`cheaper-rig-adapter`)

`crates/cheaper-rig-adapter` を新設し、`packages/runtime-pi` の
Rig 接線責務(session 構築・tool 接着・hook→accumulator・error 写像・
completion-call transient retry・streaming run 面)を Rig `=0.43.0`
上に移植した(plan step 9-14 相当)。T3 の run orchestration
(`run_worker`・repair loop・meta 構築)は含まず、再利用 primitive のみ
を提供する。

module map:

- `src/accumulator.rs` — `Accumulator`/`RunSnapshot`/`UsageSnapshot`。
  hook 発火で更新される唯一の集計源(turns/usage/invalid/tool/
  transientRetries/repairs)。`toolCalls`/`rejectedToolCalls` は
  `attach_budget` で cheaper-tools の `ToolBudget` live view を読む
  — admission 時点計上・access-denied 両方計上の TS `accounting.ts`
  contract を budget が所有するため outcome 観察で二次推定しない。
- `src/hooks.rs` — `AccountingHook`。`on_outcome` で completion usage
  と tool `details`(`ToolResultContext` slot 経由)を回収、
  `on_model_turn_finished` で受理 turn を計上(`FinishReason::
  ToolCalls` / content 内 call で `last_turn_tool_calls` 判定)、
  `on_invalid_tool_call` で `InvalidToolCallAction::Retry` + corrective
  feedback を返す(計数のみ、tool counter は触らない)、
  `on_run_settled` で `PromptResponse.usage` に reconcile(rollback
  された invalid-tool turn の課金分を回収)。
- `src/error.rs` — `ErrorClass`(429→RateLimit/5xx→Server/無応答・
  retryable→Transport/他→Permanent)と `PromptError`/`ProviderError`/
  `ErrorReport`/`StreamingError`/`AbortReason` → `CheaperError` 写像。
  provider 接線失敗は run.ts parity で一律 `PROVIDER_FAILURE`、分類は
  `details.class`/`retryable`/`httpStatus`/`providerRequestId` に残す。
  `AbortReason.code()` の逆引きで `MAX_TURNS_EXCEEDED` 等の code を
  復元する(T3 の policy abort 経路がそのまま使える)。
- `src/retry.rs` — `RetryingCompletion`(`Serve` handler、
  `ModelAdapter` と同じ completion seam)。transient 分類のみ retry、
  `transientRetries` 計上 + 指数 backoff、streaming は最初の item を
  emit する前の失敗に限定(先読み item を chain で繋ぎ戻す)。run 全体
  を retry しないので tool 副作用の二重実行は起きない。
- `src/tools.rs` — `cheaper-tools` `ToolRuntime` → `DynamicTool` 接着。
  canonical name/description/schema を保持し callback は
  `runtime.execute` に委譲(sandbox/budget/scheduler/journal は
  cheaper-tools 側の所有物)。`AbortSignal` は `ToolContext` scope 経由
  で dispatch に届ける。`CanonicalToolResult` → `ToolOutput`、
  `ToolError` → `ToolExecutionError`(`CheaperError` code →
  `with_code` で machine code 保持・kind 近似写像)。
- `src/agent.rs` — `RigSession`。`AgentBuilder` に preamble・
  `default_max_turns`・`tool_concurrency(1)`・
  `max_invalid_tool_call_retries`・`model_route_handler` 登録 +
  `set_model_label` で retry route を選択。`stream_run` が唯一の公開
  実行面(blocking API 非公開)。`build_system_prompt`/`build_user_prompt`
  は run.ts の文字列完全再現で唯一の実装(T3 はここを呼ぶ)。
- `src/run_stream.rs` — `RunStream`(adapter 固有 `RunEvent` 列)。
  `MultiTurnStreamItem`/`StreamingError` は内部 detail で公開 signature
  に出さない。`AbortSignal` と `inner.next()` を biased select し、
  abort 時は Rig stream を drop + reason 由来の終端 `CheaperError` を
  1 件 emit。`CompletionCall` item から per-call usage、`ToolResult`
  item に hook 経由の `details` を添付、`ModelTurnRetried` で
  provisional/rollback を通知、`FinalResponse` → `Finished`。

### Slice T3 — `run_worker` orchestration(abort/grace/output/meta/repair/panic 境界)

`crates/cheaper-runtime` に `runWorker` 相当の orchestration を実装した
(plan step 18-25 相当)。T2 の `Runtime`/`ResolvedAuth`/`CatalogModel`
と T1 の `RigSession`/`Accumulator` を `WorkerSession`/`SessionFactory`
seam 越しに結線し、abort 一元化・prompt grace・structured output
抽出・bounded repair・`WorkerRunMeta` 構築・`WorkerRunError`(runMeta
同梱)・panic/`JoinError` 正規化を run 経路で組み立てた。fixture DSL
本体(T4)と production provider model factory(T5)は含まず、T3 検証用の
最小 scripted fake seam(`fake_provider`)のみを同梱した。

module map:

- `src/run.rs` — `run_worker`。`checkCapabilities`(run 外・生 error)
  → `RunAbort` + timeout task + 外部 signal listener 構築 →
  `RunCtx`(TS の try 範囲クロージャ変数相当)で invocation/scope/
  model/auth/session を段階解決 → `prompt_with_abort_grace` →
  output 抽出 → repair → `build_meta`。timeout task・signal
  listener・scope 破棄を finally 相当で必ず実行する。session seam
  は `WorkerSession`(stream_run/abort/snapshot)+ `SessionFactory` +
  `CompletionModelFactory` + `RigSessionFactory` — 呼び出し側は
  opaque session のみ受け取り signature に Rig 型は出ない。
  `ExecutionBinding`(invocation `ToolRuntime` + owner + parent
  lease)は cheaper-tools の同名 struct が scope 側 runtime を表す
  ため run 経路で別定義した(Phase 2 命名差異)。
- `src/abort.rs` — `RunAbort`(first-reason-wins)。reason を
  `OnceLock` で原子的に確定し、`AbortController` 側に伝搬。
  `RunAbortReason`(timeout/signal/maxTurns/maxToolCalls)が
  `ABORT_CODES` 写像と `details.reason` 文字列を所有する。
- `src/grace.rs` — `prompt_with_abort_grace`。stream 終端を監視し、
  abort signal 到達時に `session.abort()` を呼んでから既定
  `DEFAULT_ABORT_GRACE_MS`(5000ms)の grace で settled error を待つ
  (`PromptOutcome::Aborted`)。grace 超過で provider が cancel を
  無視しても `PromptOutcome::Ignored` で run を打ち切る。
- `src/output.rs` — `extract_output`。末尾 textual output の
  code fence 除去 → JSON parse → object 要求 → `CompiledSchema`
  validate。failure は `ExtractFailureReason`(
  `NoText`/`NotJson`/`NotObject`/`SchemaViolation`)に分類。
- `src/repair.rs` — `repair_output`。raw output + schema error を
  corrective prompt に載せ、attempt 毎に tool-less session を
  `SessionFactory` 経由で新規生成。`repairAttempts` 上限、
  errors 20 件・rawOutput 2000 文字 cap、run abort で即中断、
  尽きれば `INVALID_OUTPUT`(details = errors + rawOutput)。
  attempt 成功/失敗に関わらず session snapshot を返し呼び出し側で
  run snapshot に `merge_snapshots` で畳む。
- `src/meta.rs` — `build_meta`/`merge_snapshots`/`WorkerRunMeta`(
  model/usage/turns/toolCalls/rejectedToolCalls/repairs/
  transientRetries/changes/execution/savings)。token・turn・
  tool 計数は main + repair snapshot の畳込値のみが正源で、
  `costUsd` は `resolved.pricing`(cheaper 価格)× token から算出
  (provider 応答値は使わない)。savings は
  `sourceTokens`/`returnedTokens`/`avoidedContextTokens`/
  `directlyComparable` の TS shape。
- `src/run_error.rs` — `WorkerRunError`。`CheaperError` + optional
  `PartialRunMeta`(usage-bearing failure は full meta、setup 段は
  `{execution}` のみ)を保持し `exit_code`/`run_meta` accessor を
  公開。large Err 回避のため内部を Box 化。
- `src/panic_boundary.rs` — `catch_run_panic`
  (`AssertUnwindSafe` + `std::panic::catch_unwind` の async 版)で
  panic payload を `INTERNAL_ERROR` に写し、`JoinError` も同 code に
  正規化する。
- `src/fake_provider.rs` — `FakeSessionFactory`/`FakeResponse`。
  Complete/Events/Fail/WaitAbort/Hang/ToolCalls/Panic の scripted
  応答で T3 経路(4 abort 分類・grace・repair・partial mutation・
  panic)を実 session 経路で検証する。session spec・prompt を記録し
  repair prompt・spec(tool 無し・schema hint)も検証可能。

### Slice T4 — fixture DSL + fake provider 3 層 seam + replay 検証基盤

plan step 15-17 + 34 相当を `crates/cheaper-runtime` に実装した。
T3 の `FakeSessionFactory`/`FakeResponse` を base に、fake seam を
G7 の 3 層構成へ拡張した:

- **catalog 層**: `CheaperFakePlugin`(plugin registration)が
  `cheaper-fake/fake-model` を catalog `get_available` に露出する。
  fixture の `model` 節(id/api/cost)で bootstrap model を差し替え
  られる。
- **keyless credential 層**: plugin `api_key` は literal
  `"fake-key"` が既定で、env/credential 無しに configured+resolve
  済みになる(TS testsupport `apiKey.resolve → {auth:{}}` 相当)。
  fixture の `provider.api_key` を `"$VAR"` template に変えれば env
  解決経路、`auth_logins` で `AuthService::login`(api_key method)
  永続化 → stored credential 経路を辿れる。
- **scripted stream 層**: `ScriptedSession`/`ScriptStep` +
  `FakeResponse::Scripted`。fixture DSL の step を `stream::unfold`
  state machine で replay する。`error` step の retry 分類
  (status/retryable → transient/permanent)は adapter `classify` /
  `status_details` の parity mirror を fake 側で担う(session seam
  差し替えは adapter 内部の transient retry wrapper を含むため)。

`src/fixture.rs` は fixture DSL の serde schema・parser・変換
(worker .md 生成・`RuntimePolicy` 部分 override・plugin/env/
sessions/`SessionSpec` retry 予算)を所有する。`tests/fixtures/` に
JSON corpus 24 件、`tests/run_fixture_test.rs` に `run_worker`
end-to-end replay harness を置いた。

### Slice T6 — OAuth flow contract + github-copilot flow

`auth/oauth/` に pi-ai `auth/oauth/*` の flow contract + shared
helper と github-copilot flow を実装した(plan step 32-33 相当)。
`AuthEvent` union・injectable `HttpClient`/`FlowSleeper` seam・
RFC 8628 device-code poll・429 rate-limit retry・
`GitHubCopilotFlow`(device login → Copilot token 交換 → models
fetch + individual policy enable → credential 化、`to_auth` の
proxy-ep 由来 base_url 導出、refresh)を揃え、`Runtime::create` が
`builtin_oauth_flows()` で `"github-copilot"` flow を自動登録する
(引数 `oauth_flows` は同名 id の override として test 注入 seam)。
`AuthInteraction` に `event`/`is_cancelled`、`AuthPrompt` に
`Text`/`ManualCode`/`Select`、`ResolvedAuth` に `base_url` を追加。
browser+loopback flow(openai-codex/anthropic 等)と CLI wiring は
Phase 4 scope — T6 では contract 面のみ提供する。

module map:

- `src/auth/oauth/mod.rs` — `AuthEvent`(info/auth_url/device_code/
  progress)、`HttpClient` seam(`UreqHttpClient` 実装、`fetch`
  parity: 非 2xx も response として取り出す)、`FlowSleeper`/
  `ThreadSleeper`、`poll_device_code_flow`(`pollOAuthDeviceCodeFlow`
  port — interval min 1s・default 5s・slow_down +5s/server interval
  優先・deadline・cancel・slow_down 後 timeout は WSL clock drift
  message)、`fetch_json`/`fetch_with_rate_limit_retry`(429
  exponential backoff + `Retry-After` 優先)、
  `builtin_oauth_flows()`。
- `src/auth/oauth/github_copilot.rs` — `GitHubCopilotFlow`。
  `loginGitHubCopilot`/`refreshGitHubCopilotToken`/
  `fetchGitHubCopilotModels`/`enableGitHubCopilotModel`/
  `getGitHubCopilotBaseUrl`/`copilotEnterpriseDomain` の port。
  client id `Iv1.b507a08c87ecfe98`・copilot headers・api version
  `2026-06-01` を TS と同値で保持。

### Slice T5 — Devin Local provider(`cheaper-provider-devin`)

plan step 26-31 相当を `crates/cheaper-provider-devin` に実装した。
`packages/runtime-pi/src/providers/devin/`(vendored pi-devin、MIT —
crate 直下に `NOTICE` を同梱)の Rust port。module map:

- `src/wire.rs` — protobuf-lite encoder/decoder(varint/tag/length-
  delimited/fixed64-double/timestamp)+ Connect `flags|len(4B BE)|
  payload` frame(`0x01` gzip・`0x02` EOS trailer)。byte-level parity。
- `src/metadata.rs` — `buildMetadata` field 順 byte 一致(1 ide /
  2,7 version / 3 apiKey / 4 "en" / 5 os / 9 requestId / 10
  sessionId / 12 ide / 16 timestamp / 21? jwt / 25 triggerId / 26
  "Unset" / 28 ide)。`ide="devin-desktop"` 固定 — Devin Local model
  gate。macOS product.json → `windsurfVersion`、fallback `3.6.27`。
- `src/jwt.rs` — `GetUserJwt` unary POST(`application/proto` +
  `Connect-Protocol-Version: 1`)、応答 field 1 `eyJ*` 抽出、`exp`
  claim 由来 expiry(fallback now+600s)、60s margin cache +
  `(host, apiKey)` 単位 in-flight dedup(tokio Mutex で mint 全体を
  serialize)、30s timeout、`clear_cached_user_jwt`。
- `src/context_map.rs` — Rig `CompletionRequest` → chat message/
  tool 変換(system/user/assistant/tool、image は Base64/String/Raw
  のみ、URL/FileId は wire 不可のため省略)。assistant text は
  `"\n"` join、tool call args は `JSON.stringify` 済み string。
- `src/stream.rs` — `buildGetChatMessageRequest`(field 3 prompts・
  7=5・8 config・10 tools・16 cascade・21 modelUid・22 promptId)+
  `collapseSystemIntoUser` + response proto → `ChatEvent` decode
  (3 text/9 reasoning/6 tool_call{1id,2name,3args}/5 finish varint
  10|11|1|3/28 usage)+ `deframe_connect`(partial chunk・multi-frame・
  gunzip・EOS trailer error 抽出)。
- `src/cli.rs` — `DevinCliRunner` seam + `ProcessCliRunner`
  (argv 分離 `Command`、stdout/stderr drain thread、timeout kill、
  inherit-stdio login)、`devin_child_env`(`XDG_DATA_HOME`、Windows
  は `LOCALAPPDATA` 併用)、`DEVIN_CLI` override、ENOENT →
  `DevinCliNotFoundError`。
- `src/credentials.rs` — `<userConfigDir>/providers/devin/data/
  devin/credentials.toml` parse(`key = "value"` 行正規表現)+
  `devin_auth_status`(CLI authoritative、未到達時 toml fallback)+
  `login_with_devin_cli`(stale → `auth login` inherit stdio → toml
  再読、未生成は明示 error)+ `logout_devin_session`(best-effort)。
- `src/models.rs` — `devin models list --format json` → family →
  `ModelDefinitionJson`(priority/fast/thinking-1m 除外・全除外時全
  variant fallback・明示 null level・preferred default
  `high→medium→max→xhigh→low→minimal→off`・cost summary parse・
  cache read/write = input ×0.1/×1.25)+ `FALLBACK_MODELS` 4 件 +
  `resolve_model_uid`。
- `src/rig_provider.rs` — Rig `Wire`(`DevinWire`、encode は pure —
  JWT mint は transport 側で field 21 append)+ `Transport`
  (`DevinTransport`: cached JWT mint → Connect frame POST →
  `deframe_connect`)+ `Decoder`(`DevinDecoder`: `TextPart`/
  `ReasoningPart`/`CallPart` open/close 遷移、EOS trailer →
  `out.end(Finish)`、EOS 未到達終端 → `ProviderError::Truncated`)。
  `DevinCompletionModelFactory` が `CompletionModelFactory` を実装し
  `CompletionModel::from_rig(Model::new(wire, transport))` +
  `.with_max_tokens` で runtime seam へ接続。host 解決は
  `DEVIN_API_SERVER_URL` env → credential `api_server_url`
  (`ResolvedAuth.base_url`)→ `https://server.codeium.com`。
- `src/plugin.rs` — `DevinProviderPlugin`(id `devin`、name
  `Devin Local`、api `devin-local`、bootstrap `fallback_models`、
  `subscription: true`、`PluginAuth` = CLI login/status/logout +
  toml→`StoredCredential::Oauth`(extra `tomlPath`)+ `to_auth` が
  `ResolvedAuth{api_key, base_url}` を返す、`PluginCatalog` =
  `devin models list`)。
- `src/tests/` — 51 unit test(wire/Connect/context_map/stream/
  models/cli/credentials/jwt cache/plugin registration)。

### Validation (Slice T5)

- `mise exec -- cargo test -p cheaper-provider-devin` → 51 passed /
  0 failed。
- `mise exec -- cargo clippy -p cheaper-provider-devin
  --all-targets -- -D warnings` → clean(`ProviderError` は rig 所有の
  大きな Err 型のため `result_large_err` を helper 2 箇所で局所抑制)。
- `mise exec -- cargo fmt --check` → workspace 全体 clean。
- `mise exec -- cargo test -p cheaper-rig-adapter -p cheaper-runtime
  -p cheaper-provider-devin` → 全パス(provider 51 + adapter 14 +
  runtime 86、run_fixture_test 含む)。
- devin 実 endpoint smoke(auth login/status/logout・models list・
  run)は未実施 — 実 Devin account/CLI 環境が必須のため CI 外手動の
  まま残す。

### TS との divergence / 残差 (Slice T5)

- `prompt token 概算(field 4)`: TS `text.length`(UTF-16 単位)vs
  Rust `chars().count()`(Unicode scalar)— Astral 面で差が出得る
  が概算値のため実害なし。
- `close_call` の malformed JSON args は `MalformedToolInput` error
  として surface(TS は `JSON.parse` 失敗を飲み込んで途中値を残す
  — ここだけ厳格化)。
- `content_filter` finish reason は Rig `FinishReason::ContentFilter`
  として保持(Pi `stopReason` への "stop" 潰し込みは行わない)。
- child env の base は process env + scoped `DEVIN_CLI` のみ
  (scoped env accessor は列挙不可のため明示取込 — TS
  `context.env` 相当)。
- `SessionSpec.output_schema` の provider 配線は未着手(T3 の
  残差どおり — repair は prompt + attempt 上限で成立)。
- `DevinCompletionModelFactory` を runtime の `RigSessionFactory`
  へ組み込む composition root 配線は Phase 4 scope。
- 実 endpoint smoke 未実施(上記 Validation 参照)。

## Context

slice T1 は T2(cheaper-runtime の catalog/auth 層)と独立に並行作業。
root `Cargo.toml` の workspace dependencies は T1 が所有し
(`rig = "=0.43.0"`・`futures = "0.3"` を追加)、`Cargo.lock` は rig
crate graph(optional dep を含む解決済み全体)を取り込んだ。
`cheaper-core`/`cheaper-tools`/`cheaper-runtime` は Rig 型を import
しない(isolation 原則 — Rig 参照は `cheaper-rig-adapter` と後続の
`cheaper-provider-devin` のみ)。

slice T2 は T1(rig-adapter)と独立に並行作業。root `Cargo.toml` は
rig-adapter 担当 agent が所有するため変更せず、依存は
`crates/cheaper-runtime/Cargo.toml` に直接宣言した
(`cheaper-core` path dep + workspace `serde`/`serde_json`/`jsonschema`/
`tokio` + dev-dep `tempfile`)。

上流 contract:

- `docs/test-specs/rust-migration-auth-catalog.md` — G1(catalog)/
  G2(credential)/G3(api_key+env)/G5(plugin seam)・secret 非開示・
  `<userConfigDir>/pi/` 不読・silent fallback 禁止。
- `packages/runtime-pi/src/auth.ts` — `ProviderAuthInfo` 形状
  (`providerId`/`name`/`methods`/`subscription`/`configured`)。
- `pi-coding-agent/dist/core/provider-composer.js` —
  `applyModelsJson`/`applyExtension`/`applyModelOverride`/`mergeCompat`/
  `composeApiKeyAuth`/`composeOAuthAuth`/`rawModelHeaders`。
- `pi-coding-agent/dist/core/resolve-config-value.js` — template/
  command 解決・error message 文面。
- `pi-ai/dist/auth/resolve.js`・`models.js` — `resolveProviderAuth`/
  `checkProviderAuth`(stored credential が provider を所有、
  handler 無し credential は env fallback しない)。
- `pi-ai/dist/providers/data/` — bundled model snapshot。

## Implementation

### Changes (Slice T2)

- `crates/cheaper-runtime/` 新設。`Cargo.toml` + `src/` 9 file +
  `tests/` 3 file + `src/catalog/data/` bundled JSON 40 file。
- bundled snapshot: `node_modules/.../pi-ai/dist/providers/data/` から
  byte 同一 copy(39 provider file + manifest)。`include_str!` で embed
  し、`{<api>: {<modelId>: ModelJson}}` を group/model 宣言順に
  flatten(`preserve_order`)。`openai-codex.json` は data file が
  存在するため `BUNDLED_DATA` に含めた(当初 remote-only と誤認)。
- user overlay: `<userConfigDir>/models.json` を `load_models_json` で
  読み、parse/schema error は `CatalogStore::errors()` に記録して
  bundled/plugin 側で構築を続行(TS `ModelConfig.error` +
  `compositionErrors` 相当)。compose error も同様に記録し、builtin は
  untouched base fallback、custom/plugin-only は provider を drop。
- provider id 順: builtin(宣言順)→ user config(登場順)→ plugin
  (TS `providerIds()` Set 順)。
- `SUPPORTED_APIS = ["anthropic-messages", "openai-completions",
  "openai-responses"]` + plugin 宣言 api。これ以外の api を持つ
  model は `ensure_supported_api`/`get_supported_model` で
  `INVALID_MODEL_CONFIG`(details に provider/id/api/supportedApis)。
- credential store: `<userConfigDir>/credentials.json`
  (`{"version":1,"providers":{id: cred}}`)。temp file +
  `set_permissions(0600)`(unix)+ rename で atomic write。`providers`
  map は file 宣言順を `Vec<(id, cred)>` で保持し、
  `listCredentials`/`status().credentials` の順序を TS auth.json の
  object key 順(挿入順)に揃えた。oauth credential は
  `availableModelIds`(github-copilot filter)と任意 extra claim
  (devin `tomlPath` 等)を保持。
- auth service: `methods` は `api_key_name` 有無 + oauth advertise
  (plugin auth 存在 or builtin oauth 宣言 + 登録済み flow)。
  `subscription` は advertise された oauth method のみに紐付けた
  (TS は宣言の存在で判定するが、method を出さない provider に
  subscription を出しても参照されない — Rollback 規則の延長)。
- `get_auth`: stored oauth → plugin `to_auth` or flow(refresh は
  `OAUTH_MINIMUM_VALIDITY_MS` = 5 分 window)+ handler 無しなら None;
  stored api_key → credential key 優先; 未登録 → configured apiKey
  template 解決(`resolveConfigValueOrThrow` と同じ error message)→
  env table 解決(anthropic 3 var / vertex ADC / bedrock ambient)。

### Decisions During Implementation (Slice T2)

- **credential file 名**: `<userConfigDir>/credentials.json` を採用。
  Pi `auth.json` とは別 schema(`version`/`providers` 直下)で、
  旧 file・`<userConfigDir>/pi/`・`PI_CODING_AGENT_DIR` を一切読まない
  ことを unit test(`credential_store_never_reads_legacy_pi_state`)で
  固定した。
- **env accessor**: pi-ai `authContext`(`env(name)`/`fileExists`)に
  `home_dir()` を足した `ProviderEnv` trait を導入。CLI は
  `ProcessEnv`、test は注入 map。scoped env overlay は
  `OverlayEnv`(`env?.[name] || process.env[name]` と
  `overlayEnvAuthContext` の両 pattern を `overlay.get() ||
  base.env()` に統一)。
- **`!command` 実行**: `executeCommand` を実装(sh -c / cmd /C、10s
  timeout、非零 exit・空出力は未解決、process 内 cache)。`check`
  (status/get_available 判定)は `isCommandConfigValue` で実行せず
  configured 扱いにする点まで TS と一致させた。
- **`authHeader`/headers**: `composeApiKeyAuth` 後段の
  `configContextEnv` + `resolveHeadersOrThrow` + `withConfiguredAuth`
  を port。model 単位 headers は `rawModelHeaders` 相当の merge source
  (modelOverride.headers < models[].headers < plugin model.headers)を
  `ProviderEntry::model_headers()` に保持し、request 時解決は adapter
  側に委ねた(template 未解決のまま保持)。
- **oauth method advertise**: builtin oauth 宣言(anthropic・
  github-copilot・openai-codex 等)は `OAuthFlow` trait の登録有無で
  `methods` を決める。flow 未登録でも stored oauth credential は
  `configured` 扱い(TS `checkProviderAuth` の oauth 分岐は flow 本体を
  参照しない)し、`get_auth` は handler 無しを None(env fallback
  禁止)にした。`radius`(`oauth:"radius"` config)は api_key
  + RADIUS_API_KEY env + oauth 宣言として compose するが flow は
  実装しない(T6 以降)。
- **plugin contract**: TS `CheaperProviderPlugin.register(runtime,
  context)` を `register(&mut ProviderRegistry,
  &ProviderPluginContext)` に narrow 化。registry は
  `register_provider`(catalog/auth 一体)と `register_logout_hook`
  (hook のみ)の 2 経路。plugin auth は TS extension `oauth` 相当の
  `PluginAuth`(login → StoredCredential を返し persist は runtime)
  に揃え、catalog refresh は `PluginCatalog::refresh_models` を別
  trait に分離した。`CatalogStore::replace_plugin_models` で login 後
  refresh を差し替える。
- **`ProviderEntry` の auth metadata 集約**: TS では composed provider
  object が `auth.apiKey`/`auth.oauth`/`headers` を閉じ込めるが、
  Rust では closure 合成ではなく data 化(env 規則・rawKey・headers・
  authHeader・oauth 宣言)して `check`/`resolve` を api_key.rs の純粋
  関数で再構成する。これにより plugin/builtin/custom を同じ関数で
  判定できる。
- **azure baseUrl**: bundled data の azure model は `baseUrl: ""`
  (deployment 固有 URL が別途必要)。catalog はそのまま保持し、
  request 時解決は adapter の責務とした。

### TS との divergence / 残差 (Slice T2)

- plugin model は extension path で cost/contextWindow 等の default を
  埋めない(TS は `{...definition}` verbatim)。Rust struct は required
  field なので `unwrap_or_default`(0 cost/128k/16k)を採用 — TS で
  undefined になる値を 0 既定にする差分。devin は全 field を供給
  するため実害なし。
- `subscription` は oauth method を advertise する provider のみ
  true(宣言のみ・flow 無しでは false)。TS は宣言存在で true。
- api_key resolve の credential env 伝搬(`credential.env`)は新
  schema が持たないため省略(TS は oauth 由来 credential が env を
  保持する)。plugin `to_auth` が ResolvedAuth を直接返す設計で代替。
- radius gateway(`oauth:"radius"`)は remote catalog
  (`getRadiusModelsFromConfig`)を読まず空 catalog で compose
  (offline v1 規則)。
- `models-store.json` / remote catalog refresh(`withRemoteCatalog`)は
  v1 非実装(manifest `generatedAt` の expose のみ)。

### Decisions During Implementation (Slice T1)

- **tool 計数の所有者**: `toolCalls`/`rejectedToolCalls` は hook の
  outcome 観察ではなく `ToolRuntime::budget()` の live view から読む
  (`Accumulator::attach_budget`)。TS `accounting.ts` は access-denied
  error を `toolCalls` と `rejectedToolCalls` の**両方**に計上し、
  maxToolCalls 上限拒否は rejected のみ・runtime 未到達 call は不計上
  — hook の outcome 数ではこの 3 区分を再現できないため。
- **retry seam**: `AgentBuilder::model_route_handler` に
  `RetryingCompletion`(`Serve` impl)を登録し `set_model_label` で選択
  (`named_model` は `Into<DynModel>` しか受けず handler を差し替え
  られないため route 経由)。completion call 1 回が retry 境界 —
  dispatch loop を巻き込まないので tool 二重実行は起きない。
- **streaming retry の限界**: stream open 後・最初の item emit 後の
  失敗は partial turn としてそのまま流す(retry すると provider 応答
  prefix が重複する)。unary(`stream: false`)は丸ごと retry 可能。
- **`details` の運搬**: `CanonicalToolResult.details` は model 非可視
  metadata で Rig stream item に乗らないため、callback が
  `ToolResultContext` slot(`ToolCallDetails`)へ publish →
  `on_outcome` で call id 付き回収 → `RunEvent::ToolResult.details`
  に添付する side-channel。`tool_concurrency(1)` なので race しない。
- **`CompletionModel::from_rig` は唯一の Rig 型公開面**: provider
  crate(`cheaper-provider-devin` 等、Rig-aware が許される側)が
  `DynModel` を adapter へ渡す入口。adapter 利用側(T3 以降)の
  signature に Rig 型は出ない(`RunEvent`/`RunSnapshot`/`CheaperError`)。
- **`tool_concurrency(1)`**: sequential 実行は provider ではなく
  runner 設定として `stream_run` 内で固定 — spec から外部化しない
  (TS の直列 contract を誰が呼んでも壊れない形にする)。
- **invalid-tool retry の既定**: `DEFAULT_INVALID_TOOL_RETRIES = 8`
  (budget 0 だと Rig が `Fail` に fail-closed するため明示必須、
  発散防止と回復余地の両立値)。`DEFAULT_MAX_TURNS = 1024` は内部
  guard(ISS-11 — cheaper `policy.execution.maxTurns` の abort が
  正の経路で、到達は `INTERNAL_ERROR` として区別)。
- **`max_output_tokens`**: `min(policy.output.maxTokens,
  model.maxTokens)` を runner の `max_tokens` に注入(session.ts の
  streamFn wrap 相当)。

### TS との divergence / 残差 (Slice T1)

- `RunEvent` は Rig stream の正規化形であり TS の event 列とは粒度が
  異なる(TS は Pi session event を直接中継)。`ToolCommitted`/
  `CompletionCall`/`ModelTurnRetried` は adapter 固有の追加情報で、
  T3 が envelope 化する際に間引く想定。
- tool の error 結果は model-visible text に写されるが、TS 同様
  呼び出し自体は継続する(error tool result → 次 turn)。
- `details` side-channel は skipped/denied 判定には使われない
  (tool counter は budget が計上するため hook の拒否判定は不要)。
- `repairs` カウンタは T1 では発火しない — `record_repair` は T3 の
  repair loop が呼ぶ口として公開済み。

### Validation (Slice T1)

- `cargo check -p cheaper-rig-adapter` → 成功。
- `cargo test -p cheaper-rig-adapter` → 14 passed / 0 failed
  (accumulator 集計・reconcile / 429・5xx・transport・request error
  分類 / abort reason code 復元 / stream delta+finish / invalid tool
  call retry + rollback reconcile(rolled-back turn の課金 100+150=250
  が残り turn は 1) / transient retry(500→回復・budget 尽きで最終
  error・400 で retry 無し) / 実 ToolRuntime 経由の read tool
  実行・access denied の両方計上・same-turn 直列順序 / abort
  終端 error(pre-start と mid-run の partial snapshot) /
  build_system_prompt・build_user_prompt の TS 文字列 parity)。
- `cargo clippy -p cheaper-rig-adapter --all-targets -- -D warnings`
  → warning 0(`result_large_err` は `DynModel::call` の Err 型由来で
  `call_with_retry` に scoped allow)。
- `cargo fmt -p cheaper-rig-adapter` → clean(`--check` 差分なし)。

### Changes (Slice T3)

- `crates/cheaper-runtime/src/` に T3 module 8 file
  (`abort.rs`/`grace.rs`/`output.rs`/`run_error.rs`/`meta.rs`/
  `repair.rs`/`panic_boundary.rs`/`fake_provider.rs`)を新設し、
  `run.rs` stub を orchestration 本体に置き換え。`tests/run.rs` を
  新設。`Cargo.toml` に `cheaper-tools`/`cheaper-rig-adapter`/
  `tokio`/`futures` を追加。
- `run_worker` は `catch_run_panic` で包んだ `run_worker_inner` に
  委譲。inner は `checkCapabilities` → abort 配線 → `RunCtx` で
  invocation → tool scope → `Runtime::resolve_model_auth`(explicit
  `provider/id` ref 優先、tier 解決 + auto-tier 判定) → auth →
  `SessionFactory::create_session` → `prompt_with_abort_grace` →
  `extract_output` → `repair_output` → `build_meta` の順で段階解決。
  error 経路は TS の try/catch 境界に揃え、capability 段は生 error、
  setup 段は `runMeta:{execution}`、prompt/extract/repair 段は
  `withRunMeta` の full meta を付ける。
- abort 配線: `tokio::spawn` の timeout task + 外部 signal の
  `on_abort` listener を `RunAbort` へ集約。finally 相当で timeout
  task を `abort()`、listener を `remove_listener`、scope を
  dispose。maxTurns/maxToolCalls は cheaper 側判定 —
  `policy.execution.maxTurns` は snapshot の `turns` +
  `last_turn_tool_calls` から run 側で abort request し、
  `maxToolCalls` は tool budget の `on_limit` callback で run abort
  に伝搬する(adapter 内部 `DEFAULT_MAX_TURNS` guard は正の経路では
  ない)。
- `fake_provider` は `SessionFactory`/`WorkerSession` を実装し、
  `ToolCalls` 応答は実 `tool_runtime.execute` を直列実行して実
  budget/journal/`on_limit` 経路を通す。tool budget は fake
  accumulator に `attach_budget` で結線する。

### Decisions During Implementation (Slice T3)

- **session seam の所有**: `RigSession` 生成(= Rig type 境界)と
  run 経路の差し替え(fake 検証)の両方を runtime 内 trait 化した
  `WorkerSession`/`SessionFactory`/`CompletionModelFactory`/
  `RigSessionFactory` で表現。runtime public signature には
  `CompletionModel`(opaque adapter 型)までしか出ず、`DynModel` 等の
  Rig 型は provider/plugin の `CompletionModelFactory` 実装側に
  閉じる。これにより `cheaper-runtime` は adapter crate への crate
  依存を持つが、Rig 本体(`rig` crate)には依存しない。
- **`RunCtx` による段階状態**: TS run.ts が 1 関数内の closure 変数で
  journal/invocation/scope/resolved/session を持ち回る構造を、borrow
  境界の都合で `RunCtx` struct + `deps` 参照に写した。error path
  毎の meta 付与規則(`fail`=setup / `with_run_meta`=prompt 以降 /
  `classified`=abort wrap)を method に閉じ込めている。
- **first-reason-wins は reason の確定のみを担う**: `RunAbort` は
  `OnceLock` で最初の reason を記録し、後続 request は無視。
  `throwClassified` 相当では mutation journal が非空なら元の abort
  error を cause に `PARTIAL_MUTATION` で wrap する(TS の
  `PartialMutationError` 包みと同じ — abort code 自体は cause に
  残る)。
- **grace の outcome 2 分岐**: abort 後 settled error が grace 内に
  来れば `Aborted`、来なければ `Ignored`(stream を諦めて run を
  classified error で終了)。両方とも最終的に `classified(reason)` で
  code 化するが、Ignored では provider が cancel を無視した旨を
  details に残す。
- **`ExecutionBinding` の再定義**: cheaper-tools の
  `WorkerExecutionBinding` は scope 側 `ToolRuntime` を持つ型で
  invocation runtime + owner + parent lease を表せない。TS の
  `deps.execution` 相当を run 経路で `ExecutionBinding` として別
  定義し、borrowed 経路(`owns_invocation=false`、scope dispose
  のみ)と owned 経路(invocation 生成 + dispose)を分岐させた。
- **repair session の spec 分岐**: repair では
  `SessionSpec.tool_runtime=None`・`output_schema=Some(schema)` に
  なる。`output_schema` は provider json_schema mode 向け hint で、
  adapter `WorkerSessionSpec` には載せない(Rig 配線は
  provider/model factory 側責務)。fake は spec を記録して
  tool-less/schema hint が検証可能。
- **error size**: `WorkerRunError` が `CheaperError` +
  `PartialRunMeta` を inline で持つと clippy `result_large_err` に
  抵触するため `inner: Box<WorkerRunErrorInner>` にした。

### TS との divergence / 残差 (Slice T3)

- Rust tool 層には TS `sandboxed()` に正確に対応する API がなく、
  `ready()` + worker scope で代替。tool 実行 sandbox 境界の差分は
  Phase 2( cheaper-tools )側の残差として継続。
- provider stream 終端の error 分類は adapter の `RunStream` が
  終端 `CheaperError` emit で完結するため、TS の
  `session.messages`/`stopReason==="error"` 精査は runtime 側では
  行わない(adapter が同一 code を付ける前提)。
- `rawOutput` cap は `slice(0,2000)` を Rust Unicode scalar count で
  実装(JS は UTF-16 code unit)。非 BMP 文字を含む出力のみ差分。
- `RigSession::abort()`(WorkerSession 経由)は no-op — cancel 伝搬は
  `stream_run` の signal select が担う。`prompt_with_abort_grace` は
  session-local cancel を試みる trigger として `abort()` を呼ぶ
  seam を残す(fake で abort 観測用)。
- repair 中の run abort で session snapshot のみ残し prompt 結果を
  捨てる点は TS と同一だが、grace outcome は repair prompt にも
  `prompt_with_abort_grace` を通し `Ignored` で抜ける。

### Changes (Slice T4)

- `src/fake_provider.rs` を拡張: `ScriptedSession{steps}` +
  `ScriptStep`(Emit/Delay/ToolCalls/UsageDelta/Turn/ProviderError/
  WaitAbort/Hang/AbortResponse/Panic/Finish)と
  `FakeResponse::Scripted` を追加。`FakeSession` に
  `retry_budget`(`SessionSpec.transient_retries` 由来)を持たせ、
  `fake_stream` へ伝搬する。scripted stream は `Step::Scripted` 状態を
  追加した `stream::unfold` state machine で、script 1 step = unfold
  1 iteration として既存の per-item abort 判定と同じ位置で abort を
  観測する。
- scripted `error` の分類は adapter `classify`/`status_details` の
  parity mirror(`ScriptedErrorClass`:
  429→TransientRateLimit・status>=500→TransientServer・その他
  status→Permanent・status 無しは `retryable` 主張→
  TransientTransport else Permanent)。終端 error は
  `PROVIDER_FAILURE` + `details.{class,retryable,httpStatus}`。
- `CheaperFakePlugin`(provider `cheaper-fake`・api
  `cheaper-fake-api`・`api_key` 既定 literal `"fake-key"`・bootstrap
  `fake-model` cost = tests/run.rs の fake と同一 rates
  input=1.0/output=2.0/cache*=0.5)を実装。
- `src/fixture.rs` を新設: fixture DSL serde schema
  (`RunFixture`/`FixtureStep`/`FixtureExpect` 等)・`RunFixture::load`
  parser・worker .md 生成(frontmatter YAML + schema JSON inline)・
  `default_policy()` + `FixturePolicy` 部分 override・
  `CheaperFakePlugin`/`FixtureEnv`/`FakeSessionFactory` 変換・
  `ScriptedAuthInteraction`(`auth_logins` 用)・`FixtureStep →
  ScriptStep` の 1:1 変換を集約。
- `tests/fixtures/` に JSON corpus 24 件、`tests/run_fixture_test.rs`
  に replay harness(`Runtime::create` → `auth_logins` → catalog
  `get_available` 確認 → worker .md → `run_worker` → `expect` 比較)
  を新設。`expect` 宣言 field のみ比較 = normalize 規則の実装。

#### fixture DSL grammar

1 file = `run_worker` 1 呼び出し。top-level 節:

- `worker{name,description,tools,write_scope,model_tier,output_schema,
  prompt}` → worker .md 生成(name = file stem 規則)。
- `policy{timeout_ms,max_turns,max_tool_calls,repair_attempts,
  transient_attempts}` → `default_policy()` への部分 override。
- `provider{api_key}`、`model{id,api,cost{input,output,cacheRead,
  cacheWrite}}` → plugin/model 差し替え。
- `explicit_model`、`env{}`(ProviderEnv overlay)、
  `auth_logins{providerId:secret}`、`input{}`、`abort{after_ms,
  message}`、`abort_grace_ms`、`sessions[]`。
- `expect{status("ok"|"error"),result,errorCode,causeCode,
  errorDetails(subset),exitCode,meta{model,usage(costUsd 含む),turns,
  toolCalls,rejectedToolCalls,repairs,transientRetries,changesFiles,
  changesFilesMin,savings},pendingResponses}` — camelCase は
  `WorkerRunMeta` JSON 名に揃える。

step 形(bare string と `{"key":…}` の両形を受理):

`{"emit":{"text_delta"|"reasoning_delta":"…"}}`・`{"delay_ms":n}`・
`{"tool_calls":[{"name","input"}]}`・`{"usage_delta":{input,output,
cacheRead,cacheWrite,total?,reasoning?,toolUsePrompt?}}`・
`{"turn":{"tool_calls":bool}}`・`{"error":{status?,retryable?,
message?}}`・`"hang_until_abort"`/`{"hang_until_abort":…}`・
`"hang"`/`{"hang":…}`・`"abort_response"`/`{"abort_response":…}`・
`{"panic":"…"}`・`{"finish":{"text":"…"}}`/`{"finish":"…"}`。

#### fixture corpus(24 件)

- 正常系: `01-success-basic`(usage+turns+fenced output+costUsd)、
  `02-success-tool-calls`(write→journal)。
- repair: `03-repair-success`(2 attempt で usage 合算)、
  `04-repair-exhausted`(INVALID_OUTPUT + usage meta)。
- abort 分類: `05-timeout-delay`(delay_ms→timeout)、`06-max-turns`、
  `07-max-tool-calls`(rejected 計上)、`08-signal-abort`
  (hang_until_abort)、`09-partial-mutation`(write→abort→
  PARTIAL_MUTATION cause ABORTED)、`10-hang-grace`(signal 非観測の
  grace 打切り)、`11-abort-response`。
- provider error/retry: `12-error-429-retry`、
  `13-error-429-exhausted`(予算尽き PROVIDER_FAILURE +
  details.class)、`14-error-5xx-retry`、
  `15-error-5xx-exhausted`(transient_attempts=1)、
  `16-error-permanent`(status 400 → retry 無し)、
  `17-error-retryable-transient`(status 無し retryable 主張)、
  `23-emit-then-error`(emit 後の error は retry しない partial-turn
  規則)。
- usage/meta: `18-usage-delta-accumulation`(3 call 合算)。
- auth/catalog: `19-auth-env-template`(`$VAR` api_key 解決)、
  `20-auth-login`(login → stored credential)、
  `21-unsupported-api`(model.api 不一致 → INVALID_MODEL_CONFIG)、
  `22-unknown-model`(MODEL_TIER_UNAVAILABLE)。
- panic: `24-panic`(panic 境界 → INTERNAL_ERROR)。

### Decisions During Implementation (Slice T4)

- **fixture DSL は `src/fixture.rs`(library 側)に所有**:
  plan は step 15-17 で DSL schema + parser を runtime 側に置く。
  adapter 側 `fake_stream` dev-only feature が来た際に同一 DSL を
  共有できるよう library export にした(`tests/` 内 util ではなく
  `cheaper_runtime::fixture`)。
- **scripted `error` の retry は fake 側で分類**: session seam
  (`WorkerSession`)の差し替えは adapter 内部の transient retry
  wrapper を含めて置き換わるため、retry 意味論を fake stream 内で
  再現する必要がある。adapter `classify` の分類表だけを
  `{status,retryable}` から再構成する mirror を実装し、重複する
  writeup ではなく同一分類表の再定義として扱った(分類根拠は
  adapter 側が正源、fake は fixture 検証のための mirror と明記)。
  `Debug` 名を `ErrorClass` に揃えて details.class でも比較可能。
- **call 境界**: `emit` が item 記録、`turn`/`tool_calls` step が
  completion call 境界を切る(`emitted_in_call` reset)。emit 後の
  `error` は partial turn として retry しない(adapter 規則の
  fixture 23 で固定)。
- **`delay_ms` は abort 観測付き sleep**: `tokio::select!` で
  signal を優先(biased)し、中断後は loop-top の `abort_item` が
  reason 由来の終端 error を emit する — delay 中でも timeout/
  signal abort 分類が効く(fixture 05)。
- **fixture `error.details` は subset 比較**: `assert_json_subset`
  で宣言 key のみ再帰比較 — normalize 規則(message 本文・
  非決定的 field の除外)を「比較対象を宣言 field に限定する」形で
  実装した。meta も serde JSON(camelCase)に写して宣言 field のみ
  比較し、`costUsd` は pricing 由来値を abs diff < 1e-9 で厳密比較。
- **`ScriptedAuthInteraction`**: `auth_logins` は実
  `AuthService::login`(api_key method、secret prompt)を通して
  credential store に永続化する — login→stored credential→run の
  実経路を fixture で辿る(fixture 20 + 層単位 test)。
- **workspace 解決の横断修正**: 検証時点で並行 T6 slice が
  `auth/service.rs` に `super::oauth` 参照を追加済みだったが
  `auth/mod.rs` の `pub mod oauth;` 宣言と `auth/oauth/` の compile
  error(ureq 2.x に無い `http_status_as_error`・`into_reader` move
  後の `status()` 借用)が未解決だった。`pub mod oauth;` 宣言 +
  `Error::Status` から response を取り出す `fetch` parity 実装 +
  status/status_text の先取りで最小修正した(T6 owner へ共有
  推奨)。

### TS との divergence / 残差 (Slice T4)

- fixture DSL は TS `CHEAPER_TEST_FAKE_STREAM` の「応答配列 JSON 化」
  ではなく、step primitive を直に記述する形を採った(plan の DSL
  要求 `delay_ms`/`error{status}`/`hang_until_abort`/
  `abort_response`/`usage_delta` を直接表現)。TS 応答配列との
  機械的変換は行っていない。
- fake stream の `error` retry は「pre-item の transient error のみ
  retry」までを再現する。adapter `RetryingCompletion` の backoff
  時間・attempt logging は fixture では deterministic に畳み込む
  (瞬時 retry)。
- `emit` step は `RunEvent` のうち `TextDelta`/`ReasoningDelta` のみ
  対応(tool call event は `tool_calls` step が担うため DSL では
  不要と判断 — YAGNI)。
- fixture DSL 全体を differential golden(TS↔Rust binary 間
  20-40 case)へ横展開するのは Phase 4 以降の残差 — 本 slice では
  Rust 単体 replay まで。
- `CheaperFakePlugin` は `PluginAuth`/`PluginCatalog` hook を
  実装しない(keyless api_key auth で login/resolve が完結する
  ため)。oauth 経路の fake は対象外。

### Changes (Slice T6)

- `src/auth/oauth/mod.rs` + `src/auth/oauth/github_copilot.rs` を
  新設、`tests/oauth.rs` を新設(18 test — mock `HttpClient` +
  記録 `FlowSleeper` + scripted `AuthInteraction` で実 network・
  実 sleep 無しに全 step を駆動)。
- `auth/service.rs`: `AuthInteraction` に `event(&AuthEvent)`(default
  は CLI `notify` 相当の text render へ畳込)と `is_cancelled()`
  (default false、`interaction.signal.aborted` 相当)を追加。
  `AuthPrompt` に `Text`/`ManualCode`/`Select` variant を追加
  (TS prompt union の port)。
- `auth/api_key.rs`: `ResolvedAuth` に `base_url: Option<String>`
  を追加(oauth `toAuth().baseUrl` 相当 — credential 依存の
  per-credential endpoint)。既存の構築 site(`with_configured_auth`・
  plugin/fake test literal)は `None`。
- `runtime.rs`: `builtin_oauth_flows()` を seed に引数
  `oauth_flows` を重ねる(同名 id は引数側が勝つ test 注入 seam —
  service wiring test は mock HTTP 差込み flow をこの経路で登録)。
- `Cargo.toml`: `ureq = "2"`・`url = "2"` を追加(両方とも
  Cargo.lock に transitive として存在する同一 version —
  lockfile の package graph 変動なし)。
- `catalog/bundled.rs`: `days_from_civil` を `pub(crate)` 化
  (`Retry-After` HTTP-date parse が再利用 — date 演算の正源は
  catalog 側に置き oauth 側に複製しない)。

### Decisions During Implementation (Slice T6)

- **sync `OAuthFlow` + injectable `HttpClient`**: T2 確定の sync
  contract を維持し、network は `HttpClient` trait seam
  (`send(&HttpRequest) -> Result<HttpResponse>`)に隔離。production
  は `ureq` blocking client(sync `fetch` 相当 — 非 2xx の
  `Error::Status` も response として取り出し `response.ok` 判定は
  caller に残す)、test は mock で request 形状・応答列を制御する。
  `reqwest` async 化は `OAuthFlow` 自体を async にする設計変更を
  伴うため見送り(YAGNI — flow の対話面は sync `AuthInteraction`
  と同じ粒度)。
- **sleep も seam 化**: `FlowSleeper`(production `ThreadSleeper`)で
  poll interval・`waitBeforeFirstPoll`・slow_down 待機・retry
  backoff を全て差し替え可能にし、test は記録 sleeper で sleep
  予定列を assert する。
- **cancel は `is_cancelled` 集約**: TS `interaction.signal`/
  `AbortSignal` は `AuthInteraction::is_cancelled` に写し、
  `abortableSleep` の mid-sleep throw は持たず sleep 前後 check +
  poll loop 先頭 check で拾う(sync CLI では Ctrl+C が process
  全体を終了させるため、sleep 中の割込み検知は実害がない)。
- **`AuthEvent` は構造のまま interaction へ**: device_code/
  auth_url/progress/info を enum で運び、default `event` は CLI
  `notify` 相当の 1 段 text へ畳む。構造のまま扱いたい層(将来の
  TUI 等)は override する設計で、cli boundary 追加型を生やさない。
- **`proxy-ep` claim から `base_url` 導出**: `to_auth` は Copilot
  token の `proxy-ep=proxy.X` → `https://api.X`、無ければ
  `enterpriseUrl` claim → `https://copilot-api.<domain>`、最後に
  `https://api.individual.githubcopilot.com`。TS
  `getGitHubCopilotBaseUrl` の priority をそのまま再現。
- **policy enable は login 時・individual endpoint のみ**: TS と
  同じく `fetchGitHubCopilotModels` の `allowPolicyFallback` は
  `base === individual` のときのみ。`unconfigured` + known model
  (bundled catalog id)へ `POST <base>/models/<id>/policy
  {state:"enabled"}` を順に試し、成功 id を `availableModelIds`
  に union する。非中断 error で batch 打切り(TS `break`)、
  refresh 経路では policy enable を行わない。
- **`availableModelIds` も credential 化**: picker ∪ enabled の順序
  保持 dedup を `available_model_ids` に載せ、catalog
  `get_available` filter は T2 の `model_filter` 経路がそのまま
  拾う(新規 wiring 不要)。
- **extra claim は provider 固有名を保持**: `enterpriseUrl` は
  extra map に camelCase 名のまま格納(T2 schema の snake_case
  固有 field と共存 — serialization contract を service test で
  固定)。
- **error code 分類**: provider 接線の失敗(device response
  validation・HTTP error・token exchange・models fetch)は一律
  `PROVIDER_FAILURE`、cancel は `ABORTED`("Login cancelled")、
  device expiry は `TIMEOUT`(slow_down 観測後は WSL clock drift
  message)。enterprise domain prompt の入力 invalid は
  `INVALID_REQUEST`("Invalid GitHub Enterprise URL/domain")。

### TS との divergence / 残差 (Slice T6)

- OAuth flow・interaction・HTTP は全て sync(TS は async +
  `AbortSignal`)。cancel は signal 監視ではなく `is_cancelled`
  polling 判定 — mid-sleep 中断(`abortableSleep` の throw)は
  再現せず sleep 境界で拾う。
- `Retry-After` の HTTP-date 値は IMF-fixdate(`"Sun, 06 Nov 1994
  08:49:37 GMT"`)のみ解釈 — `Date.parse` の全書式(obsolete 形式・
  ローカル時刻書式)は port しない。解釈不能は TS の NaN 分岐と
  同じく retry せず response を返す。
- credential file の Rust 固有 field は `available_model_ids` 等
  snake_case(T2 の新規 schema)。provider extra claim
  (`enterpriseUrl`)は camelCase — TS `auth.json` とは byte 互換
  ではなく新規 schema(T2 の意図的 divergence の延長)。
- `AuthEvent::AuthUrl` 用の browser `openUrl` + loopback callback
  listener は contract 面のみ — browser+loopback flow
  (anthropic/openai-codex/openrouter/radius)実装時に各 flow が
  持つ。manual code prompt は `AuthPrompt::ManualCode` で表現済み。
- `expires_in`/`interval`/`expires_at` は f64 秒経由(TS number
  parity)。負値は即時 timeout、非 finite は無期限(
  `Number.POSITIVE_INFINITY` 相当)。
- CLI wiring(device code の stderr 表示・`openUrl` 呼出・
  `is_cancelled` の signal 配線)は Phase 4 scope — task doc の
  `cheaper auth login github-copilot` 実 endpoint smoke は CLI 層
  確立後の open item。
- T4 slice が oauth module の compile blocker(`pub mod oauth;` 宣言・
  ureq response 取り出し)を横断修正済み — 本 slice ではその state
  を継承して完成させた。

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

### Validation (Slice T2)

- `cargo check -p cheaper-runtime` 成功(scratch workspace、下記注記)。
- `cargo test -p cheaper-runtime` → 41 passed / 0 failed
  (unit 9 + tests/auth 18 + tests/catalog 12 + tests/plugin 2)。
- `cargo clippy -p cheaper-runtime --all-targets` → warning 0。
- `cargo fmt` 適用済み(`fmt --check` 相当 clean)。
- 並行作業の rig-adapter crate が作成中(manifest 着陸済みだが
  `src/lib.rs` 未到着 = target 未定義)のため root
  workspace(`members=["crates/*"]`)は現時点で manifest 解決不能。
  同等構成の scratch workspace(`target/t2-ws/`、workspace.dependencies
  を写し members を cheaper-core+cheaper-runtime に限定)で検証した。
  rig-adapter `lib.rs` 着陸後に `mise exec -- cargo test -p
  cheaper-runtime` 等での再検証が必要(依存 spec は同一のため差異は
  lockfile 解決のみ)。なお `rustfmt --check --edition 2021` を
  cheaper-runtime 全 file に直接実行し clean を確認済み(workspace
  edition = 2021)。
- 実施した検証コマンド(scratch ws): `cargo check` / `cargo test` /
  `cargo clippy --all-targets` / `cargo fmt`。

### Validation (Slice T3)

- `cargo check -p cheaper-runtime` → 成功(root workspace 上)。
- `cargo test -p cheaper-runtime` → 60 passed / 0 failed
  (lib unit 14 + tests/auth 18 + tests/catalog 12 + tests/plugin 2
  + tests/run 14)。tests/run の 14 test は success/meta(usage・
  turns・transientRetries・pricing 由来 costUsd=0.005075・savings
  shape)・invalid model ref・4 abort 分類(timeout/signal/maxTurns/
  maxToolCalls)・first-wins(signal 後の timeout は再分類しない)・
  abort grace(Hang stream が grace で打ち切る)・repair
  成功(usage merge + tool-less repair spec 記録)・repair 尽き
  (INVALID_OUTPUT + usage meta)・journal 非空 abort →
  PARTIAL_MUTATION・provider failure の code/usage 保持・
  panic/`JoinError` → INTERNAL_ERROR を網羅。
- `cargo clippy -p cheaper-runtime --all-targets` → warning 0。
- `cargo fmt -p cheaper-runtime` 適用済み(`--check` 差分なし)。
- 検証は `cargo` 直接実行(toolchain は mise pin の stable 同一、
  `mise exec -- cargo --version` = cargo 1.99.0 で一致確認済み)。
- 注記: T3 検証完了後に T5 の `crates/cheaper-provider-devin/`
  manifest が着陸し、`src/lib.rs` 未到着の間は workspace 解決が
  fail する(他 crate の一時 scaffold で T3 側の残差ではない)。

### Validation (Slice T4)

- `mise exec -- cargo test -p cheaper-runtime` → 86 passed /
  0 failed(lib unit 19 + tests/auth 18 + tests/catalog 12 +
  tests/oauth 18 + tests/plugin 2 + tests/run 14 +
  tests/run_fixture_test 3)。oauth test suite は並行 T6 slice の
  追加分。
- `mise exec -- cargo test -p cheaper-runtime --test
  run_fixture_test` → 3 passed(fixture corpus 24 file 全 replay +
  catalog/keyless 層 + login→stored credential 層)。
- `mise exec -- cargo clippy -p cheaper-runtime --all-targets --
  -D warnings` → warning 0。
- `mise exec -- cargo fmt --check` → clean(`cargo fmt` 適用後)。
- `crates/cheaper-provider-devin/src/lib.rs` は workspace 解決の
  ため最小 stub のまま(T5 scope、T4 では実装しない)。

### Validation (Slice T6)

- `mise exec -- cargo test -p cheaper-runtime` → 86 passed /
  0 failed(lib unit 19 + tests/auth 18 + tests/catalog 12 +
  tests/oauth 18 + tests/plugin 2 + tests/run 14 +
  tests/run_fixture_test 3)。
- `cargo test -p cheaper-runtime --test oauth` → 18 passed /
  0 failed(device-code request/response 検証・default +5s slow_down
  と server interval 優先・expiry timeout・slow_down 後 WSL clock
  drift message・cancel・transport error 伝播・enterprise endpoint/
  domain validation・429 retry・policy enable + individual fallback・
  refresh・`to_auth` proxy-ep 導出・credentials.json 永続化 +
  status secret 非開示・service login/logout wiring・runtime
  builtin 登録・`AuthEvent` default rendering)。
- `mise exec -- cargo clippy -p cheaper-runtime --all-targets --
  -D warnings` → warning 0。
- `mise exec -- cargo fmt --check` → cheaper-runtime の T6 変更
  file 全て clean。workspace 全体では並行 T5 slice
  (`cheaper-provider-devin/src/tests/*.rs`、他 agent 着手中)に未
  fmt 差分が残る — T6 範囲の file には差分なし。
- `cargo check -p cheaper-provider-devin -p cheaper-rig-adapter` →
  成功(`ResolvedAuth.base_url` 追加は構造上の破壊的変更だが、
  provider-devin 側も同 field を供給済みで compile 影響なし)。

## Risks

- rig-adapter `lib.rs` は着陸済みのため root workspace の manifest
  解決は可能になった。`cheaper-runtime` 側の最終検証は T3 で root
  workspace 上に再実行済み(cargo test 60 pass / clippy 0 / fmt
  clean)。
- `cheaper-provider-devin` は T5 が scaffold 中で、`src/lib.rs`
  未到着の間は workspace 解決が fail する(T3 の残差ではないが、
  並行作業中の cargo 実行に影響)。
- plugin `refresh`/`to_auth` の devin 実装(T5)・github-copilot
  `OAuthFlow` 実装(T6)で seam の追加調整が必要になる可能性がある。
- T5 は `CompletionModelFactory` を実装して `RigSessionFactory` に
  供給する責務を持つ(`CompletionModel::from_rig` 呼び出しは
  provider 側)。SessionSpec.output_schema の provider 配線(json
  schema mode 等)は T5 以降の残差 — 未配線でも repair は prompt +
  attempt 上限で成立する。

## Follow-ups

- T4: 実施済み(本 file `### Slice T4` 節)。fixture DSL +
  `CheaperFakePlugin` 3 層 + `tests/run_fixture_test.rs` 完備。
  残差: adapter 側 `fake_stream` dev-only feature への DSL 共有
  (plan step 17 の adapter 側実装)、TS↔Rust differential golden
  への corpus 横展開。
- T5: devin provider。`PluginProviderRegistration` +
  `PluginAuth`(devin CLI 委譲 login→oauth-typed credential、extra
  claim `tomlPath`)+ `PluginCatalog`(`devin models list`)+ plugin
  宣言 api kind(devin 独自)が `SUPPORTED_APIS` 外でも supported と
  なる seam を使用。
- T6: 実施済み(本 file `### Slice T6` 節)。`GitHubCopilotFlow`
  (device-code login + refresh + `to_auth` base_url 導出)を
  `builtin_oauth_flows()` 経由で `Runtime::create` が自動登録し、
  `auth status` の methods に `"oauth"` が出る。残差: Phase 4 の
  CLI wiring(device code 表示・`openUrl`・signal→`is_cancelled`)
  と `cheaper auth login github-copilot` の実 endpoint smoke、
  browser+loopback flow(openai-codex/anthropic 等)の flow 実装。

## Review cycle 1 fixes (T5 devin)

レビュー findings(B3/B4/B5/B10・A6 consumer)に対する T5 後追い修正。
範囲は `crates/cheaper-provider-devin/` のみ。TS 正源は
`packages/runtime-pi/src/providers/devin/` + `session.ts`/`repair.ts`。

- **D1(B3) 非 UTF-8 env panic** — `plugin.rs` `register` の child env
  base 収集を `std::env::vars()` → `vars_os()` + `to_string_lossy()`
  へ変更(Unix では任意 byte 列の env が合法で `vars()` は panic
  する)。test: `register_survives_non_utf8_process_env`(`#[cfg(unix)]`
  — `OsStr::from_bytes` で非 UTF-8 var を set して register が完走
  することを確認。Windows では `set_var` が OsStr 任意 byte を
  受けないため cfg 限定)。
- **D2(B4) thinking level 喪失** — `DevinWire::encode` の
  `resolve_model_uid` 第 3 引数を `None` → `Some("off")`。
  TS `stream.ts:530` は `options?.reasoning` を渡し、
  `session.ts:69` が `thinkingLevel: "off"` に pin する(worker
  session は常に off)。修正前は off uid を持つ model(例
  `gpt-5.6-sol` → `gpt-5-6-sol-none`)に preferred default(high)
  の uid を送っていた。spec に per-request reasoning field が増え
  たらそこから流す設計で、現時点の固定値 `"off"` は TS と一致。
  test: `encode_uses_off_thinking_level_uid`(wire field 21 が
  off uid になることを encode 経路で確認)。
- **D3(B5) `check` を credential-presence へ** — `DevinPluginAuth`
  の `check` override を削除し trait default
  (`credential.is_some()`)に戻した。TS `plugin.ts` には
  `oauth.check` が存在せず Pi は credential-exists を
  `configured: true` とみなす — subprocess(`devin auth status`)は
  spawn しない。CLI authoritative な `devin_auth_status` は
  login-reuse 経路(`login_with_devin_cli` 内の stale 判定)だけに
  残る — TS と同じ配置。test:
  `check_is_credential_presence_without_cli_subprocess`(runner 未
  呼出を記録で確認)。
- **D4(A6) `SessionSpec.output_schema`** — runtime の `SessionSpec`
  は `output_schema: Option<&CompiledSchema>` を既に持つ(repair
  session が `Some` を渡す)。`DevinCompletionModelFactory` で
  検証した結果、TS `streamDevin` は
  `model.samplingParams.response_format` を一切読まず
  (`buildGetChatMessageRequest` に該当 field 無し)— devin は
  repair でも json_schema hint を wire へ encode しないため、
  Rust 側も `let _ = spec.output_schema;` で意図的に無視し
  comment を残した(TS 同様 corrective prompt + attempt 上限のみ
  が contract)。配線不要の clean boundary。
- **D5-1(B10) `DevinFamily` optional 化** — `family_label`/
  `family_uid`/`slug` を `Option<String>`(`#[serde(default)]`)へ。
  TS は `||` fallback で読むため欠損は degraded model に留まるが、
  serde-required だと 1 family の欠損で catalog 全体が deserialize
  error になっていた。`family_to_models` は TS 相当の
  `slug || family_uid || defaultUid` / `label || slug || defaultUid`
  fallback(空文字も falsy として skip)。`variants` は required
  のまま(TS でも `family.variants.filter` が TypeError で同様に
  fail する)。test: 欠損 family の degraded 変換 2 件。
- **D5-2(B10) exit 127 → not-found** — `DevinCli::run` で
  `result.code == 127` を `CliError::NotFound`(`DevinCliNotFoundError`)
  へ写像(TS `isNotFound` の `code === 127` 相当)。OS ENOENT は
  従来どおり `io::ErrorKind::NotFound` 経路。test:
  `exit_127_maps_to_cli_not_found`。
- **D5-3(B10) trailing slash は 1 個だけ除去** —
  `trim_end_matches('/')` だと `https://x//` の全 slash を削る。
  TS `.replace(/\/$/,"")` に合わせ `strip_suffix('/')` へ —
  `rig_provider::resolve_host`、`jwt.rs` 2 箇所、`credentials.rs`
  の `api_server_url` を全て揃えた。test:
  `resolve_host_strips_one_trailing_slash`。
- **D5-4(B10) `HOST_ENV` を `ProviderEnv` seam 経由へ** —
  `completion_model` の `std::env::var(DEVIN_API_SERVER_URL)` を
  `DevinCompletionModelFactory.env: Arc<dyn ProviderEnv>`(default
  `ProcessEnv`、`with_env` で注入)経由に変更。空文字/空白は
  `ProviderEnv::env` が unset 扱い(TS `||` falsy と同じ)。host
  解決は `resolve_host(env, auth)` helper に抽出。test:
  `resolve_host_prefers_env_then_auth_then_default`。
- **D6-1(B10) JWT float `exp`** — `jwt_expires_at` が
  `as_i64()`(float 拒否→600s fallback)だったのを
  `as_f64()`(整数も float も受ける、TS `typeof exp === "number"`
  相当)へ変更し i64 へ truncate。test: `float_exp_claim_is_accepted`
  (float は expiry、非数値は fallback)。
- **D6-2(B10) mint timeout を単一 30s deadline へ** — TS は
  `AbortSignal.timeout(30_000)` が send + body 読み取り全体を覆う。
  Rust 側は send 30s + body 30s の 2 段(最大 60s)だったため、
  `tokio::time::timeout(MINT_TIMEOUT, async { send→status→body })`
  の 1 段へ統一。test 無し(実 30s 待機が必要 — code inspection で
  検証)。
- **D6-3(B10) `GetChatMessage` 非 2xx message** — TS
  `GetChatMessage HTTP {status}: {body.slice(0,300)}` 形式へ写す
  `chat_send_error` + `chat_status_message` を追加。
  `InvalidStatusCodeWithDetails` は `ProviderResponseError` で
  status/headers を保持するため adapter の transient 分類
  (429/5xx retry)は維持される(TS message は body へ埋め込み、
  Display は `status {status}: GetChatMessage HTTP {status}: …`
  と rig prefix 付き)。status を Err 化しない `HttpClientExt` 用に
  `!status.is_success()` 時は body を drain して同形式の error を
  返す経路も追加。test: `chat_status_message_matches_ts_shape` +
  `chat_send_error_preserves_status_and_ts_message`。
- **D6-4(B10) finish で open part を close** — `DevinDecoder` の
  `ChatEvent::Finish` で text/reasoning/call の open part を EOS
  待たずに close(TS `stream.ts:614-617` と同じ)。finish 後の text
  event は新 part を開く TS 挙動と一致。test 無し(`Out` は
  `pub(crate)` コンストラクタで decoder 単体 test 不可 —
  driver 経由の統合試験は composition root 側の scope)。
- test 数: 51 → 61(新規 10 件)。`cargo test -p
  cheaper-provider-devin` 61 pass / `clippy --all-targets -D
  warnings` clean / `cargo fmt --check` clean / `cargo check` OK。

## Review cycle 1 fixes (runtime/adapter)

レビュー findings F1–F13 に対する後追い修正。範囲は
`crates/cheaper-runtime/`・`crates/cheaper-rig-adapter/` のみ
(`crates/cheaper-provider-devin/` は別 agent の D1–D6 cycle で並行
修正済み・本 cycle の対象外)。TS 正源は `packages/runtime-pi/src/`。

- **F1 maxTurns abort は最終 turn の tool batch 完了を待つ** —
  `run.rs` の `on_event` が `RunEvent::ToolCall`/`ToolResult` で
  pending tool call 数を追跡し、`turns >= maxTurns &&
  last_turn_tool_calls && pending == 0` の時点でのみ abort する。
  従来は turn 受理直後に abort していたため Rig の
  `drive_tool_calls` が batch 途中で drop され最終 turn の tool
  副作用・結果 event が欠落していた。invalid-tool rollback は
  対応する `ToolResult` を持たないため、pending は
  `CompletionCall`/`ModelTurnRetried` 境界で reset する。batch が
  stream 終端(provider error 等)と衝突して deferred abort が
  発火しなかった場合の post-outcome fallback も追加。fake
  provider の `ScriptStep::ToolCalls` は Rig `drive_tool_calls`
  と同じ emit 順序(model `ToolCall` event 群 → `QueuedWork::
  Execute` marker → `ToolResult` event 群)を再現するよう変更し、
  run 側の pending 追跡が実 stream と同じ item 形状を観測できる
  ようにした。test:
  `max_turns_waits_for_final_tool_batch_before_aborting`
  (PARTIAL_MUTATION + cause=MAX_TURNS_EXCEEDED + write 実効)、
  `max_turns_still_aborts_without_tool_calls_in_final_turn`。
- **F2 rolled-back turn の usage 保持** — usage 記録を
  `AccountingHook::on_outcome` から `run_stream` の
  `MultiTurnStreamItem::CompletionCall` item 境界へ移した。
  invalid-tool retry で rolled back される streamed turn は
  outcome hook・accepted-turn hook いずれも発火しないが
  `CompletionCall` item は emit されるため、item 境界が唯一の
  完全な記録点。run が後で error/abort で終わっても usage が残る。
  `on_run_settled` の `reconcile_usage` は防御的 aggregate 経路と
  して維持。test: `rolled_back_turn_usage_survives_later_run_error`
  (invalid call 後の provider failure で turns=0 でも usage が残る)。
- **F3 `strip_json_comments` の UTF-8 panic** —
  `catalog/schema.rs` の trailing-comma pass で escaped char 直後の
  byte を `bytes[i] as char` で 1 byte 扱いしていたため、escaped
  backslash + 多 byte UTF-8(`\あ`)で continuation byte 上の slice
  → panic/文字化けがあった。`utf8_len` で UTF-8 span 全体を copy へ
  修正。test:
  `escaped_backslash_before_multibyte_utf8_does_not_panic_or_corrupt`。
- **F4 `raw_output` は fence-strip 前の原文** —
  `extract_output` が `raw_output` に strip 済み text を返していた
  のを TS `rawOutput: raw` 相当の assistant 原文へ修正(parse・
  validate 対象は従来どおり strip 後)。success/failure 全 variant で
  原文を保持。test: `failure_variants_keep_original_raw_output` +
  fenced success の assertion 更新。
- **F5 `policy.filesystem.symlinks` の tool runtime 配線** —
  `run.rs` の invocation-level (`create_invocation_tool_runtime`)
  と worker-scope (`create_worker_scope`) の双方で hardcode されて
  いた `SymlinkPolicy::Deny` を `policy.filesystem.symlinks` へ
  (TS `symlinkPolicy: policy.filesystem.symlinks`)。test:
  `filesystem_symlinks_policy_reaches_tool_scope`(`#[cfg(unix)]` —
  workspace 内 symlink read が Deny で rejected=1、
  WithinWorkspace で accepted)。
- **F6 invalid tool call の retry budget + accounting** —
  `WorkerSessionSpec.invalid_tool_retries` → Rig
  `max_invalid_tool_call_retries`(runtime default
  `DEFAULT_INVALID_TOOL_RETRIES = 8`)。`invalid_tool_calls` meta は
  「観測した invalid call event 数」を数える(消費 retry 数では
  ない)。**F6a**: `RigSessionFactory` は
  `invalid_tool_retries.max(internal_max_turns)` で floor する —
  Rig の invalid-tool retry は model-call budget を消費するため、
  内部 guard 未満の retry 上限では `invalid_tool_call_retries`
  枯渇の PROVIDER_FAILURE が cheaper の `MAX_TURNS_EXCEEDED` 分類
  より先に発火し得る。**F6b**: rolled-back turn は完了 model
  call として `turns` に計上する(`Accumulator::
  record_rolled_back_turn` — `on_invalid_tool_call` hook で
  `HookContext::turn` dedup 付き)。Pi は invalid call を error
  tool result で turn を閉じて `turn_end` で計上するため、
  数えないと invalid-call ループが `policy.execution.maxTurns`
  を回避できてしまう。rolled-back turn は常に tool 継続終了なので
  `last_turn_tool_calls = true` も立て、run 側の maxTurns 判定
  (F1)が機能するようにする。fake 側は `ScriptStep::
  RolledBackTurn` が同じ計上点を replay する。test:
  `invalid_tool_call_retry_budget_exhausts_and_counts_observations`
  (budget=1 で 2 回目の観測で PROVIDER_FAILURE、observed=2、
  turns=2)、`invalid_tool_call_recovers_and_reconciles_usage`
  (turns = rollback 1 + 受理 1)、run 経路
  `rolled_back_turn_counts_toward_max_turns`(rolled-back turn で
  MAX_TURNS_EXCEEDED 分類 + meta.turns)。
  **residual**: retry budget を使い切った unhandled call 自体の
  usage は観測不能 — Rig の Fail 経路は stream を `finish()` せず
  破棄するため `CompletionCall` item が emit されず、hook/settle
  経路にも usage が載らない(engine.rs Fail arm)。retried call の
  drain 済み usage は残る。adapter 側で補完不能なため残差として
  記録。
- **F7 内部 max-turns guard** — `SessionSpec.policy_max_turns` を
  追加し `RigSessionFactory` が `max(internal_max_turns(1024),
  policy_max_turns)` を `WorkerSessionSpec.max_turns` へ渡す。
  policy maxTurns が内部 guard を上回ると runaway 対策が cheaper の
  MAX_TURNS_EXCEEDED 分類より先に発火し error code/event 列が
  ずれるのを防ぐ。main/repair 双方の SessionSpec で policy 値を
  配線。test: `internal_max_turns_is_at_least_policy_max_turns`
  (formula)、`policy_max_turns_flows_to_session_spec`(main+repair
  wiring)、adapter 側
  `internal_max_turns_guard_stops_runaway_tool_loop`。
- **F8 `SessionSpec.output_schema` の配線** — main session にも
  `Some(&worker.output_schema)` を渡すよう修正(従来は `None`)。
  TS `createWorkerSession` には outputSchema option が無く prompt
  埋め込みのみ — 本項目は provider の json_schema mode 等へ向けた
  Rust 側の拡張 seam で、未対応 provider では無視される
  (`CompletionModelFactory` 経路でのみ消費、Rig
  `WorkerSessionSpec` には載らない)。既存 test
  `repair_merges_usage_and_returns_repaired_result` の
  `has_output_schema` assertion を main=true へ更新。
- **F9 `total_tokens` fallback** — `UsageSnapshot::from(&Usage)` で
  provider が total を省略した場合 `input + output` を採用(明示
  total は従来どおり優先)。test:
  `usage_snapshot_total_falls_back_to_input_plus_output`。
  **decision(F9)**: 明示 total と input+output が不一致でも明示値を
  優先する — provider の total は additive とは限らず
  (cache/reasoning 内訳を含む provider がある)、TS と同じく
  「provider が total を与えたら尊重」のみを規則とする。
- **F10 JavaScript `JSON.stringify` 互換の number formatting** —
  provider 可視 text(prompt 埋め込み schema/input、repair prompt
  schema、meta の returned-token 計数 text)の serde_json 出力が
  JS とずれる点を新 module `js_json.rs`(`stringify` /
  `stringify_pretty`)で一箇所に集約:`-0`→`0`、非有限→`null`、
  整数の `.0` 除去、指数表記境界(>=1e21 / <1e-6)、正指数の
  明示 `+`(`1e+21`)。call site: `agent.rs` system/user prompt、
  `repair.rs` repair schema、`meta.rs` returned tokens。test:
  `js_json.rs` unit tests(number 境界・pretty layout・escape)、
  adapter `js_json_prompts_use_json_stringify_number_format`。
- **F11 `!command` stdout >64KB deadlock** — `auth/api_key.rs` の
  `execute_command_uncached` で stdout を reader thread が逐次
  drain するよう変更。exit 後 `read_to_string` では pipe buffer
  超過で子が write block し `try_wait` が完了せず deadline まで
  待って kill されるため、大きな stdout の command が常に未解決
  扱いになっていた。exit 後も pipe 残量の drain 完了を deadline
  内で待つ。test:
  `command_stdout_over_pipe_buffer_does_not_deadlock`
  (unix `seq 1 50000` ~289KB / windows `for /l` ~260KB の cfg pair)。
- **F12 Unix での atomic credential 置換** — `auth/store.rs` の
  `write_file` で Unix は `fs::rename` が既存 target を直接原子置換
  するよう変更(remove→rename では crash window に
  credentials.json が消える)。Windows の MoveFile 系は target 存在
  で失敗するため従来どおり事前 remove(残差 — platform 制約)。
  tmp file は同 directory(同一 filesystem 保証)+ 0o600。
  test: `credential_store_replace_never_exposes_missing_file_on_unix`
  (writer 64 回 replace 中に並行 reader が file 不在・壊れた JSON
  を観測しない)、既存
  `credential_store_atomic_replace_and_remove`。
- **F13 plugin catalog handle 保持 + login 後 refresh** —
  `ProviderEntry.plugin_catalog` に `Arc<dyn PluginCatalog>` を保持
  し、`AuthService::login` が credential 保存成功後に
  `refresh_plugin_catalog` を呼ぶ(TS の login 後
  `refreshModels().catch(() => {})` 相当)。`CatalogStore` は
  `dynamic_models: RwLock` + `record_error`/`refresh_plugin_models`
  を持ち、refresh 結果は `applyExtension` と同一規則(
  `extension_models`)で解決してから `modelOverrides` を再適用。
  refresh/変換の失敗は login を失敗させず catalog errors へ記録。
  空 list は bootstrap list 維持。test:
  `login_refreshes_plugin_catalog_models`、
  `login_succeeds_when_plugin_refresh_fails`。

### Accounting decisions (cycle 1)

- `invalid_tool_calls` は observed invalid-call event 数(F6)。
- rolled-back turn は Pi `turn_end` parity で `turns` に計上する
  (F6b — `record_rolled_back_turn`、turn index dedup)。これにより
  invalid-call ループは `maxTurns` を回避できない。
- completion usage の authoritative recording point は
  `CompletionCall` stream item(F2)。`on_run_settled` の
  `reconcile_usage` は `max` 畳込みの defensive aggregate。
- 残差: unhandled(budget 尽き)invalid call の usage は Rig が
  stream を drain しないため記録不能(F6b)。upstream 変更なしに
  adapter 側では補完できない。
- `total_tokens` は provider 明示値を優先、欠時のみ
  `input + output` fallback(F9)。

### Validation (cycle 1)

`cargo test -p cheaper-rig-adapter` 23 pass(unit 4 + adapter 19)、
`cargo test -p cheaper-runtime` 96 pass(run 18 / auth 19 /
catalog 12 / oauth 18 / plugin 4 / fixture 3 / unit 22)、
`cargo test -p cheaper-provider-devin` 61 pass(同 crate 未変更 —
trait 互換のみ確認)、`cargo clippy -p cheaper-rig-adapter -p
cheaper-runtime --all-targets -- -D warnings` clean、
`cargo fmt --check` clean。

## Slice T7 — builtin provider factory + Ollama real-provider e2e

### Changes (Slice T7)

- `crates/cheaper-rig-adapter/src/builtin.rs`(新設)— catalog `api`
  種別 → Rig provider client の組立関数 `builtin_completion_model` と
  adapter 所有の入力型 `BuiltinAuth`/`BuiltinModelSpec`。
  `cheaper-runtime` → `cheaper-rig-adapter` の依存方向があるため、
  runtime 側 trait `CompletionModelFactory` は adapter 内に実装
  できない(circular)。代わりに adapter は組立 primitive だけを提供し、
  runtime 側の factory 実装(e2e test / Phase 4 CLI composition root)が
  `SessionSpec` → `BuiltinModelSpec` へ写して呼ぶ。戻り値は opaque
  `CompletionModel` のみで Rig 型は signature に出ない。
- `crates/cheaper-rig-adapter/src/lib.rs` — `mod builtin` + re-export
  (`builtin_completion_model`/`BuiltinAuth`/`BuiltinModelSpec`)、
  module doc の公開面リストに追記。
- `crates/cheaper-runtime/tests/ollama_e2e.rs`(新設)— real-provider
  e2e gate。`CHEAPER_E2E_OLLAMA=1` かつ `GET {base}/v1/models`(3s
  timeout、ureq)に model id があれば実行、それ以外は早期 return で
  skip(`packages/cli/test/e2e/real-provider-ollama.test.ts` parity)。
  env: `CHEAPER_E2E_OLLAMA_MODEL`(default `qwen3.5:9b`)、
  `CHEAPER_E2E_OLLAMA_BASE`(default `http://localhost:11434`)。
  test-local `BuiltinFactory`(`CompletionModelFactory` impl)が
  `builtin_completion_model` を呼ぶ — Phase 4 composition root が
  再利用する seam を先行して exerciser として使う。
  - catalog: tmpdir `<userConfigDir>/models.json` に `ollama` custom
    provider(api `openai-completions`・`baseUrl {base}/v1`・literal
    `apiKey: "ollama"`・model `{id, reasoning:true, cost:0,
    contextWindow:262144, maxTokens:32768, samplingParams}`)— TS e2e
    の IMPL-034r05iHy2gdnpoDlOXUs6 recipe と同形。
  - `ModelConfig`: `ollama/<model>` を override で `low` に pin
    (9b は built-in 分類表に match しない)+ `preferences.low` 先頭。
  - policy: timeout 600s / maxTurns 16 / repairAttempts 4 / tools
    allow = worker 3 種の union(read/grep/find/ls/write/edit/
    count_tokens)。
  - worker: repo 内 canonical 定義 `packages/cli/workers/{bulk-reader,
    code-writer,doc-writer}.md` を `CARGO_MANIFEST_DIR` 相対で load
    (`file:` schema ref は worker dir 相対に解決 — 実 file 経由で
    differential parity を保持)。

### api 種別 → Rig client 写像表

| catalog `api`         | Rig client / wire                                            |
|-----------------------|--------------------------------------------------------------|
| `openai-completions`  | `OpenAIConfig::with_key(&OPENAI, key)` + `with_base_url` → `connect(transport)` → `.chat(model)` |
| `openai-responses`    | 同上 → `.responses(model)`                                   |
| `anthropic-messages`  | `AnthropicConfig::with_dialect(key, &ANTHROPIC)` + `with_base_url` → `connect(transport)` → `.completion(model)` |
| その他                | `INVALID_MODEL_CONFIG`(`details.api` 付き)                   |

- **Ollama は `providers::ollama` ではなく OPENAI dialect + `with_base_url`
  を使う**: catalog の api 種別は `openai-completions`(= Chat Completions
  wire)を宣言しており、Ollama の `/v1/chat/completions` はその方言に合致。
  rig の `ollama` provider は native `/api/chat` 用で api kind と対応しない。
- **`Dialect` は `OPENAI`/`ANTHROPIC` 固定**: `Dialect` field は全て
  `&'static str` で runtime 構築不能なため、custom provider でも登録済み
  dialect の `with_base_url`/`with_auth` で差し替える。
  `OPENAI` dialect の `completion_route: Route::Responses` は
  `.chat()`/`.responses()` の明示 route 選択で回避。
  `OutputCap::OpenAiReasoningFamilies` は model 名で scoped
  (`is_openai_reasoning_model`)されるため非 OpenAI model には legacy
  `max_tokens` がそのまま送られる(qwen3.5 で実測済み)。
- **api_key `None`**(ambient/keyless)→ `Auth::OptionalBearer` で空
  `Authorization` を送らない。
- **custom headers**: Rig config に任意 header 注入面が無いため、headers
  非空時のみ `reqwest::Client::builder().default_headers(...)` の専用
  transport を `connect` へ渡す(request 側 header が default に勝つ規則
  で Bearer 等は wire 値が優先)。headers 空は shared `ReqwestClient` で
  pool 共有。header 名/値が invalid → `INVALID_MODEL_CONFIG`。
- **`auth.base_url` > `spec.base_url`(catalog)> dialect 既定** の順で
  endpoint を解決(copilot `toAuth` の credential 依存 endpoint 対応)。

### Residual / 未配線(Slice T7)

- `model.samplingParams` / `model.compat` / model-level `headers` は
  request 単位の `additional_params` として session 構築側(`RigSession`)
  に seam が無いため未適用。TS e2e recipe の `reasoning_effort:"none"` +
  `temperature:0` は models.json に書いても wire へ載らない
  (qwen3.5:9b は載せずとも収束したが、小 model では repair 回数に影響
  し得る)。Phase 4 で `WorkerSessionSpec` 拡張 or wire 側 default 検討。
- anthropic `x-api-key` vs `Authorization: Bearer`(oauth token 由来)の
  送り分けは未実装 — 現状一律 `x-api-key`。
- `ProviderEntry.model_headers`(rawModelHeaders)は catalog 側で保持するが
  `get_auth` → `ResolvedAuth.headers` への merge 経路が未接続
  (provider-level configured headers のみ到達)。
- plugin 宣言 api 種別(devin 等)は `builtin_completion_model` では
  `INVALID_MODEL_CONFIG` — plugin 側 `CompletionModelFactory` が担う
  (devin は `DevinCompletionModelFactory` 実装済み)。

### e2e 実測(qwen3.5:9b、Ollama container、3 test 並行実行)

`CHEAPER_E2E_OLLAMA=1 cargo test -p cheaper-runtime --test ollama_e2e
-- --nocapture`:

| worker       | turns | toolCalls | usage in/out/total   | repairs | transientRetries | duration |
|--------------|-------|-----------|----------------------|---------|------------------|----------|
| bulk-reader  | 5     | 2         | 13249 / 1848 / 15097 | 1       | 2                | 293.8s   |
| code-writer  | 3     | 1         | 4843 / 431 / 5274    | 1       | 0                | 125.3s   |
| doc-writer   | 4     | 2         | 7359 / 816 / 8175    | 1       | 0                | 163.8s   |

全件 `run_worker` 成功: output schema-valid(`extract_output` で抽出、
9b の出力ゆらぎで各 1 回 repair を消費 — `repair_output` の実経路も
実 provider で動作確認)、`meta.model.provider=ollama`・`meta.model.id`
一致・`meta.usage.input/output` 非零。code-writer/doc-writer は対象
file を実際に workspace へ作成(`meta.changes.files` 非空)。bulk-reader
は real transient retry(provider 側応答遅延)が 2 回観測され retry→成功
経路も実到達。3 test を同時並行で実行したため wall time は直列より長い
(Ollama 1 台に 9b を 3 session 並行)。

### Validation (Slice T7)

`cargo test -p cheaper-rig-adapter` 26 pass(unit 7 — builtin 3 含む +
integration 19)、`cargo test -p cheaper-runtime` 99 pass(unit 22 /
run 18 / auth 19 / catalog 12 / oauth 18 / plugin 4 / fixture 3 /
ollama_e2e 3 — gate 未満では skip)、`cargo test -p
cheaper-provider-devin` 61 pass、`cargo clippy -p cheaper-rig-adapter
-p cheaper-runtime -p cheaper-provider-devin --all-targets --
-D warnings` clean、`cargo fmt --check` clean。real-provider e2e は
上表のとおり全 pass(gate ON)。

### 手動 smoke の延期記録(Slice T7 時点)

- `auth login github-copilot` → run: 実 GitHub credential が必要で本
  環境では検証不能。oauth flow は mock transport の単体 test 済み
  (`tests/oauth.rs` 18)。Phase 4 CLI 組立後の手動 smoke へ延期。
- api_key auth の実 credential login→run(Anthropic/OpenAI): 実 key が
  無いため延期。openai-compatible(Ollama configured `apiKey`)経路は
  `ollama_e2e` が configured→run 実到達をカバー。
- devin 実 endpoint smoke: 実 endpoint/credential 不可のため未実施。
  task item 規定どおり切り下げ判断のユーザー承認待ち。
