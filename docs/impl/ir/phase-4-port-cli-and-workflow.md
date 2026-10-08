---
id: "IMPL-ZSSIk1yMEAiQZUUzhlvEsF"
type: "impl"
status: "in-progress"
title: "Phase 4: port CLI and workflow"
created: "2026-10-08"
updated: "2026-10-08"
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
    - "docs/tasks/phase-4-port-cli-and-workflow.md"
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

# Phase 4: port CLI and workflow

## T1 — `cheaper-workflow` crate port(Sequence A 全区間)

### Scope

`packages/workflow`(~2.1k 行、12 module)を Rig 非依存の新規 lib crate
`crates/cheaper-workflow` へ port した。error/cancellation/resource の
canonical 型(`CheaperError`・`ErrorCode`・`AbortSignal`/`AbortReason`・
`ResourceLease`・`ExecutionMetrics`・artifact ports)は `cheaper-core` /
`cheaper-tools` の既存 seam をそのまま使い、engine scheduler のみ tokio
(pump/admit/execute event loop)で実装した。

### Files

- `crates/cheaper-workflow/Cargo.toml` — 依存は `cheaper-core`・
  `cheaper-tools`・serde/serde_json・tokio・tokio-util・uuid(locked
  1.27.0、`randomUUID()` 相当の runId/artifact id)。dev-dep に
  tempfile + tokio `test-util`(paused clock で `vi.useFakeTimers`
  相当の abortGrace/timeout deterministic 再現)。
- `src/json.rs` — `clone_json`(深い clone + object key の UTF-16
  code-unit 順ソート)・`is_object`・`has_own`(prototype 安全)。
- `src/types.rs` — `ValidatedDefinition` への型付き view
  (`jobs()`/`job(id)`/`outputs()`、`kind` 判定・`needs`/`resources`
  parse)。TS の loosely-typed object access を parse 関数に集約。
- `src/bindings.rs` — `$input`/`$from`/`$literal` の scan・
  materialize・JSON Pointer(`pointer_segments`/`resolve_pointer`、
  `~0`/`~1` escape・array index 規則・inherited key 拒否)・
  `validate_pointer_schema`(schema 経路の provably-impossible 判定)。
- `src/dag.rs` — `compile_dag`(code-unit 順 deterministic sort、
  self/cycle/unknown dep の definition error)。
- `src/schema.rs` — workflow definition の JSON Schema 検証
  (`WORKFLOW_JSON_SCHEMA` 埋込 + `jsonschema` crate、local `$defs` は
  core 2020-12 semantics)。
- `src/interface.rs` — `project_workflow_interface`(public schema・
  default・required のみ、invocation なし)。
- `src/resource_inference.rs` — `WorkspaceResourceInference`。
  produces→reads edge・writer conflict・fallback diagnostic・
  manifest 正規化(`cheaper_core::resources` の claim/manifest
  normalize・`SymlinkPolicy`・workspace escape 検査)を port。
- `src/artifacts.rs` — `RunArtifactStore`(`ArtifactReader` +
  `ArtifactPublisher` 両 trait 実装)。per-artifact/total byte cap・
  duplicate producer 拒否・foreign/malformed URI・provenance 改竄
  拒否・bounded copy・UTF-8 preview(invalid byte/BOM 保持・
  codepoint 安全 cap)・abort 即時 clear・pre-abort 拒否・
  `with_artifact_store` の成否両側 dispose・metadata projection
  (`includePreview` optional)。
- `src/compiler.rs` — `WorkflowCompiler`(`WorkflowCompilerPorts` の
  executor/catalog/resource 注入 seam)。`$from` edge dedupe・default
  完 fill・materialized path・manifest port・bad edge 事前拒否。
- `src/engine.rs` — `run_workflow` pump/admit/execute scheduler。
  `WorkflowEnginePorts`(worker/tool `JobExecutor`・`acquire_job_lease`/
  `release_job_lease`・`abort`・`execution_metrics`)・`maxJobs`
  admission・lease retain/release・timeout(engine start origin)・
  abortGrace(first outer abort origin・grace 中 conflict admission
  保留)・diagnostic UTF-8 codepoint 安全 2048B cap・`DEPENDENCY_FAILED`
  skip・`WorkflowOutcome`/`WorkflowDiagnostic` shape。
- `src/loader.rs` — `load_workflow_file`。JSON-only・workspace
  containment・builtin denylist + logical/canonical deny・symlink
  policy・Windows case-alias・read/parse error の compiler 前 mapping。
- `src/lib.rs` — 上記 module + `packages/workflow/src/index.ts`
  相当の re-export 面。`JsonValue = serde_json::Value`。
- `tests/common/mod.rs` — fixture builder・`err_code`
  (`toMatchObject({code})` 相当 — `AbortReason`/`ResourceError` wrap を
  `source()` chain で透過)・`err_details`・`flush`(microtask 相当)。
- `tests/{artifacts,bindings,compiler,engine,interface,loader,
  no_pi_deps,process,resource_inference}.rs` — `packages/workflow/
  test/*.test.ts` の port。

### 検証

- `cargo test -p cheaper-workflow` — 111 test 全パス(lib unit 2 +
  artifacts 14・bindings 10・compiler 29・engine 22・interface 3・
  loader 7・no_pi_deps 2・process 1・resource_inference 21)。
- `cargo clippy -p cheaper-workflow --all-targets` — 警告ゼロ
  (`ExecutionMetricsFn` type alias 化・`stop!` macro 内の borrow
  解放を `let _ =` 化・冗長 closure・test の lifetime/flatten)。
- `cargo fmt --check -p cheaper-workflow` — clean。
- no-Pi 依存 guard 2 件(Cargo.toml forbidden entries・src 内
  forbidden specifier)が crate 内に常駐。

### 既知の divergence / 留意点

- process integration test は bare `"node"` ではなく `where`/`which`
  で解決した絶対 path を policy command に使う — Rust process 層が
  bare `node` を `POLICY_VIOLATION` で拒否するのは既定の固定 policy
  (Rust runtime に Node host executable を持たないため)であり、
  production 側を緩めず test 側を runtime contract に合わせた。
- scheduler/executor 経路の error は `AbortReason` wrap で返る
  (`error_object()`/`code()` で canonical error に到達)。core の
  `ResourceError` も `Invalid(CheaperError)`/`Escape` 分岐を持つため、
  test の code 取得は `source()` chain walk に統一。
- engine の時計は `Instant` + tokio paused clock。`abort_grace_ms`/
  `timeout_ms` は f64 入力(JS number 規則のまま)で non-finite/
  負値は admission 前に `INVALID_REQUEST`。
- `execution_metrics` port は `Arc<ExecutionMetricsFn>` type alias
  (`null` → `None`)。`run_workflow` 呼び出し側(T7)が owner metrics
  snapshot を供給する seam。
- T7 接続用に `JobExecutor`・`JobLeaseRequest`・`WorkflowEnginePorts`・
  `project_artifact_metadata`/`with_artifact_store` を lib の公開面に
  出してある — workflow CLI glue はこれら trait object 経由で接続する。

