---
id: "IMPL-034EPz49PYz6zXgEeh5Fly"
type: "impl"
status: "completed"
title: "shared managed-process machinery extraction into tools proc"
created: "2026-10-12"
updated: "2026-10-13"
owners: []
relations:
  source: []
  changes:
    added:
      - {"type":"code","path":"crates/cheaper-tools/src/tools/proc/mod.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/tools/proc/tests.rs"}
      - {"type":"doc","path":"docs/impl/ir/shared-managed-process-machinery-extraction-into-tools-proc.md"}
    modified:
      - {"type":"code","path":"crates/cheaper-tools/src/tools/mod.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/tools/process/mod.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/tools/process/prepare.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/tools/process/tests.rs"}
      - {"type":"doc","path":"docs/tasks/proc-extraction.md"}
      - {"type":"doc","path":"docs/impl/ir/README.md"}
    deleted: []
    renamed: []
    moved:
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process/executable.rs -> crates/cheaper-tools/src/tools/proc/executable.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process/tree.rs -> crates/cheaper-tools/src/tools/proc/tree.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process/spawn.rs -> crates/cheaper-tools/src/tools/proc/spawn.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process/monitor.rs -> crates/cheaper-tools/src/tools/proc/monitor.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process/cleanup.rs -> crates/cheaper-tools/src/tools/proc/cleanup.rs"
    generated: []
  implements:
    - "docs/tasks/proc-extraction.md"
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

# shared managed-process machinery extraction into tools proc

## Summary

T2。`tools/process/` の managed-subprocess lifecycle 機構を shared
module `tools/proc/` へ抽出し、`process` tool を task 固有 glue
(schema/declaration/Options/tool surface/prepare の manifest・
writeScope 検査)+ `pub use` re-export へ縮小した。generic surface
は `proc::run(prepared, limits, identity, outer, on_timeout)` +
`ProcLimits`/`ProcPrepared`/`ProcIdentity` + `minimal_env` +
`prepare_cwd`(cwd/denylist/is_dir)+ abort helper + `executable`/
`tree` pub module。挙動変更なしの refactor-only — 既存 process
test・`proc_process.rs`・integration suite が全て無変更で pass。

## Context

TASK-034EOHIeC6KauJaRop87sk / PLAN-034EO1kmKwJ8MAHSHWyCws。
issue #101 の policy-gated `command` tool の第 2 task で、
design §(e) の module 抽出を実施する。T3 の `command` tool が
`proc::run`・shared prepare・`resolve_process_executable` の移設先に
依存する必須先行条件。T1(b706188)の `pub mod command` と同一 file
(`tools/mod.rs`)に `pub mod proc` を追加。

## Implementation

### Changes

- `tools/proc/mod.rs`(新規): `pub mod executable`・`pub mod tree` +
  private `mod spawn/monitor/cleanup/tests`。`ProcLimits`
  (`timeout_ms`/`kill_grace_ms`/`max_output_bytes`/
  `heartbeat_interval_ms`、呼出し側 `parse_duration` 済み)・
  `ProcPrepared {executable, argv, cwd, env}`・`ProcIdentity
  {details, label, display, event_prefix}`・`OnTimeout`・
  `find_cheaper_error` を `pub` で集約。`run` は timeout
  controller・captures 構築・`build_command`+spawn・reader task・
  Diagnostics・heartbeat spawn・`drive`・finally を orchestrate し、
  spawn 失敗 details は `identity.details` + `{state, exitCode,
  signal, stdout, stderr, spawnCode}` を従来順で挿入する。
  `aborted_err`/`wait_aborted`/`combined_reason`/`BoxedCause`/
  `minimal_env`/`denied_declared_path`/`prepare_cwd` は `pub(crate)`。
  `pub use monitor::StreamCaptureResult`。
- `tools/proc/spawn.rs`: `build_command` を `(prepared: &ProcPrepared)`
  へ汎化(`&ProcessTask`/`&Prepared` 依存除去、argv/env は
  `ProcPrepared` から取得)。`spawn_code`/`spawn_failure_is_async`/
  `status_parts`/`display` は `pub(super)` のまま。
