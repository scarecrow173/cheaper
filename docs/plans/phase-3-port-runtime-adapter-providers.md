---
id: "PLAN-034BepZr0oX3bWeqVXNk91"
type: "plan"
status: "approved"
title: "Phase 3 port runtime adapter providers"
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
  implements:
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
  implemented-by: []
  depends-on:
    - "docs/plans/phase-1-port-cheaper-core.md"
    - "docs/plans/phase-2-port-cheaper-tools.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
    - "docs/designs/rust-rig-runtime-migration-architecture.md"
    - "docs/adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md"
    - "docs/plans/migrate-cheaper-runtime-to-rust-rig.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rust-migration-run-semantics.md"
    - "docs/test-specs/rust-migration-auth-catalog.md"
    - "docs/test-specs/rust-migration-differential-golden.md"
  references: []
  defers: []
  deferred-by: []
---

# Phase 3 port runtime adapter providers Implementation Plan

## Goal

`cheaper-rig-adapter`・`cheaper-runtime`・`cheaper-provider-devin` の 3 crate を
新設し、`packages/runtime-pi` の責務(session・model 解決・run
オーケストレーション・usage 集計・repair・auth/catalog・devin provider)を
Rig `=0.43.x` 上に移植する。ギャップ G1(model catalog store)・G2(credential
store + OAuth)・G3(transient retry)・G4(devin provider)・G7(fake provider
seam)は本フェーズ内で自前実装する。

## Scope

1 つの実装ストリーム(3 crate + fake seam/fixture 基盤)だが、作業単位は
Task Breakdown の T1-T6 に分割して並行させる。境界は以下のとおり。

- **In scope**: Rig 接着(adapter)・run オーケストレーション・catalog・
  credential/auth(api_key・env・devin CLI 委譲・OAuth は provider 毎
  sub-task、github-copilot を必須 1 件)・devin provider・fake seam +
  fixture DSL・`WorkerRunMeta` 完全互換。
- **Out of scope**: envelope serializer・CLI dispatch・usage.jsonl 記録・
  hook 4 target・builtin resource 埋込(Phase 4)、workflow engine(Phase 4)、
  配布切替(Phase 5)、models.dev online refresh(v1 不実装、Deferred)、
  `<userConfigDir>/pi/` の読み取り・import。
- 移植元は `packages/runtime-pi/src/`(run.ts 474 行・session.ts・
  model-runtime.ts・usage.ts・repair.ts・output.ts・auth.ts・
  provider-plugin.ts・providers/devin/ ≈1.5k 行)。`journal.ts` は
  tool-runtime 再 export のため Phase 2 側の責務。
- Rig 型(`rig::`/`rig_core::`/`rig_agent::`)の `use` は
  `cheaper-rig-adapter` と `cheaper-provider-devin`(設計上の例外)のみ。

## Upstream Documents

- `docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md`
- `docs/adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md`
- `docs/plans/migrate-cheaper-runtime-to-rust-rig.md`(親 plan)
- `docs/research/rig-agent-runtime/migration-plan.md` §Phase 3(成果物 a-f・
  完了条件・検証)・§3.3(differential golden 戦略)
- `docs/plans/phase-1-port-cheaper-core.md`(`cheaper-core` の提供 API 前提)
- `docs/plans/phase-2-port-cheaper-tools.md`(`cheaper-tools` の tool scope・
  journal・budget/meter・scheduler 前提)

## Design Inputs

- `docs/designs/rust-rig-runtime-migration-architecture.md` — crate 表・
  run オーケストレーション手順・意味論固定(maxTurns 主従・
  `InvalidToolCallAction::Retry{feedback}`・`tool_concurrency=1`・invalid call
  非計上)・structured output repair・G1/G2/G4・fake seam 3 層・
  Review Findings(ISS-11/14/15/18/19/22/23/24)・Verification Notes
- `docs/designs/pi-execution-engine.md` — 現行 session/run の構造契約
- `docs/designs/model-resolver.md` — tier/explicit 解決と catalog の契約
- `docs/designs/repair-abort-and-failure-normalization-hardening.md` —
  abort 分類・grace settle・PROVIDER_FAILURE 正規化の現行契約
- `docs/designs/auth-command.md`・`docs/designs/auth-io-boundary.md`・
  `docs/designs/auth-url-opener-seam.md` — `auth login/status/logout` の
  CLI contract と terminal I/O 境界
