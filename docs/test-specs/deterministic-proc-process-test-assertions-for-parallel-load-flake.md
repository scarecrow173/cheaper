---
id: "TSPEC-034LIvllLcelrOMU0brubs"
type: "test-spec"
status: "draft"
title: "Deterministic proc_process test assertions for parallel-load flake"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/180"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/deterministic-proc-process-tests-issue-180.md"
    - "docs/designs/deterministic-proc-process-tests-issue-180.md"
    - "docs/adr/resolve-proc-process-test-flake-at-the-test-boundary.md"
  derived-by: []
  verifies:
    - "docs/specs/deterministic-proc-process-tests-issue-180.md"
    - "docs/designs/deterministic-proc-process-tests-issue-180.md"
  verified-by:
    - "docs/tasks/implement-pids-d-emit-protocol-and-test-helper-layer.md"
    - "docs/tasks/verify-proc-process-determinism-and-mise-check.md"
  references:
    - "docs/discovery/proc-process-test-flake-under-parallel-load.md"
  defers: []
  deferred-by: []
---

# Deterministic proc_process test assertions for parallel-load flake

## Purpose

issue #180 で観測された process-tree 系 flaky test(2026-10-23、Windows
local、full-suite 並列負荷で transient fail)が、「test の assertion が
制御下にない wall-clock/ambient 状態(3 段 spawn chain 準備・helper
subprocess 連鎖・poll spawn 自己負荷)を timing budget 内と仮定して観測
する」という invariant 違反を二度と持ち込まないことを pin する。本 test
spec は、将来 `crates/cheaper-tools/tests/proc_process.rs` の fixture・
helper・assertion を書き換える・削除する変更が determinism の意図を
維持しているかを判定するための契約。

対象の本番実装(`proc::run`/`drive`/`monitor`/`cleanup`/`tree`)が変わって
assertion 形状の再設計が必要になった場合、または diagnostic が tool 側
帰属を確定して conditional phase-2 の tool 側 hardening が走った場合に
廃止・supersede を検討する。

## Feature

process-tree 系 test の lifecycle 検証(mode 別 outcome・descendant 消滅・
ambient PATH 解決)は、wall-clock 負荷・helper subprocess 遅延・poll spawn
自己負荷に関わらず platform 間で同じ結果を返す — pid 観測は file
artifact(atomic per-key emit)、resolution は task の実結果、kill は同期
OS call、failure は自己説明的 diagnostic で表す。

## Rules

- fixture の pid 通知は `pids.d/{parent,child,grandchild}` の incremental
  per-key atomic emit(tmp+rename)とし、owner process が pid を知った時点
  で書く。単一完備 write(`pids.json`)や exists→parse の retry 経路を
  持たない — TOCTOU race を構造的に消す。
- task `timeout` は timeout を期待しない mode(exit/fail/abort)では
  fixture の観測可能な全 lifetime(spawn chain → per-key emit → exit/park
  → 解決)を上回る値とし、timeout 期待 mode(deep-stubborn `2s`・sibling
  `500ms`)のみ kill 検証に必要な最小値を使う。
- non-timeout mode で `Err` code=Timeout が返った場合、発火位置を問わず
  legitimate outcome として受理する(「期待 outcome OR Timeout」の二択)。
  受理しても emit 済み全 pid の消滅検証は必ず行う — 検証対象の
  lifecycle invariant を弱めない。
- emit floor は mode 依存で強制する: 自然解決(Ok/ToolExecutionFailure)
  は **全 expected key の emit を要求**する(deep mode の自然解決なら
  `{parent,child,grandchild}` 全 3 件が post-`collect_emitted` grace 後に
  揃う — 自然解決した fixture は全 emit を終えている)。60s-armed mode
  (exit-tree/fail-tree/deep-tree — timeout 値ではなく mode identity で
  指定)の Timeout 解決は emit ≥1 を要求し、0-emit は broken
  fixture/plumbing として失敗。0-emit Timeout の受理は genuine
  pre-emit kill window の残る 2s/500ms mode のみ。
- `fail-tree` の受理は `code==ToolExecutionFailure && exitCode==7` に加え
  `message=="managed process failed"` を要求する — cleanup-failed arm
  が同一 details を運ぶため code+exitCode だけでは区別できない。
- Windows liveness は unfiltered `tasklist /NH /FO CSV` を poll ごとに
  1 回だけ spawn して client 側集合照合する。`tasklist /FI` filter の
  複数併記は AND 評価で常に空を返すため禁止(vacuous-pass 化)。
  nonconforming output・helper 失敗は conservative に全員 alive 扱い。
- unix `graceful` mode は `parent.armed` marker で handler install 済みを
  区別し、kill-before-arming は受理、armed なのに `.terminated` が出ない
  ケースは real failure とする。
- wait は非 panic の `wait_until` に統一する(panic 版 `wait_for`・
  `wait_emitted` は作らない)。wait/verify の失敗は必ず
  `dump_failure` → `diag_fail` を通り、panic message は
  `[{mode}] ... timed out after {budget:?} (poll {poll:?}): {what}`
  shape の context を含む — `"{key} gone"` 型の uninterpolated
  literal を残さない。`dump_failure` が panic に先行する順序を守る
  (dump 未出力の裸 panic を残さない)。failure diagnostics は error
  `source()` chain・`find_cheaper_error` details JSON・mode・elapsed・
  emit 状態を stderr 採取し、process-tree 系 test の全 failure 経路
  (panic 経路を含む)で必ず呼ぶ。
