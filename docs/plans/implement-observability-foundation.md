---
id: "PLAN-034DVGKbl3V2g7msSfpXv0"
type: "plan"
status: "approved"
title: "Implement observability foundation"
created: "2026-10-11"
updated: "2026-10-11"
owners: []
relations:
  source:
    - "docs/specs/observability-foundation-scope.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/observability-foundation-scope.md"
  implemented-by:
    - "docs/tasks/create-cheaper-observability-crate.md"
    - "docs/tasks/wire-cli-log-flags-and-observability-init.md"
    - "docs/tasks/instrument-cli-dispatch-and-run-pipeline.md"
    - "docs/tasks/instrument-tools-workflow-provider-and-adapter.md"
    - "docs/tasks/instrument-hook-auth-catalog-and-core.md"
    - "docs/tasks/observability-contract-tests-and-verification.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/consolidate-observability-behind-a-dedicated-crate.md"
    - "docs/adr/logging-output-and-activation-contract.md"
    - "docs/designs/cli-contract-envelope-exit-codes.md"
    - "docs/discovery/observability-foundation-logging-and-profiling.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/observability-foundation-scope.md"
    - "docs/designs/observability-foundation-logging-and-instrumentation-wiring.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/observability-foundation-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement observability foundation Implementation Plan

## Goal

SPEC-034DUFyMl47o924O1KW8Ya の observability foundation(logging +
profiling)を、DESIGN-034DV9ypzmxdAobGSJ6Cm6 が確定した module map・
instrumentation 配置・test 戦略に従い、`cheaper-observability` crate
新設 → cli activation 配線 → 3 wave の tracing instrumentation →
contract test + 検証の 6 task で実装する。

## Scope

単一 plan として実行する。activation contract・sink 構成・instrumentation
規約が 1 つの contract として相互依存しており、crate 新設から配線・
instrumentation・test までを同一の release intent(log 基盤の導入)で
実施するため分割しない。

spec の Out of Scope(最適化の実施・CPU sampling / memory profiling・
OTLP 等の外部 backend・raw payload capture の opt-in 機構)は全 task で
対象外。envelope・exit code・usage.jsonl・hook contract・`--quiet`
semantics・`panic = "unwind"` は不変条件として維持する。

## Upstream Documents

- `docs/specs/observability-foundation-scope.md`(approved)— 実施
  スコープ・acceptance criteria
- `docs/adr/consolidate-observability-behind-a-dedicated-crate.md`
  (accepted)— crate topology(everything → cheaper-observability →
  tracing-subscriber の一方向)
- `docs/adr/logging-output-and-activation-contract.md`(accepted)—
  dual sink・format・activation(flag > env・既定 OFF)・
  metadata-only・`--quiet` 共存ルール

## Design Inputs

- `docs/designs/observability-foundation-logging-and-instrumentation-wiring.md`
  (approved、DESIGN-034DV9ypzmxdAobGSJ6Cm6)— AUTHORITATIVE。
  crate 公開面・init 呼出し点・rolling 既定値・span/field canonical
  naming・instrumentation 配置表・test 戦略・risk は全て design に
  従う

## File Map

### T1: `cheaper-observability` crate 新設

- `crates/cheaper-observability/Cargo.toml` — 新規 crate。依存は
  `tracing`・`tracing-subscriber`(`fmt`・`json`・`env-filter`
  相当の feature)・`tracing-appender`(0.2 系、
  `RollingFileAppender::builder().max_log_files(n)` が必要なため
  **0.2.3 以降**)のみ。workspace 内 crate への依存は 0
- `crates/cheaper-observability/src/lib.rs` — `Activation`・
  `ObservabilityGuard`・`init` の公開面
- `crates/cheaper-observability/src/config.rs` — `LogLevel`
  (Off|Error|Warn|Info|Debug|Trace)・`LogFormat`(Text|Json)・
  `resolve(&Activation) -> ResolvedConfig`(flag > env > default、
  invalid 値は warnings + fail-open)