- `docs/designs/devin-local-provider-plugin.md` — devin provider の
  auth/catalog/stream 責務と隔離 data dir 契約
- 移植元実装(正の仕様源): `packages/runtime-pi/src/run.ts`・`session.ts`・
  `model-runtime.ts`・`usage.ts`・`repair.ts`・`output.ts`・`prompt-grace.ts`・
  `auth.ts`・`provider-plugin.ts`・`testsupport.ts`・`providers/devin/*`

## File Map

### `cheaper-rig-adapter`(crate dir は `crates/cheaper-rig-adapter/`)

- `crates/cheaper-rig-adapter/Cargo.toml` - `rig`/`rig-core`/`rig-agent`
  を `=0.43.x` pin、`rig-reqwest` opt-in・rustls、tokio・`cheaper-core`
  (`CheaperError` 写像)・`cheaper-tools`(`ToolRuntime` expose)に依存
- `crates/cheaper-rig-adapter/src/lib.rs` - 公開面(crate 外向け型は
  cheaper 側型のみ、Rig 型を signature に出さない)
- `crates/cheaper-rig-adapter/src/agent.rs` - `AgentBuilder` 構成:
  system prompt(run.ts `buildSystemPrompt` の output contract・batching
  guidance を文字列完全再現)・`.tool()` 登録・`default_max_turns` へ
  十分大きな値(ISS-11: 具体値は実装時選定、cheaper hook が正)・
  `tool_concurrency(1)`・`InvalidToolCallAction::Retry{feedback}` 明示
- `crates/cheaper-rig-adapter/src/tools.rs` - `cheaper-tools` の
  `ToolRuntime` worker scope を Rig `Tool`/`PortableTool` trait 実装で
  expose(ToolContext で invocation scope 注入、`createSandboxedTools` 相当)
- `crates/cheaper-rig-adapter/src/hooks.rs` - `ModelTurnFinished`
  (accepted turn 計数)→ accumulator、`on_outcome` completion outcome で
  per-call usage 加算、`on_run_settled`(`SettledOutcome::Response`)で
  run 総量 usage へ reconcile(rollback attempt は両 hook 不発火のため
  billed usage 捕捉に必須 — Phase 0 実測)、tool outcome は
  `is_skipped()`/`Err` を `rejectedToolCalls` へ、`ToolCallDelta` →
  tool call 計測、「`turns >= maxTurns` かつ最終 stop が tool 継続」
  判定の callback 面
- `crates/cheaper-rig-adapter/src/accumulator.rs` - usage/turns の
  逐次集計型(`UsageCollector` 相当。**唯一の usage 収集源**。
  `costUsd` は provider 応答値を使わず token 数のみ保持 — pricing 層で算出)
- `crates/cheaper-rig-adapter/src/retry.rs` - completion-call 単位の
  transient retry wrapper(429/5xx/network timeout 分類・backoff・
  retry 毎に `transientRetries` 計上、G3)
- `crates/cheaper-rig-adapter/src/run_stream.rs` - streaming run API の
  唯一の公開面(prompt 実行 + `cancel()` abort 中継 + delta/turn event の
  hook 配送)。non-streaming API は expose しない
- `crates/cheaper-rig-adapter/src/error.rs` - `PromptError`/provider
  error → `CheaperError` 写像(HTTP status/429/5xx/timeout 分類を含み、
  `MaxTurnsError` は内部 guard 到達として区別。Rig error を上位へ漏らさない)
- `crates/cheaper-rig-adapter/src/fake_stream.rs` - dev-only feature。
  fixture DSL を replay する scripted stream(delay/error(HTTP status 付き)/
  hang-until-abort/abort 応答/usage delta を表現、G7 の stream 層)

### `cheaper-runtime`(`crates/cheaper-runtime/`)

- `crates/cheaper-runtime/Cargo.toml` - tokio・`jsonschema`・serde 系。
  `rig` 系依存は持たない(adapter trait 境界経由)。`cheaper-core`・
  `cheaper-tools`・`cheaper-rig-adapter`・tokio・`jsonschema`・serde 系に
  依存。**devin 組込は feature 経由にしない**(`cheaper-runtime →
  cheaper-provider-devin` の optional dep は devin→runtime の plugin trait
  依存と合わせて Cargo が拒否するパッケージ循環になる)。plugin 登録は
  composition root である `cheaper-cli` が runtime+devin の両方に依存して
  行う(Phase 4 配線)。runtime は plugin trait の公開面のみ提供
