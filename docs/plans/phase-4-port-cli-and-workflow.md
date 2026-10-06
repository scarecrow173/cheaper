---
id: "PLAN-034Beq0dg6IYk2lQCGpWDG"
type: "plan"
status: "approved"
title: "Phase 4 port CLI and workflow"
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
    - "docs/plans/phase-3-port-runtime-adapter-providers.md"
  blocks:
    - "docs/plans/phase-5-switch-distribution-to-rust-binary.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
    - "docs/designs/rust-rig-runtime-migration-architecture.md"
    - "docs/plans/migrate-cheaper-runtime-to-rust-rig.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rust-migration-envelope-parity.md"
    - "docs/test-specs/rust-migration-differential-golden.md"
    - "docs/test-specs/rust-migration-usage-log-hooks.md"
  references: []
  defers: []
  deferred-by: []
---

# Phase 4 port CLI and workflow Implementation Plan

## Goal

`packages/cli`(≈3.6k 行)と `packages/workflow`(≈2.1k 行)の全責務を Rust crate
`crates/cheaper-cli`・`crates/cheaper-workflow` へ移植し、envelope(`schemaVersion:1`・
1 JSON + 末尾改行)・exit code 表・hook 4 target・usage.jsonl の byte/意味互換を、
TS binary との binary-to-binary differential golden(normalize 規則適用後厳密比較)で
担保する。

## Scope

1 つの実装ストリームとして扱う(2 crate + differential harness)。Deferred Concern
「workflow engine 移植のフェーズ位置」は **Phase 4 同居** と決定する:
`cheaper-workflow` は Rig 非依存で先行実装できるが、`workflow run/interface`
コマンドが TS 版に残ると CLI 切替が完了しないため、分離する意味が薄い。

境界:

- **含む**: `cheaper-workflow`(workflow 定義 schema・compiler・DAG engine・job 並行
  実行・lease・executor port・artifact store・bindings・resource inference・
  interface 投影)、`cheaper-cli`(argv 解釈・command dispatch・envelope
  serializer・stdin input・`--format text`・usage.jsonl writer/reader・
  `cheaper usage` 集計・`auth` 3 コマンドの CLI 結線・hook 4 target adapter +
  routing policy・SIGINT/SIGTERM 中継・builtin resource 埋込 G8・version 解決・
  panic 境界)、differential golden harness(`crates/cheaper-cli/tests/differential/`)。
- **含まない(Phase 3 の成果物を consumer として利用)**: `runWorker` 相当の run
  オーケストレーション・`WorkerRunMeta`/`WorkerRunError` 型・model catalog store・
  credential store/OAuth flow 本体・transient retry・devin provider・fake provider
  seam(G7)の内部実装。`cheaper-runtime` が公開する trait 面(runner・catalog・
  auth port)経由でのみ接続し、Rig 型は一切参照しない。
- **含まない(他 Phase)**: cargo release matrix・asset 名・SHA256SUMS・launcher
  (Phase 5)、TS 撤去(Phase 6)、`cheaper-tools` の内部(Phase 2。ただし
  workflow executor port から invocation runtime・lease・ToolCatalog を組み立てる
  結線は本フェーズ)。
- 新機能・envelope schema 拡張・worker 追加は非スコープ。

## Upstream Documents

- `docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md` —
  不変 contract・parity 検証規則(normalize 除外/厳密比較)・Acceptance Criteria。
- `docs/adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md` —
  Rust+Rig 採用・adapter 隔離・並走/撤去戦略の決定。
- `docs/research/rig-agent-runtime/migration-plan.md` — §3.3 並走比較戦略
  (fixture 収録→Rust replay・hook byte 一致)、Phase 4 成果物/完了条件表、
  R8(silent regression)緩和。
- `docs/plans/migrate-cheaper-runtime-to-rust-rig.md` — 親 plan の共通ルール
  (stdout 契約・panic → INTERNAL_ERROR・hook fail-open 等)。

## Design Inputs

