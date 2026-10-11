---
id: "TASK-034DVH0SzTXtnpmsbbiKuu"
type: "task"
status: "todo"
title: "T1: cheaper-observability crate 新設"
created: "2026-10-11"
updated: "2026-10-11"
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
    - "docs/plans/implement-observability-foundation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-observability-foundation.md"
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
---

# T1: cheaper-observability crate 新設

## Work

plan `docs/plans/implement-observability-foundation.md`
(PLAN-034DVGKbl3V2g7msSfpXv0)の T1。activation 解釈・sink 構築・
format 選択・filter 配線・subscriber install・guard を所有する
`crates/cheaper-observability` を新設する。design の公開面
(`Activation`・`ObservabilityGuard`・`init`)と内部構成
(config.rs/sink.rs/init.rs)は確定済みで、そのまま実装する。
run/business crate への依存は 0(everything →
cheaper-observability → tracing-subscriber の一方向)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-observability-foundation.md` — 本 task が
  implement する plan。File Map「T1」が作業の正
- `docs/designs/observability-foundation-logging-and-instrumentation-wiring.md`
  (DESIGN-034DV9ypzmxdAobGSJ6Cm6)— 公開面・内部構成・file sink
  既定値・repeat-init contract の AUTHORITATIVE 定義
- `docs/adr/consolidate-observability-behind-a-dedicated-crate.md` —
  crate 境界と依存方向の根拠
- `docs/adr/logging-output-and-activation-contract.md` — dual
  sink・format・activation 優先順位の根拠

## Dependencies

- task 依存なし(plan の Dependencies table で T1 は起点)
- `docs/tasks/wire-cli-log-flags-and-observability-init.md`(T2)と
  `docs/tasks/instrument-tools-workflow-provider-and-adapter.md`
  (T4)を block する — init 呼出し側・leaf crate の emit 側は crate
  確定後に着手

## Affected Files

- `crates/cheaper-observability/Cargo.toml` — 新規 crate。
  `[workspace] members = ["crates/*"]` の glob で自動包含。
  依存は `tracing`・`tracing-subscriber`(`fmt`・`json`・
  `env-filter` 相当の feature)・`tracing-appender`(0.2 系、
  `RollingFileAppender::builder().max_log_files(n)` が必要なため
  **0.2.3 以降**)のみ。workspace 内 crate への依存は 0
- `crates/cheaper-observability/src/lib.rs` — `Activation`
  (flag_level/flag_format/env_level/env_format/log_root/
  stderr_enabled)・`ObservabilityGuard`(Drop で non_blocking
  writer を flush)・`pub fn init(&Activation) ->
  (Option<ObservabilityGuard>, Vec<String>)` の公開面
- `crates/cheaper-observability/src/config.rs` — `LogLevel`
  (Off|Error|Warn|Info|Debug|Trace)・`LogFormat`(Text|Json)・
  `resolve(&Activation) -> ResolvedConfig`。優先順位 flag > env >
  default。invalid 値は warnings に積み level は OFF・format は
  `text` 扱い(fail-open)
- `crates/cheaper-observability/src/sink.rs` — file roller
  (`Rotation::DAILY`・`filename_prefix("cheaper.log")` —
  `max_log_files` の削除対象判定は prefix 一致を要求するため必須・
  `max_log_files(7)`・build 先 `<log_root>/.cheaper/logs`)+
  `non_blocking` 化、stderr writer(`std::io::stderr`、
  `with_ansi(false)`)。dir 作成/書込み失敗は stderr-only で継続 +
  warnings(fail-open)
- `crates/cheaper-observability/src/init.rs` — `Registry` +
  file/stderr 2 つの `fmt::Layer`(format が `Format<Full>`/
  `Format<Json>` で layer の型が分岐するため `Layer::boxed()` で
  `Box<dyn Layer<Registry> + Send + Sync>` に揃える)+ level
  filter + `with_span_events(FmtSpan::CLOSE)`(close event に
  `time.busy`/`time.idle` が付く)+ `set_global_default`。
  repeat-init: install 済みでの再呼出しは first subscriber wins で
  `(None, warnings + "subscriber already installed")` を返して継続
  (error 化しない)。初回が OFF 解決なら後続呼出しは install 可能。
  OFF 解決時は dir も作らず `(None, warnings)` を返す
- `Cargo.toml` — `[workspace.dependencies]` に `tracing`・
  `tracing-subscriber`・`tracing-appender` の version pin を追加
- `Cargo.lock` — `tracing-appender` 追加の更新(committed lockfile
  check の対象)
- crate-internal unit test(`src/config.rs`・`src/init.rs` 内):
  activation 解釈 matrix(flag>env・invalid→fail-open・OFF 判定)
  と repeat-init contract(1 test 内で `init` を 2 回呼び 2 回目の
  `(None, warning)` を確認 — `resolve` 直接呼出しは subscriber
  install を伴わないため in-process でよい)

## Done When

- [ ] `crates/cheaper-observability` が `Activation`・
      `ObservabilityGuard`・`init` の公開面を持ち、workspace
      members glob で build 対象に含まれる
- [ ] file sink が `Rotation::DAILY`・`filename_prefix("cheaper.log")`・
      `max_log_files(7)`・`<log_root>/.cheaper/logs` で構築され、
      stderr sink は `with_ansi(false)`、両 layer に
      `with_span_events(FmtSpan::CLOSE)` が設定されている
- [ ] format 選択が file/stderr 両 sink 共通で、`Layer::boxed()`
      経由で型分岐が統一されている
- [ ] fail-open contract(invalid 値→warnings・file sink 失敗→
      stderr-only・repeat-init→first subscriber wins・OFF→dir
      未作成 + `(None, _)`)が実装・test されている
- [ ] `cheaper-observability` から workspace 内 crate への依存が 0
      である

## Out of Scope

- cli 側の init 呼出し・`--log`/`--log-format` flag 配線 — T2 の範囲
- 各 crate への span/event instrumentation — T3/T4/T5 の範囲
- contract test・benchmark 計測 — T6 の範囲
- stdout sink・外部 backend(OTLP)・raw payload capture — spec Out
  of Scope(恒久的対象外 / deferred)

## Verification

- [ ] `mise exec -- cargo build -p cheaper-observability` exits 0
- [ ] `mise exec -- cargo test -p cheaper-observability` exits 0
      (activation 解釈・repeat-init の unit test 含む)
- [ ] `mise exec -- cargo tree -p cheaper-observability` が
      workspace 内 crate への依存を持たないこと(tracing/
      tracing-subscriber/tracing-appender + std のみ)
- [ ] `mise exec -- cargo fmt --check` と `mise exec -- cargo clippy
      -p cheaper-observability --all-targets -- -D warnings` exits 0
- [ ] `mise exec -- node scripts/check-lockfile.mjs` exits 0
      (committed lockfile が `tracing-appender` 追加を反映)
- [ ] `tracing-appender` が 0.2.3 以降で resolve されること
      (`cargo tree -p cheaper-observability | grep tracing-appender`
      で version 確認)
