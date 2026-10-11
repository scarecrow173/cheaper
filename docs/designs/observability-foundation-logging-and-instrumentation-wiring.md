---
id: "DESIGN-034DV9ypzmxdAobGSJ6Cm6"
type: "design"
status: "draft"
title: "Observability foundation logging and instrumentation wiring"
created: "2026-10-11"
updated: "2026-10-11"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/observability-foundation-scope.md"
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
    - "../adr/logging-output-and-activation-contract.md"
    - "../discovery/observability-foundation-logging-and-profiling.md"
    - "./cli-contract-envelope-exit-codes.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/observability-foundation-scope.md"
    - "../adr/consolidate-observability-behind-a-dedicated-crate.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Observability foundation logging and instrumentation wiring

## Context

SPEC-034DUFyMl47o924O1KW8Ya が定める observability foundation
(logging + profiling)を、ADR-034DUFzrXJxOwfl1qveXWR(専用 crate
集約)と ADR-034DUFA70cQdAyG7Lbuuay(出力・activation contract)の
決定に沿って実装するための配線設計。本 design が確定するのは
spec の Deferred Design Concerns に挙げられた open item:

- `cheaper-observability` crate の公開面と内部構成
- init の呼出し点・guard 寿命・activation 解釈の入力形状
- rolling の既定値(rotation・世代数・file 命名)
- span/field の canonical naming と JSON field schema
- stderr sink の `CliIo` diagnostics との共存機構、hook 経路の扱い
  (ADR-2 の「実装時検証の open item」を本 design で確定)
- log 無効時 overhead の計測方法

backend は `tracing` + `tracing-subscriber` + `tracing-appender`
(discovery H3)。`tracing` 0.1.44 / `tracing-subscriber` 0.3.23 は既に
Cargo.lock に transitive dep として存在し、`tracing-appender` のみ
新規追加(0.2 系、`RollingFileAppender::builder().max_log_files(n)`
が必要なため 0.2.1 以降 — Cargo.lock 更新は committed lockfile check
の対象)。

## Scope

### In Scope

- `crates/cheaper-observability` 新設(activation 解釈・sink 構築・
  format 選択・filter 配線・subscriber install・guard)
- `cheaper-cli` の args/dispatch への `--log`/`--log-format` 追加と
  `run_cli` 先頭での init 呼出し
- 全 crate への `tracing` span/event instrumentation
  (cli dispatch・hook・tool internals・run pipeline stages・
  auth/catalog I/O boundary)
- `.cheaper/logs/` file sink(daily rolling + 世代数 retention)と
  stderr sink の dual routing
- span/field canonical naming、metadata-only redaction 規約、
  contract test 群
- log 無効時 overhead の計測方法の定義

### Out of Scope

- 最適化の実施、CPU sampling / flamegraph / memory profiling
- 外部 backend(OTLP 等)への送出、stdout への log 出力
- raw payload capture の opt-in 機構(spec で deferred)
- envelope・exit code・usage.jsonl・hook contract・`--quiet`
  semantics の変更

## Components and Boundaries

### `cheaper-observability` crate(`crates/cheaper-observability/`)

activation 解釈・sink 構築・subscriber install を所有する唯一の
境界。run/business crate に依存しない(`everything →
cheaper-observability → tracing-subscriber` の一方向)。`cheaper-core`
はこの crate に依存しない(tokio/rig-free 不変条件の維持)。

公開面(最小):

