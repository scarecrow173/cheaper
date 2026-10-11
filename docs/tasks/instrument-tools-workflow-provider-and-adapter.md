---
id: "TASK-034DVHfUoazX4qqAdssXAr"
type: "task"
status: "todo"
title: "T4: instrumentation wave 2 — tools/workflow/provider/adapter"
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
    - "docs/tasks/create-cheaper-observability-crate.md"
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

# T4: instrumentation wave 2 — tools/workflow/provider/adapter

## Work

plan `docs/plans/implement-observability-foundation.md`
(PLAN-034DVGKbl3V2g7msSfpXv0)の T4。design の instrumentation
配置表のうち、tool 実行境界(`cheaper-tools`)・workflow engine・
devin provider・rig-adapter へ `tracing` span/event を配置する。
各 crate には `tracing.workspace` のみ追加し、
`cheaper-observability` には依存しない(emit は subscriber の
有無を意識しない — ADR-034DUFzrXJxOwfl1qveXWR の pattern)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-observability-foundation.md` — 本 task が
  implement する plan。File Map「T4」が作業の正
- `docs/designs/observability-foundation-logging-and-instrumentation-wiring.md`
  (DESIGN-034DV9ypzmxdAobGSJ6Cm6)— instrumentation 配置表・
  span/field canonical naming・metadata-only 規約の AUTHORITATIVE
  定義

## Dependencies

- `docs/tasks/create-cheaper-observability-crate.md`(T1)—
  workspace `tracing` pin と crate 構成の前提。触る file は
  T2/T3/T5 と disjoint のため T1 後は並列可能
- `docs/tasks/observability-contract-tests-and-verification.md`
  (T6)を block する

## Affected Files

- `crates/cheaper-tools/Cargo.toml` — `tracing.workspace`
- `crates/cheaper-tools/src/registry.rs`(`ToolRuntime::execute`)—
  `cheaper.tool` span(field `tool.name`)+ 結果 `debug!` event
  (`result.ok`/`error.code`)。**全 tool call の単一の正
  instrumentation 点** — per-tool の動的 span 名は `span!` の
  `'static` 制約と相性が悪いため `cheaper.tool` 単一 span +
  `tool.name` field を canonical とし、個別 tool file には散らさない
- `crates/cheaper-tools/src/tools/process/`(mod/executable/tree)+
  `src/tools/grep_eval.rs` — `cheaper.tool.process` event
  (spawn/exit・`task`・`exit.code`)。subprocess 境界のみ。
  `__grep-eval` spawn も同様の event(内部 subprocess で、親側が
  spawn を記録する)
- `crates/cheaper-workflow/Cargo.toml` — `tracing.workspace`
- `crates/cheaper-workflow/src/engine.rs` — `cheaper.workflow`
  span(`run.id`)+ node `debug!` event(node id/name)。
  engine の job 実行境界
- `crates/cheaper-provider/Cargo.toml` — `tracing.workspace`
- `crates/cheaper-provider/src/devin/credentials.rs`・
  `src/devin/cli.rs`・`src/devin/rig_provider.rs`・
  `src/devin/stream.rs` — `cheaper.provider.devin` event
  (credential load ok/miss・`devin models list` 起動/終了・
  stream connect)。**JWT・token・credential 値は出さない**
- `crates/cheaper-rig-adapter/Cargo.toml` — `tracing.workspace`
- `crates/cheaper-rig-adapter/src/run_stream.rs`
  (`map_item`/`unfold`)— RunEvent kind 毎の `debug!`/`trace!`
  event(`call.id`・`tool.name`・`turn`・completion usage
  counts)。`TextDelta` 本文は出さず `text.len` のみ
- `crates/cheaper-rig-adapter/src/agent.rs` —
  `cheaper.adapter.session` span(session build)。
  model/auth 値は出さない

## Done When

- [ ] `ToolRuntime::execute` が `cheaper.tool` span +
      `tool.name` field の単一 instrumentation 点になり、個別
      tool file に tracing emit が散らばっていない
- [ ] `cheaper.tool.process`・`cheaper.workflow`・
      `cheaper.provider.devin`・`cheaper.adapter.session` +
      `run_stream` の RunEvent event が配置表通りの命名で配置
      されている
- [ ] credential・JWT・token・model/auth 値・`TextDelta` 本文・
      file path が一切 field 値に含まれない(metadata-only:
      `*.len`・`*.count`・id・分類値のみ)
- [ ] 4 crate(tools・workflow・provider・rig-adapter)の
      `Cargo.toml` に `tracing.workspace` が追加され、いずれも
      `cheaper-observability` に依存していない

## Out of Scope

- cli dispatch・run pipeline・hook・auth/catalog・core の
  instrumentation — T3/T5 の範囲
- tool 実行内容の payload logging(opt-in raw capture)— spec で
  deferred、本フェーズは構築しない
- subprocess の親側 tracing 以外のライフサイクル変更 —
  instrumentation のみ、process 制御は不変

## Verification

- [ ] `mise exec -- cargo build -p cheaper-tools -p cheaper-workflow
      -p cheaper-provider -p cheaper-rig-adapter` exits 0
- [ ] `mise exec -- cargo test -p cheaper-tools -p cheaper-workflow
      -p cheaper-provider -p cheaper-rig-adapter` exits 0
- [ ] `git grep -n 'tracing' -- crates/cheaper-tools/src/tools/
      | grep -v 'tools/process\|grep_eval'` で個別 tool file への
      emit 分散がないことを確認
- [ ] `git grep -n 'cheaper\.' -- crates/cheaper-tools/src
      crates/cheaper-workflow/src crates/cheaper-provider/src
      crates/cheaper-rig-adapter/src` で span 命名が canonical
      規約に一致することを確認