- `crates/cheaper-observability/src/sink.rs` — file roller
  (`Rotation::DAILY`・`filename_prefix("cheaper.log")`・
  `max_log_files(7)`・`<log_root>/.cheaper/logs`)+ `non_blocking`、
  stderr writer(`std::io::stderr`、`with_ansi(false)`)
- `crates/cheaper-observability/src/init.rs` — `Registry` +
  file/stderr `fmt::Layer`(format 分岐は `Layer::boxed()` で
  `Box<dyn Layer<Registry> + Send + Sync>` へ揃える)+ level
  filter + `with_span_events(FmtSpan::CLOSE)` +
  `set_global_default`。repeat-init は first subscriber wins で
  `(None, warnings + "subscriber already installed")`(fail-open)
- `Cargo.toml` — `[workspace.dependencies]` に tracing 3 crate の
  version pin を追加
- `Cargo.lock` — `tracing-appender` 追加の更新(committed lockfile
  check の対象)
- crate-internal unit test: activation 解釈 matrix・repeat-init
  contract(`src/config.rs` / `src/init.rs` 内)

### T2: cli `--log`/`--log-format` 配線と init 呼出し

- `crates/cheaper-cli/Cargo.toml` — `cheaper-observability =
  { path = "../cheaper-observability" }` + `tracing.workspace`
- `crates/cheaper-cli/src/args.rs` — `STRING_OPTIONS` に `"log"`・
  `"log-format"` 追加、`Values::set_string` 受理 arm、`workflow`
  両 action の `allowed` 表へ追加(他 command には rejection guard
  を置かない = globally accepted)、lenient accessor
  `global_log_flags(argv)`(`--name value`/`--name=value`・`--`
  terminator・last-wins)+ bool 版 `global_flag_present(argv,
  "quiet")`
- `crates/cheaper-cli/src/cli.rs` — `run_cli` 先頭(`hook` 早期分岐・
  `parse_cli_args` より前)で init: `global_log_flags`・`io.env` 経由
  env 読取り・`resolve_log_root`(`--cwd` が意味を持つ経路 =
  run/run alias/workflow/models/count-tokens のみ argv `--cwd` を
  lenient scan、それ以外は `io.cwd`、`canonicalize_workspace_root`
  規約・失敗時 `io.cwd` fallback)・`stderr_enabled: !is_hook`・
  `_log_guard` を run_cli ローカルに束縛。hook 経路の init 警告は
  `io.stderr` に出さず install 済み subscriber へ `tracing::warn!`
  (未 install 時は drop)、非 hook 経路は `warning:` 行を
  `io.stderr` へ(`--quiet` 時抑制)
- `crates/cheaper-cli/tests/args.rs` — strict parse 受理・
  lenient scan の `--` 解釈の test 追加

### T3: instrumentation wave 1 — cli dispatch + run pipeline

- `crates/cheaper-cli/src/cli.rs` — `cheaper.cli.dispatch` span +
  汎用 catch の `warn!` event(`error.code`)。envelope 書出しは
  instrument しない
- `crates/cheaper-cli/src/run.rs` — `cheaper.cli.command`
  (`command="run"`・`worker.name`)
- `crates/cheaper-cli/src/workflow_run.rs` — `cheaper.cli.command`
  (`command="workflow-run"|"workflow-interface"`)
- `crates/cheaper-cli/src/models.rs`・`src/auth.rs`・
  `src/usage_report.rs`・`src/cli.rs` CountTokens arm —
  `cheaper.cli.command`(`command` field)
- `crates/cheaper-runtime/Cargo.toml` — `tracing.workspace`
- `crates/cheaper-runtime/src/run/mod.rs` — `cheaper.run` span
  (`worker.name`)+ stage 子 span(`tool_runtime`・`model_resolve`・
  `tool_scope`・`session`・`prompt`・`extract`/`repair`・`meta`)、
  `on_event` の `RunEvent::ToolCall`/`ToolResult`/`CompletionCall`
  で `trace!`/`debug!` event(`call.id`・`tool.name`・usage counts)

### T4: instrumentation wave 2 — tools/workflow/provider/adapter