## T2 — `cheaper-cli` envelope/args/entry 基盤(Sequence B)

### Scope

`crates/cheaper-cli/` を新規 scaffold し、envelope serializer・argv
解析・stdin input・count-tokens・builtin resource 埋込・version 解決・
fake-stream seam・panic 境界・`__grep-eval` 内部 mode dispatch を
実装した。command 本体(run/models/usage/auth/hook/workflow)は T3–T7
が埋める seam として `commands` 相当の module shell を張った。

### Files

- `crates/cheaper-cli/Cargo.toml` — bin crate(`cheaper`)+ lib target。
  依存: `cheaper-core`・`cheaper-tools`・`cheaper-runtime`・
  `cheaper-rig-adapter`・`cheaper-provider-devin`・serde/serde_json・
  tokio(`signal`/`rt-multi-thread`・workspace features 含め
  `macros`/`rt` も有効)・`include_dir` 0.7・`rpassword` 7・
  dev-dep `tempfile`。
- `crates/cheaper-cli/build.rs` — `packages/cli/package.json` から
  `$OUT_DIR/version.json` を生成(SEA VFS の `version.json` 相当)。
- `src/main.rs` — process 境界。`argv[0] === "__grep-eval"` の
  早期分岐(envelope/usage/diagnostic を出さない)→ panic hook
  (stderr のみ)→ 全体を `std::panic::catch_unwind` で包み、
  tokio current-thread runtime 上で
  `cheaper_runtime::catch_run_panic(run_cli(..))` を `block_on`。
  panic/JoinError/entry 失敗は `INTERNAL_ERROR`(70)envelope。
- `src/lib.rs` — module 一覧 + composition root 方針の注記。
- `src/cli.rs` — `run_cli` dispatch(`cli.ts` 相当)。`CliIo`
  (stdin/stdout/stderr/env/cwd trait 境界)・`CliHooks`
  (builtin_dir/now/signal)・`BoxError`・`CommandOut`・`RunCtx`。
  help/version/count-tokens は実装済み、run/models/usage/auth/
  workflow は module stub へ委譲。汎用 catch は `parseCliArgs`
  再試行で `format`/`quiet`/`workflow-run && quiet` 抑止を復元し、
  `error <code>: <message>` stderr + envelope + best-effort
  `record_usage` を行う。HELP text は literal byte 一致。
- `src/envelope.rs` — 4 形態。`serde_json::Map`(preserve_order)で
  TS object literal の key 挿入順を再現し、
  `cheaper_core::json::stringify::stringify` + `"\n"` で
  `JSON.stringify` byte 互換。error 形態は `details`/`cause` を
  値があるときのみ key 出力、`meta` の未取得 field は `null`。
  `cause` は CheaperError 相当(`CheaperError`/`WorkerRunError`
  downcast)のときのみ `{code,message}`。
- `src/args.rs` — `parseArgs`(strict+allowPositionals+tokens)相当の
  自前 scanner(`--name=v`・ambiguous/missing/unknown option・
  `--` terminator・last-wins)と command 別 option 許可表・alias
  表・`NAME_PATTERN`・`INVALID_WORKER_NAME` 分類。
- `src/input.rs` — `readTaskInput`(TTY/empty/JSON object 必須)。
- `src/count.rs` — `count_workspace_file_tokens`・
  `CHEAPER_COUNT_MAX_BYTES`(10 進正整数 + safe integer のみ採用、
  `js_trim` + `MAX_SAFE_INTEGER` で JS `Number` 規則を再現)。
- `src/embedded.rs` — `include_dir!` で `workers/`(schemas 含む)・
  `policy.yaml`・生成 `version.json`・`package.json` を埋込み、
  `<temp>/cheaper-builtin-<version>/` へ materialize して core の
  filesystem 解決(`<dir>/workers/<name>.md`・`<dir>/policy.yaml`・
  worker `.md` の `./schemas/` 相対参照)をそのまま再利用する。
  file 単位 tmp→rename + `.ready` marker。`package_version(dir)`
  は `version.json` → `package.json` の順(TS `packageVersion`)。
- `src/fake_stream.rs` — `CHEAPER_TEST_FAKE_STREAM` seam。
  `NODE_ENV === "test"` かつ env 指定時のみ
  `cheaper_runtime::fixture::RunFixture::load` で fixture JSON を
  読み、run 経路(T3)へ渡す。無効・load 失敗は
  `None`/INVALID_REQUEST。
- `src/usage_log.rs` — `buildUsageRecord`/`appendUsageLog`/
  `recordUsage` を Value ベースで port(ISO-8601 `ts` は dep を増やさず
  civil-from-days + 月長 loop で算出)。
- `src/text_format.rs` — `--format text` の 4 分岐 + `formatUsageReport`
  を `Value` 駆動で port。
- `src/run.rs`/`models.rs`/`auth.rs`/`usage_report.rs`/`pricing.rs`/
  `workflow_run.rs`/`workflow_context.rs`/`hook/mod.rs` — T3–T7 の
  実装 seam(現状は INTERNAL_ERROR stub / hook は fail-open 0)。
- `scripts/ts-envelope-vectors.mjs` — TS `envelope.js` 実装からの
  byte vector generator(tests/envelope.rs の期待値と対応)。
- `tests/{envelope,args,cli,input,usage_log}.rs`。

### 共有 JSON stringify の移設

`cheaper-rig-adapter/src/js_json.rs` にあった JS 互換
`stringify`/`stringify_pretty` を
`cheaper-core/src/json/stringify.rs` へ移し、core の canonical
number formatter(`json/number.rs`)と同居させた。adapter 側は
re-export shim(`pub use cheaper_core::json::stringify::*`)で既存
call site を無変更に保つ — envelope・run meta・usage record が同一
formatter を共有するための境界整理。

### Dispatch seam(T3–T7 向け)

`run_cli(argv, io, hooks)` → `dispatch` → 各 command module:
`async fn execute(args: &XxxCliArgs, io: &mut CliIo, hooks: &CliHooks,
started_at: f64, ...) -> Result<CommandOut, BoxError>`。
stdout 書出し(`--format text` 切替・`envelope_json`)と
`record_usage` 呼出しは `cli.rs` 側が担い、command module は
envelope 構築に専念する。run 経路のみ `&mut RunCtx` を受けて
worker 名・canonical root・userConfigDir を error path 記録用に
供給する。`CliHooks` には `run_worker`/`create_model_runtime`/
`open_url` seam を T3/T5 が追加する余地を残した。

### 検証

- `mise exec -- cargo test -p cheaper-cli` — 53 test 全パス
  (envelope byte vector 6+・args table・input・count_max_bytes・
  usage_log schema/ISO-8601・cli dispatch smoke・`__grep-eval`
  spawn)。
- `mise exec -- cargo build -p cheaper-cli` / `cargo clippy
  -p cheaper-cli --all-targets -- -D warnings` / `cargo fmt --check
  -p cheaper-cli` — clean。