- `crates/cheaper-runtime/src/lib.rs` - `run_worker`・auth service・
  catalog・plugin trait の公開面
- `crates/cheaper-runtime/src/run.rs` - `runWorker` 相当の run
  オーケストレーション本体(capability 適用 → abort 一元化 → invocation/
  worker scope 生成 → catalog model 解決 → adapter streaming run → output
  抽出 → repair → meta 構築、run.ts の構造をそのまま移植)
- `crates/cheaper-runtime/src/abort.rs` - `CancellationToken` +
  `AbortReason`(timeout/signal/maxTurns/maxToolCalls → TIMEOUT/ABORTED/
  MAX_TURNS_EXCEEDED/MAX_TOOL_CALLS_EXCEEDED)先勝ち記録・外部 signal 接続
- `crates/cheaper-runtime/src/grace.rs` - prompt abort grace settle
  (既定 5000ms、stream が signal を無視する provider のハング防止、
  `promptWithAbortGrace` 相当)
- `crates/cheaper-runtime/src/output.rs` - 最終 assistant text 抽出・
  code fence 除去(1 組のみ)・JSON parse・`jsonschema` crate validate・
  `SchemaValidationError { instancePath, message }`・last-assistant stop
  正規化(error/aborted → PROVIDER_FAILURE/ABORTED)
- `crates/cheaper-runtime/src/repair.rs` - tool 無し repair session
  loop(各 attempt 新規 session・`response_format: json_schema`・
  raw output + schema error feedback・attempts 使い切り `INVALID_OUTPUT`
  details: errors 先頭 20 件・rawOutput 先頭 2000 文字・usage/turns/
  transientRetries は main accumulator に合算)
- `crates/cheaper-runtime/src/meta.rs` - `WorkerRunMeta` 全 field
  構築(model{provider,id,tier,selection,tierSource,effectiveCost,...}・
  usage・turns・toolCalls・rejectedToolCalls・repairs・transientRetries・
  changes・execution・savings{sourceTokens,returnedTokens,
  avoidedContextTokens,directlyComparable})・`usage.costUsd` を pricing 層
  (pricing snapshot × token 数)で算出
- `crates/cheaper-runtime/src/run_error.rs` - `WorkerRunError` 相当
  (meta snapshot を運ぶ error 型。error/abort/maxTurns/INVALID_OUTPUT の
  全経路で meta は hook accumulator から構築、PARTIAL_MUTATION 包装規則)
- `crates/cheaper-runtime/src/panic_boundary.rs` - run/command 境界の
  panic/tokio `JoinError` → `INTERNAL_ERROR` 写像 helper
- `crates/cheaper-runtime/src/catalog/mod.rs` - model catalog store
  (bundled 静的 snapshot + user `<userConfigDir>/models.json` 統合、
  `get_available`(認証済み model のみ候補化)・`get_model`・未知/スコープ外
  api 種別は `INVALID_MODEL_CONFIG` 系の明示 error、silent fallback 禁止、G1)
- `crates/cheaper-runtime/src/catalog/schema.rs` - Pi `models.json`
  互換 serde schema(unknown key 無視・OpenAI-compat custom provider・
  `samplingParams` merge・`reasoning`/`maxTokens` metadata。merge 規則は
  fixture で固定、ISS-14 残)
- `crates/cheaper-runtime/src/catalog/bundled.rs` - bundled snapshot
  埋込(`include_str!` で vendored JSON、TS 側 bundled catalog から生成)
- `crates/cheaper-runtime/src/auth/store.rs` - credential store
  (`<userConfigDir>` 直下の新規 file、Pi `auth.json` 形式非互換。
  `<userConfigDir>/pi/` は一切読まない。secret を stdout に出さない、G2)
- `crates/cheaper-runtime/src/auth/api_key.rs` - api_key login +
  env var 解決(Anthropic/OpenAI/OpenAI-compatible/Ollama 系)
- `crates/cheaper-runtime/src/auth/service.rs` - `auth status/login/
  logout` の service 面(`result.credentials[]`/`result.providers[]`
  `{providerId,name,methods,subscription,configured}` 再現、ISS-19a。
  login 成功後 result `{provider,authenticated,method,subscription}`、
  logout `{provider,loggedOut:true}`)