```rust
/// activation 入力 — cli が flag 値と env 値を生のまま渡す
/// (解釈・優先順位・検証は本 crate の責務)。
pub struct Activation {
    /// `--log <level>`(未指定 None)。
    pub flag_level: Option<String>,
    /// `--log-format <text|json>`。
    pub flag_format: Option<String>,
    /// `CHEAPER_LOG` env 値。
    pub env_level: Option<String>,
    /// `CHEAPER_LOG_FORMAT` env 値。
    pub env_format: Option<String>,
    /// `.cheaper/logs` のアンカー(workspace root — 呼出し側で
    /// `--cwd`/process cwd を解決済み)。
    pub log_root: PathBuf,
    /// hook 経路では `false`(stderr sink 抑制 — 後述)。
    pub stderr_enabled: bool,
}

/// init 結果の guard。`Drop` で non_blocking writer を flush する。
/// OFF 解決時は `init` が `None` を返すため本型自体が不要な経路は
/// 存在しない。
pub struct ObservabilityGuard { /* WorkerGuard 保持 */ }

/// activation 解釈 + sink 構築 + subscriber install。
/// - 解決結果が OFF → `Ok(None)`(subscriber は install しない)。
/// - file sink 構築失敗 → stderr-only で継続 + `warnings` へ理由を積む
///   (fail-open — logging 設定で command を失敗させない)。
/// - level/format の値が不正 → `warnings` へ積み、level は OFF・
///   format は `text` 扱い(fail-open)。
pub fn init(activation: &Activation) -> (Option<ObservabilityGuard>, Vec<String>);
```

`init` の戻り値 `Vec<String>` は init 時警告
(`warning: invalid CHEAPER_LOG value ...` 等の message 本体)。
cli が `io.stderr` へ `warning:` 行として流す(`--quiet` 時は抑制 —
init 警告は diagnostics の一種とみなす)。

内部構成:

- `config.rs` — `LogLevel`(`Off|Error|Warn|Info|Debug|Trace` の
  最小集合、文字列からの parse)+ `LogFormat`(`Text|Json`)+
  `resolve(&Activation) -> ResolvedConfig`。
  優先順位は **flag > env > default**。`flag_level`/`env_level` の
  双方が None、または解決後が `Off` → OFF。
- `sink.rs` — file roller(`RollingFileAppender` builder:
  `rotation(Rotation::DAILY)`・`filename_prefix("cheaper.log")`・
  `max_log_files(7)`・`build(<log_root>/.cheaper/logs)`)+
  `non_blocking` 化、stderr writer。
- `init.rs` — `Registry` + 2 つの `fmt::Layer`(file layer:
  non_blocking appender、stderr layer: `std::io::stderr`
  MakeWriter)+ level filter + `set_global_default`。

依存: `tracing`、`tracing-subscriber`(features:
`fmt`・`json`・`env-filter` 相当の filter 配線に必要な feature)、
`tracing-appender`。`cheaper-observability` が保有するのは
この 3 つ + `std` のみで、workspace 内 crate への依存は 0。

#### file sink の既定値(確定)

- rotation: **`Rotation::DAILY`**。per-startup(`Rotation::NEVER` +
  起動毎 file)は `cheaper run` が親 agent の tool call ごとに起動
  されるため invocation 毎に file が増殖し、retention が file 数
  ベースでしか効かず追跡しにくい。daily は 1 日 1 file で調査単位と
  一致し、`max_log_files` の世代管理と相性がよい。
- 世代数: **`max_log_files(7)`**(直近 7 file ≒ 7 日分)。「m 個残
  したいなら m+1」の挙動注意(0.2.x doc)は実装時に実測で確認する。
- file 命名: prefix `cheaper.log` → `cheaper.log.YYYY-MM-DD`。
  **`filename_prefix` は必須** — `max_log_files` の削除対象判定は
  prefix/suffix 一致を要求し、無指定では「file 名が日付として
  parse できる」もののみが候補になるため。
- dir が作成できない/書けない場合: **fail-open で stderr-only** に
  継続し、警告を `warnings` へ積む(command を logging 都合で
  失敗させない)。

#### stderr sink

`fmt::Layer` の writer は `std::io::stderr`(process stderr 直書き)。
`io.stderr`(`CliWrite`)経路に載せない — `MakeWriter` が要求する
`'static + Send` を `&mut dyn CliWrite` borrow では満たせず、
CliIo は trait object 共有を前提としていないため、tracing を
io boundary 通しにすると boundary 自体の再設計が必要になる。
両 channel は同一 fd への行単位 write として interleave する
(ADR-2 の確定ルール)。1 event = 1 行(text も JSON も single-line)
を fmt layer の既定で満たす。

ANSI は stderr layer で `with_ansi(false)` — diagnostics 行が常に
plain text である規約に揃え、hook/pipe 経路での制御文字混入を
構造的に排除する。

#### format