- `docs/designs/rust-rig-runtime-migration-architecture.md` —
  crate 表の `cheaper-workflow`/`cheaper-cli` 行、境界ルール(Rig 型参照禁止・
  tokio 依存許容範囲)、「envelope / CLI 互換(`cheaper-cli`)」節
  (数値フォーマット規則・panic 境界・null 意味論・usage.jsonl 新旧互換・
  builtin resource 埋込)、段階計画表 Phase 4 行、Verification Notes の
  normalize 規則・fixture DSL 要件・ISS-19/ISS-22 申し送り。
- 移植元(責務把握済み、全て `packages/` 配下):
  - `cli/src/args.ts` — `node:util.parseArgs` ベースの argv 解析。command 別の
    option 許可表、worker alias 表(`bulk-read`→`bulk-reader` 等)、
    `NAME_PATTERN`、`INVALID_WORKER_NAME`/`INVALID_REQUEST` 分類。
  - `cli/src/envelope.ts` — success/utility/error/workflow 4 形態、
    `errorEnvelope` の null 意味論・`details`/`cause` optional 省略規則、
    `envelopeJson` = `JSON.stringify` + `"\n"`。
  - `cli/src/cli.ts` — `runCli` dispatch・`executeRun`(worker/policy/model 解決、
    SIGINT/SIGTERM → AbortSignal 中継、model line stderr)・`executeModels`・
    `usage`/`count-tokens`/`workflow`/`auth` 配線・汎用 catch → error envelope。
  - `cli/src/input.ts` / `count.ts` — stdin/file task input(JSON object 必須)、
    workspace file token 計測経路、`CHEAPER_COUNT_MAX_BYTES` 解釈、
    `FileTooLargeError`/`WorkspaceEscapeError`/`ENOENT` → CheaperError 写像。
  - `cli/src/usage-log.ts` — `buildUsageRecord`(`kind:"run"|"count"`、
    usage-bearing error のみ記録)・`appendUsageLog`・best-effort `recordUsage`。
  - `cli/src/usage-report.ts` + `pricing.ts` — `readUsageRecords`(破損行 skip)・
    filter(`--since`/`--worker`/`--provider`/`--model`)・aggregate・
    `--compare-model` estimate(pricing 解決: runtime catalog → user
    `<userConfigDir>/pricing.json` → bundled snapshot、stale=30 日)。
  - `cli/src/auth.ts` — `auth status/login/logout` dispatch、
    `result.credentials[]`/`result.providers[]`、terminal interaction bridge
    (select prompt・secret raw-mode・auth_url opener・`--method` 即答・
    非 TTY → INVALID_REQUEST)。
  - `cli/src/hook/{hook,targets,routing-policy}.ts` — 緩い argv 解析・
    KNOWN_EVENTS・stdin JSON・4 target(windsurf/copilot/pretooluse/cursor)の
    normalize/render・shared routing policy(threshold・byte fast path・
    shell tokenizer + `extractReadCandidates`・30s measure timeout)・
    fail-open catch-all。
  - `cli/src/workflow-context.ts` / `workflow-run.ts` — compiler factory
    (worker 事前解決・ToolCatalog・ResourceInference)、executor/lease port の
    実装、WorkflowEnvelope 組立(artifacts 投影含む)。
  - `cli/src/text-format.ts` / `version.ts` / `main.ts` / `sea-main.ts` —
    `--format text` 全分岐、`version.json`→`package.json` version 解決、
    entry 構成。
  - `workflow/src/{types,json,bindings,dag,schema,compiler,interface,
    resource-inference,artifacts,engine,loader}.ts` — workflow 定義 JSON
    schema 検証・`$input`/`$from`/`$literal` binding・JSON Pointer・
    topological compile・`runWorkflow` DAG engine(pump/admit/lease/
    abortGrace/diagnostic UTF-8 2048B cap)・RunArtifactStore・
    interface 投影・loader(workspace path 解決 + denylist)。

## File Map

- `crates/cheaper-workflow/Cargo.toml` - crate 定義(依存: `cheaper-core`・
  `cheaper-tools`・tokio・serde/serde_json・`jsonschema`。Rig 非依存)
- `crates/cheaper-workflow/src/lib.rs` - 公開面 re-export(`index.ts` 相当)
- `crates/cheaper-workflow/src/json.rs` - `JsonValue` helper・`clone_json`
  (plain JSON 限定・cyclic 拒否) ← `workflow/src/json.ts`