- `crates/cheaper-runtime/src/auth/oauth/mod.rs` - OAuth flow contract
  (browser `auth_url` + device code・AuthInteraction 相当の callback 面・
  SIGINT 中断)
- `crates/cheaper-runtime/src/auth/oauth/github_copilot.rs` -
  github-copilot OAuth flow(必須 1 件。pi-ai flow 実装(MIT)を spec 源
  として参照)
- `crates/cheaper-runtime/src/plugin.rs` - provider plugin contract
  trait(credential 永続化・catalog 登録・logout hook の面を
  `cheaper-runtime` が所有。`ProviderPluginContext` 相当に env +
  userConfigDir)
- `crates/cheaper-runtime/src/fake_provider.rs` - dev-only。
  `cheaper-fake/fake-model` の catalog `get_available` 登録 + keyless
  credential 解決(G7 の catalog/auth 層、stream 層は adapter 側)
- (session trait 境界は `cheaper-rig-adapter/src/session.rs` 側に配置する。
  runtime → adapter の一方向依存を維持するため、trait 定義は adapter が
  所有し runtime はそれを消費する。`RunWorkerDeps.modelRuntime`/`streamFn`
  seam 相当を cheaper 側型で表現)

### `cheaper-provider-devin`(`crates/cheaper-provider-devin/`)

- `crates/cheaper-provider-devin/Cargo.toml` - Rig crate 直接依存
  (設計上の例外、custom `Wire`/`Decoder`/`Transport` trait 実装のため)・
  `cheaper-runtime`(plugin trait)・tokio
- `crates/cheaper-provider-devin/src/cli.rs` - Devin CLI 子プロセス
  runner(argv 分離・`XDG_DATA_HOME`/win32 `LOCALAPPDATA` 隔離 env・
  `DevinCliNotFoundError` 相当・test 用 runner seam。`cli.ts` 141 行 port)
- `crates/cheaper-provider-devin/src/credentials.rs` -
  `<userConfigDir>/providers/devin/data` 解決・`credentials.toml` 読取・
  `auth status/login/logout` 委譲・logout 時の credentials.toml cleanup
  (stale token 再利用防止、`credentials.ts` 144 行 port)
- `crates/cheaper-provider-devin/src/models.rs` - `devin models list`
  catalog 取得・family/variant → model 変換・`FALLBACK_MODELS`・
  `resolveModelUid`・cost summary parse(`models.ts` 286 行 port)
- `crates/cheaper-provider-devin/src/wire.rs` - protobuf-lite
  encode/decode(varint/tag/string/message/fixed64/field iterator)+ Connect
  stream framing(gzip + 5byte header、`wire.ts` 118 行 port)
- `crates/cheaper-provider-devin/src/jwt.rs` - user JWT mint + cache
  (`jwt.ts` 115 行 port)
- `crates/cheaper-provider-devin/src/metadata.rs` - request metadata
  (client ide/version。macOS PRODUCT_JSON 参照は Rust では
  `FALLBACK_WINDSURF_VERSION` 固定に切り下げ可、要 fixture 固定)
- `crates/cheaper-provider-devin/src/context_map.rs` - Context →
  ChatHistoryItem 写像(system → user collapse・tool call 変換・image part、
  `context-map.ts` port)
- `crates/cheaper-provider-devin/src/stream.rs` - Connect streaming
  transport(GetChatMessage request build・event decode・usage event・
  text/reasoning/tool_call/finish 写像、`stream.ts` 656 行 port)
- `crates/cheaper-provider-devin/src/rig_provider.rs` - Rig custom
  `Wire`/`Decoder`/`Transport` 接着(G4)
- `crates/cheaper-provider-devin/src/plugin.rs` - `cheaper-runtime` の
  plugin trait 実装(credential 永続化・catalog 登録・logout hook。
  `runtime.logout` wrap 相当は trait 面で表現)

### fake seam / fixture / test

- `crates/cheaper-runtime/tests/fixtures/` - fixture DSL JSON
  (応答 item 列 + `delay_ms`/`error{status,retryable}`/`hang_until_abort`/
  `abort_response`/`usage_delta` を表現。TS `CHEAPER_TEST_FAKE_STREAM`
  の応答配列を JSON 化した形に揃える)
- `crates/cheaper-runtime/tests/run_fixture_test.rs` - fixture replay
  → meta/envelope 検証
- `crates/cheaper-rig-adapter/tests/` - adapter 単体(hook→accumulator・
  retry 分類・Retry action・cancel 分類)
