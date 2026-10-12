---
id: "TASK-034EOHIeC6KauJaRop87sk"
type: "task"
status: "todo"
title: "T2: shared managed-process 機構の tools/proc 抽出"
created: "2026-10-12"
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
    - "docs/plans/implement-policy-gated-command-tool.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-policy-gated-command-tool.md"
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
    - "docs/test-specs/policy-gated-command-tool.md"
  references: []
  defers: []
  deferred-by: []
---

# T2: shared managed-process 機構の tools/proc 抽出

## Scope

`tools/process/` 配下の managed-subprocess lifecycle 機構を shared module
`tools/proc/` へ抽出し、`process` tool を task 固有 glue + `pub use` re-export へ
縮小する。**挙動変更なしの refactor-only**（AC1: process の contract・error shape・
details field・lifecycle 順序は不変）。T3 の `command` tool が `proc::run`・
shared prepare・executable 解決の移設先に依存するため、T3 着手前に完了必須。

### 所有ファイル

- `crates/cheaper-tools/src/tools/proc/`（新規）— `executable.rs`/`tree.rs`/
  `spawn.rs`/`monitor.rs`/`cleanup.rs` を `tools/process/` から移動・汎化。
  `mod.rs` に `ProcLimits`（`timeout_ms`/`kill_grace_ms`/`max_output_bytes`/
  `heartbeat_interval_ms`）・`ProcPrepared`・`ProcIdentity`・`proc::run`
  （captures 構築と heartbeat spawn 込み — 旧 `process/mod.rs:305-309`・`:379` の
  `options.process` 直接参照を `ProcLimits` 経由へ移すだけに限定）・
  `minimal_env`・cwd prepare・abort helper を集約。
- `crates/cheaper-tools/src/tools/process/{mod.rs,prepare.rs,tests.rs}` — task 固有
  glue に縮小し、移動 item は `pub use` re-export で外部公開 path を維持。
  `pub use` は `pub(super)` の到達範囲を維持できないため、移動 helper
  （`minimal_env`・`display`・`spawn_code`・`spawn_failure_is_async`・
  `StreamCapture`/`Captures`/`Diagnostics`/`read_stream`）は `pub(crate)` へ拡張
  するか、unit test を `tools/proc/` へ同伴移動させる。対象 test 群:
  `capture_finish_*`・`display_*`・`minimal_env_*`・`spawn_code_*`/
  `spawn_failure_*`。`executable.rs` の in-file `#[cfg(test)]` は file ごと移動。
  **test を暗黙に削除・無効化しない（件数不減）**。
- `crates/cheaper-tools/src/tools/mod.rs` — `pub mod proc;`。
  **sequencing note**: 本 file は T1（`pub mod command;`）と共有する唯一の file。
  T1/T2 は並行実装可能だが、後から merge する側が 1 行の trivial conflict を
  解消する（両行を残す）。
- `tools/proc/executable.rs` の error message label — `resolve_process_executable`
  の bare `node` / `unsupported()` の "process task" 固定 wording
  （executable.rs:33-38・:147）を呼出し側 label 引数へ parameterize し、
  command 経路で "process task" と出ないようにする。**process 側の message は
  不変**とする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-policy-gated-command-tool.md` — 実装対象 plan（T2）
- `docs/designs/policy-gated-command-tool.md` — §(e) module 構造・§(k) file map
- `docs/specs/policy-gated-command-tool.md` — AC1（process 不変）
- `docs/test-specs/policy-gated-command-tool.md` — 検証 spec

## Dependencies

- なし（plan のみ。T1 と並行可能。T3 の必須先行条件）

## Steps

1. `tools/proc/` へ `executable.rs`/`tree.rs`/`spawn.rs`/`monitor.rs`/`cleanup.rs`
   を移動し、`mod.rs` に `ProcLimits`/`ProcPrepared`/`ProcIdentity`/`proc::run`/
   `minimal_env`/cwd prepare/abort helper を集約する。
2. `tools/process/` を task 固有 glue に縮小し、移動 item を `pub use` re-export
   で公開 path を維持する。
3. `pub(super)` の到達範囲問題を `pub(crate)` 拡張または unit test の
   `tools/proc/` 同伴移動で解決する（test 削除禁止）。
4. `executable.rs` の error label を呼出し側引数へ parameterize する
   （process 経路の message は不変）。
5. 既存 `proc_process.rs`・process unit test・golden が無変更で pass することを
   確認する（refactor-only の証明）。

## Done When

- [ ] `tools/proc/` に shared lifecycle 機構が集約され、`tools::process` の
  公開 path が `pub use` で維持されている
- [ ] 既存 process test（unit + `proc_process.rs` + golden）が無変更で pass
- [ ] test 件数が減っていない（relocate/widen のみ、暗黙削除なし）
- [ ] `resolve_process_executable` の error label が引数化され、process 経路の
  message が変わっていない

## Verification

- [ ] `mise exec -- cargo test -p cheaper-tools` exits 0
- [ ] `mise exec -- cargo test -p cheaper-tools --test proc_process` exits 0