`LogFormat::Text` → `fmt::format::Format<Full>`(既定 pretty/full
形式)、`LogFormat::Json` → `fmt::format::Format<Json>`。
format は file/stderr **両 sink 共通**(sink 別 format は要件外、
YAGNI)。

### activation surface(`cheaper-cli`)

`--log`・`--log-format` は args.rs の既存 option 機構に載せる
(lenient pre-scan ではない — strict parse の対象にする):

- `STRING_OPTIONS` に `"log"`・`"log-format"` を追加し、
  `Values` に `log: Option<String>`・`log_format: Option<String>`
  を追加。env var 名 `CHEAPER_LOG`/`CHEAPER_LOG_FORMAT` と揃えた
  kebab-case flag(ADR-2)。
- 全 command の `allowed` オプション表に `"log"`・`"log-format"`
  を追加(global option 化 — `format`/`quiet` と同じく per-command
  allowed list に入れる既存規約に従う)。`ParsedCli` 各 variant の
  args struct には載せない — log 設定は command 引数ではなく
  process 全体の activation であり、args struct 増設は表現の分散に
  なる。代わりに `parse_cli_args` とは別の軽量 accessor
  `global_log_flags(argv) -> (Option<String>, Option<String>)` を
  args.rs に用意し、`run_cli` が init 前に呼ぶ(strict parse が後で
  検証するため、accessor は `--name value`/`--name=value`・`--`
  terminator・last-wins のみを解釈する lenient scan でよい)。

### init 呼出し点と guard 寿命

`crates/cheaper-cli/src/cli.rs::run_cli` の先頭 — `hook` 早期分岐
より前、`parse_cli_args` より前:

```rust
pub async fn run_cli(argv: &[String], io: &mut CliIo<'_>, hooks: &CliHooks) -> i32 {
    let is_hook = argv.first().map(String::as_str) == Some("hook");
    let (flag_level, flag_format) = crate::args::global_log_flags(argv);
    let activation = cheaper_observability::Activation {
        flag_level, flag_format,
        env_level: (io.env)("CHEAPER_LOG"),
        env_format: (io.env)("CHEAPER_LOG_FORMAT"),
        log_root: resolve_log_root(argv, io),
        stderr_enabled: !is_hook,
    };
    let (_log_guard, warnings) = cheaper_observability::init(&activation);
    for w in &warnings {
        if !argv.iter().any(|a| a == "--quiet") {
            io.stderr.write_str(&format!("warning: {w}\n"));
        }
    }
    if is_hook {
        return crate::hook::execute_hook(&argv[1..], io, hooks).await;
    }
    // ...既存 dispatch
}
```

- env 値は `io.env` closure 経由で読む(`std::env::var` ではない) —
  test が `EnvLookup` で注入でき、hook 経路・fixture overlay の env
  と同一経路になる。
- `resolve_log_root`: argv の `--cwd`(`--cwd value`/`--cwd=value`)
  を lenient scan し、あればそれ、無ければ `io.cwd` を
  `canonicalize_workspace_root` で解決(run 経路の workspace root
  解決と同一規約 — `.cheaper/workers` 等と同じ anchor)。canonicalize
  失敗時は `io.cwd` に fallback(fail-open)。
- `_log_guard` は `run_cli` のローカルに束縛し、dispatch・record
  usage 完了後の `run_cli` return 時に drop → non_blocking writer
  が flush される。tokio runtime(`main.rs::run_process` の
  `rt.block_on`)は `run_cli` より外側・長命なので、guard drop は
  runtime 生存中に完了する。install は
  `tracing::subscriber::set_global_default`(process 全域 —
  spawned task を跨ぐ span 継承のため)。
- `hook` 経路は `stderr_enabled: false` → file sink のみ。

#### hook 経路の stderr 判断(ADR-2 open item の確定)

hook 経路では stderr channel は「親 agent が無視してよい診断口」
ではなく target native contract の一部: windsurf は deny 時の
guidance を stderr で返す(`targets.rs::windsurf_render` →
`HookResponse.stderr`、exit 2)。cursor/pretooluse/copilot は
stderr を emit しないが、親 agent が stderr を user 向け log 面へ
流す実装差があり、tracing の行が target 応答に混入する経路を
残したくない。hook invocation は短命で file sink で十分追跡できる
ため、**hook 経路は stderr sink を構造的に無効化し file sink のみ
有効**とする(env var activation は伝播する — flag を通せない
経路を env が cover する ADR-2 の狙い通り)。これで「stderr が
hook contract に干渉するか」の open item は「干渉経路を作らない」
で確定的に解消する。