- `crates/cheaper-provider-devin/tests/` - wire framing・context map・
  models parse の単体(実 endpoint は CI 外)

## Implementation Sequence

- [ ] 1. cargo workspace に 3 crate を scaffold(`rig`/`rig-core`/`rig-agent` を `=0.43.x` pin、`rig-reqwest` opt-in、rustls、workspace root は Phase 1 確定値に従う)
- [ ] 2. `cheaper-runtime` に Pi `models.json` 互換 serde schema を実装する(unknown key 無視・`samplingParams` merge・`reasoning`/`maxTokens` metadata)
- [ ] 3. TS 側 bundled catalog を JSON として vendored 化し `catalog/bundled.rs` に `include_str!` で埋め込む
- [ ] 4. catalog store を実装する(bundled + user `<userConfigDir>/models.json` の merge・`get_available`/`get_model`・merge 規則を fixture で固定)
- [ ] 5. credential store を実装する(`<userConfigDir>` 直下の新規 file・Pi 形式非互換 schema・atomic write・`<userConfigDir>/pi/` 非参照)
- [ ] 6. api_key + env var credential 解決を実装する(provider 定義表: anthropic/openai/openai-compatible/ollama)
- [ ] 7. `auth status/login/logout` の service 面を実装する(`result.credentials[]`/`result.providers[]` の shape 再現・secret 非出力)
- [ ] 8. provider plugin contract trait を `cheaper-runtime` に定義する(credential 永続化・catalog 登録・logout hook の 3 面)
- [ ] 9. adapter に `cheaper-tools` scope を expose する Rig `Tool`/`PortableTool` 実装を書く(ToolContext で invocation scope 注入)
- [ ] 10. adapter に `AgentBuilder` 構成を実装する(system prompt 完全再現・`.tool()` 登録・`default_max_turns` 十分大きな値・`tool_concurrency(1)`・`InvalidToolCallAction::Retry{feedback}`)。**PoC 知見**: `max_invalid_tool_call_retries` は既定 0 で、未設定だと `Retry` が `Fail` に潰れる。run 単位で budget を必ず設定する(例: `prompt(...).max_invalid_tool_call_retries(n)`)
- [ ] 11. adapter に hook 配線と accumulator を実装する(`ModelTurnFinished`→turn 計数・`on_outcome` completion→per-call usage・`on_run_settled`→usage reconcile・tool outcome `is_skipped()`/`Err`→`rejectedToolCalls`・maxTurns 判定 callback)
- [ ] 12. adapter に `PromptError`→`CheaperError` 写像を実装する(HTTP status/429/5xx/timeout 分類・`MaxTurnsError` は内部 guard として区別)
- [ ] 13. adapter に completion-call 単位の transient retry wrapper を実装する(`transientRetries` 計上・backoff・run 全体/tool 実行は包まない)
- [ ] 14. adapter に streaming run API(cancel/abort 中継・delta/turn event 配送)を実装し公開面を streaming のみに絞る
- [ ] 15. fixture DSL schema(delay/error{status}/hang-until-abort/abort 応答/usage delta)を定義し parser を実装する
- [ ] 16. fake provider の catalog 登録 + keyless credential(`cheaper-fake/fake-model`)を `cheaper-runtime` に実装する
- [ ] 17. adapter に fixture DSL を replay する scripted stream(fake stream 層)を実装する
- [ ] 18. `cheaper-runtime` に abort 一元化を実装する(`CancellationToken`・reason 先勝ち・timeout/signal/maxTurns/maxToolCalls 分類)
- [ ] 19. prompt abort grace settle(既定 5000ms)を実装する
- [ ] 20. output 抽出(最終 assistant text・fence 除去・JSON parse・`jsonschema` validate・last-assistant stop 正規化)を実装する
- [ ] 21. `run_worker` オーケストレーション本体を実装する(capability→scope→model 解決→adapter run→extract の直列骨格)
- [ ] 22. `WorkerRunError`(meta snapshot 運搬)・abort 分類・PARTIAL_MUTATION 包装を実装する
- [ ] 23. `build_meta` 相当を実装する(`WorkerRunMeta` 全 field・hook accumulator のみ・`savings` 計算・`costUsd` = pricing 層算出)
- [ ] 24. repair session loop を実装する(tool 無し・`json_schema` mode・error feedback・`INVALID_OUTPUT` details 規則・accumulator 合算)
- [ ] 25. run/command 境界の panic/`JoinError` → `INTERNAL_ERROR` 写像を実装する(async 境界は spawn + JoinError または `AssertUnwindSafe` + `catch_unwind` で包む)
- [ ] 26. devin の wire/protobuf-lite + Connect framing を port する(`wire.rs`)
- [ ] 27. devin の context_map・metadata・jwt を port する
- [ ] 28. devin の stream transport(GetChatMessage build/decode/usage)を port する(`stream.rs` 656 行)
- [ ] 29. devin を Rig custom `Wire`/`Decoder`/`Transport` に接着する(`rig_provider.rs`)
- [ ] 30. devin CLI runner・credentials・models catalog を port する
- [ ] 31. devin plugin trait 実装を完成させる(logout hook 含む)。組込は
      Phase 4 の `cheaper-cli` composition root で行う(本 phase では trait
      実装と単体検証まで)