- `crates/cheaper-workflow/src/types.rs` - `WorkflowDefinition`/`WorkflowJob`/
  `Binding`/`CompiledWorkflow`/`WorkflowOutcome`/`WorkflowEnginePorts` trait 群
  ← `workflow/src/types.ts` + `engine.ts` の port 型
- `crates/cheaper-workflow/src/bindings.rs` - `parseBinding`/`scanBindings`/
  `materializeBindings`/JSON Pointer ← `bindings.ts`
- `crates/cheaper-workflow/src/dag.rs` - `compile_dag`(topological order・
  self/cycle/unknown dep → INVALID_REQUEST、sort 済み deterministic order)
  ← `dag.ts`
- `crates/cheaper-workflow/src/schema.rs` - `WORKFLOW_JSON_SCHEMA` +
  `validate_definition`(inputs/jobs/outputs の schema 検証、tool catalog・
  worker catalog 参照)← `schema.ts`
- `crates/cheaper-workflow/src/interface.rs` - `project_workflow_interface`
  (definition 検証のみ、実行を伴わない公開射影)← `interface.ts`
- `crates/cheaper-workflow/src/resource_inference.rs` - `WorkspaceResourceInference`
  (worker/tool から manifest/edge/conflict 推論)← `resource-inference.ts`
- `crates/cheaper-workflow/src/artifacts.rs` - `RunArtifactStore`・
  `projectArtifactMetadata` ← `artifacts.ts`
- `crates/cheaper-workflow/src/compiler.rs` - `WorkflowCompiler`(
  `validateDefinition`/`compile`/`materializeJobInput`/`materializeOutput`/
  `validateJobOutput`/`projectInterface`)← `compiler.ts`
- `crates/cheaper-workflow/src/engine.rs` - `run_workflow`(tokio 化した
  pump/admit/execute ループ、`maxJobs` 並行、lease acquire/release、
  timeout/abort + abortGrace detach、job outcome/diagnostic/execution metrics、
  output materialize + outputErrors)← `engine.ts`
- `crates/cheaper-workflow/src/loader.rs` - `load_workflow_file`(workspace path
  解決・denylist・JSON parse → INVALID_INPUT)← `loader.ts`
- `crates/cheaper-cli/Cargo.toml` - bin crate 定義(依存: `cheaper-core`・
  `cheaper-tools`・`cheaper-runtime`・`cheaper-workflow`・
  `cheaper-rig-adapter`・`cheaper-provider-devin`・tokio・serde_json・
  `include_dir`・`ctrlc` or signal-hook。**composition root**:
  adapter session 構築と devin plugin 登録(runtime+devin 両方への dep)は
  cli が担う — runtime→devin feature は循環依存で不可。Rig 型参照禁止)
- `crates/cheaper-cli/src/main.rs` - tokio entry、panic hook(stdout 抑止)+
  catch_unwind/JoinError → `INTERNAL_ERROR`(exit 70)envelope、`runCli` 呼出し、
  process exit code ← `main.ts`/`sea-main.ts`
- `crates/cheaper-cli/src/envelope.rs` - 4 形態 serde serialize、
  `skip_serializing_if` による optional 省略、error null 意味論、
  `envelope_json` = serialize + `"\n"` ← `envelope.ts`
- `crates/cheaper-cli/src/args.rs` - argv → `ParsedCli` enum(command 別 option
  許可表・alias・`NAME_PATTERN`・`INVALID_WORKER_NAME` 分類)← `args.ts`
- `crates/cheaper-cli/src/cli.rs` - `run_cli` dispatch 本体(help/version/
  run/alias/count-tokens/usage/models/auth/workflow/hook 早期分岐、
  汎用 catch → error envelope + recordUsage + stderr diagnostic、
  `--quiet` 時の workflow error diagnostic 抑止)← `cli.ts`。
  **内部 mode dispatch**: `cheaper __grep-eval`(envelope 外・早期分岐、
  argv 先頭一致で `cheaper_tools::grep_eval::helper_main` を起動。
  Phase 2 の `grep_eval.rs` subprocess supervisor の spawn 先)を配線する
  — 欠落すると production で grep tool が全滅する(必須)