- wait の poll cadence は file-wait 20ms / pid-death 100ms に pin する
  (R5 self-load 削減 — tasklist spawn は待機あたり最大 ~10/s)。
- panic mid-`drive()` の fixture leak は containment 必須 —
  `TaskRun`(canonical execute seam の 'static future `tokio::spawn` +
  drop-guard 同期 `kill_emitted`)と fixture 側 `SpawnedChild`。
  `TaskRun::Drop` は `handle.abort()` を呼ばない(JoinHandle abort は
  future を次 yield で drop するだけで production cleanup は走らず
  leak を悪化させる)。runtime 生存中の正式 give-up 経路は
  `abort_and_drain`。
- emit 責務を持つ mode で `PID_DIR_ENV`(`CHEAPER_PROC_PID_DIR`)が
  未設定なら fixture は mode 名入り panic で fail-fast する(emit は
  test 観測の基盤 — env 未設定の silent skip を許さない)。

## Examples

- deep mode `exit-tree`/`fail-tree`/`deep-stubborn` で parent が entry で
  `pids.d/parent` を emit、branch が `pids.d/child`・`pids.d/grandchild`
  を spawn 次第 emit → test は per-key file の parseable 到着を待ち、
  早期 `Timeout` 後も emit 済み全件の消滅を検証する — full-suite 並列
  負荷下で fixture 準備が遅れても pass。
- abort mode(deep-tree)で emit 待ち `wait_until` と `&mut handle` を
  `select!` — 全 emit 到着で `abort("outer tree cancellation")` →
  `err.to_string() == "outer tree cancellation"` + 3 descendant 消滅。
  emit stall は `abort_and_drain` → `diag_fail`(unwind させない)。
- `times_out_native_process_and_cleans_tree`(500ms timeout): fixture が
  emit 前に kill されたケースは `Err` code=Timeout + 0-emit 受理、emit
  済みならその pid の消滅を検証 — `:543` の context-free
  `read_to_string().unwrap()` 経路は残さない。
- unix `allows_sigterm_exit_within_grace_before_force_kill`: `parent.armed`
  が無ければ kill-before-arming で受理、`.armed` 有で
  `parent.terminated`=="graceful" が無ければ real failure — grace `10s`
  の分離で SIGTERM→exit が観測される。
- `node_task_resolves_via_path` の Err 経路で `diag_fail` が
  exitCode/state/error chain を採取 — 次回 exit 255 再発時に
  `"managed process cleanup failed"`(tool 側 helper 帰属)と node 起因を
  output から判別できる。

## Guarantees

- 対象 test は ambient 負荷・3 段 spawn chain 遅延・`WinTree::terminate`
  helper subprocess 連鎖・poll spawn 自己負荷に関わらず deterministic で
  ある。
- 検証対象の lifecycle invariant(mode 別 outcome・emit 済み descendant
  の消滅・ambient PATH 解決)は早期 `Timeout` 受理を含めて弱めない —
  emit 済み pid は必ず全件検証される。
- failure 時の diagnostic 出力は error chain・details・mode・elapsed・
  emit 状態を含み、tool 側帰属(`"process tree helper failed"` 系)と
  test 側帰属を再発時に self-explaining に区別できる。
- panic 経路を含む全 failure 経路で **emit 済み・spawn 済みの**
  managed process が containment(`TaskRun`/`abort_and_drain`/
  `SpawnedChild`)される — Windows の `remove_dir_all` 失敗を誘発する
  parked fixture leak を防ぐ。parent 未 emit(~ms window)での panic は
  ≤1 process(parent 単独)が残りうる accepted residual とする
  (design D6 Risks — emit-first 配置で window は構造的に最小化済み)。

## Non-goals

- `crates/cheaper-tools/src/tools/proc/` 本番実装の正しさや変更 —
  `WinTree::terminate` helper 連鎖・eager CIM snapshot・taskkill 処理は
  diagnostic が tool 側帰属を示した場合の conditional phase-2(別 issue)。
- ambient `node` PATH 解決自体の変更 — `node_task_resolves_via_path` の
  テスト意図は維持。
- test 側 `live_pids`/`kill_emitted` の creation-time guard(PID-reuse
  照合)— `tasklist` に安価な born-time oracle が無く、accepted residual
  として残余受容。同 image で再利用された pid は killable のまま。
- `executes_literal_argv_...`・`fails_nonzero_exit_...`・
  `caps_combined_output_...`・`touch_with_matching_write_scope_...` の
  assertion 意味 — park しない短命 fixture で race/leak 経路を持たない。
- 他 test file・CI 構成(runs-on・retry・OS matrix)の determinism 監査。

## Risk

- この保証が落ちると、assertion が再び timing/ambient 状態を観測して
  full-suite 並列負荷下で断続失敗し、`mise run check` の verification
  信号が毀損される — 「本物の regression か flake か」の判別コストを
  全 PR が負う。
- 早期 `Timeout` 受理を「検証 skip」と誤解して emit 済み pid の消滅検証
  まで緩めると、早期 kill が descendant を残す regression が test を
  通り抜ける。
- `tasklist /FI` 複数併記を再導入すると `alive()`/`live_pids` が常に
  空を返し「何も検証せず pass」になる — 検証が vacuous 化する。
- diagnostic helper を落とすと、exit 255 の帰属判別手段を失い次回再発
  時に tool 側 phase-2 の正当化根拠を持てない。