- [ ] 32. OAuth flow contract 骨組み(AuthInteraction 相当 callback・auth_url/device_code event・中断)を実装する
- [ ] 33. github-copilot OAuth flow を実装する(pi-ai flow 実装を参照、browser/device code・credential 永続化)
- [ ] 34. fake fixture による `run_worker` 単体検証と differential fixture 基盤を整備する(完了条件へ収束)

## Dependencies and Risks

- **依存**: Phase 1(`cheaper-core` の `check_capabilities`/`resolveModel`/
  `resolveExplicitModel`/`parseModelRef`/pricing snapshot/`CheaperError`
  ERROR_TABLE/`isDirectlyComparableWorker`/token 計測)と Phase 2
  (`cheaper-tools` の `createInvocationToolRuntime`/worker scope・
  `MutationJournal`・budget/meter・resource manifest・scheduler)の完了。
  Phase 0 go 条件(hook 経由 usage 粒度・`Retry` action・HTTP status 露出・
  streaming `cancel()` 分類・panic 境界)が検証済みであること。
- **Rig error surface の HTTP status 露出が弱い場合**(go 条件 c の残余
  リスク): error 文字列分類へ退避する。retry 分類の判断根拠を
  `adapter/src/error.rs` に集約し、分類表を fixture で固定する。
- **`maxToolCalls`/`maxTurns` の abort 意味を Rig hook で再現できない場合**:
  `rig-agent` hook 表現力不足と判明したときのみ `rig-core` Model/streaming
  へ退避し自前 loop に切替(ADR の fallback 設計)。adapter の公開面を
  変えない範囲で実施する。
- **devin wire 互換(G4)は実 endpoint 依存**: CI 外の手動 smoke に切り分け、
  不達時は「devin を初期リリースから外す」切り下げ判断が事前合意対象
  (切り下げ時は feature flag で組込を外し `auth status` に unsupported
  表示)。TS 版 devin を残して differential 比較する。
- **OAuth(G2)の見えないコスト**: api_key + env + devin CLI 委譲を先に
  成立させ、OAuth は provider 毎の独立 sub-task(本 plan では
  github-copilot を必須 1 件。openai-codex/anthropic 等他 flow は
  Deferred scope 決定時に追加、ISS-23)。
- **panic 境界の実装注意**: `catch_unwind` は async fn を直接包めないため、
  run 境界を `tokio::spawn` + `JoinError` で包むか `AssertUnwindSafe` を
  明示する。panic hook で stdout への panic 出力を抑止するのは
  Phase 4 CLI 側の責務(runtime 側は写像 helper まで)。
- **usage 集計の唯一の正**: `PromptResponse`/`completion_calls` は
  abort/error/`MaxTurnsError`/`PromptCancelled` で存在しないため meta は
  hook accumulator からのみ構築する(usage-bearing error record 契約の維持
  に必須)。repair・transient retry も同一 accumulator に合算。
- **`savings.returnedTokens` の key 順序**: `JSON.stringify(result)` の
  token 数が key 順序依存のため、result serialize は key 順序保持
  (`preserve_order` or 自前、ISS-19d)で行う。
- **credential/catalog file 非存在時の `models`/`run` 動作**(ISS-19b)と
  `auth status` の `result.providers[]` 再現規則(ISS-19a)は TS 版の
  観測挙動を fixture で固定してから実装する。
- **workspace root**: repo root `Cargo.toml` + `crates/`(Phase 1 確定値)。

## Task Breakdown