- `crates/cheaper-cli/src/input.rs` - `read_task_input`(stdin TTY/空/file
  読込/JSON object 必須)← `input.ts`
- `crates/cheaper-cli/src/run.rs` - `execute_run`(root canonicalize・
  config dirs・worker 解決・policy restrict・`--timeout`・model runtime 取得・
  SIGINT/SIGTERM → CancellationToken 中継・model line stderr)← `cli.ts`
  executeRun
- `crates/cheaper-cli/src/count.rs` - `count_workspace_file_tokens`・
  `CHEAPER_COUNT_MAX_BYTES` 解釈・error 写像 ← `count.ts` + `cli.ts`
  count-tokens 分岐
- `crates/cheaper-cli/src/models.rs` - `execute_models`(catalog `get_available`
  → exclude/autoContext → classify → filter → `result.models[]`)← `cli.ts`
  executeModels
- `crates/cheaper-cli/src/usage_log.rs` - `UsageLogRecord`(serde、新旧 record
  を Option field で読む additive-only schema)・`build_usage_record`・
  `append_usage_log`・best-effort `record_usage` ← `usage-log.ts`
- `crates/cheaper-cli/src/usage_report.rs` - `read_usage_records`・filter・
  `aggregate`・`estimate`・`build_usage_report` ← `usage-report.ts`
- `crates/cheaper-cli/src/pricing.rs` - `load_pricing_snapshot`(user
  `pricing.json`、fail-soft・entry 単位 skip)← `pricing.ts`
- `crates/cheaper-cli/src/auth.rs` - `execute_auth`(status/login/logout・
  `result.providers[]`/`credentials[]` 再現)+ terminal interaction
  (select/secret/auth_url/device_code notify、`--method` 即答、非 TTY →
  INVALID_REQUEST、OS 別 URL opener)← `auth.ts`
- `crates/cheaper-cli/src/hook/mod.rs` - `execute_hook`(緩い argv 解析・
  KNOWN_EVENTS・stdin 読取・catch-all → fallbackAllow)← `hook/hook.ts`
- `crates/cheaper-cli/src/hook/targets.rs` - windsurf/copilot/pretooluse/cursor
  normalize + render(byte 互換、copilot は常に exit 0)← `hook/targets.ts`
- `crates/cheaper-cli/src/hook/routing_policy.rs` - threshold・byte fast path・
  measure timeout 30s・shell tokenizer/`extract_read_candidates`・
  `evaluate` ← `hook/routing-policy.ts`
- `crates/cheaper-cli/src/workflow_context.rs` - compiler factory 組立
  (worker 事前解決・ToolCatalog・ResourceInference)← `workflow-context.ts`
- `crates/cheaper-cli/src/workflow_run.rs` - `run_workflow_command`(executor/
  lease port 実装、`selectWorkerMeta` 投影、WorkflowEnvelope 組立、artifact
  ledger)← `workflow-run.ts`
- `crates/cheaper-cli/src/text_format.rs` - `format_text`/`format_usage_report`
  ← `text-format.ts`