`__grep-eval` helper(`main.rs` の argv 先頭一致分岐)は `run_cli`
に到達しないため subscriber は install されない — tool 側 emit は
no-op のまま(内部 subprocess、parent が spawn を記録する)。

## Data and Control Flow

### activation 解決順

1. `run_cli` 先頭で `global_log_flags(argv)` が `--log`/
   `--log-format` を lenient scan、`resolve_log_root` が `--cwd`
   を解決、`io.env` から `CHEAPER_LOG`/`CHEAPER_LOG_FORMAT` を取得。
2. `cheaper_observability::init` が flag > env > default で level/
   format を解決 → OFF なら `None` を返して終了(subscriber 無し、
   以降の emit は `tracing` の no-subscriber fast path で ~zero
   cost)。
3. file sink 構築(`<log_root>/.cheaper/logs/` に
   `create_dir_all` + `RollingFileAppender::builder()...build()`)→
   失敗時は stderr-only + warning。
4. `Registry` + file/stderr `fmt::Layer`(format・`with_ansi(false)`・
   `with_span_events(FmtSpan::CLOSE)`)+ level filter で
   `set_global_default`。
5. 通常 dispatch。envelope・exit code・`io.stderr` diagnostics・
   `--quiet` semantics は不変。

### stage timing の記録方式(確定)

span close 時に `tracing-subscriber` の **span timing
(`FmtSpan::CLOSE`)** を使う。両 `fmt::Layer` に
`with_span_events(FmtSpan::CLOSE)` を設定すると span close 時に
close event が emit され、`time.busy`(span の合計 busy 時間)/
`time.idle` が field として付く(JSON format では close event の
`fields` に `time.busy`/`time.idle`、text format では close 行に
timing が出る)。

- 採用理由: stage timing を span 配置 1 箇所で得られ、各 site に
  `Instant` + `elapsed_ms` 記録の二重実装を持たない(DRY)。
  busy/idle 分解は current_thread runtime 上の wait 時間と実働時間の
  区別にも効く。
- explicit `elapsed_ms` field を併記する案は不採用 — 同一情報の
  二重表現(表現ドリフト)になり、site 毎の計測コードは配置ミスの
  温床。payload 的に span tree から stage 分解が取れることが
  profiling 要件そのものなので、close event + span 親子関係で
  足りる。

### span/field canonical naming

- span 名: `cheaper.<subsystem>[.<stage>]` の static 名。
  per-tool の動的名(`cheaper.tool.grep`)は `span!` の
  `'static` 制約と相性が悪いため **`cheaper.tool` 単一 span +
  field `tool.name`** を canonical とする(tool 名は field で
  機械検索可能)。
- field 命名(全 crate 共通): `worker.name`、`command`、
  `tool.name`、`call.id`、`model.provider`/`model.id`/`model.tier`、
  `event`/`target`/`verdict`(hook)、`provider`/`auth.type`
  (auth)、`error.code`(失敗時)。
- metadata-only 規約: payload 本体(prompt・tool input/output・
  credential)は field に載せない。載せてよいのは `*.len`(byte
  数: `input.len`/`output.len`/`raw_output.len`)、`*.count`/
  `turns`/`tool_calls`、`*.id`、`name`/`code`/`tier` のような
  分類値。hash は本フェーズでは不要(len+id で相関可能、YAGNI)。
- event 名: span 内の point event は `cheaper.<subsystem>.event`
  を `event!` の target ではなく span field で表現せず、通常の
  `debug!`/`info!`/`warn!` event + message で構造化する(JSON
  format の `fields.message` + span context で十分検索可能)。

### instrumentation 配置(実 file 指定)