- `tools/proc/monitor.rs`: `drive` を
  `(limits, identity, outer, inner, timeout_controller, child,
  captures, diagnostics, phase, stdout_task, stderr_task, on_timeout)`
  へ汎化 — `options.process.*` 直接参照を `ProcLimits` 経由へ移し、
  details の `task` field は `identity.details`、display text は
  `identity.display`、on_timeout は引数経由へ。`Diagnostics.task`
  を `label` へ rename(emit key `"task"` は不変)+ `event_prefix`
  field を追加。`StreamCapture`/`Captures`/`StreamKind`/`read_stream`/
  `PHASE_*` は `pub(super)` のまま。
- `tools/proc/cleanup.rs`: ほぼそのまま。`"process.cleanup-failed"`
  literal を `{event_prefix}.cleanup-failed` へ汎化。
- `tools/proc/executable.rs`: `resolve_process_executable` に
  `subject: &str` 引数を追加 — `unsupported()` は
  `"{subject} requires a supported native executable..."`、bare
  `node` は `"{subject} task \"node\" is unsupported..."` を生成。
  `process` 経路は `"process"` を渡して両 message を従来と byte
  identical に維持。in-file `#[cfg(test)]` は file ごと移動し、
  subject parameterize を固定する `error_subject_is_parameterized_by_caller`
  を新規追加。
- `tools/proc/tree.rs`: 変更なしの純粋移動。
- `tools/proc/tests.rs`(新規): `capture_finish_*`(3)・`display_*`(2)・
  `minimal_env_*`(2)・`spawn_code_*`/`spawn_failure_*`(2)の計 9 test
  を `process/tests.rs` から同伴移動 — helper の可視性は
  `pub(super)` のまま維持できるため `pub(crate)` 拡張は不要だった。
- `tools/process/mod.rs`: task 固有 glue に縮小。`pub use
  crate::tools::proc::{executable, tree, find_cheaper_error, OnTimeout,
  StreamCaptureResult}` で公開 path を維持。execute glue は task
  resolve→`prepare`→`ProcLimits`/`ProcPrepared`(`argv =
  task.command[1..]`、`env = minimal_env(&task.env)`)/
  `ProcIdentity({task:name}, label=name, display="process {name}",
  event_prefix="process")` 組み立て→`proc::run` 呼出し。
- `tools/process/prepare.rs`: cwd/denylist/is_dir stat を
  `proc::prepare_cwd(root, cwd, symlink_policy, denylist, "process")`
  へ委譲し、manifest normalize・宣言 path/claim deny・writeScope
  coverage・`resolve_process_executable(command, opts, "process")`
  の task 固有部分を残す。`minimal_env` は proc へ移設。
- `tools/process/tests.rs`: process glue test 6 件を残し、shared
  機構の test 9 件を proc/tests.rs へ移設(削除なし)。import を
  明示化(mod.rs の incidental use への依存を除去)。
- `tools/mod.rs`: `pub mod proc;` 追加(fmt sort 順 — `proc` が
  `process` の prefix のため `proc`→`process` の並び)。

### Decisions During Implementation

- **test 同伴移動を採用**(visibility widening ではなく): 移動した
  helper(`spawn_code`/`spawn_failure_is_async`/`display`/
  `StreamCapture`/`Captures`/`Diagnostics`/`read_stream`/
  `minimal_env`)は `tools/proc/` 内で `pub(super)` のまま sibling
  test(`proc::tests`)から到達できるため、`pub(crate)` 拡張は
  `minimal_env`/`prepare_cwd`/`denied_declared_path`/abort helper
  (`aborted_err`/`wait_aborted`/`combined_reason`/`BoxedCause`)に
  限定した — これらは `tools::process`/`tools::command` の sibling
  module から使う共有 helper で、proc module の公開 API ではない
  ため `pub` にはしなかった。削除 test はゼロ(9 件移設 + 1 件新規
  `error_subject_is_parameterized_by_caller`、unit 総数 247→248)。
- **`ProcIdentity` の構成**: design §(e) の「details へ差し込む
  identity field + heartbeat/diagnostic の label」を
  `{details: Vec<(String,Value)>, label, display, event_prefix}`
  として表現。`details` は spawn 失敗 details・timeout details・
  終了 details の 3 箇所で先頭挿入され、`event_prefix` で
  `"process.heartbeat"`/`"process.cleanup-failed"` の event 名を
  tool 側が選べる。process は `{task:name}`/"process" を渡し従来
  出力と同一。