- TS byte 比較: `node packages/cli/dist/main.js` vs
  `target/debug/cheaper.exe` で `frobnicate`(unknown command)・
  `count-tokens` 成功/ENOENT・`--help`・`--version` を比較 —
  `durationMs` と tokenizer identity(`tiktoken-rs`、規定の
  normalize 除外対象)を除き stdout/stderr/exit code が一致。
  `scripts/ts-envelope-vectors.mjs` 出力 6 件も tests の literal と
  byte 一致を確認。
- `cargo test -p cheaper-core -p cheaper-rig-adapter` — stringify
  移設の regression なし(227 test)。

### 既知の divergence / 留意点

- `tokenizer.implementation`/`version` は `tiktoken-rs` 版の値を出す
  (Phase 1 で既定済み・differential normalize 除外対象)。
- builtin resource は temp dir materialize 方式(SEA VFS と異なる
  実手段だが、worker/policy 解決の公開 contract — project → user →
  builtin の優先順序と `searched` 列挙 — は不変)。
- `WorkerRunMeta` field 順は TS `buildMeta` の literal 順に一致。
- usage record の `ts` は ISO-8601(`toISOString` 互換、ms 精度)。
- run/models/usage/auth/workflow command は stub(T3–T7 で実装)。
- `cargo fmt --check`(workspace 全体)は `crates/cheaper-workflow` の
  差分のみ残る(T1 担当者の作業領域)。

## T3 — `run`/`models` command 経路(Sequence C 前半)

### Scope

`crates/cheaper-cli/src/run.rs`・`src/models.rs` の INTERNAL_ERROR
stub を `executeRun`/`executeModels`(packages/cli/src/cli.ts)の
本体に置き換えた。`count.rs` は T2 で実装済み。`cli.rs` は
`now`/`resolve_builtin_dir` を `pub(crate)` 化したほか、
`system_now` を `Date.now()` 同様の整数 ms truncate に修正
(detail は divergence 節)。

### Files

- `src/run.rs` — `executeRun` 本体 + T7 再利用用の配線 helper。
  - 順序は TS 一致: builtinDir → `canonicalize_workspace_root` →
    `resolve_config_dirs` → `RunCtx` 記録(root/userConfigDir は
    worker 解決前 — error path の usage fallback が読む)→ worker
    解決 → `ctx.worker_name` → `read_task_input` +
    `validate_worker_input` → `load_policy` + `--timeout`
    `parse_duration`/`restrict_policy` → `load_model_config` →
    runtime → fake-stream → signal 中継 → `run_worker` →
    `success_envelope` + `model: <prov>/<id> (tier <t>)` model line
    (stderr への出力と `--quiet` 抑止は dispatch 側の契約)。
  - error は `WorkerRunError` をそのまま `BoxError` へ —
    `error_envelope` が downcast で `runMeta` を取り出し usage-bearing
    failure を記録できる(TS の `err.runMeta` 相当)。
  - `create_runtime(user_config_dir, fixture)`: `Runtime::create` に
    `DevinProviderPlugin` を常時同梱(extension loader 不使用の
    既存方針)。fixture 有効時は `env` overlay
    (`ScopedProviderEnv` = fixture map + `ProcessEnv` fallback)・
    `cheaper-fake` plugin・`auth_logins` → `AuthService::login(
    api_key, ScriptedAuthInteraction)` を適用(replay harness と同じ
    経路)。
  - `CliCompletionModelFactory`: `SessionSpec.model.api` で
    `devin-local` → `DevinCompletionModelFactory`、他 →
    `builtin_completion_model`(resolved auth → `BuiltinAuth`
    1:1 写像)。runtime が Rig 型を知らないため dispatch は本
    composition root の責務。
  - `resolve_named_worker`: named resolution(project→user→builtin)
    とは別に、fixture 有効時のみ `WORKER_NOT_FOUND` を fixture
    `worker` 節の `worker_definition` へ fallback
    (`cheaper run fixture-worker` で corpus fixture を CLI から
    そのまま replay できる)。`--worker-file`・INVALID_WORKER_NAME・
    read error は fallback しない。
  - fake-stream policy 適用: `load_policy` の結果へ fixture
    `policy` 節の宣言 field のみ `apply_to` で重ねる(fixture 完結
    policy ではなく実 layer の上の override — `maxTurns` 等 corpus
    前提を保持しつつ user/project layer を無視しない)。
  - `explicit_model`: `args.model` 優先、fixture 有効時の既定は
    `fixture.explicit_model()`(`cheaper-fake/<model>`)。
  - `abort_grace_ms`: fixture 有効時は harness 既定 80ms、通常は
    `None`(DEFAULT_ABORT_GRACE_MS=5000)。fixture `abort` 節は
    `after_ms` 遅延で `AbortReason::new("fixture", msg, None)` を
    発火する tokio task として replay harness を再現。
  - `SignalRelay`: `tokio::signal` の SIGINT(`ctrl_c`)/SIGTERM
    (unix のみ — `SignalKind::terminate`)と `hooks.signal` の
    listener を `AbortController` へ中継。reason は TS の
    `ac.abort()` 同様既定 `AbortReason::aborted()`(signal →
    ABORTED 分類)。Drop で spawned task abort + listener 解除
    (TS `finally` の `process.off`/`removeEventListener` 相当)。
- `src/models.rs` — `executeModels` 本体。
  `Runtime::create` → `get_available(None)` → `toModelInfo` 相当の
  `ModelInfo` 変換 → exclude 後 eligible 母集団で
  `build_auto_tier_context`(--worker の `workload.outputPerInput`
  供給)→ 各 model を `resolve_pricing` + `classify_model` →
  `--provider`/`--tier` filter。`result.models[]` の key 順は TS
  literal 構築順(provider→id→excluded→tier→tierSource→auto fields
  →pricing、pricing 不在は `null`)。usage record は書かない。
- `src/cli.rs` — `now`/`resolve_builtin_dir` を `pub(crate)` 化
  (command module 共有)。`system_now` を `as_millis()` truncate に
  変更。
- `tests/run.rs`/`tests/models.rs` — `run_cli` 経路の結合 test
  (fixture fake-stream・signal abort・worker-file・timeout・
  fallback 優先度・envelope shape・filters)。`builtin_dir` は
  `packages/cli` を直接指す(repo 内同一 layout、embedded
  materialize を回避)。
- `tests/auth.rs` — clippy `len_zero` 1 行修正(T5 所有 file だが
  `-D warnings` gate が落ちるため最小修正のみ)。

### 検証

- `mise exec -- cargo test -p cheaper-cli` — run.rs 10 件・
  models.rs 6 件を含め、所有範囲は全パス(T5 作業中の
  `tests/auth.rs::login_api_key_tty_persists_credential` 1 件のみ
  失敗 — 本 slice 非依存、別途報告)。
- `mise exec -- cargo build -p cheaper-cli` / `cargo clippy
  -p cheaper-cli --all-targets -- -D warnings` / `cargo fmt -p
  cheaper-cli --check` — clean。