| crate | file | span/event | 備考 |
| --- | --- | --- | --- |
| cheaper-cli | `src/cli.rs` | `cheaper.cli.dispatch`(dispatch 全体)＋汎用 catch の `warn!` event(`error.code`) | `run_cli` 内 `dispatch(...)` を `info_span!` で包む。envelope 書出しは instrument しない(stdout 経路とは無関係の span のみ) |
| cheaper-cli | `src/run.rs` | `cheaper.cli.command` field `command="run"`,`worker.name` | `execute` 冒頭で `info_span!` + `Instrument` |
| cheaper-cli | `src/workflow_run.rs` | `cheaper.cli.command` field `command="workflow-run"|"workflow-interface"` | 同上 |
| cheaper-cli | `src/models.rs`・`src/auth.rs`・`src/usage_report.rs`・`src/cli.rs` CountTokens arm | `cheaper.cli.command` field `command` | 各 command module の `execute` 系 entry |
| cheaper-cli | `src/hook/mod.rs` + `src/hook/targets.rs` | `cheaper.hook` span(field `event`,`target`)＋完了 `debug!` event(`verdict`,`stdout.len`/`stderr.len`) | `execute_hook_inner`→`handle_hook` 境界。`emit` の stderr 書出し自体は instrument しない |
| cheaper-runtime | `src/run/mod.rs`(`RunCtx::run`) | `cheaper.run`(field `worker.name`)＋stage 子 span: `cheaper.run.tool_runtime`(invocation 生成)、`cheaper.run.model_resolve`(available→resolve→`resolve_model_auth`)、`cheaper.run.tool_scope`、`cheaper.run.session`、`cheaper.run.prompt`(`prompt_with_abort_grace`)、`cheaper.run.extract`/`cheaper.run.repair`、`cheaper.run.meta` | stage span は `info_span!` + `Instrument` で async 区間を包む。`on_event` の `RunEvent::ToolCall`/`ToolResult`/`CompletionCall` で `trace!`/`debug!` event(`call.id`,`tool.name`,usage counts — token 数は metadata)|
| cheaper-runtime | `src/auth/store.rs` | `cheaper.auth.store` event(load/save、`provider`,`path` — path は metadata) | credential 値は一切出さない |
| cheaper-runtime | `src/auth/service.rs` | `cheaper.auth` span(`provider`,`auth.type`) | login/logout/status の op 単位 |
| cheaper-runtime | `src/catalog/mod.rs` | `cheaper.catalog` event(merge/persist、`provider.count`,`error.count`) | bundled/persisted/user overlay の境界 |
| cheaper-tools | `src/registry.rs`(`ToolRuntime::execute`) | `cheaper.tool` span(field `tool.name`)＋結果 `debug!` event(`result.ok`/`error.code`) | 全 tool call の単一の正 instrumentation 点 — 個別 tool file に散らさない |
| cheaper-tools | `src/tools/process/` | `cheaper.tool.process` event(spawn/exit、`task`,`exit.code`) | subprocess 境界のみ。`__grep-eval` spawn(`grep_eval.rs`)も同様の event |
| cheaper-rig-adapter | `src/run_stream.rs`(`map_item`/`unfold`) | `debug!`/`trace!` event per RunEvent kind(`call.id`,`tool.name`,`turn`,completion usage counts) | `TextDelta` 本文は出さない — `text.len` のみ |
| cheaper-rig-adapter | `src/agent.rs` | `cheaper.adapter.session` span(session build) | model/auth 値は出さない |
| cheaper-provider | `src/devin/credentials.rs`・`cli.rs`・`rig_provider.rs`/`stream.rs` | `cheaper.provider.devin` event(credential load ok/miss、`devin models list` 起動・終了、stream connect) | JWT・token・credential 値は出さない |
| cheaper-workflow | `src/engine` 相当(run 経路) | `cheaper.workflow` span(`run.id`)＋node `debug!` event(node id/name) | engine の job 実行境界 |
| cheaper-core | `src/worker/resolve.rs`・`src/policy/load.rs`・`src/model/config.rs`・`src/model/resolve.rs` | `debug!`/`trace!` event(resolved name/path/choice) | 同期純ロジックのまま — `tracing` の emit のみ、tokio/rig 流入なし |

