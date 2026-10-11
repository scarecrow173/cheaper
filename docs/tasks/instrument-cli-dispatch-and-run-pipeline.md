---
id: "TASK-034DVHc8VqMfpOOrjqF65s"
type: "task"
status: "done"
title: "T3: instrumentation wave 1 — cli dispatch + run pipeline"
created: "2026-10-11"
updated: "2026-10-12"
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
    - "docs/tasks/wire-cli-log-flags-and-observability-init.md"
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

# T3: instrumentation wave 1 — cli dispatch + run pipeline

## Work

plan `docs/plans/implement-observability-foundation.md`
(PLAN-034DVGKbl3V2g7msSfpXv0)の T3。design の instrumentation
配置表のうち、`cheaper-cli` の dispatch/command 系と
`cheaper-runtime` の run pipeline stage 群へ `tracing`
span/event を配置する。span 名は `cheaper.<subsystem>[.<stage>]`
の static 名、stage timing は subscriber 側の
`FmtSpan::CLOSE`(T1 で設定済み)で得る — site 毎の
`Instant`/`elapsed_ms` 記録は持たない(DRY・表現ドリフト防止)。
metadata-only 規約: payload 本体・file path は field に載せない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-observability-foundation.md` — 本 task が
  implement する plan。File Map「T3」が作業の正
- `docs/designs/observability-foundation-logging-and-instrumentation-wiring.md`
  (DESIGN-034DV9ypzmxdAobGSJ6Cm6)— instrumentation 配置表・
  span/field canonical naming・stage timing 方式の AUTHORITATIVE
  定義

## Dependencies

- `docs/tasks/wire-cli-log-flags-and-observability-init.md`(T2)—
  `src/cli.rs` を共有するため直列(T2 の init 配線完了後に着手)
- `docs/tasks/instrument-hook-auth-catalog-and-core.md`(T5)を
  block する — `cheaper-runtime/Cargo.toml` の `tracing` dep 追加は
  本 task が担う(T5 は runtime auth/catalog file のみ触る)

## Affected Files

- `crates/cheaper-cli/src/cli.rs` — `run_cli` 内の `dispatch(...)`
  を `info_span!("cheaper.cli.dispatch")` で包み、汎用 catch に
  `warn!` event(`error.code` field)を追加。envelope 書出しは
  instrument しない(stdout 経路とは無関係の span のみ)
- `crates/cheaper-cli/src/run.rs` — `cheaper.cli.command` span
  (`command="run"`・`worker.name` field)。`execute` 冒頭で
  `info_span!` + `Instrument`
- `crates/cheaper-cli/src/workflow_run.rs` — `cheaper.cli.command`
  span(`command="workflow-run"|"workflow-interface"`)
- `crates/cheaper-cli/src/models.rs`・`src/auth.rs`・
  `src/usage_report.rs`・`src/cli.rs` CountTokens arm —
  各 command module の `execute` 系 entry に
  `cheaper.cli.command` span(`command` field)
- `crates/cheaper-runtime/Cargo.toml` — `tracing.workspace` を
  `[dependencies]` に追加(runtime crate の `tracing` dep は本
  task が一元で追加 — T5 の runtime file もこれを使う)
- `crates/cheaper-runtime/src/run/mod.rs`(`RunCtx::run`)—
  `cheaper.run` span(`worker.name`)+ stage 子 span:
  `cheaper.run.tool_runtime`(invocation 生成)、
  `cheaper.run.model_resolve`(available→resolve→
  `resolve_model_auth`)、`cheaper.run.tool_scope`、
  `cheaper.run.session`、`cheaper.run.prompt`
  (`prompt_with_abort_grace`)、`cheaper.run.extract`/
  `cheaper.run.repair`、`cheaper.run.meta`。stage span は
  `info_span!` + `Instrument` で async 区間を包む。
  `on_event` の `RunEvent::ToolCall`/`ToolResult`/
  `CompletionCall` で `trace!`/`debug!` event(`call.id`・
  `tool.name`・usage counts — token 数は metadata)
- field 命名は全 crate 共通規約: `worker.name`・`command`・
  `tool.name`・`call.id`・`model.provider`/`model.id`/`model.tier`・
  `error.code`(失敗時)

## Done When

- [x] `cheaper.cli.dispatch`・`cheaper.cli.command`・`cheaper.run`
      + 8 stage 子 span(`tool_runtime`・`model_resolve`・
      `tool_scope`・`session`・`prompt`・`extract`・`repair`・
      `meta`)が配置表通りの命名で配置されている
- [x] 全 span が `info_span!` + `Instrument`(async 区間)で配置
      され、per-site の `Instant`/`elapsed_ms` 記録が存在しない
      (timing は `FmtSpan::CLOSE` の close event に委譲)
- [x] RunEvent 系 event が `call.id`・`tool.name`・usage counts の
      metadata のみを emit し、prompt・tool input/output 本体・
      file path を field に含まない
- [x] `cheaper-runtime/Cargo.toml` に `tracing.workspace` が追加
      され、runtime 外の leaf crate(T4/T5 対象)には本 task で
      dep を追加していない

## Out of Scope

- hook・auth/catalog・core・tools・workflow・provider・adapter の
  instrumentation — T4/T5 の範囲
- file sink 出力・stage timing の contract test — T6 の範囲(本
  task は emit 配置のみ。log OFF の既定経路が無変更 pass である
  ことが gate)
- stdout への log 出力・diagnostics 形式の変更 — 不変条件
- `tracing::instrument` attribute への全面書き換え — design は
  `info_span!` + `Instrument` 方式。attribute 化は本 task では
  行わない

## Verification

- [x] `mise exec -- cargo build -p cheaper-cli -p cheaper-runtime`
      exits 0
- [x] `mise exec -- cargo test -p cheaper-cli -p cheaper-runtime`
      exits 0(log OFF の既定経路で無変更 pass)
- [x] `git grep -n 'info_span!\|tracing::' --
      crates/cheaper-cli/src/{cli,run,workflow_run,models,auth,usage_report}.rs
      crates/cheaper-runtime/src/run/mod.rs` で span 命名が
      `cheaper.` prefix 規約に一致することを確認
- [x] `git grep -n 'elapsed_ms\|Instant' --
      crates/cheaper-runtime/src/run/mod.rs` が timing 二重実装を
      導入していないことを確認(既存の非計測用途を除く)