- TS↔Rust byte 比較(実 binary、`OPENAI_API_KEY=dummy` +
  `CHEAPER_CONFIG_DIR` 隔離・cwd=temp):
  - `models` / `models --tier low` / `models --worker bulk-reader`
    が `meta.durationMs` 以外 byte 一致。
  - fake-stream e2e(`NODE_ENV=test` + `CHEAPER_TEST_FAKE_STREAM`):
    `cheaper run fixture-worker` が `ok:true`/`result:{ok:true}`/
    `meta.model=cheaper-fake/fake-model` の envelope を出し、stderr
    に `model: cheaper-fake/fake-model (tier medium)`、
    `<userConfigDir>/usage.jsonl` に `kind:"run"` record を記録。
    `--format text` 出力・`--quiet` 抑止も確認。

### 既知の divergence / 留意点

- `system_now` が sub-ms を出していたのを `Date.now()` の整数 ms
  に揃えた(T2 実装の修正 — 全 envelope の `durationMs` に影響、
  parity 改善側)。
- runtime provider env は `ProcessEnv` + fixture overlay —
  `io.env`(`&dyn Fn`、非 `'static`)は `ProviderEnv` trait object に
  載せられないため。production では `io.env === process.env` で
  等価。test は fixture `env` または process env で注入する。
- fixture load を worker resolution 前に行う(TS は modelConfig 後)。
  fixture load 失敗と worker not-found が両立する場合のみ error
  優先順位が異なる — test-only seam 内の差分。
- `hooks.runWorker`/`createModelRuntime` seam は未実装(TS hook) —
  fixture JSON seam が代替として機能する。必要になれば T8 で追加。
- devin factory の env は `ProcessEnv` 固定(`DynHttpClient` 等の
  Rig 型を cli に持ち込まないため `DevinCompletionModelFactory::
  default` を使う)。

### T7 への引継ぎ(worker execution seam)

`run.rs` は T7 workflow の worker job 実行に必要な部品を
`pub(crate)` helper として切り出してある:

- `create_runtime(user_config_dir, fixture)` — `ensureModel` 相当
  (devin plugin + fixture overlay + auth_logins)。
- `production_session_factory()` / `session_factory(fixture)` —
  `RigSessionFactory<CliCompletionModelFactory>` or fake。
- `CliCompletionModelFactory` — builtin/devin api dispatch。
- `resolve_named_worker(name, dirs, root, fixture)` /
  `box_resolve_error` — worker 解決 + error 畳み込み。
- `SignalRelay` — OS signal + external signal → AbortController
  中継(first-reason-wins)。

残る T7 側の責務: `prepareWorkflowContext`(workflow loader/
compiler 成果物)→ job 毎の `RunWorkerDeps` 組立て(worker map/
policy/input/`execution` binding)→ `run_worker` 呼出し → meta を
`job.meta` projection へ。`execute()` 本体は reuse せず deps 組立て
は T7 が組む(worker/job model の違いで single-run の input/
explicit-model 配線はそのまま流用できないため)。

## T4 — usage report / pricing(`usage_report.rs`・`pricing.rs`)

### Scope

`src/pricing.rs` に `packages/cli/src/pricing.ts` を、`src/usage_report.rs`
に `packages/cli/src/usage-report.ts` + `cli.ts` の `parsed.kind ===
"usage"` 分岐を port した。`usage_log.rs` 自体は T2 で実装済みのため、
本 slice はその writer が出す additive-only record を読む側 + 価格解決。

### Files

- `src/pricing.rs` — `loadPricingSnapshot` 相当。`<userConfigDir>/
  pricing.json` を fail-soft に読み、core `PricingSnapshot`(`&'static
  str` field を持つ bundled const 向け型)へ `Box::leak` で bridge する
  (process 寿命の読み取り専用データ — core 型の変更を避ける bridge)。
  top-level は `version`/`updatedAt`/`models` を必須検証し、entry 単位の
  不正(`input`/`output` が非負 finite number でない等)はその entry のみ
  skip する。`serde_json::Map`(preserve_order)の挿入順を維持し、
  `resolve_pricing` の first-match 走査が `Object.entries` 順と一致
  することを保証する。
- `src/usage_report.rs` — `readUsageRecords`(per-line JSON parse、
  破損行・`kind` 不明行・file 不在は skip/空)、`filterRunRecords`
  (`kind:"run"` のみ + `sinceMs`/`worker`/`provider`/`model` filter。
  `model` は core `parse_model_ref` で `<provider>/<id>` を first `/`
  split)、`aggregate`(run record の `usage`/`turns`/`toolCalls`/
  `durationMs` 合計 + `savings` 持ちのみ `measured` + `directlyComparable
  === true` のみ `comparable` subset — costUsd は run の `usage.costUsd`)、
  `estimate`(comparable run 0 件・pricing 未解決・stale では emitted
  しない。`parentEquiv = (sourceTokens*input + returnedTokens*output)
  /1e6`、`savings = parentEquiv - costUsd`、percent は parentEquiv > 0
  のときのみ)、`isStalePricing`(updatedAt 不在 = runtime-catalog は
  stale にしない、parse 不能 = stale、`now - updatedAt > 30日` で
  stale — 境界値は fresh)、`buildUsageReport`(runtime catalog cost →
  user `pricing.json` → bundled snapshot の順に core `resolve_pricing`
  で価格解決、user snapshot 由来は provenance を `"pricing.json"` に
  読み替え)、`execute_usage`(`cli.ts` 分岐相当 — option 検証は
  `INVALID_REQUEST`(2)、config dir 解決 → best-effort catalog 観測
  (`Runtime::create` + DevinPlugin 登録、`catch_unwind` で panic も
  飲み込む)→ report → `utility_success_envelope`。usage command 自体は
  記録対象外)。
- `src/text_format.rs` — `utility_text` に report 形状
  (`result.records` number + `result.measured` object)の検出分岐を
  追加し、TS の `formatUsageReport(report)` 直書きと同じ出力を
  `format_usage_report` で返す(dispatch の `write_output` seam は
  envelope 経由のため)。
- `tests/pricing.rs` — pricing.test.ts の port(5 case: file 不在・
  malformed/missing field → None・entry skip・insertion order・
  snapshot contract)。
- `tests/usage_report.rs` — usage-report.test.ts の port + dispatch
  経路(32 case: aggregate/filter/estimated/stale/新旧 record 混在/
  malformed JSONL/`kind:"count"` 受入・非加算/INVALID_REQUEST/envelope
  key 順/`--format text`/usage.jsonl 非書込)。

### 検証

- `mise exec -- cargo test -p cheaper-cli` — 全パス(137 test、うち
  T4 新規 37)。
- `mise exec -- cargo clippy -p cheaper-cli --all-targets
  --all-features -- -D warnings` — clean。
- `cargo fmt -p cheaper-cli -- --check` — clean(workspace 全体の
  `cargo fmt --all -- --check` は `crates/cheaper-workflow` の差分のみ
  残る — T1 担当者の作業領域)。
