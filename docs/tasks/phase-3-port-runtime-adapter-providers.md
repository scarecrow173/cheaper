---
id: "TASK-034BgKjvy0B4BNsyAhtFht"
type: "task"
status: "todo"
title: "Phase 3: port runtime adapter providers"
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
    - "docs/plans/phase-3-port-runtime-adapter-providers.md"
    - "docs/plans/migrate-cheaper-runtime-to-rust-rig.md"
  implemented-by: []
  depends-on:
    - "docs/plans/phase-3-port-runtime-adapter-providers.md"
    - "docs/tasks/phase-1-port-cheaper-core.md"
    - "docs/tasks/phase-2-port-cheaper-tools.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rust-migration-run-semantics.md"
    - "docs/test-specs/rust-migration-auth-catalog.md"
  references: []
  defers: []
  deferred-by: []
---

# Phase 3: port runtime adapter providers

## Work

plan `docs/plans/phase-3-port-runtime-adapter-providers.md`
(PLAN-034BepZr0oX3bWeqVXNk91)の実行単位。`cheaper-rig-adapter`・
`cheaper-runtime`・`cheaper-provider-devin` の 3 crate を新設し、
`packages/runtime-pi` の責務(session・model 解決・run オーケストレーション・
usage 集計・repair・auth/catalog・devin provider)を Rig `=0.43.x` 上に移植
する。ギャップ G1(catalog store)・G2(credential store + OAuth)・
G3(transient retry)・G4(devin provider)・G7(fake provider seam)は本
task 内で自前実装する。

plan の Task Breakdown T1-T6 が本 task の作業単位:

- T1 `cheaper-rig-adapter`(plan step 9-14): `cheaper-tools` scope を expose
  する Rig `Tool`/`PortableTool` 実装、`AgentBuilder` 構成(system prompt
  完全再現・`default_max_turns` 大値・`tool_concurrency(1)`・
  `InvalidToolCallAction::Retry{feedback}`)、hook 配線と accumulator
  (**唯一の usage 収集源**、`costUsd` は token 数のみ保持)、
  `PromptError`→`CheaperError` 写像、completion-call 単位 transient
  retry(`transientRetries` 計上)、streaming run API(cancel 中継・
  delta/turn event 配送、公開面は streaming のみ)
- T2 catalog + credential + auth(plan step 2-8): Pi `models.json` 互換
  schema、bundled snapshot `include_str!` 埋込、catalog store(bundled +
  user merge・`get_available`/`get_model`・silent fallback 禁止)、
  credential store(`<userConfigDir>` 直下新規 file・`<userConfigDir>/pi/`
  非参照・atomic write)、api_key + env var 解決、`auth status/login/
  logout` service 面、provider plugin contract trait
- T3 run オーケストレーション(plan step 18-25): abort 一元化
  (`CancellationToken`・reason 先勝ち・timeout/signal/maxTurns/
  maxToolCalls 分類)、prompt abort grace settle(既定 5000ms)、output
  抽出、`run_worker` 本体、`WorkerRunError`(meta snapshot 運搬・
  PARTIAL_MUTATION 包装)、`build_meta`(`WorkerRunMeta` 全 field・
  `savings` 計算・`costUsd` = pricing 層算出)、repair session loop、
  panic/`JoinError` → `INTERNAL_ERROR` 写像
- T4 fake seam + fixture(plan step 15-17 + 34): fixture DSL
  (`delay_ms`/`error{status,retryable}`/`hang_until_abort`/`abort_response`/
  `usage_delta`)、`cheaper-fake/fake-model` の catalog/keyless credential/
  scripted stream の 3 層、fixture replay 検証基盤
- T5 devin provider(plan step 26-31): wire/protobuf-lite + Connect
  framing、context_map/metadata/jwt、stream transport(GetChatMessage)、
  Rig custom `Wire`/`Decoder`/`Transport` 接着、Devin CLI runner・
  credentials・models catalog、plugin trait 実装(組込は Phase 4 の
  composition root)
- T6 OAuth(plan step 32-33): OAuth flow contract 骨組み + github-copilot
  flow(必須 1 件)