`cheaper-tools`・`cheaper-core` 等 leaf crate は `tracing` のみを
`[dependencies]` に追加し、`cheaper-observability` には依存しない
(emit は subscriber の有無を意識しない — ADR-1 の pattern)。
`cheaper-observability` に依存するのは `cheaper-cli` のみ。

### workspace layout / Cargo.toml 変更

- `crates/cheaper-observability/` — 新規 crate。
  `[workspace] members = ["crates/*"]` の glob で自動包含。
- root `Cargo.toml` `[workspace.dependencies]` に `tracing`・
  `tracing-subscriber`・`tracing-appender` の pin を追加(version
  整合を workspace で一元化)。
- `crates/cheaper-cli/Cargo.toml` — `cheaper-observability =
  { path = "../cheaper-observability" }` + `tracing.workspace`。
- `crates/cheaper-runtime`/`cheaper-tools`/`cheaper-workflow`/
  `cheaper-provider`/`cheaper-rig-adapter`/`cheaper-core` —
  `tracing.workspace` のみ追加(core への追加で tokio/rig は
  流入しない — `tracing` は runtime-agnostic)。
- `cheaper-observability` の `Config` 型を cli の flag parse が
  使う案は不採用 — flag parse は string のまま `Activation` に
  載せ、値の解釈(LogLevel/LogFormat への写像)は observability が
  行う(activation surface の単一所有 — ADR-1/-2)。

### off-state overhead(確定)

- 方式: **runtime filter + subscriber 未 install**。OFF 解決時は
  subscriber を install しないため、全 emit は `tracing` の
  no-subscriber fast path(global dispatch の atomic check +
  callsite `Interest` cache)で数 ns に収まる。
- compile-time `release_max_level_*` feature は不採用 — `--log`/
  `CHEAPER_LOG` による runtime 有効化と両立しない(installed
  binary で instrumentation を剥がすと flag が効かなくなる)。
- 計測方法(acceptance criterion 用): `node benchmarks/run.mjs
  --scenario <id>` の median wall-clock を、(a) instrumentation
  導入前 HEAD、(b) 導入後 CHEAPER_LOG 未設定、(c) 導入後
  `CHEAPER_LOG=debug` の 3 点で比較し、(a)→(b) の delta が
  ノイズ範囲内(目安: median で ±3% 未満)であることを確認する。
  計測結果は `benchmarks/results/` の JSON(再現可能な observation
  列)として残す。scenario は tool 実行を含む
  `single-simple-tool-call` 等、run pipeline を通るものを選ぶ。

### test 戦略

| 検証 | 場所 | 方式 |
| --- | --- | --- |
| activation 解釈(flag>env、invalid→fail-open、OFF 判定) | `cheaper-observability` unit test(`src/config.rs` 内) | `resolve` 直接呼出し |
| `.cheaper/logs/` 生成・file sink 出力 | `cheaper-cli/tests/observability.rs`(新規、non-seam) | mock `CliIo`(`io.cwd` = tempdir、`io.env` で CHEAPER_LOG 注入)で `run_cli` を呼び `<cwd>/.cheaper/logs/cheaper.log.*` の存在と内容を確認 |
| JSON format parse 可能性・text format 出力 | 同上 | `CHEAPER_LOG_FORMAT=json` で file 行を `serde_json` parse、text 指定で JSON 以外の行形式を確認 |
| flag>env override(level・format) | 同上 + observability unit | `CHEAPER_LOG=warn` + `--log debug` で debug event が出る、等 |
| `--quiet` + log 有効化 | 同上 | diagnostics(error/model/warning line)が出ず、log event が file に出る |
| secrets 非記録(credential・prompt・tool payload) | `cheaper-cli/tests/observability_dev_seam.rs`(新規、`required-features = ["dev-seam"]`) | fixture run(scripted fake session + `auth_logins` credential)を `CHEAPER_LOG=debug` で実行し、`.cheaper/logs/*` と stderr から secret 文字列・prompt 本文を grep して 0 件を assert |
| run pipeline stage timing event | 同上(seam 経路) | `cheaper.run` span tree の close event(`time.busy`)と stage span 名が file に出ること |
| hook 経路: stderr sink 無効・file sink 有効 | `tests/hook.rs` 拡張 または observability test | hook invocation で stderr に tracing 行が出ない + file に `cheaper.hook` event がある |
| envelope byte-parity・exit code・regression 55 | 既存 `tests/envelope.rs`・`tests/regression/` | log OFF(既定)で無変更 pass がそのまま証拠 |
| `.cheaper/logs/` の `.gitignore` cover | 静的確認(本 design で確認済み) | root `.gitignore` の bare `logs` pattern が `.cheaper/logs/` を cover — 追従不要 |