- TS/Rust differential: fixture usage.jsonl(run 3 + count 1 + 破損行
  1、旧 format record・`directlyComparable` true/false 混在)に対し
  `node packages/cli/bin/cheaper.js` vs `target/debug/cheaper.exe` で
  `usage`・`--since`・`--worker`・`--provider`・`--model`・
  `--compare-model`(runtime-catalog=openai/gpt-5、user pricing.json
  wildcard+entry updatedAt、bundled 未収録=estimated なし、stale
  snapshot=estimated なし)・`--format text`・INVALID_REQUEST 3 種を
  比較 — `meta.durationMs` を normalize すれば stdout/stderr/exit code
  が全 case 一致(text は byte 一致)。

### 既知の divergence / 留意点

- `PricingSnapshot` の `&'static str` に対する `Box::leak` bridge —
  snapshot 1 回 load あたり小さな allocation が process 寿命分残る
  (usage command は process 1 回の read-only 集計で、leak 量は bounded
  — core 型を owned 化する場合はここを見直す)。
- `date_parse` は JS `Date.parse` の ISO-8601 subset で、offset 無し
  date-time を UTC 近似する(JS では local time — `usage_log.rs` の
  writer は常に `Z` 付き ISO を出すため実害なし)。
- `usage`/`savings` が明示 `null` の record は TS で `TypeError` →
  `INTERNAL_ERROR` envelope となる挙動を `CheaperError`(InternalError)
  で再現。
- `text_format.rs` への分岐追加は当初指示の file scope 外だが、TS の
  `formatUsageReport` 直書きを dispatch seam 経由で再現する最小の
  coherent な変更として採用した(dispatch 本体 `cli.rs` は共有領域の
  ため未変更)。

## T6 — `cheaper hook` 経路(envelope 外 contract)

### Scope

`src/hook/{mod,targets,routing_policy}.rs` に
`packages/cli/src/hook/` 3 file(hook.ts・targets.ts・routing-policy.ts)
を port した。envelope 機構と独立し、panic を含む全 error 経路が
target 適合の allow + exit 0 へ畳まれる fail-open contract。

### Files

- `src/hook/mod.rs` — `executeHook` 相当。`KNOWN_EVENTS` の 5 event、
  `--target <v>` / `--target=<v>` のみ解釈する緩い argv 解析(未知
  option は flag のみ skip、後続値は positional に残る TS 規則)。
  stdin は TTY なら空文字列、pipe なら EOF まで読み UTF-8 lossy。
  `emit` は stdout/stderr optional write(TS try/catch 相当に
  `catch_unwind` で writer panic も飲み込む)。最深部は
  `cheaper_runtime::catch_run_panic` で inner future を包み、panic
  を `fallback_allow` + exit 0 へ畳む — main.rs の panic →
  `INTERNAL_ERROR`(70)envelope 経路には到達しない(copilot は非 0
  を deny と解釈するため envelope JSON・非 0 exit は禁止)。
- `src/hook/targets.rs` — 4 target adapter。windsurf(deny = stderr
  `reason+\n` + exit 2、allow = silent)・copilot(deny =
  `{"permissionDecision":"deny","permissionDecisionReason":…}` exit
  0、foreign shape は dual-schema allow `{"permission":"allow",
  "permissionDecision":"allow"}`、JS `||` truthiness を `js_truthy`
  で再現)・pretooluse(`hookSpecificOutput{hookEventName:
  "PreToolUse",…}`)・cursor(常時 schema-valid JSON — normalize 失敗・
  malformed stdin でも `{"permission":"allow"}`)。JSON 出力は
  `cheaper_core::json::stringify` + `"\n"` で `JSON.stringify` byte
  互換、`serde_json::Map`(preserve_order)で literal key 順を再現。
  deny reason の複数行は `split("\n").join(" ")` で flatten。
- `src/hook/routing_policy.rs` — `threshold`(`CHEAPER_READ_TOKEN_
  THRESHOLD` を JS `parseInt` 部分 parse で読み f64 のまま比較、
  非正・NaN は 8192)・workspace containment(`nodepath::resolve`/
  `relative`/`is_absolute` で Node lexical 規則、相対 cwd は
  `process.cwd()` anchor の TS 規則)・byte fast path(`st.size <=
  limit` で計測・builtin materialize・policy 読込を skip)・30s
  計測 timeout(`count_workspace_file_tokens` は同期実装のため
  `spawn_blocking` + `tokio::time::timeout(30s)` — `&dyn Fn` 非
  Send の env は計測経路が transitive に読む 5 key
  (`CHEAPER_CONFIG_DIR`/`XDG_CONFIG_HOME`/`USERPROFILE`/`HOME`/
  `CHEAPER_COUNT_MAX_BYTES`)を snapshot して受け渡し)・
  `FileTooLargeError` downcast → byte-cap wording の fail-closed
  deny・shell tokenizer + `extract_read_candidates`。
  tokenizer は char 走査で JS code-unit 規則を再現(対象 regex は
  全て ASCII のみ match、comment は `\n` も消費して segment 継続、
  `\s` は Unicode White_Space + U+FEFF)。`evaluate` は TS の
  `unknown` 検証の代わりに typed `RoutingRequest` を取り、
  malformed request は adapter の normalize(None → allow)で表現。
- `tests/hook.rs` — hook-contract.test.ts の移植 15 case(spawn
  ではなく `run_cli` + CliIo stub。panic 注入は stdin read_all
  panic で 4 target 全て exit 0 / cursor は allow JSON を確認) +
  `HookTarget::parse` table。lib 内 unit test は argv 解析・
  threshold/parseInt・tokenizer・`extract_read_candidates` の TS
  fixture table。

### 検証

- `mise exec -- cargo test -p cheaper-cli` — 全パス(既存 53 + 新規
  28 = 81 test。hook 関連: lib 内 unit 12 + tests/hook.rs 16)。
- `mise exec -- cargo clippy -p cheaper-cli --all-targets -- -D
  warnings` / `cargo fmt --check -p cheaper-cli` — clean。
- byte parity matrix(`tmp/t6-hook-byte-parity.cjs`):
  `node packages/cli/bin/cheaper.js hook …` vs `target/debug/
  cheaper.exe hook …` を isolated env(makeIsolatedEnv 相当)+
  tempfile workspace で spawn し、stdout/stderr/exit を byte 比較。

| target | event | 結果 |
| --- | --- | --- |
| (command path) | missing/unknown target/unknown event/extra flag | identical 7/7 |
| windsurf | pre-read-code × 11 + pre-run-command × 6(deny/small/fast-path/missing/dir/malformed/wrong-event/empty-info/numeric-path/outside/FILE_TOO_LARGE/cwd-change/quoted/unsafe/redirect/whitespace) | identical 17/17 |
| copilot | pre-tool-use × 10(deny/malformed/missing-args/broken-args/unrelated/foreign/falsy/string-args/shell-deny/small) | identical 10/10 |
| pretooluse | pre-tool-use × 9(Bash deny/Read deny/malformed/{}/Write/no-input/empty-input/missing/small) | identical 9/9 |
| cursor | before-read-file/before-shell-execution × 8(deny×2/malformed/{}/other/missing/outside/no-cwd) + unknown event/no args | identical 10/10 |

  **53/53 byte-identical**。deny reason の token 数も一致(tiktoken-
  rs の o200k_base カウントが js-tiktoken と fixture 上一致)。