依存関係: T1↔T2 は並行可。T4 の catalog/auth 層は T2 後、stream 層は T1
後。T5 は step 26-28 を先行し 29/31 は T1/T2 確定後。T6 は T2 後に独立。
Rig 型の `use` は `cheaper-rig-adapter` と `cheaper-provider-devin` のみに
限定する(設計上の隔離原則)。

実装中に残 gap として追加した slice: T7 builtin provider factory
(`cheaper-rig-adapter/src/builtin.rs` — catalog `api` 種別 → Rig provider
client 写像) + Ollama real-provider e2e(`cheaper-runtime/tests/
ollama_e2e.rs`)。dependency direction(runtime → adapter)上
`CompletionModelFactory` 自体は adapter に実装できないため、adapter は
組立関数 `builtin_completion_model` のみ提供し、runtime 側の factory 実装
(e2e test / Phase 4 composition root)が呼ぶ形にした(IR Slice T7)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/phase-3-port-runtime-adapter-providers.md`(本 task が
  implement する plan。File Map・Implementation Sequence・Verification
  の正)
- `docs/plans/migrate-cheaper-runtime-to-rust-rig.md`(親 plan)
- `docs/research/rig-agent-runtime/migration-plan.md` §Phase 3・§3.3
  (differential golden 戦略)
- `docs/designs/rust-rig-runtime-migration-architecture.md`(crate 表・
  意味論固定・G1/G2/G4・fake seam 3 層)
- `docs/designs/pi-execution-engine.md`・`docs/designs/model-resolver.md`・
  `docs/designs/repair-abort-and-failure-normalization-hardening.md`・
  `docs/designs/auth-command.md`・`docs/designs/auth-io-boundary.md`・
  `docs/designs/auth-url-opener-seam.md`・
  `docs/designs/devin-local-provider-plugin.md`(各領域の元設計)
- `docs/test-specs/rust-migration-run-semantics.md`・
  `docs/test-specs/rust-migration-auth-catalog.md`(本 task を検証する spec)

## Dependencies

- `docs/tasks/phase-1-port-cheaper-core.md` — `cheaper-core` の
  `check_capabilities`/`resolveModel`/`resolveExplicitModel`/
  `parseModelRef`/pricing snapshot/`CheaperError` ERROR_TABLE/
  `isDirectlyComparableWorker`/token 計測が確定済みであることが前提
- `docs/tasks/phase-2-port-cheaper-tools.md` — `cheaper-tools` の
  `createInvocationToolRuntime`/worker scope・`MutationJournal`・
  budget/meter・scheduler が確定済みであることが前提
- `docs/plans/phase-3-port-runtime-adapter-providers.md` — 承認済み plan
  への従属
- Phase 0 の go 条件(hook 経由 usage 粒度・Retry action・HTTP status
  露出・streaming `cancel()` 分類・panic 境界)の実測記録済みであること
- devin 初期同梱可否の切り下げ・OAuth 追加 provider scope はユーザー承認
  が前提

## Affected Files

- `crates/cheaper-rig-adapter/`(新設。`src/{lib,agent,tools,hooks,
  accumulator,retry,run_stream,error,fake_stream}.rs` + `tests/`)
- `crates/cheaper-runtime/`(新設。`src/{lib,run,abort,grace,output,repair,
  meta,run_error,panic_boundary,plugin,fake_provider}.rs`・
  `src/catalog/{mod,schema,bundled}.rs`・`src/auth/{store,api_key,service,
  oauth/mod,oauth/github_copilot}.rs` + `tests/fixtures/` +
  `tests/run_fixture_test.rs`)
- `crates/cheaper-provider-devin/`(新設。`src/{cli,credentials,models,
  wire,jwt,metadata,context_map,stream,rig_provider,plugin}.rs` + `tests/`)
- `Cargo.toml`(workspace member 追加)

## Done When

- [x] T1: `cheaper-rig-adapter` が Rig `Tool`/`PortableTool` 接着・
      `AgentBuilder` 意味論固定・hook→accumulator 逐次集計・error 写像・
      transient retry・streaming run API を実装し、crate 外向け signature
      に Rig 型を出さない
      (`docs/impl/ir/phase-3-port-runtime-adapter-providers.md` Slice T1。
      tool 接着は `DynamicTool` seam を使用。`CompletionModel::from_rig`
      の `DynModel` 引数は provider→adapter 境界の明示的 seam のみ)
- [x] T2: catalog store・credential store・api_key/env 解決・auth service・
      plugin contract trait が実装される
      (`docs/impl/ir/phase-3-port-runtime-adapter-providers.md` Slice T2。
      `src/catalog/{mod,schema,bundled}.rs`・`src/auth/{store,api_key,
      service,mod}.rs`・`src/plugin.rs`・`src/runtime.rs` — catalog 12 /
      auth 19 / plugin 4 test + fixture 19-22 が auth/catalog 経路を検証)
- [x] T3: `run_worker` オーケストレーション(abort/grace/output/meta/
      repair/panic 境界を含む)が実装され、`WorkerRunMeta` 全 field が hook
      accumulator のみから構築される
      (`docs/impl/ir/phase-3-port-runtime-adapter-providers.md` Slice T3。
      `src/{run,abort,grace,output,repair,meta,run_error,panic_boundary,
      fake_provider}.rs` + `tests/run.rs`)
- [x] T4: fixture DSL + fake provider 3 層が実装され、fixture replay による
      `run_worker` 検証基盤が成立する
      (`docs/impl/ir/phase-3-port-runtime-adapter-providers.md` Slice T4。
      `src/fixture.rs` + `fake_provider.rs` の
      `ScriptedSession`/`ScriptStep`/`CheaperFakePlugin` +
      `tests/fixtures/*.json` 24 件 + `tests/run_fixture_test.rs`)
- [x] T5: devin provider(wire/context_map/jwt/stream/Rig 接着/CLI 委譲/
      plugin trait 実装)が port される
      (`docs/impl/ir/phase-3-port-runtime-adapter-providers.md` Slice T5。
      `crates/cheaper-provider-devin/src/{wire,metadata,jwt,context_map,
      stream,cli,credentials,models,rig_provider,plugin,lib}.rs` +
      `src/tests/` 51 test + `NOTICE`。`DevinCompletionModelFactory` が
      `CompletionModelFactory` 経由で `CompletionModel::from_rig` へ接続)
- [x] T6: OAuth flow contract + github-copilot flow が実装される
      (`docs/impl/ir/phase-3-port-runtime-adapter-providers.md` Slice T6。
      `src/auth/oauth/{mod,github_copilot}.rs` + `AuthInteraction::event`/
      `is_cancelled`・`AuthPrompt::{Text,ManualCode,Select}`・
      `ResolvedAuth.base_url` + `Runtime` builtin flow 自動登録 +
      `tests/oauth.rs` 18 test)
- [x] `Cargo.toml` の `rig` 系依存が `=0.43.x` pin で、Rig 型参照が
      `cheaper-rig-adapter`(+ `cheaper-provider-devin`)に限定される
      (workspace root `rig = "=0.43.0"`。`grep "rig::" crates/
      cheaper-runtime/src/` は 0 件 — runtime の公開 signature に Rig 型
      なし。Rig 型を持つのは `cheaper-rig-adapter`・`cheaper-provider-
      devin`・PoC(`crates/poc-rig`、workspace 外)のみ。`builtin.rs`
      (T7)も Rig 型を adapter 内に閉じ込め opaque `CompletionModel` のみ
      返す)

## Verification

- [x] `mise exec -- cargo test -p cheaper-rig-adapter -p cheaper-runtime
      -p cheaper-provider-devin` が全パス(hook→accumulator 逐次集計・
      retry 429/5xx/timeout 分類・Retry action の error feedback→loop
      継続・cancel reason 分類・wire framing round-trip・context map・
      models parse の単体 test を含む)(T7 後の実測: provider 61 +
      adapter 26(unit 7 + integration 19)+ runtime 99(うち
      `ollama_e2e` 3 は gate 未満で skip)= 186 test 全パス)
- [x] `mise exec -- cargo test -p cheaper-runtime --test run_fixture_test`
      で fake fixture による `run_worker` 検証が全パス(正常系/invalid
      output→repair/timeout・maxTurns・maxToolCalls・signal abort 分類/
      journal 非空 abort→PARTIAL_MUTATION/error envelope。`meta` 全 field
      が期待値と一致。normalize: `meta.durationMs`・`usage.costUsd`・
      schema error message 文言は除外、`instancePath` は厳密)
      (fixture corpus 24 file 全 replay。normalize は `expect` 宣言
      field のみ比較で実装 — 非決定 field は宣言しない限り比較対象外)
- [x] fixture DSL primitive が動作: `delay_ms`→timeout abort、
      `error{status:429}`→transientRetries 計上 + 成功、`error{status:5xx}`
      連続→PROVIDER_FAILURE、`hang_until_abort`→signal abort 分類、
      `abort_response`→ABORTED、`usage_delta`→accumulator 反映
      (fixtures 05/12/15/08/11/18 が各 primitive を検証)
- [ ] `cheaper auth login github-copilot` から run まで成功し credential
      が新 credential store に永続化される(手動 smoke、CI 外)。
      `<userConfigDir>/pi/` の auth.json/models.json を参照しないこと
      — **未実施(Phase 3 末延期)**: 実 GitHub credential が必要で CI/
      agent 環境では検証不能。oauth flow 自体は `tests/oauth.rs` 18 test
      で mock HTTP transport 経由済み。実 credential smoke は Phase 4 の
      CLI 組立後に手動で実施する
- [ ] api_key auth(Anthropic/OpenAI/OpenAI-compatible)で login→run が
      成立し secret が stdout に出ないこと
      — **部分カバーのみ**: openai-compatible 経路(Ollama custom
      provider・configured literal `apiKey`)は `ollama_e2e` で login 相当
      (configured)→run 実到達を検証済み(T7)。Anthropic/OpenAI 実
      credential の login→run と stdout 非漏洩は Phase 4 CLI 後の手動
      smoke に延期(実 API key 不可のため)
- [x] 未知・スコープ外 provider api 種別で catalog 層が明示 error
      (`INVALID_MODEL_CONFIG` 系)を返し silent fallback しないこと
      (fixture 21: model.api が plugin 宣言 api と不一致 → run 経路で
      `INVALID_MODEL_CONFIG` exit 3。`tests/catalog.rs`
      `unsupported_api_kind_returns_invalid_model_config` でも固定済み)
- [x] `docker compose up -d` + model pull 済み Ollama で built-in worker
      3 種が成功 envelope を返す Rust e2e が通ること
      (`docs/impl/ir/phase-3-port-runtime-adapter-providers.md` Slice T7。
      `crates/cheaper-rig-adapter/src/builtin.rs` の catalog api→Rig
      client 組立 + `crates/cheaper-runtime/tests/ollama_e2e.rs`
      (`CHEAPER_E2E_OLLAMA=1` + `/v1/models` probe で auto-skip)。
      実測(qwen3.5:9b、3 test 並行): bulk-reader turns=5 toolCalls=2
      usage in=13249/out=1848 repairs=1 transientRetries=2 293.8s、
      code-writer turns=3 toolCalls=1 in=4843/out=431 repairs=1 125.3s、
      doc-writer turns=4 toolCalls=2 in=7359/out=816 repairs=1 163.8s —
      全件成功 envelope + `meta.usage` 非零 + provider/model id 一致)
- [ ] devin 実 endpoint smoke(`auth login/status/logout`・`devin models
      list`・1 件の run)を実施し結果を plan に記録する(CI 外手動。
      不達時は切り下げ判断をユーザー承認に回す)
      — **未実施・escalation pending**: 実 Devin endpoint/credential への
      到達が本環境では不能なため、item 本文の規定どおり切り下げ判断の
      ユーザー承認待ち
- [x] `meta.usage.costUsd` が provider 応答値ではなく pricing 層算出で
      あることを fixture/単体 test で確認する
      (`tests/run.rs` `success_extracts_output_and_builds_meta` が
      cheaper pricing 由来の costUsd=0.005075 を assert。
      fixture `01-success-basic.json` でも replay 経路で
      `meta.usage.costUsd=0.005075` を厳密比較)
- [ ] `mise run check` が現行通り全 green(TS 側無破壊)
      — 本変更は `crates/` + `docs/` のみで TS package には未触。
      Rust 側は `cargo test`(186 test)/`cargo clippy --all-targets
      -D warnings`/`cargo fmt --check` 全 green(実施済み)。TS を含む
      統合 `mise run check` の再実行は Phase 4 切替時に実施