- `crates/cheaper-tools/Cargo.toml` — `tracing.workspace`
- `crates/cheaper-tools/src/registry.rs` — `ToolRuntime::execute`
  に `cheaper.tool` span(`tool.name`)+ 結果 `debug!` event
  (`result.ok`/`error.code`)。全 tool call の単一の正
  instrumentation 点 — 個別 tool file には散らさない
- `crates/cheaper-tools/src/tools/process/`(mod/executable/tree)・
  `src/tools/grep_eval.rs` — `cheaper.tool.process` event
  (spawn/exit・`task`・`exit.code`)。subprocess 境界のみ
- `crates/cheaper-workflow/Cargo.toml` + `src/engine.rs` —
  `cheaper.workflow` span(`run.id`)+ node `debug!` event
- `crates/cheaper-provider/Cargo.toml` + `src/devin/credentials.rs`・
  `src/devin/cli.rs`・`src/devin/rig_provider.rs`・
  `src/devin/stream.rs` — `cheaper.provider.devin` event
  (credential load ok/miss・`devin models list` 起動/終了・stream
  connect)。JWT・token・credential 値は出さない
- `crates/cheaper-rig-adapter/Cargo.toml` + `src/run_stream.rs`
  (RunEvent kind 毎の `debug!`/`trace!` — `call.id`・`tool.name`・
  `turn`・completion usage counts。`TextDelta` 本文は `text.len`
  のみ)+ `src/agent.rs`(`cheaper.adapter.session` span)

### T5: instrumentation wave 3 — hook/auth/catalog/core

- `crates/cheaper-cli/src/hook/mod.rs` + `src/hook/targets.rs` —
  `cheaper.hook` span(`event`・`target`)+ 完了 `debug!` event
  (`verdict`・`stdout.len`/`stderr.len`)。`emit` の stderr 書出し
  自体は instrument しない
- `crates/cheaper-runtime/src/auth/store.rs` — `cheaper.auth.store`
  event(load/save・`provider`・`file.name` = basename のみ。
  credential 値・full path は出さない)
- `crates/cheaper-runtime/src/auth/service.rs` — `cheaper.auth`
  span(`provider`・`auth.type`)
- `crates/cheaper-runtime/src/catalog/mod.rs` — `cheaper.catalog`
  event(merge/persist・`provider.count`・`error.count`)
- `crates/cheaper-core/Cargo.toml` — `tracing.workspace`(core は
  emit のみ。tokio/rig・`cheaper-observability` への依存なし)
- `crates/cheaper-core/src/worker/resolve.rs`・`src/policy/load.rs`・
  `src/model/config.rs`・`src/model/resolve.rs` — `debug!`/`trace!`
  event(resolved name/choice 等。file path は field に載せない)

### T6: contract test + verification

- `crates/cheaper-cli/tests/observability.rs`(新規、non-seam)—
  `.cheaper/logs/` 生成・file sink 出力・JSON parse 可能性・text
  format・flag>env override・`--quiet` + log 有効化・repeat-init・
  hook stderr contract。file sink を assert する test は全て
  subprocess isolation(`CARGO_BIN_EXE_cheaper` + `env_clear`
  spawn — `tests/regression/{main.rs,harness.rs::spawn_collect}`
  前例)
- `crates/cheaper-cli/tests/observability_dev_seam.rs`(新規、
  `required-features = ["dev-seam"]`)— secrets 非記録 scan
  (credential・prompt・tool payload が `.cheaper/logs/*`・stderr
  から grep 0 件)+ run pipeline stage timing(`cheaper.run`
  span tree の close event `time.busy`・stage span 名)
- `crates/cheaper-cli/Cargo.toml` — dev-seam test target の
  `[[test]] required-features` 宣言
- `benchmarks/lib/cli.mjs` — `benchmarkEnvironment` に
  `CHEAPER_LOG*` env passthrough allowlist 追加
- `benchmarks/lib/run-scenario.mjs` +
  `benchmarks/scenarios/scenarios.json` — per-arm env 指定(または
  `--log debug` flag 注入 arm)の受け口((c) `CHEAPER_LOG=debug`
  arm 計測の required work)