- fail-open: malformed argv(上表)+ stdin read panic 注入(4 target
  → exit 0、cursor は `{"permission":"allow"}`、panic trace は
  stderr のみ、envelope 非出力)を確認。

### 既知の divergence / 留意点

- `evaluate` は typed request を取る(TS の runtime shape 検証は
  adapter の normalize で表現)— 外部 contract への影響なし。
- 計測 env は 5 key の snapshot(TS は `opts.env` 参照渡し)— 現在の
  `count_workspace_file_tokens` が読む key 集合と一致。core 側が
  新たな env key を読むようになった場合は `MEASURE_ENV_KEYS` への
  追加が必要。
- spawn_blocking 上の計測が timeout した場合、thread は detach され
  て残る(TS の subprocess timeout と同じく verdict は resolve)。
- copilot foreign-shape 検出は JS truthiness(falsy `tool_name` 等は
  dual allow を emit せず silent allow)。

## T7 — workflow CLI glue(`workflow_context.rs`・`workflow_run.rs`)

### Scope

`packages/cli/src/workflow-context.ts` / `workflow-run.ts` を
`src/workflow_context.rs` / `src/workflow_run.rs` に port し、T1 の
`cheaper-workflow` engine と T3 の `run_worker` 経路を接続した。
`cli.rs` の dispatch(`workflow interface`/`workflow run` 両分岐・
exit code・quiet 抑止)は T2 骨組み時点で既に配線済みだったため
変更は `CliHooks.now` の `Arc` 化と `system_now` の `pub(crate)` 化
のみ。

### Files

- `crates/cheaper-cli/src/workflow_context.rs` —
  `prepareWorkflowContext` 相当。builtin dir → workspace root
  canonicalize → config dirs → `load_policy` →
  `ToolCatalog::from_resolved_policy`(TS の
  `createToolCatalog({process: policy.process})` と同じ resolved
  policy 経路 — serialize→再 resolve の二重経路を避ける)→
  `WorkspaceResourceInference`。`loadWorkflowFile` 相当は
  `WorkflowContext::load_workflow` に分離した —
  `WorkflowCompiler<'a>` が ports への借用を保持するため
  self-referential を避ける必要があった(TS は `{root, dirs, policy,
  loaded, workers}` を一度に返す)。worker 事前解決
  (`resolve_worker_index`)は raw definition の `jobs` object から
  `kind === "worker"` の `worker` 名を収集し、code-unit sort 順で
  `resolve_worker` する。`WORKER_NOT_FOUND`/`INVALID_WORKER_NAME`
  のみ preload で無視し、それ以外の失敗(`Read` variant の I/O error
  を含む)は `failures` map に保持して `WorkerCatalog::get` で再
  throw(TS `failures.get(name)` throw 相当 — `ToolError` は Clone
  不可のため `Arc` + `source()` 透過の `SharedToolError` で共有する)。
- `crates/cheaper-cli/src/workflow_run.rs` — `runWorkflowCommand` 相当。
  `execute_interface` は `project_interface` →
  `utility_success_envelope` のみ(worker/tool/model を実行しない)。
  `execute_run` は input(`--input` / stdin)→ `--timeout` parse →
  `restrict_policy` → `compile` → abort 中継 → engine ports 組立 →
  `run_workflow` → `workflow_envelope` 投影。
  - executor seam: `WorkerJobExecutor`/`ToolJobExecutor` が
    `JobExecutor` trait を実装し、`Arc<Shared>` を介して lazy
    `ensure_model`/`ensure_invocation` を共有する。worker job 毎に
    `RunWorkerDeps` を組み立て `execution` binding(shared
    `InvocationToolRuntime` + `owner_id` + parent lease + manifest)
    を渡す。`run_worker` の future は `&mut dyn FnMut` callback を
    await 間に持つため `!Send` — engine が `tokio::spawn` で駆動する
    `BoxFuture + Send` 要求を満たすため `spawn_blocking` +
    専用 current_thread runtime の `block_on` で駆動する
    (auth.rs の signal relay と同じ構成)。
  - tool job は per-job `MutationJournal` + `create_worker_scope` で
    `scope.execute`、meta は `{toolCalls, changes}`(TS と同じ選択)。
    `finally { scope.dispose() }` 相当は成否に関わらず実行する。
  - `acquireJobLease` は tool job の `tools.allow` admission 検査
    (`POLICY_VIOLATION`)と worker job の `check_capabilities` を
    engine 側 `tokio::spawn` 内で呼ぶ。`releaseJobLease`/`abort`/
    `executionMetrics` は shared invocation runtime へ委譲する
    (abort は controller + invocation + store dispose)。
  - artifact ledger は `LedgerPublisher`(`ArtifactPublisher` wrap)
    が `put` 成功ごとに `ArtifactReference` を畳み込み、envelope
    `meta.artifacts` へ `project_artifact_metadata({include_preview:
    false})` で投影する(`availability: "unavailable"`)。
  - `selectWorkerMeta` は serialize 済み map から `execution`/
    `durationMs` を落とす — `WorkerRunMeta`/`PartialRunMeta` の
    field 順で除くと TS の手選択(`{model, usage, turns, toolCalls,
    rejectedToolCalls, repairs, transientRetries, changes, savings?}`)
    と同じ key 集合・順序になる。
  - `CliHooks.now` を `Arc<dyn Fn>` に変更し engine の `now` option
    へそのまま渡す(duration 計測が test clock と一致)。
- `crates/cheaper-cli/src/cli.rs` — `CliHooks.now` の型を
  `Option<Arc<dyn Fn() -> f64 + Send + Sync>>` へ、`system_now` を
  `pub(crate)` へ。dispatch 本体は既存のまま。
- `crates/cheaper-cli/Cargo.toml` — `cheaper-workflow` path 依存を
  追加(T7 の「T1+T3 の成果物を接続」のために必須の workspace 内部
  依存 — external dep は追加していない)。
- `crates/cheaper-cli/tests/usage_report.rs` — `now` field の型変更に
  伴う `Box::new` → `Arc::new` 修正のみ。
- `crates/cheaper-cli/tests/workflow.rs` — TS `workflow-run.test.ts`/
  `workflow-interface.test.ts` の public contract を移植(13 test)。
  `run_cli` + CliIo stub + `builtin_dir` hook。fake-stream fixture
  (`CHEAPER_TEST_FAKE_STREAM` + `NODE_ENV=test`)で 2 worker job +
  `needs` edge の E2E を deterministic に実行(session は job 順に
  消費)。artifact ledger・partial failure・timeout/quiet/error
  経路・pre-aborted signal の abort 伝播を網羅。