| ID | 範囲 | 内容 | 依存 |
| --- | --- | --- | --- |
| T1 | `cheaper-rig-adapter` | step 9-14(tools/agent/hooks/accumulator/error/retry/stream) | Phase 2 tool scope API |
| T2 | catalog + credential + auth | step 2-8(models.json schema・bundled・store・api_key/env・service・plugin trait) | Phase 1 model 解決型 |
| T3 | run オーケストレーション | step 18-25(abort・grace・output・run_worker・meta・repair・panic) | T1 + T2 |
| T4 | fake seam + fixture | step 15-17 + 34(DSL・catalog/auth/stream 3 層・fixture 検証基盤) | T2(catalog/auth 層)・T1(stream 層) |
| T5 | devin provider | step 26-31(wire→context/jwt→stream→Rig 接着→CLI 委譲→plugin 実装) | T1(Rig 接着面)・T2(plugin trait 確定後に plugin.rs のみ) |
| T6 | OAuth(github-copilot) | step 32-33(flow contract・copilot flow) | T2(credential store) |

- T1 と T2 は互いに独立で並行可。T4 の catalog/auth 層は T2 完了後、
  stream 層は T1 完了後。T5 は wire/context/stream(step 26-28)を先行し、
  Rig 接着(29)・plugin(31)は T1/T2 確定待ち。T6 は T2 後に独立 sub-task。
- OAuth 追加 provider(openai-codex/anthropic 等)は T6 と同形の独立
  sub-task として Deferred scope 決定時に派生させる。
- devin 初期同梱可否の切り下げ判断は T5 の実 endpoint smoke 後、
  ユーザー承認を経て決定する。

## Verification

- [ ] `cargo test -p cheaper-rig-adapter -p cheaper-runtime -p cheaper-provider-devin` が全パス(単体: hook→accumulator 逐次集計・retry 429/5xx/timeout 分類・Retry action の error feedback→loop 継続・cancel reason 分類・wire framing round-trip・context map・models parse)
- [ ] fake fixture による `run_worker` 単体検証: 正常系/invalid output→repair(usage 合算)/timeout・maxTurns・maxToolCalls・signal abort 分類/journal 非空 abort→PARTIAL_MUTATION/error envelope が fixture 通りに再現され、`meta` 全 field(usage token field・turns・toolCalls・rejectedToolCalls・repairs・transientRetries・changes・execution・savings)が期待値と一致(normalize 規則: `meta.durationMs`・`usage.costUsd`・schema error message 文言は除外、`instancePath` は厳密)
- [ ] fixture DSL の各 primitive が動作: `delay_ms`→timeout abort、`error{status:429}`→transientRetries 計上 + 成功、`error{status:5xx}` 連続→PROVIDER_FAILURE、`hang_until_abort`→signal abort 分類、`abort_response`→ABORTED、`usage_delta`→accumulator 反映
- [ ] `cheaper auth login github-copilot`(OAuth、device または browser flow)から run まで成功し、credential が `<userConfigDir>` 直下の新 credential store に永続化される。`<userConfigDir>/pi/` の auth.json/models.json を参照・読み込みしないこと
- [ ] api_key auth(Anthropic/OpenAI/OpenAI-compatible)で login→run が成立し、secret が stdout に出ないこと
- [ ] 未知・初期スコープ外の provider api 種別で catalog 層が明示 error(`INVALID_MODEL_CONFIG` 系)を返し silent fallback しないこと
- [ ] 実 provider negative differential(ISS-18): 同一 worker・同一入力を TS 版と Rust 版で実行し、429 分類(retryable・transientRetries 計上)・totalTokens 未報告 fallback・cache token 差の扱いが一致すること(手動 smoke、CI 外)
- [ ] devin wire 互換: 実 Devin CLI + endpoint で `auth login/status/logout`・`devin models list` 反映・1 件の run が成立すること(CI 外の手動 smoke。結果は本 plan へ記録)
- [ ] Ollama 実 provider コンテナ(`docker compose up -d` + model pull)で built-in worker 3 種が成功 envelope を返す Rust e2e が通ること(migration-plan Phase 3 完了条件)
- [ ] `Cargo.toml` の `rig` 系依存が `=0.43.x` pin で、Rig 型参照が `cheaper-rig-adapter`(+ `cheaper-provider-devin`)に限定されることの code review
- [ ] `meta.usage.costUsd` が provider 応答値ではなく pricing 層算出であることを fixture/単体 test で確認