- `mise run check` green

## Implementation Sequence

- [ ] T1 `cheaper-observability` crate 新設
- [ ] T2 cli `--log`/`--log-format` 配線と init 呼出し
- [ ] T3 instrumentation wave 1(cli dispatch + run pipeline)
- [ ] T4 instrumentation wave 2(tools/workflow/provider/adapter)
- [ ] T5 instrumentation wave 3(hook/auth/catalog/core)
- [ ] T6 contract test + verification

各 task 完了時に `cargo build -p <対象 crate>` + 対象 crate test を
実行し、最終 task で全 verification matrix を実行する。

## Dependencies and Risks

| Step | Depends on | Blocks |
| --- | --- | --- |
| T1 observability crate | — | T2, T4 |
| T2 cli wiring | T1 | T3 |
| T3 wave 1 | T2(`src/cli.rs` 共有のため直列) | T5, T6 |
| T4 wave 2 | T1 | T6 |
| T5 wave 3 | T3(`cheaper-runtime/Cargo.toml` の `tracing` dep は T3 が追加) | T6 |
| T6 contract test | T3, T4, T5 | — |

file overlap による順序付け: T3 は T2 と `crates/cheaper-cli/src/cli.rs`
を共有するため直列。T5 は `cheaper-runtime` を触るが `tracing` dep
追加を T3 に集約したため T3 後。T4 は触る crate(tools・workflow・
provider・rig-adapter)が T2/T3/T5 と disjoint のため T1 後から並列
可能。

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | `STRING_OPTIONS`・`workflow` `allowed` 表への追加漏れで strict parse が Unknown option / "not valid for workflow" を返す | med | med | T2 diff で両 table を網羅確認、T6 contract test で `--log`/`--log-format` 受理を固定 |
| 2 | lenient scan(`global_log_flags`/`global_flag_present`)と strict parse の解釈ずれ(`--` 以降の扱い) | med | low | `--` terminator で打ち切る規約を両者で共有し test で固定 |
| 3 | file sink assert test が同一 process で `set_global_default` 競合し不安定になる | med | med | design の test 戦略通り file assertion は subprocess isolation、in-process は repeat-init contract のみ |
| 4 | `tracing-appender` の `max_log_files` 削除挙動(prefix 依存・「m 個残したいなら m+1」)が想定と異なる | low | low | T1 で `filename_prefix("cheaper.log")` 固定 + 実測確認、T6 の retention 確認 |
| 5 | instrumentation field に payload・path が混入し secrets 非記録不変条件を破る | low | high | metadata-only allowlist(len/count/id/name/code/tier、`file.name` は basename のみ、path 不記録)を各 instrumentation task の Done-When に明記、T6 secrets scan で regression 防止 |
| 6 | log OFF の既定経路で regression 55・envelope byte-parity が壊れる | low | high | OFF 時は subscriber 未 install + dir 未作成。既存 suite が無変更 pass することが各 task の gate |

## Task Breakdown

各 task は task-doc で独立文書化する。粒度は「1 task = 1 wave の
atomic な変更 + その verification」で、上記 T1-T6 に対応する。

- `docs/tasks/create-cheaper-observability-crate.md`(T1)
- `docs/tasks/wire-cli-log-flags-and-observability-init.md`(T2)
- `docs/tasks/instrument-cli-dispatch-and-run-pipeline.md`(T3)
- `docs/tasks/instrument-tools-workflow-provider-and-adapter.md`(T4)
- `docs/tasks/instrument-hook-auth-catalog-and-core.md`(T5)
- `docs/tasks/observability-contract-tests-and-verification.md`(T6)

## Verification

- [ ] 各 task 後: `mise exec -- cargo build -p <対象 crate>` +
      `cargo test -p <対象 crate>`
- [ ] T2 後: `mise exec -- cargo tree -p cheaper-cli` に
      `cheaper-observability` が現れ、`cheaper-observability` から
      run/business crate への依存が無い