- `crates/cheaper-cli/src/embedded.rs` - `include_dir!` builtin resource
  (workers/*.md・schemas/*.json・policy.yaml・version.json)・version 解決・
  `defaultBuiltinDir` 相当(G8)← `count.ts`/`version.ts`/`sea-main.ts`
- `crates/cheaper-cli/src/fake_stream.rs` - fixture JSON を G7 fake provider
  seam へ ingest する dev/test 経路(`CHEAPER_TEST_FAKE_STREAM` 相当)
  ← `cli.ts` `loadTestStreamFn`
- `crates/cheaper-cli/tests/differential/mod.rs` - harness: fixture 読込、
  TS binary(`node cheaper.mjs` or SEA)と Rust binary を同一 input/env/cwd で
  実行、normalize 適用後に stdout envelope・exit code を比較
- `crates/cheaper-cli/tests/differential/normalize.rs` - normalize 規則の単一
  実装(除外: `meta.durationMs`・`meta.execution` の時刻系 field
  `operationMs`/`activeMs`/`overlapMs`/`queueMs`・`usage.costUsd`・
  `result.tokenizer.*`・`details.tokenizer.*`・`error.message` 本文・
  stderr 文言・key 順・schema error の message/件数/code。
  schema error は `instancePath` のみ契約。`error.code`/`exitCode`・
  `execution` の決定的 field は厳密比較)
- `crates/cheaper-cli/tests/differential/fixtures/` - scripted provider 応答 +
  worker input + 期待属性を記述する JSON fixture 20-40 case(§fixture DSL は
  設計書 Verification Notes 準拠: delay/stream 途中 error/hang/usage delta を
  表現可能)
- `crates/cheaper-cli/tests/envelope.rs` - envelope 形態・null 省略・
  exit code 表・canonical number formatter の contract test
- `crates/cheaper-cli/tests/hook.rs` - 4 target × event × allow/deny/malformed
  の byte 一致 + fail-open test
- `crates/cheaper-cli/tests/usage_log.rs` - usage.jsonl 新旧 record 読込・
  additive-only 記録・best-effort 失敗非伝播 test
- `docs/guides/usage.md` - `meta.usage.costUsd` 口径変更(pricing 層算出)と
  新旧 record 混在の明記(modified)

## Implementation Sequence

### A. cheaper-workflow(Phase 3 非依存で先行可能)

- [ ] `crates/cheaper-workflow` を cargo workspace に追加し依存を配線する
- [ ] `json.rs` を port する(`clone_json`・plain JSON 限定・cyclic 拒否)
- [ ] `types.rs` に `WorkflowDefinition`/`CompiledWorkflow`/port trait 群を定義する
- [ ] `bindings.rs` を port する(`$input`/`$from`/`$literal`・JSON Pointer)
- [ ] `dag.rs` を port する(deterministic sort・cycle/self/unknown dep error)
- [ ] `schema.rs` を port する(`WORKFLOW_JSON_SCHEMA`・`validate_definition`)
- [ ] `interface.rs` を port する(`project_workflow_interface`)
- [ ] `resource_inference.rs` を port する
- [ ] `artifacts.rs` を port する(`RunArtifactStore`・`projectArtifactMetadata`)
- [ ] `compiler.rs` を port する(input default 補完・job input/output 検証・
      binding materialize)
- [ ] `engine.rs` を port する(tokio: pump/admit/execute・`maxJobs`・lease・
      timeout/abortGrace・diagnostic UTF-8 2048B cap・executionMetrics)
- [ ] `loader.rs` を port する(workspace path・denylist・JSON parse error)
- [ ] `packages/workflow` の unit/contract test を `cargo test -p cheaper-workflow`
      相当として移植し全パスさせる

### B. cheaper-cli 基盤

- [ ] `crates/cheaper-cli` bin crate を scaffold し `main.rs` に panic hook +
      catch_unwind/JoinError → `INTERNAL_ERROR`(70)境界を実装する
- [ ] `envelope.rs` を実装する(4 形態・`skip_serializing_if`・`cheaper-core`
      canonical number formatter 適用・`envelope_json` 末尾改行)
- [ ] `args.rs` を実装する(command 別 option 許可表・alias 表・worker name
      検証を `args.ts` の分類・文言規則で port)
- [ ] `input.rs`・`count.rs` を port する(stdin/file input・count-tokens の
      error 写像)
- [ ] `embedded.rs` を実装する(`include_dir!` 埋込 + version 解決)

### C. cheaper-cli コマンド

- [ ] `run.rs` を実装する(worker/policy/model 解決・signal 中継・model line)
- [ ] `models.rs` を実装する(`executeModels` の filter/classify/autoContext 経路)
- [ ] `usage_log.rs` を実装する(record schema・append・best-effort)
- [ ] `pricing.rs`・`usage_report.rs` を実装する(集計・`--compare-model` の
      runtime catalog → user pricing.json → bundled snapshot 解決・stale 判定)
- [ ] `auth.rs` を実装する(3 action・`result.providers[]`/`credentials[]`・
      terminal interaction・`--method`・URL opener)
- [ ] `hook/` を実装する(dispatch + targets + routing policy、fail-open
      catch-all)
- [ ] `workflow_context.rs`・`workflow_run.rs` を実装する(executor/lease
      port・WorkflowEnvelope)
- [ ] `text_format.rs` を実装する
- [ ] `cli.rs` の dispatch・汎用 catch・recordUsage 配線を完成させる
- [ ] `cheaper __grep-eval` 内部 mode dispatch を `cli.rs` に配線する
      (argv 先頭一致の早期分岐で `cheaper_tools::grep_eval::helper_main`
      を起動。envelope 対象外・Phase 2 supervisor の `current_exe` spawn
      先。欠落すると release binary で grep tool が全滅)
- [ ] adapter session 構築 + devin plugin 登録の composition root 配線を
      `cli.rs`/`main.rs` に実装する(cli が `cheaper-rig-adapter`・
      `cheaper-provider-devin`・`cheaper-runtime` に依存し plugin trait
      実装を runtime へ注入する。runtime→devin feature は循環依存で不可)

### D. 検証基盤

- [ ] `fake_stream.rs` を実装する(fixture JSON → G7 seam 注入。Phase 3 の
      seam 仕様に合わせる)
- [ ] `tests/differential/` harness を実装する(fixture 読込・両 binary 起動・
      normalize・厳密比較)
- [ ] golden fixture を TS 側で収録し 20-40 case に揃える(正常系・invalid
      output→repair・timeout・maxTurns・maxToolCalls・SIGINT・
      PARTIAL_MUTATION・provider error・error envelope。fake provider は
      `cheaper-fake` catalog/keyless credential/scripted stream の 3 層)
- [ ] TS CLI の e2e/contract test 相当を Rust binary 向けに再現する
- [ ] hook byte 一致 matrix test(4 target × allow/deny/malformed)を実装する
- [ ] usage.jsonl 新旧互換 test(旧 schema record の欠落 field 許容・新 record
      を旧 reader が読める)を実装する
- [ ] `docs/guides/usage.md` に `costUsd` 口径変更と新旧 record 混在を追記する

## Dependencies and Risks

- **Phase 3(必須前提)**: `cheaper-runtime` が公開する runner trait・
  `WorkerRunMeta`/`WorkerRunError`(error 経路で meta snapshot を運ぶ型)・
  catalog/auth port・G7 fake seam(catalog/keyless credential/scripted stream
  の 3 層注入)が確定済みであること。trait 面が未確定なら Phase 3 側で
  確定させてから本フェーズに着手する — CLI 側で勝手に境界を引かない。
- **Phase 1(前提)**: `cheaper-core` の canonical number formatter(JS 互換
  serializer helper)・ERROR_TABLE・`parseDuration`・`resolvePricing`・
  auto tier 分類・config dir 解決。`savings.returnedTokens` 算出の key 順序
  保持 serialize(ISS-19d)は runtime/tools 側の責務だが、differential の
  `savings` 厳密比較で regression を検出する。
- **Phase 2(前提)**: invocation runtime・`acquireJobLease`/`releaseJobLease`・
  `ExecutionMetrics`・ToolCatalog(`workflow_run` port が直接組み立てる)。
- リスク: serde_json の f64 出力(`1.0`/指数表現)が `JSON.stringify` と
  byte 非互換 → envelope/usage.jsonl/report の全数値 field に canonical
  number formatter を適用し、contract test で `1.0`/`1e-7`/非有限値を網羅。
- リスク: usage.jsonl への後方互換破壊 → record は additive-only(旧 reader が
  読める key のみ追加、新 reader は `savings`/`turns`/`toolCalls` 等を
  Option)。新旧双方向の読込 test を必須とする。
- リスク: hook の fail-open 欠落 → panic 含む全 error を target 適合
  allow/exit 0 に変換(copilot は非 0 を返さない)。envelope/recordUsage/
  汎用 catch の対象外とする早期分岐構造を維持。
- リスク: engine の pump/microtask 意味を tokio で再現する際の job 実行順・
  cancel 伝播の drift → deterministic ordering(job id sort 済み)を契約化し、
  TS test の schedule/cancel 意味を移植して固定。
- リスク: Windows 固有差分(SIGINT 中継・stdin raw-mode・`rundll32` URL
  opener・path 区切り)→ Windows runner での contract test を完了条件に含める。
- リスク: differential fixture の TS 側収録経路(Node `cheaper.mjs` 実行)が
  flaky → fixture は「input + scripted 応答 + env」を JSON 化して両側で
  replay する方式とし、時刻/乱数は注入で固定。
- リスク: `auth login` OAuth の対話経路(select/secret/device_code)は Phase 3
  G2 の flow 実装に依存 → CLI 結線のみ本フェーズ、flow 検証は Phase 3 側の
  smoke に委譲(実 provider OAuth は CI 外手動)。

## Task Breakdown

- **T1 `cheaper-workflow` crate port**(Sequence A 全 step)— Rig 非依存、
  Phase 3 と完全に並行可能な独立 stream。
- **T2 envelope/args/entry 基盤**(B)— 他コマンドの共通土台、最優先。
- **T3 run/count-tokens/models**(C 前半)— Phase 3 trait 面が必要。
- **T4 usage writer/report**(C)— `usage.jsonl` 互換が契約。
- **T5 auth サブコマンド**(C)— Phase 3 auth port に依存。
- **T6 hook 経路**(C)— envelope 機構と独立、T2 完成後に並行可。
- **T7 workflow CLI glue**(C)— T1 + T3 の成果物を接続。
- **T8 differential harness + fixture 収録**(D)— 全 command 移植後に本格化、
  harness 自体は T2 直後に先行構築可。
- **T9 互換 test + docs**(D)— gate 相当。
- 分解ルール: 1 task = 上記 Sequence の連続区間。crate 単位・機能単位で分割し、
  「envelope serializer 変更」と「usage writer 変更」のような contract 面を
  同一 task に混ぜない。

## Verification

- [ ] `cargo test -p cheaper-workflow` 全パス(TS `packages/workflow` の
      unit/contract test 相当を移植)
- [ ] `cargo test -p cheaper-cli` 全パス(envelope・args・usage・auth・
      workflow glue の unit/contract test)
- [ ] differential golden 全パス: `crates/cheaper-cli/tests/differential/` で
      TS binary と Rust binary の stdout envelope + exit code を normalize
      規則適用後に 20-40 case 厳密比較(厳密対象: `usage` token field・
      `turns`・`toolCalls`・`rejectedToolCalls`・`repairs`・
      `transientRetries`・`changes`・`savings`・`result`・exit code。
      除外: `meta.durationMs`・`meta.execution` 時刻系 field・
      `usage.costUsd`・tokenizer identity・`error.message` 本文・
      stderr 文言・JSON key 順・schema error message 文言/件数/code)
- [ ] envelope byte 比較: run/utility/error/workflow の全形態で
      `schemaVersion:1`・1 JSON + 末尾改行 1 つ・optional field 省略・
      null 意味論が TS 版と一致
- [ ] hook byte 一致: `cheaper hook <event> --target <agent>` の 4 target ×
      event × allow/deny/malformed stdin で stdout/stderr/exit code が TS 版と
      byte 一致、fail-open(malformed・未知 event・内部 error・panic)維持
- [ ] usage.jsonl 新旧互換: Rust writer が書いた新 record を TS `cheaper
      usage` が集計でき、旧 schema record(`savings`/`turns`/`toolCalls`
      欠落)を Rust `cheaper usage` が読める。writer は additive-only
- [ ] `cheaper usage --compare-model` の pricing 解決順(runtime catalog →
      user `pricing.json` → bundled snapshot)・stale 30 日判定・
      `directlyComparable` 母数が TS 版と一致
- [ ] `cheaper auth status` の `result.providers[]`/`credentials[]` 形状が
      TS 版と一致(ISS-19a)
- [ ] panic 注入で `INTERNAL_ERROR`(exit 70)error envelope が stdout に出て
      panic trace が stdout に漏れないこと。hook 経路では panic が allow/
      exit 0 に落ちること
- [ ] Windows runner で contract test(envelope/hook/count-tokens/run 系)が
      全パス
- [ ] `docs/guides/usage.md` に `costUsd` 口径変更と新旧 record 混在が記載済み

## Rollback Strategy

- 本フェーズの成果物は `crates/` 配下の新規 crate と `docs/guides/usage.md` の
  追記のみで、TS 実装(`packages/`)・配布物・Release contract には一切触れない。
  全て additive のため、`git revert` または `crates/cheaper-cli`・
  `crates/cheaper-workflow` の削除で完全に撤去できる。
- differential golden が通らない状態は単に Phase 5(配布切替)へ進めない
  だけで、ユーザー影響はない — TS SEA binary が引き続き唯一の配布物。
- `crates/` を Git に入れる場合も配布経路(launcher・SHA256SUMS・asset 名)は
  無変更のため、巻き戻し時に consumer 側の state は破壊されない。
- `usage.md` の追記は Rust 版の costUsd 口径を説明する内容のため、撤去する
  場合は当該段落のみ revert する。

## Open Questions / Gaps

- **workflow 定義の file 形式**: 設計書 crate 表は "workflow YAML schema" と
  記すが、現行 `loader.ts` は `JSON.parse` のみ(JSON 契約)。byte 互換を優先し
  本フェーズは JSON のみを port する。設計書の表記 drift は別途修正を申し送る。
- **`TOKENIZER_IMPLEMENTATION` 表記**(Deferred): `details.tokenizer`/
  `result.tokenizer` の `implementation` 値を tiktoken-rs 実装名にするか互換
  表示にするかは未決。normalize 規則で tokenizer identity は比較除外済みの
  ため実装をブロックしないが、Phase 5 までに決定が必要。
- **fake stream seam の env 形式**: `CHEAPER_TEST_FAKE_STREAM`(TS は module
  path)に対応する Rust 側の指定形式(fixture JSON path)は Phase 3 の G7
  seam 仕様と合わせて実装時に確定する。
- **secret prompt の raw-mode 実装**: Windows/macOS/Linux で stdin echo off を
  再現する crate(候補: `crossterm` 等)の採用は依存追加のため実装時に
  確認する(Ask first)。対話不能な非 TTY 環境では現行通り INVALID_REQUEST。
- **signal 中継の crate 選定**: SIGINT/SIGTERM → CancellationToken の実装
  (`tokio::signal` vs `ctrlc`)は実装時に決定。contract(先勝ち abort
  分類・grace)は不変。
- **`auth status` の providers[] 内容**: `describeProviderAuth`/`listProviderAuth`
  相当の出力 schema は Phase 3 の credential/catalog store 実装に依存。
  providers 非存在時の `models`/`run` 動作(ISS-19b、`models` exit 0 維持)も
  Phase 3 側で確定する前提。

## Self-Review

- [x] Every spec requirement maps to a task(envelope 4 形態・exit code 表・
      hook 4 target・usage.jsonl 新旧互換・`auth status` providers[]・
      models・workflow interface/run・differential golden・normalize 規則・
      costUsd 口径 docs 記載まで Task Breakdown T1-T9 と Verification に割当済み)。
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent
      (crate 名・移植元ファイル・規約名は設計書/移植元実装と照合済み)。

## Implementation Handoff

- Recommended mode: subagent-capable implementation
- Independent work streams: T1(`cheaper-workflow`、Rig 非依存)と B+D 先行部
  (`cheaper-cli` scaffold/envelope/args/harness)は並行着手可。T3-T7 は
  Phase 3 trait 面の確定後。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 前提: Phase 3 plan が runner/catalog/auth trait 面と G7 fake seam(3 層注入)を
  確定済み。未確定なら本 plan の C/D 区間は着手不可。
- レビュー重点: (a) canonical number formatter の適用範囲が envelope・
  usage.jsonl・report の全数値 field を網羅しているか、(b) hook fail-open が
  panic を含む全 error 経路を覆うか、(c) normalize 規則が spec/design の
  Verification Notes と一致し harness 以外に複製されていないか、(d)
  usage.jsonl writer が additive-only で新旧双方向 reader 互換があるか、
  (e) workflow engine の tokio 移植が job ordering・lease・abortGrace の
  決定性を保つか。
- 未解決の引き継ぎ: workflow file 形式(JSON)の設計書表記 drift、
  `TOKENIZER_IMPLEMENTATION` 表記、fake seam env 形式、raw-mode crate 選定は
  Open Questions に列挙済み。