### mise/CI 配線

**追加の gate は作らない** — observability は production infra で
あり常時 compile 対象(spec の In Scope・ADR-1)。`dev-seam`
feature は test seam の gate 前例として activation 機構の参照に
留まり、observability 自体を feature-gate しない(consumer install
binary でも `--log` が効く必要がある)。`mise run check` の task 列・
`mise.toml` への変更は不要(新 crate は `members` glob・workspace
test/clippy で自動包含)。`tracing-appender` 追加による Cargo.lock
更新は `check-lockfile.mjs` の対象として自然に検証される。

## Non-Functional Constraints

- stdout は envelope 専用: sink は file + stderr のみ。fmt layer の
  writer に `std::io::stdout` を渡す経路を作らない。
- `io.stderr` diagnostics の形式・`--quiet` semantics 不変。
  tracing stderr 行は diagnostics と同 fd に行単位で interleave
  するが、diagnostics の emit 経路(`CliWrite`)には手を入れない。
- `panic = "unwind"`・panic hook 経路(`main.rs` の `eprintln!`)は
  不変 — panic 行を tracing に載せ替えない。
- `cheaper-core` は `tracing` emit のみ — `cheaper-observability`
  への依存・tokio/rig の流入なし(`cargo tree -p cheaper-core` で
  確認)。
- metadata-only: credential(`api_key`/`oauth` token/JWT)、
  `system_prompt`/`prompt`、tool input/output 本体を field 値に
  しない。secrets 非記録 test が regression を防ぐ。

## Risks and Trade-offs

| risk | 影響 | 緩和 |
| --- | --- | --- |
| `--log`/`--log-format` を全 command の `allowed` 表に追加し忘れると strict parse が Unknown option を返す | flag が使えない command が生じる | 全 command allowed list への追加を diff で網羅確認;`global_log_flags` の lenient scan と strict parse の解釈ずれ(`--` 以降の `--log` は strict でも positional 扱いで flag にならないが lenient scan も `--` で打ち切る規約に揃える)を test で固定 |
| 非 `run` command(help/version/auth login 対話等)でも `.cheaper/logs/` が作られる | 空 dir 汚染 | OFF 時は dir 作成しない(subscriber 自体を作らない)。有効時も file sink 構築は初回 event 書込みではなく init 時 — 空 file が残る程度は許容(rolling prefix file は初 write で生成される実装上の挙動、test で確認) |
| stderr に tracing 行が混ざると diagnostics 消費者を混乱させる | consumer の stderr 解析 | `--quiet` が log を抑制しない確定ルールと引換えに、明示的有効化のみでしか出ないこと・`.cheaper/logs` が恒久的な正で stderr は補助と文書化 |
| `FmtSpan::CLOSE` の close event が span 大量発生経路(tool call 反復)で file を肥大化させる | log 量 | level 既定は flag/env 指定必須で、close event は span 数比例。daily+7 世代で肥大の上限は見通せる。必要なら後続で `FmtSpan` を subsystem 別に絞る余地 |
| `tracing` dep を全 crate に追加すること自体の compile time・binary size 増 | build/binary | subscriber 未 install 時は callsite cost のみ。binary size 増は `fmt`/`json`/`appender` が `cheaper-cli` 側にのみ載る構成で最小化(leaf は `tracing` の emit 部分のみ)。off/on delta は benchmark 計測で記録 |
| hook 経路で file sink が `.cheaper/logs` を hook 呼出し毎に触る | hook レイテンシ | hook は stderr sink 無効で file のみ。daily file への追記は既存 file open → write で軽量。それでも問題なら後続で hook 経路の file sink も opt-out にできる余地を `Activation` に残す(本フェーズは YAGNI で残さない — 計測してから判断) |
| `tracing-appender` の `max_log_files` 削除対象判定が prefix 依存で、prefix 変更/外部 file 混入時に retention が効かない | disk 肥大 | `cheaper.log` prefix を固定値にし、`.cheaper/logs/` 配下の手編集は想定外と明記 |
| observability init が `catch_run_panic` より外で panic すると未処理 panic | exit 70 経路 | `run_cli` 先頭は `rt.block_on(catch_run_panic(run_cli))` の内側なので panic は INTERNAL_ERROR envelope に畳まれる。init は fail-open 設計(panic しない形で error を warnings に畳む)|