### 検証

- `mise exec -- cargo test -p cheaper-cli` — 全パス(既存 + 新規 13 =
  134 test)。
- `mise exec -- cargo test -p cheaper-workflow` — 全パス。
- `mise exec -- cargo clippy --workspace --all-targets -- -D warnings`
  / `cargo fmt --check -p cheaper-cli` — clean。
- E2E: `workflow run` で `bulk-reader` × 2 job(`needs` edge)+
  `$from` output binding を fake-stream fixture で実行し、`result`
  に両 job の output が順序どおり投影され、`jobs[].status` が
  `succeeded`、`meta.model.provider` が `cheaper-fake` になることを
  確認(tests/workflow.rs `workflow_run_two_worker_jobs_with_needs`)。

### 既知の divergence / 留意点

- `prepareWorkflowContext` の戻り値分割(`load_workflow` は
  `WorkflowContext` の method)は Rust の借用制約由来 — 外部 contract
  への影響なし。
- `ensureModel` 内の順序は `load_model_config` → fixture load →
  `create_runtime`(TS は `createModelRuntime` → `loadStream`)。
  `create_runtime` が fixture を引数に取るため入れ替わる — fixture
  load 失敗と runtime 生成失敗が両立する場合のみ error 優先順位が
  違う(test-only seam 内の許容差分)。
- worker executor の `spawn_blocking` + inner current_thread runtime
  は `run_worker` の `!Send` future を engine の `Send` 要求へ橋渡し
  するための構成差分 — 内部の `tokio::spawn`/`sleep` は専用 runtime
  で駆動され、observable な結果(envelope/exit code)は同じ。
- `hooks.signal` の external abort は reason をそのまま forward する
  (TS `abortController.abort(external?.reason)`)。`run` 側の
  `SignalRelay` が reason を捨てて `AbortReason::aborted()` に揃える
  のとは対称差分。
- `WorkflowExecutorError` の fallback は `TOOL_EXECUTION_FAILURE` —
  `isCheaperError` 相当の downcast 失敗時のみ(TS 同様)。
- `workflow run` の `--model`/`--worker-file` 等の flag は args 層で
  拒否済みのため executor には到達しない(TS 同様 fixture
  `explicitModel` のみが worker job の model 選択に効く)。

## T8 — binary-to-binary differential golden harness

### Scope

`tests/differential/` — Rust binary(`cheaper`)と canonical TS entry
(`packages/cli/bin/cheaper.js` via node)を同一 case の下に subprocess
実行し、normalize 後の stdout envelope/raw bytes + exit code を比較する
golden harness。`docs/test-specs/rust-migration-differential-golden.md`
の replay 比較 gate。

### 構成

- `tests/differential/main.rs` — `cargo test -p cheaper-cli --test
  differential` の entry。`cases/*.json` を列挙し 1 test が全 case を
  実行する。unpinned diff で fail、`expectDivergence` pin は diff が
  残っている間だけ通る(解消すれば "resolved" として report)。
- `tests/differential/harness.rs` — case schema・workspace/config/home
  の per-side materialize(fixture `worker` → `.cheaper/workers/*.md`、
  `policy` → `.cheaper/policy.yaml`、stdin/files/usage.jsonl seed)・
  subprocess spawn(env 隔離 + watchdog kill)・binary path 解決
  (`CARGO_BIN_EXE_cheaper`/`CHEAPER_DIFF_RUST`/`CHEAPER_DIFF_NODE`/
  `mise where node`/非 shim PATH node)。
- `tests/differential/normalize.rs` — 唯一の canonical normalization
  (exclude と厳密比較の対象は `tests/differential/README.md` 参照)。
- `tests/differential/adapter.mjs` — TS 側 `CHEAPER_TEST_FAKE_STREAM`
  の module seam。`CHEAPER_DIFF_FIXTURE` の RunFixture JSON を pi の
  per-call stream 列へ翻訳する(turn 規約・errorMessage encode・
  usage/cost の決定的埋めは README と先頭コメント参照)。
- `tests/differential/cases/*.json` — 43 cases(run 24 + argv/hook 19)。
  fixture inventory・step 規約・pinned divergence は
  `tests/differential/README.md` に集約。
- `src/fake_stream.rs` / `workflow_*.rs` — 変更無し(既存 seam で足りた)。

### 検証

- `cargo test -p cheaper-cli --test differential` — 43 cases、
  pass=35 / divergence(pinned)=8 / fail=0。
- `cargo test -p cheaper-cli` 全パス・`cargo clippy -p cheaper-cli
  --tests -- -D warnings` clean・`cargo fmt -p cheaper-cli --check` clean。

### 既知の divergence(pin 済み — README §pinned divergences)

- fake stream `panic` → Rust INTERNAL_ERROR(70) vs TS PROVIDER_FAILURE(5)
  — TS seam に stream 内 panic の等価物が無い(seam 限界として受容)。
- abort 時の in-flight call `turn_end`(pi は aborted message に
  turn_end を立てるため +1)と maxToolCalls reject の operation 状態
  計上(cancelled vs completed)— `run-max-turns`/`run-max-tool-calls`
  (fixture DSL が abort 後の turn step を消化できず表現不能のため
  受容)。

解消済み(pin 解除 — 本 IR 末尾 §Phase 4 follow-up fix 参照):
PROVIDER_FAILURE の `error.details` shape(Rust を TS `{errorMessage}`
のみに揃えた、`run-error-*` 4 case)と `auth status` の oauth
`methods`/`subscription`(declaration ベース化、`cli-auth-status`)。

## T9 — 互換 test + docs(hook byte matrix・usage.jsonl 新旧互換・usage.md)

### Scope

T6 が `tmp/` の manual script(53/53 byte-identical)で一回限り検証した
hook byte parity を durable test 化し、usage.jsonl の新旧双方向互換を
`tests/usage_log.rs` に追加、`docs/guides/usage.md` に `costUsd` 口径
変更と新旧 record 混在を追記した。T8 corpus(`cli-hook-*` 4 case)を
重複せず拡張する形で、target×event matrix を binary-to-binary に再現。

### Files

- `tests/differential/harness.rs` — case schema に `compareStderr`
  (既定 false)を追加。hook deny は stderr が contract の一部
  (windsurf deny = guidance + exit 2)なため、opt-in で stderr も
  path scrub 後 byte 比較する。raw/envelope mode と独立に動く。
- `tests/differential/main.rs` — `compare_stderr` 時に両側 stderr を
  `scrub_public` 適用して比較する分岐を追加。
