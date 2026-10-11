---
id: "TASK-034DVHnoYPCyYkFMFU2pZw"
type: "task"
status: "done"
title: "T5: instrumentation wave 3 — hook/auth/catalog/core"
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
    - "docs/tasks/instrument-cli-dispatch-and-run-pipeline.md"
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

# T5: instrumentation wave 3 — hook/auth/catalog/core

## Work

plan `docs/plans/implement-observability-foundation.md`
(PLAN-034DVGKbl3V2g7msSfpXv0)の T5。design の instrumentation
配置表の残り — hook 経路(`cheaper-cli/src/hook/`)、
auth/catalog I/O boundary(`cheaper-runtime`)、core の
workspace/policy/model resolve 境界(`cheaper-core`)— へ
`tracing` span/event を配置する。credential・file path 非記録の
metadata-only 規約が最も効く wave で、`file.name`(basename のみ)
等の代替分類値を使う。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-observability-foundation.md` — 本 task が
  implement する plan。File Map「T5」が作業の正
- `docs/designs/observability-foundation-logging-and-instrumentation-wiring.md`
  (DESIGN-034DV9ypzmxdAobGSJ6Cm6)— instrumentation 配置表・
  metadata-only 規約(`file.name` basename 規約を含む)の
  AUTHORITATIVE 定義
- `docs/adr/logging-output-and-activation-contract.md` — hook 経路
  stderr 不干渉の contract 根拠

## Dependencies

- `docs/tasks/instrument-cli-dispatch-and-run-pipeline.md`(T3)—
  `cheaper-runtime/Cargo.toml` の `tracing` dep 追加は T3 が
  一元で担う(T5 は runtime の auth/catalog file のみ触り
  Cargo.toml には触れない)
- `docs/tasks/observability-contract-tests-and-verification.md`
  (T6)を block する

## Affected Files

- `crates/cheaper-cli/src/hook/mod.rs` +
  `crates/cheaper-cli/src/hook/targets.rs` — `cheaper.hook` span
  (field `event`・`target`)+ 完了 `debug!` event(`verdict`・
  `stdout.len`/`stderr.len`)。`execute_hook_inner`→`handle_hook`
  境界に配置。`emit` の stderr 書出し自体は instrument しない
- `crates/cheaper-runtime/src/auth/store.rs` —
  `cheaper.auth.store` event(load/save・`provider`・`file.name`)。
  **credential 値は一切出さない。`file.name` は basename のみ**
  (`credentials.json`)— full `path` は `<userConfigDir>` 経由で
  OS username を含み得るため allowlist 外とし、識別不要な分類値で
  代替する
- `crates/cheaper-runtime/src/auth/service.rs` — `cheaper.auth`
  span(`provider`・`auth.type`)。login/logout/status の op 単位
- `crates/cheaper-runtime/src/catalog/mod.rs` —
  `cheaper.catalog` event(merge/persist・`provider.count`・
  `error.count`)。bundled/persisted/user overlay の境界
- `crates/cheaper-core/Cargo.toml` — `tracing.workspace` を
  `[dependencies]` に追加。**core は emit のみ** —
  `cheaper-observability` への依存・tokio/rig の流入なし
  (`tracing` は runtime-agnostic で不変条件を壊さない)
- `crates/cheaper-core/src/worker/resolve.rs`・
  `src/policy/load.rs`・`src/model/config.rs`・
  `src/model/resolve.rs` — `debug!`/`trace!` event(resolved
  name/choice 等の分類値)。file path は field に載せない。
  同期純ロジックのまま — async 化・I/O 追加はしない

## Done When

- [x] `cheaper.hook` span(`event`・`target`)+ 完了 event
      (`verdict`・len 系)が hook 境界に配置され、stderr emit 経路
      自体は instrument されていない
- [x] `cheaper.auth.store`・`cheaper.auth`・`cheaper.catalog` の
      event/span が配置表通りで、credential 値・full file path を
      field に含まない(`file.name` は basename のみ)
- [x] `cheaper-core` の 4 file に `debug!`/`trace!` event が配置
      され、同期純ロジック・rig-free/tokio-free が維持されている
- [x] `cheaper-runtime/Cargo.toml`・cli hook file 以外に本 task が
      触る file が T3/T2 と重ならない(runtime Cargo.toml は T3
      管轄のまま無変更)

## Out of Scope

- hook contract 自体の変更(stderr_enabled・emit・exit code)—
  T2 の activation 配線 + 不変条件の範囲
- `cheaper-runtime`・`cheaper-cli` の `Cargo.toml` 変更 —
  `tracing` dep は T3(runtime)・T2(cli)が追加済み
- `.cheaper/logs` への hook event 出力確認 — T6 contract test の範囲

## Verification

- [x] `mise exec -- cargo build -p cheaper-cli -p cheaper-runtime
      -p cheaper-core` exits 0
- [x] `mise exec -- cargo test -p cheaper-cli -p cheaper-runtime
      -p cheaper-core` exits 0(log OFF の既定経路で無変更 pass)
- [x] `mise exec -- cargo tree -p cheaper-core` に `tokio`/`rig`/
      `cheaper-observability` が現れない(core 不変条件)
- [x] `git grep -n 'path\b' --
      crates/cheaper-runtime/src/auth/{store,service}.rs
      crates/cheaper-runtime/src/catalog/mod.rs
      crates/cheaper-core/src/{worker/resolve,policy/load,model/config,model/resolve}.rs`
      で tracing field に full path を渡していないことを確認
      (basename `file.name` のみ)