## Deferred Design Concerns

- raw payload capture の opt-in 機構 | spec で deferred 確定 |
  metadata-only で調査力不足が観測された時点 | credential 流出経路を
  設計前に作るリスク。
- 外部 backend(OTLP)送出 | `cheaper-observability` 境界に sink 追加
  として吸収可能 | 集約要求が生じた時点 | 無視しても現行 sink は
  変わらない。
- per-startup rotation・hook 経路 file sink の opt-out | 現行
  daily+7 世代・file-only で不自由が観測された時点 | `Activation`
  拡張で対応可能。
- CPU sampling / memory profiling | 最適化フェーズ | stage timing で
  bottleneck が絞れる間は不要。

## Verification Notes

- [ ] `cargo tree -p cheaper-cli` に `cheaper-observability` が現れ、
      `cheaper-observability` から run/business crate への依存が無い
- [ ] `cargo tree -p cheaper-core` に `tokio`/`rig`/`cheaper-observability`
      が現れない
- [ ] log OFF で regression 55・envelope byte-parity・`mise run check`
      が無変更で green
- [ ] `CHEAPER_LOG=debug` + `CHEAPER_LOG_FORMAT=json` の run で
      `.cheaper/logs/cheaper.log.*` と stderr の両方に `cheaper.run`
      stage span の close event(`time.busy`)が出て JSON が parse 可能
- [ ] `--log`/`--log-format` が env var を override する test が存在
- [ ] secrets(credential/prompt/tool payload)が file/stderr に出ない
      regression test が存在
- [ ] `--quiet` + `--log` で diagnostics 抑制・log 出力の test が存在
- [ ] hook 経路で stderr に tracing 行が出ず file に `cheaper.hook`
      event が出る test が存在
- [ ] log 無効時 overhead の benchmark 計測(3 点比較)が実施・記録済み
- [ ] `.cheaper/logs/` が `.gitignore` の `logs` pattern で cover
      されること(本 design で確認済み — bare `logs` pattern)

## Sources and Relations

- 上流: SPEC-034DUFyMl47o924O1KW8Ya(実施スコープ・acceptance
  criteria)、ADR-034DUFzrXJxOwfl1qveXWR(crate topology)、
  ADR-034DUFA70cQdAyG7Lbuuay(出力・activation contract —
  `related` で保持し derives-from 鎖は spec+crate topology ADR に
  限定)、DISC-034DUx01YRjOAf4gTaftO7(D1–D9)。
- 関連不変条件: DESIGN-034r05iHrZUE6WvDuYjZB4(envelope/exit
  code contract)。
- 実コード参照: `crates/cheaper-cli/src/main.rs`(panic hook・tokio
  runtime・`__grep-eval` 分岐)、`crates/cheaper-cli/src/cli.rs`
  (`run_cli`・dispatch・`CliIo`/`CliWrite`・error diagnostics)、
  `crates/cheaper-cli/src/args.rs`(STRING_OPTIONS・allowed
  option 表)、`crates/cheaper-cli/src/hook/{mod,targets}.rs`
  (hook contract・windsurf stderr)、
  `crates/cheaper-runtime/src/run/mod.rs`(run pipeline stages)、
  `crates/cheaper-tools/src/registry.rs`(`ToolRuntime::execute`)、
  `crates/cheaper-rig-adapter/src/run_stream.rs`(`RunEvent`)、
  `crates/cheaper-core/src/config/dirs.rs`(`<cwd>/.cheaper`
  project_dir 解決)、`.gitignore`、
  `benchmarks/{run.mjs,scenarios/scenarios.json}`。