- `tests/differential/cases/` — 12 case 新規 + 既存 4 hook case に
  `compareStderr: true` を付与(計 16 hook case、全体 55 cases):
  - command path: `cli-hook-no-args`・`cli-hook-cursor-fallback`
    (event 無し → `{"permission":"allow"}`)・
    `cli-hook-corrupt-policy`(user policy.yaml 破損 → fail-open)
  - windsurf: `cli-hook-windsurf-shell-deny`・
    `cli-hook-windsurf-shell-cd-allow`(cwd 遷移後 read → fail-open)
  - copilot: `cli-hook-copilot-deny`(`permissionDecision` deny)・
    `cli-hook-copilot-allow`・`cli-hook-copilot-foreign-allow`
    (dual-schema allow)
  - pretooluse: `cli-hook-pretooluse-deny`(`hookSpecificOutput`)・
    `cli-hook-pretooluse-allow`
  - cursor: `cli-hook-cursor-deny`(before-read-file)・
    `cli-hook-cursor-shell-deny`(before-shell-execution)
  - 網羅性: copilot/pretooluse/cursor の normalize は event 名を
    参照しない(TS と同じ構造)ため、deny が発生しうる全
    target×request-kind cell + fail-open 経路を binary 比較で
    網羅した。spawn/OS 依存の cell(symlinked cwd・FIFO・
    `cheaper.mjs` bundled entry)と in-process でしか表現できない
    cell(panic → allow・usage 非記録・non-hook `--target` 拒否)は
    `tests/hook.rs`・`tests/args.rs` が既に担保 — gap 一覧と理由は
    `tests/differential/README.md` fixture inventory に集約。
- `tests/usage_log.rs` — 新旧双方向互換 test 3 件追加:
  - `ts_writer_shape_records_parse_and_aggregate` — TS
    `buildUsageRecord` 現行 shape の literal record(success run /
    usage-bearing error / count)と旧 schema record(最小 run・
    flag 無し savings)の混在 file を `read_usage_records` +
    `build_usage_report` が読み集計(旧 record の `?? 0`・非
    comparable 扱いを含む)。
  - `rust_written_records_satisfy_ts_reader_field_contract` —
    Rust `build_usage_record` の 3 形が TS reader(`isRecord`・
    `filterRunRecords`・`aggregate`)が触る field を全て正しい
    JSON 型で持つことを assert + append → read round-trip。
  - `ts_reader_reads_rust_written_log_and_vice_versa` — node +
    `packages/cli/dist` があれば spawn し、(a) Rust writer の
    usage.jsonl を実 TS `readUsageRecords`/`buildUsageReport` で
    集計して Rust report と一致を確認、(b) 実 TS
    `buildUsageRecord` の record を Rust reader が読めることを
    確認。node/dist 不在時は skip(differential 規約)。
- `tests/envelope.rs` — `1e-7`/`9.9e-7` の canonical number case
  追加(Done When の `1.0`/`1e-7`/非有限網羅要件)。
- `docs/guides/usage.md` — `usage.costUsd` が cheaper pricing
  layer 解決価格(runtime catalog → bundled snapshot)× token で
  算出され provider 報告値ではないこと、新旧 record の additive-only
  混在互換(双方向読み書き可・costUsd 算出元の差)を追記。
- `tests/differential/README.md` — `compareStderr` schema・hook
  inventory・未網羅 cell の理由を記録。

### 検証

- `mise exec -- cargo test -p cheaper-cli --test differential` —
  55 cases、pass=47 / pinned divergence=8 / fail=0(hook 16 case
  全て stdout+stderr+exit code byte 一致。deny reason の token 数
  `20002` も tiktoken-rs / js-tiktoken で一致)。
- `mise exec -- cargo test -p cheaper-cli --test usage_log` —
  9 test 全パス(node cross-check 実施: Rust-written file → TS
  `buildUsageReport` が Rust report と一致、TS `buildUsageRecord`
  → Rust reader で parse+集計)。
- `mise exec -- cargo test -p cheaper-cli --test envelope` — 8 test
  全パス(`1e-7`/`9.9e-7` → `1e-7`/`9.9e-7` 形を確認)。

### 既知の divergence / 留意点

- usage costUsd の算出元差(TS = provider 報告 `usage.cost.total`
  合算、Rust = 解決 pricing × token)は spec 既定の差分であり、
  `meta.usage.costUsd` の differential 厳密比較は corpus の fake
  cost 規約で両者が一致する場合のみ成立する — `docs/guides/usage.md`
  に新旧 record の算出元差を記録済み。
- `compareStderr` は stderr を比較対象に加える opt-in — 既存 case
  (hook/version 系)には hook 系のみ付与。`cli-version` 等は既定
  非比較のまま(node の警告等の混入余地を残さない)。
- TS cross-check test は node/`dist` 不在環境では skip — CI で
  実行されない可能性は differential harness と同じ規約。

## Phase 4 follow-up fix(F1–F7、pin 解除)

### Scope

confirm 済み finding 7 件を TS 挙動へ揃える修正。詳細は
`docs/tasks/phase-4-port-cli-and-workflow.md` のチェックリスト参照。

- F1: `SharedToolError::source()` が inner を返すようにし
  (`inner()` accessor 追加)、`cheaper_view` が `SharedToolError` /
  `WorkerRunError` を再帰 unwrap する。wrap された `CheaperError` が
  `TOOL_EXECUTION_FAILURE` に潰れなくなる(`INVALID_WORKER_DEFINITION`
  exit 3・`ensure_model` の `INVALID_MODEL_CONFIG` を保持する
  regression test を `tests/workflow.rs` に追加)。
- F2: `models --format text` の `snapshotVersion` が string/number
  両方を受理(`"2025-09"` → `@2025-09`)。
- F3: `auth.rs`/`usage_report.rs` 固有の `as_secs_f64` 時計を撤去し
  `crate::cli::now()`(`as_millis` truncate = `Date.now()` 等価)へ
  統一。fractional ms duration の発生経路を排除。
- F4: PROVIDER_FAILURE の `error.details` を TS run.ts と同じ
  `{errorMessage}` のみへ揃える(fake provider + rig adapter の
  status_details shape を撤去)。`run-error-429-exhausted` /
  `run-error-5xx-exhausted` / `run-error-permanent` /
  `run-emit-then-error` の pin 解除。
- F5: `auth status` の oauth advertise を provider 宣言
  (`auth.oauth`)ベースへ変更(flow 登録ではなく宣言が
  `methods`/`subscription` を決める)。login 実行自体は引き続き
  登録 flow 必須。`cli-auth-status` の pin 解除。
- F6: `read_usage_records` を `from_utf8_lossy` + per-line parse に
  変更 — 破損 byte は U+FFFD 置換で行全体を捨てず、malformed JSON
  行のみ skip(TS reader 規約)。
- F7: auth prompt(`ask`/`ask_secret`)に read 完了直後の cancel
  再確認を追加。blocking read 自体は割り込めない残差あり
  (コード内コメントに記録)。

### 受容 divergence(残 pin — `tests/differential/README.md`)

- `run-panic`: panic 写像差(INTERNAL_ERROR(70) vs
  PROVIDER_FAILURE(5))— TS seam に等価 input が無い。
- `run-max-turns` / `run-max-tool-calls`: abort 後の in-flight
  call `turn_end` 計上差 — fixture DSL が abort 後 turn を消化
  できず表現不能。