- **`subject` 引数は単一 label**: `resolve_process_executable` の
  2 message は `{subject} requires a supported native executable`
  と `{subject} task "{cmd}" is unsupported` を共有 label で生成し、
  process は `"process"` を渡して両方を byte identical に維持。
  command 経路は interpreter deny が `node` を先に拒否するため
  bare-node 分岐に実質到達しない(defense-in-depth)。
- **`prepare_cwd`/`denied_declared_path` も `noun` 引数化**:
  `"process cwd must be an existing directory"` と
  `"Access denied for declared process path"` の `"process"` noun を
  呼出し側 label で置換可能にし、executable.rs の subject
  parameterize と同じ粒度で grant 文脈の wording 混入を防ぐ
  (process は `"process"` で不変)。
- **parse_duration を glue 側へ移動**: `ProcLimits` が ms を取るため
  process glue が timeout/killGrace/heartbeat.interval を spawn 前に
  parse する(旧 code は heartbeat を spawn 後・timeout/killGrace を
  drive 内で parse)。`resolve_process_policy` の `bounded_duration`
  が policy resolve 時に検証済みで到達不能のため、error 順序の
  observable な差分はない。
- **abort pre-check は `outer.reason()` で簡約**: 旧 code は fresh
  `timeout_controller.signal()` との `combined_reason` を resolve/
  prepare の前後に check していたが、inner signal は spawn 前に
  発火し得ないため outer のみと等価。`proc::run` 先頭では controller
  生成直後の `combined_reason` check を維持。

## Related Experiments

### Adopted

なし。design §(e)・task doc が確定済みで追加 experiment は起こさなかった。

### Rejected

- `command` から `tools::process` の private 内部へ `pub(crate)` で
  依存する代替案 — design §(e) が責務境界の曖昧化として却下済み。
- `pub use` 維持のため moved helper を一律 `pub(crate)` 化する案 —
  test 同伴移動で `pub(super)` のまま済むため採らなかった。

## Validation

- `mise exec -- cargo test -p cheaper-tools` — 全 target pass。
  unit 248/248(proc::tests 9 件移設 + proc::executable 7 件移設
  + 新規 1 件、process::tests 6 件残存)、`proc_process.rs` 9/9、
  `proc_invocation` 10/10、`proc_runtime` 7/7、`characterization`
  10/10、`differential` 11/11 — 全て無変更で pass(refactor 証明)。
- `mise exec -- cargo test -p cheaper-tools --test proc_process` —
  上記 suite に含まれ exit 0(9/9 pass)。
- `mise exec -- cargo build --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` — exit 0。
- `mise exec -- cargo fmt --all -- --check` — clean。
- `mise exec -- cargo clippy --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` —
  exit 0。

## Risks

- `tools::process::{executable,tree}` は `pub use` の module
  re-export で維持されるが、外部 code はこれらの path を現状参照して
  いない(grep 確認済み — `find_cheaper_error`/`OnTimeout`/
  `ProcessToolOptions`/schema/declaration のみが外部参照)。
- `ProcIdentity.details` の順序は挿入順 — spawn 失敗/timeout/終了
  details で identity field が先頭に来る順序は process では従来と
  同一(`task` が常に先頭)。command の `{grant, argv}` + `durationMs`
  拡張は T3 が `ProcIdentity` または details 組み立てで扱う。
- `pub mod proc` は crate 外からも到達可能 — `run`/`ProcLimits` 等の
  generic surface は `pub`(design §(e) の共有基盤)、内部 helper は
  `pub(crate)`/`pub(super)` で閉じている。

## Follow-ups

- T3: `tools/command/` の deny-stub を `proc::run` 経由の実装へ
  差し替え — grant resolve + argv match + metachar + cwd prepare +
  pin 再検証、`{grant, argv}` identity、per-grant limit override を
  `ProcLimits` 経由で流す。catalog `NAMES` 追加・registry gate・
  `is_process`→`is_managed` 一般化。
- T4+: invocation `inner.command` 再注入・builtin policy 配線・
  test/docs。