## Rollback Strategy

- 本フェーズの成果物は新設 cargo workspace 内の crate に留まり、配布物・
  外部 contract は無改変(TS SEA が現行のまま)。問題時は `git revert` で
  撤去可能。
- devin provider のみ切り下げる場合は composition root(`cheaper-cli`)の
  plugin 登録を止める。adapter/runtime 本体への影響は無い(trait
  境界で隔離済み)。
- credential store は `<userConfigDir>` 直下の新規 file で
  `<userConfigDir>/pi/` に触れないため、TS 版へ戻しても Pi credential/
  catalog state は破壊されない(双方向の移行は行わない)。
- OAuth flow が不達の場合は対象 provider を api_key/env のみに縮小し、
  `auth status` の `methods`/`configured` で判別可能にする(flow 未実装
  provider は `oauth` method を advertise しない)。

## Open Questions / Gaps

- cargo workspace root は Phase 1 確定値(repo root `Cargo.toml` +
  `members = ["crates/*"]`)に確定済み。本書の `crates/` 表記は確定値。
- `default_max_turns` に渡す具体値(ISS-11): 「十分大きな値」の選定は
  実装時。cheaper hook が正であり Rig 側は未到達が前提。
- user `models.json` の merge 規則詳細(bundled 同名 model の上書き可否・
  `samplingParams` merge の key 単位挙動、ISS-14 残): 実装前に fixture で
  固定する。
- credential store の file 名・schema(Pi `auth.json` 非互換の独自形式):
  実装時に fixture で固定。`auth status` の `result.providers[]` 再現規則
  (ISS-19a)と非存在時の `models`/`run` 挙動(ISS-19b)も同時に固定。
- OAuth 対象 provider の初期 scope(ISS-23): github-copilot を必須 1 件
  とし、openai-codex/anthropic 等は Deferred の OAuth scope 決定時に追加
  sub-task 化。
- devin の初期同梱可否: 実 endpoint smoke 結果次第。切り下げ時の UX
  (`auth status` unsupported 表示)は Phase 4 CLI 側と整合させる。
- `metadata.rs` の macOS `PRODUCT_JSON` 参照(windsurfVersion 検出)は
  Rust では `FALLBACK_WINDSURF_VERSION` 固定に切り下げるか: wire fixture
  で挙動を固定して判断。
- schema error item の field 構成(ISS-15): 現行 `{instancePath,message}`
  を維持し、`code` 追加は行わない(新契約は spec 側の明示が必要)。

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: subagent-capable implementation(crate 単位の並行
  dispatch 可。T1↔T2 並行、T5/T6 は trait 確定後に独立)
- Independent work streams: T1(adapter)と T2(catalog/auth)は並行開始可。
  T5(devin)は step 26-28 を先行し 29/31 は T1/T2 確定後。T6(OAuth)は
  T2 後に独立。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 前提: Phase 0 go 条件 a-e(hook usage 粒度・Retry action・HTTP status
  露出・cancel 分類・panic 境界)の実測が記録済みであること。Phase 1/2 の
  提供 API(`resolveModel`/`createInvocationToolRuntime`/`MutationJournal`/
  `executionMetrics`)の signature は各 Phase plan の成果物に従う。
- レビュー重点 1(意味論固定): `default_max_turns` への大きな値 +
  cheaper hook を正とする maxTurns 判定、`InvalidToolCallAction::Retry{feedback}`、
  `tool_concurrency=1`、invalid call を `toolCalls`/`rejectedToolCalls`
  のどちらにも計上しない規則が実装に忠実か。
- レビュー重点 2(usage 収集): hook ベース逐次集計が唯一の正であり、
  repair session・transient retry も同一 accumulator に合算されること。
  `PromptResponse` 依存が残っていないか。
- レビュー重点 3(error 境界): `PromptError`→`CheaperError` 写像が adapter
  境界で完結し Rig error が上位に漏れないか。abort 分類・
  PARTIAL_MUTATION・meta snapshot が error/abort/maxTurns/INVALID_OUTPUT
  の全経路で構築されるか。
- レビュー重点 4(非契約の扱い): schema error の契約は `instancePath`
  のみとし、message 文言・件数・code は非契約とすること。
  `usage.costUsd` が pricing 層算出であること。
- 未解決の Deferred は Open Questions に列挙済み。devin 切り下げ・OAuth
  追加 scope はユーザー承認が前提。
