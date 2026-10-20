---
id: "IMPL-034JimbubuiHpBv6IpimHs"
type: "impl"
status: "completed"
title: "Add proc full-output tee sink for route logging"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-tools/src/tools/proc/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/proc/monitor.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/command/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/registry.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/invocation.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_command.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-proc-output-tee.md"
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

# Add proc full-output tee sink for route logging

## Summary

`proc::run` に optional `ProcTee`(`Arc<dyn Fn(&[u8]) + Send +
Sync>`)byte sink を追加し、bounded output capture とは別に
stdout/stderr 全量を chunk 到着順(interleaved)で sink へ流す。
`CommandToolOptions.log_tee` と `ToolRuntimeOptions.log_tee` を
新設し、scope 構築層から command tool へ注入する plumbing を
通した。

## Context

Task `add-proc-output-tee`(plan Phase A2、design §proc tee、
spec R26/R29)。route log は `maxOutputBytes` cap と独立の全量
capture を要求するため、stream 読み取り層に sink seam を追加する。
sink 内の write 失敗は closure が自身へ記録して postprocess
記録へ縮退し、capture を失敗させない(R29)。

## Implementation

### Changes

- `ProcTee` type alias を proc/mod.rs に定義。
- `Captures` に `tee: Option<ProcTee>` を追加し `append` 先頭で
  chunk 全量を sink へ渡す — bounded `remaining` 消費の前なので
  cap 超過分も届く。lock 下で呼ぶため stream 交差順が保たれる。
- `proc::run` signature に `tee: Option<ProcTee>` を追加。
- `CommandToolOptions.log_tee` → `proc::run` へ透過。
- `ToolRuntimeOptions.log_tee` → `ToolCtx.log_tee` →
  `ctx.command()` 経由で command tool へ注入。
  `create_worker_scope` の runtime mapping も pass-through。
- 既存 caller(process tool・全 `ToolRuntimeOptions` literal)は
  `None` で不変。

### Decisions During Implementation

- sink 失敗の記録は closure 内(実 sink は `RouteLogWriter`)—
  `Fn(&[u8])` は error を返さず、design の「sink 内で記録し
  postprocess へ縮退」の責務分離を維持する。
- registry の `ctx.command()` は `self.log_tee` を透過するだけ
  — tee 所持条件の判定は scope 構築層(D1)の責務。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `cargo test -p cheaper-tools --test proc_command` — 29 pass
  (新規 2: `log_tee_captures_full_output_beyond_cap`・
  `log_tee_sink_failure_does_not_break_result`)。
- `cargo check --workspace --tests` — clean。
- `cargo clippy -p cheaper-tools --all-targets` — warning なし。

## Risks

- tee closure が panic すると captures Mutex が poison される —
  sink 実装(`RouteLogWriter` 経由の write)は panic しない前提。
  design の責務分離に従い proc 層では catch_unwind しない。

## Follow-ups

- D1 `add-route-run-delegated-exec` が `RouteLogStore::begin` した
  writer を sink として `runtime.log_tee` へ注入する。