- [ ] T5 後: `mise exec -- cargo tree -p cheaper-core` に
      `tokio`/`rig`/`cheaper-observability` が現れない
- [ ] log OFF(既定)で regression 55 case・envelope byte-parity・
      既存 suite が無変更 pass
- [ ] `CHEAPER_LOG=debug` + `CHEAPER_LOG_FORMAT=json` の run で
      `.cheaper/logs/cheaper.log.*` と stderr の両方に `cheaper.run`
      stage span の close event(`time.busy`)が出て JSON が parse
      可能(subprocess test)
- [ ] `--log`/`--log-format` が env var を level・format 双方で
      override する test が存在
- [ ] secrets(credential/prompt/tool payload)が file/stderr に出ない
      regression test が存在
- [ ] `--quiet` + log 有効化で diagnostics 抑制・log 出力の test が
      存在(確定ルール: `--quiet` は明示的 log 有効化を抑制しない)
- [ ] repeat-init(2 回目 `init` が `(None, warning)`・first
      subscriber wins)の test が存在
- [ ] hook 経路で stderr に tracing 行・init `warning:` 行が出ず
      file に `cheaper.hook` event が出る test が存在
- [ ] file sink assertion が subprocess(`CARGO_BIN_EXE_cheaper`
      spawn)分離で実装されている
- [ ] off-state overhead の benchmark 3 点計測((a) 導入前 HEAD・
      (b) 導入後 OFF・(c) 導入後 `CHEAPER_LOG=debug`)が実施・
      `benchmarks/results/` に記録済み((c) arm 用の env
      passthrough + per-arm 注入受け口が実装済み)
- [ ] `.cheaper/logs/` が `.gitignore` の bare `logs` pattern
      (line 2)で cover されること(design で確認済み)
- [ ] 最終: `mise run check` exits 0

## Open Questions / Gaps

- (design で確定済み)rolling 既定値: `Rotation::DAILY` +
  `max_log_files(7)` + `filename_prefix("cheaper.log")`。
  「m 個残したいなら m+1」の挙動注意は T1 で実測確認
- (design で確定済み)hook 経路の stderr sink: 構造的に無効化
  (file のみ)+ init 警告は `io.stderr` に出さず `tracing::warn!`
  で畳む — ADR の open item は「干渉経路を作らない」で解消済み
- format 選択の型分岐: `Format<Full>`/`Format<Json>` で `fmt::Layer`
  の型が分岐するため `Layer::boxed()` で統一(design init.rs 構成
  通り)。実装時に別の enum dispatch 形を取る場合も層の責務は変えない

## Self-Review

- [x] Every spec requirement maps to a task(crate 新設 → T1、cli
      配線 → T2、instrumentation 配置表 → T3/T4/T5、contract test・
      benchmark 計測 → T6)
- [x] No placeholders remain
- [x] File paths, relations, status values, and terminology are
      consistent(design の instrumentation 表・file 指定に一致)

## Implementation Handoff

- Recommended mode: inline execution(T2→T3・T3→T5 の file 共有直列
  依存があり、critical path 上の task が順次依存するため逐次実行が
  安全)
- Independent work streams: T4 のみ T1 後に T2/T3 と並列可能
  (tools・workflow・provider・rig-adapter は他 task と file が
  disjoint)。規模を考えると逐次実行で十分
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix

## Review Handoff

- 上流 doc: spec(approved)+ 2 ADR(accepted)+ design(approved)
  で module map・activation contract・instrumentation 配置・test
  戦略は確定済み。open item(hook stderr 判断・rolling 既定値・
  span timing 方式)は全て design で確定しており Open Questions に
  残る未決事項は無い
- task 分解の基準: design の instrumentation 配置表を crate/file
  overlap 単位で wave 化し、`cli.rs` 共有(T2/T3)と
  `cheaper-runtime/Cargo.toml` 共有(T3/T5)のみ直列化、残りは
  disjoint 並列
- 各 task 完了時の `cargo test -p` green を checkpoint とし、最後に
  `mise run check` + contract test matrix で exit する
