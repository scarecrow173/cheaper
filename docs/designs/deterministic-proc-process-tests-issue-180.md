---
id: "DESIGN-034LGDPeqMOtv5azFEm4Ei"
type: "design"
status: "approved"
title: "Deterministic proc_process tests for parallel-load flake (issue #180)"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/180"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_process.rs"
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
    - "deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/deterministic-proc-process-tests-issue-180.md"
    - "../adr/resolve-proc-process-test-flake-at-the-test-boundary.md"
    - "../discovery/proc-process-test-flake-under-parallel-load.md"
  derived-by:
    - "../plans/fix-proc-process-test-flake-via-test-side-determinism-issue-180.md"
    - "../test-specs/deterministic-proc-process-test-assertions-for-parallel-load-flake.md"
  verifies: []
  verified-by:
    - "../test-specs/deterministic-proc-process-test-assertions-for-parallel-load-flake.md"
  references: []
  defers: []
  deferred-by: []
---

# Deterministic proc_process tests for parallel-load flake (issue #180)

## Context

issue #180: `crates/cheaper-tools/tests/proc_process.rs` の process-tree 系
test が full-suite 並列負荷下で 2 件の transient fail を記録した(2026-10-23、
Windows local)。discovery(DISC-034LFaxMQ43rlWH4Tn8KS5)と
ADR-034LFaJmc8htAt3pDMjMQ2 で確定済みの構造:

- **R1(confirmed 構造 race)**: `cleans_live_descendants_on_exit_and_failure_
  and_abort_and_timeout`(:588)は全 mode 共通で task `timeout: "2s"` を
  使うが、timeout clock は `drive()` 冒頭の `sleep(timeout_ms)`(monitor.rs:225)
  で `command.spawn()` 成功直後に arm される(proc/mod.rs:328→425)。つまり
  2s budget は fixture の **観測可能な全 lifetime**(test binary re-exec × 3
  段の spawn chain → `ready`/`@@` 行 → `pids.json` write → exit/park)に乗る。
  発火位置で panic 形が変わる(died-early・未作成 file への
  `read_to_string().unwrap()`・wrong-error)。load があるほど発火しやすい
  **構造的** race で、確率問題ではない。
- **同一 race を持つ sibling test**: `times_out_native_process_and_cleans_tree`
  (:529-550、`timeout: "500ms"` + `:543` の `read_to_string().unwrap()`)と
  unix のみ `allows_sigterm_exit_within_grace_before_force_kill`(:692-717、
  `timeout: "500ms"` + `.terminated` marker 待ち)は、timeout が fixture の
  emit/handler-install を待たずに発火しうる同一構造。
- **R2-R3(競合・直列する負荷)**: `wait_for(pids.json, 15s)` は spawn chain +
  Windows eager CIM snapshot(`WinTree::new` が spawn 直後に powershell を
  background 起動、tree.rs:434-438)と競合し、`wait_for(!alive, 20s)` は
  `WinTree::terminate` の snapshot/taskkill 連鎖(:465-499、典型 ~4-5 回の
  powershell spawn)と直列する。
- **R5(self-load)**: `alive()`(:371-380)は poll ごとに
  `tasklist /FI "PID eq X"` subprocess を同期 spawn — 20ms 周期で最大
  ~50 spawn/s/待機を ambient 負荷へ加える。
- **panic 経路の context 欠如**: `wait_for` panic は `what` のみで mode を
  含まず、pid 消滅待ちの `"{key} gone"`(:582/:653)は `format!` を通らない
  **interpolation されない literal**。pid file read(:573/:625)、`alive()`
  の `.output()`(:376)、`spawn_fixture` の `.spawn()`(:252)も
  context-free `unwrap()`。
- **(d) panic mid-`drive()` leak**: test 側 panic で `pending` を drop しても
  managed process は kill されず(tokio Child に kill_on_drop 無し)、park 中の
  fixture が最大 3 process 残留 + TempDir の `remove_dir_all` が Windows で
  失敗する。
- **R4(未確定)**: `node_task_resolves_via_path`(:448)の exit 255 は
  Windows tree helper 失敗(`"process tree helper failed"` →
  `"managed process cleanup failed"`、monitor.rs:319-327)が最有力だが
  diagnostic 未採取で未確定。

違反している invariant は「test の assertion が制御下にない wall-clock/
ambient 状態(3 段 spawn 準備・helper subprocess 連鎖)を timing budget 内と
仮定して観測する」こと。責務境界は test 側にあり(ADR Option A)、本 design は
`proc_process.rs` 単一 file 内の test 側 determinism 再設計を定義する。
本番 `src/tools/proc/` の変更は一切行わない。

## Scope

### In Scope

- `proc_fixture_child` fixture(test コード — spec で変更対象と明記):
  - `pids.d/{parent,child,grandchild}` の incremental per-key emit
    (atomic tmp+rename)。deep mode は parent が entry で自身を emit、
    branch が child(自身)と grandchild(`leaf.id()`)を emit。`tree` mode
    も同一 protocol へ統一(parent+child の 2 key)。
  - unix `graceful` mode に `pids.d/parent.armed` marker(handler install
    直後)を追加。`stubborn`/`graceful` の `pids.d/parent` file、`hold`
    以外の全 emit を atomic write へ。
  - `spawn_fixture` の子に kill-on-drop guard(`SpawnedChild`)。
- `cleans_live_descendants_on_exit_and_failure_and_abort_and_timeout`(:588):
  mode 別 task timeout・早期 `Timeout` の受理・bounded post-resolution emit
  回収・emit 済み全 pid の消滅検証・`TaskRun` guard。
- sibling test `times_out_native_process_and_cleans_tree`(:529-550)・
  `allows_sigterm_exit_within_grace_before_force_kill`(:692-717、unix)・
  `abort_kills_managed_parent_and_descendant_and_preserves_reason`(:552-585)
  への同一構造適用。
- `wait_for`/`alive` helper 全体: `WaitCtx` context 付き `wait_until`
  統一(panic 版 wait は作らず全失敗経路を `diag_fail` へ集約)・
  unfiltered `tasklist /NH /FO CSV` snapshot の client 側照合
  (`live_pids` へ統合し `alive` wrapper は廃止)・poll cadence 緩和・
  context-carrying `expect` 化。
- panic-path leak containment: `TaskRun`(JoinHandle + drop-guard)・
  `abort_and_drain` give-up path・fixture 側 `SpawnedChild`。
- failure diagnostics helper(`dump_failure`/`diag_fail`): error `source()`
  chain・`find_cheaper_error` の details JSON・mode・elapsed・emit 状態を
  stderr 採取してから panic する経路を process-tree 系 test の failure 経路
  全体に適用(`node_task_resolves_via_path` :448 を含む)。

### Out of Scope

- `crates/cheaper-tools/src/tools/proc/`(`run`/`drive`/`monitor`/`cleanup`/
  `tree`/`spawn`/`executable`)と `tools/exec` の本番実装変更 — tool 側
  hardening は diagnostic で tool 側帰属が確認された場合の conditional
  phase-2(別 issue)。
- ambient `node` PATH 解決自体の変更(`node_task_resolves_via_path` の
  テスト意図は維持)。
- 他 test file・CI 構成(runs-on・retry・OS matrix)の変更。
- 新規依存追加(std + `libc` + `tokio` の既存依存のみ)。
- test 側 `live_pids`/`kill_emitted` に creation-time guard を持ち込むこと
  (spec Residual (c)— `tasklist` に安価な born-time oracle が無く、
  accepted residual として残余受容。documented, not fixed)。
- `executes_literal_argv_...`・`fails_nonzero_exit_...`・
  `caps_combined_output_...`・`touch_with_matching_write_scope_...` の
  assertion 意味の変更(park しない fixture で leak/race 経路を持たない
  ため変更しない)。

## Components and Boundaries

全て `crates/cheaper-tools/tests/proc_process.rs` 内の test-side 構成要素。

### Fixture 側(test binary re-exec 内で動く)

- **`emit_pid(dir, key, pid)` / `emit_marker(path, text)`**: `{dir}/{key}.tmp`
  (または `{path}.tmp-<pid>`)へ write → `fs::rename` の atomic emit。
  各 key は owner process が 1 回だけ書く(overwrite なしで rename 可能)。
  `dir` は初回 emit 時に `create_dir_all`。
- **`PID_DIR_ENV`(`CHEAPER_PROC_PID_DIR`)が pids.d の唯一の伝達路**:
  `fixture_task` が `env[PID_DIR_ENV]` を全 tree-family mode
  (`stubborn`/`graceful` 含む)で宣言し、fixture は `env::var` で読む
  (deep/tree mode の argv pid path 契約は廃止)。`spawn_fixture` の env
  継承(`Command` 既定で parent env を引き継ぐ)で branch/child
  descendant にも同じ値が伝播する。emit 責務を持つ mode で env 未設定
  なら **mode 名入り panic**(fail-fast — emit は test 観測の基盤)。
- **emit 責務の配置**(pid を知っている process が書く):
  - deep mode parent(`exit-tree`/`fail-tree`/`deep-tree`/`deep-stubborn`):
    `pids.d/parent` を arm 先頭で emit → `ignore_sigterm` →
    `spawn_fixture("branch")` → `@@` 行待ち(「chain 形成済み」の内部
    同期として維持 — pid 転送目的ではなくなった)→ exit/park。
  - `branch`: `pids.d/child` を entry で emit →
    `spawn_fixture("child")` → `pids.d/grandchild` を `leaf.id()` で
    spawn 直後に emit(`ready` 待ちより先 — spawn 返却時点で OS pid は
    確定)→ `ready` 待ち → `@@{child,grandchild}` 行(内部同期用に維持)
    → park。
  - `tree` mode parent: `pids.d/parent` emit → `spawn_fixture("child")` →
    `pids.d/child` を `child.id()` で emit → `ready` 待ち → park
    (単一 `pids.json` write は廃止)。
  - `stubborn`/`graceful`: `pids.d/parent` を atomic write。
    `graceful` は signal install 直後に `pids.d/parent.armed`、SIGTERM
    捕捉時に `pids.d/parent.terminated` を atomic write。
- **`SpawnedChild`**(`spawn_fixture` の戻りを包む kill-on-drop guard):
  Drop で direct child を best-effort kill — **`try_wait()` が未終了を
  示す場合のみ**発火(既 exit の child pid が再利用済みでも誤 kill
  しない)。Windows は `{SystemRoot}\System32\taskkill.exe` の absolute
  path(tree.rs:407 と同じ convention)で `taskkill /PID <pid> /T /F`
  (subtree)+ `child.kill()` belt、unix は
  `child.kill()`(SIGKILL、transitive は production の group kill と
  test 側 guard がカバーするため direct のみで足りる — Data and Control
  Flow §leak containment 参照)。`id()` passthrough を提供。
  fixture の panic(unwind)経路でのみ発火 — 正常 `std::process::exit` /
  `park_forever` では発火しない。
- **`spawn_fixture` の stdio**: stdin `null`・stdout piped は現行維持、
  stderr は `Stdio::null()` → **`Stdio::inherit()`** へ — descendant の
  panic message が managed parent の capture 済み stderr へ乗り、
  `details.stderr.text` を経て `dump_failure` で採取できる。

### Test 側

- **`WaitCtx`/`wait_until`**: `WaitCtx { mode, what, budget, poll }`。
  `wait_until` は満了時 `false` を返す非 panic 版のみを持つ —
  **panic 版 wait(`wait_for`/`wait_emitted`)は作らない**。wait の失敗は
  caller が `dump_failure` → `diag_fail` へ送り、panic message は
  `[{mode}] ... timed out after {budget:?} (poll {poll:?}): {what}`
  shape の context を含む(`dump_failure` の stderr dump が panic に
  先行する)。`select!` 内でも同じ `wait_until` を使うため、panic する
  wait を `select!` に置けない構造問題自体が発生しない
  (abort mode の emit 待ち — stall は `abort_and_drain` の正式 give-up
  経路で処理する)。
- **emit 観測 helpers**: `emitted_pids(dir, keys)`(per-key file の存在かつ
  parseable な `{key: pid}` 集合)、`collect_emitted`(bounded straggler
  回収 — 全 key parseable か bounded grace 経過まで `wait_until`)。
- **`live_pids(want)`**: Windows は unfiltered `tasklist /NH /FO CSV` を
  **poll ごとに 1 回だけ** spawn し client 側で `want` 集合と照合する。
  parse 規則 — 各行は `"` prefix を要求して `","` で split、field index
  1 が bare PID(field 0 の image name は `kill_emitted` の image 照合に
  共用。memory 列の `"12,345 K"` は `","` を含まないため split は安全)。
  nonconforming output・helper spawn 自体の失敗は conservative に全員
  alive 扱い(wait budget で bound され vacuous-pass にならない —
  `expect` で panic しない点は意図的)。unix は `kill(pid,0)` + `/proc`
  zombie 判定を `live_pids` 内に per-pid で畳む(subprocess 不要のため
  per-pid のまま)。旧 `alive(pid)` wrapper は **残さず全 callsite を
  `live_pids` へ置換**する — dead helper のまま残すと `cargo clippy
  -D warnings`(`--all-targets`)が fail する。
- **`verify_emitted_dead(mode, emitted, started)`**: `wait_until` で
  `live_pids(emitted).is_empty()` まで待機(集合 budget、poll=100ms)、
  残存なら live pid 一覧付きで `diag_fail`(wait/verify 系の failure 出口は
  panic 版 wait を通さず全て `diag_fail` へ集約 — Data and Control Flow
  §4 参照)。
- **`TaskRun`**: `process_tool_definition(options)` が返す definition の
  `(def.execute)(input, Some(controller.signal()))` — registry が使う
  canonical `CanonicalExecute` seam(process/mod.rs:166-181)と同一経路で
  `Pin<Box<dyn Future + Send + 'static>>` を得て `tokio::spawn` した
  JoinHandle + `AbortController` + `pids_dir`/`keys`/`mode` +
  `armed: bool`(default `true`)を保持する guard。
  - spawn する実行は `tool.execute(&input, ..)` を直接 `tokio::spawn`
    しない — `&self`/`&Value` borrow を持つ future は `'static` を
    満たさず spawn 不能。definition seam 経由なら production(registry
    経由の呼出し)と同じ 'static future が得られる。spawn しない test
    は従来どおり `tool.execute(&input, None).await` の direct 呼出しを
    維持する。
  - `settle()`: `tokio::time::timeout(SETTLE_BUDGET, &mut handle)` —
    task timeout + cleanup 連鎖を上回る wall-clock bound。超過時は
    `abort_and_drain` → `diag_fail`。
  - `abort_and_drain(why)`: `controller.abort` → `timeout(DRAIN_BUDGET,
    &mut handle)` — runtime 生存中に production の abort→cleanup(tree
    kill)を完走させる正式な give-up 経路。超過時は `kill_emitted` +
    stderr note + **`disarm()`**(guard による二重 kill を防ぐ)の上
    `diag_fail`。
  - `disarm()`: `armed = false` に解除する。(a) `abort_and_drain` の
    give-up branch で自身の `kill_emitted` 後に、(b) test が
    post-resolution verification を pass した後に呼ぶ — 完走・検証済み
    経路で Drop が再 kill しないための解除。
  - `Drop`(`armed` の場合のみ発火): `controller.abort(teardown)` +
    **`kill_emitted(pids_dir)` を同期的に実行**。`handle.abort()` は
    **呼ばない**(JoinHandle abort は future を次 yield で drop するだけで
    managed process の kill は走らず、leak を悪化させる)。`armed` の
    まま残るのは panic/unwind 中・give-up 未処理の経路のみ — 完走済み
    経路は `disarm` 済みで Drop は no-op。
  - 宣言順: `TempDir` を `TaskRun` より **先に**宣言し、`TaskRun` が先に
    drop されるようにする — `kill_emitted` が pids.d を読めるうちに
    guard を発火させ、その後の `remove_dir_all` が process 生存で
    Windows 失敗する経路を断つ。
- **`kill_emitted(dir)`**: emit 済み pid の best-effort 同期 kill。
  `pids.d` 不存在・parse 不能は tolerate し対象 0 件で return。kill
  対象は **live かつ image が `current_exe` filename と一致する pid に
  限定** — `live_pids` 系 snapshot を先に取り live ∩ emitted を照合し、
  PID reuse で無関係 process に化けた pid を殺さない(Windows は
  tasklist CSV の image-name 列、unix は `/proc/<pid>/exe` または comm
  で fixture test binary と照合)。Windows は
  `{SystemRoot}\System32\taskkill.exe` absolute path(tree.rs:407 と
  同じ convention)で per-key `taskkill /PID <pid> /T /F`(parent の
  `/T` で subtree 全体)、unix は `parent` key があれば
  `kill(-(pid), SIGKILL)`(managed parent は `process_group(0)` の
  group leader で fixture 全員がその group にいる —
  spawn.rs:163-168)+ 他 emitted pid への `kill(pid, SIGKILL)` belt。
- **`dump_failure(err: Option<&ToolError>, ctx)` / `diag_fail(ctx) -> !`**:
  ctx は `{mode, what/budget, elapsed, emitted set}` の failure context。
  `dump_failure` は `err`(あれば)の `source()` chain を逐層 eprintln、
  `find_cheaper_error` の `details()` を pretty JSON、ctx 項目を stderr
  へ採取する(error を持たない pure-wait failure は `err: None` で同じ
  経路を通る)。`diag_fail` は dump 後に context 付き message で panic
  する `-> !` helper — process-tree 系 test の wait/verify を含む
  **全 failure 出口が `dump_failure` → `diag_fail` を経由**し、
  「panic 経路を含む全 failure 経路」が文字通り成立する。

## Data and Control Flow

### 1. emit protocol(fixture → test)

```
parent(test binary re-exec, managed process)
  → emit pids.d/parent            (entry 直後 — 最初の観測可能行動)
  → spawn_fixture("branch")
      branch → emit pids.d/child  (entry 直後)
             → spawn_fixture("child")
             → emit pids.d/grandchild (leaf.id() — spawn 返却時点)
             → wait "ready" → print "@@{child,grandchild}" → park
  → wait_line("@@")               (chain 形成済みの内部同期)
  → mode 別: exit(0|7) / park / park(stubborn: ignore_sigterm)
```

`pids.d` の dir は fixture が `env::var(PID_DIR_ENV)` から取る唯一の
channel — `fixture_task` が task `env` に宣言し、`spawn_fixture` の env
継承で descendant(branch/child)へ伝播する。emit 責務を持つ mode で
未設定なら mode 名入り panic(fail-fast)。

test は `pids.d/<key>` の「存在かつ parseable」を per-key に待つ。emit は
atomic tmp+rename のため exists→parse の TOCTOU(spec residual (a))は
構造的に消える。

### 2. resolution の観測(test)

`pending` は `TaskRun` 内の JoinHandle — `process_tool_definition` の
`(def.execute)(input, Some(controller.signal()))` が返す 'static future
を `tokio::spawn` したもの(canonical execute seam と同一経路)。

- **exit/fail/timeout mode**: `run.settle()` を直接 await —
  fixture の自然 exit(fail-tree の exitCode 7)・task Timeout(deep-stubborn)
  が resolution になる。emit を事前に待たない — resolution 自体が
  「fixture の lifetime 終了 + cleanup 完了」の signal。
- **abort mode(deep-tree)**: emit 待ち(非 panic の `wait_until` で
  parent+child+grandchild、EMIT_WAIT_BUDGET)と `&mut handle` を
  `select!`:
  - 全 emit 到着 → `run.abort("outer tree cancellation")` → `settle()`
    → `err.to_string() == "outer tree cancellation"` を assert
    (live descendant 3 件がいる状態で abort する既存 invariant を維持)。
  - emit stall(EMIT_WAIT_BUDGET 超過)→ `run.abort_and_drain(
    "emit stall")` → `diag_fail`。`select!` 内でも `wait_until` のみを
    使う(panic 版 wait は存在しない) — stall を unwind ではなく
    正式 give-up 経路で処理する。
  - handle が先に解決 → early resolution として分類へ。
- **早期 `Timeout` の受理**: non-timeout mode で `code == Timeout` の Err が
  返った場合、発火位置を問わず **legitimate outcome** として受理する
  (mode 固有 assert を skip せず「期待 outcome OR Timeout」の二択判定)。
  emit 待ちは task timeout 60s 未満の EMIT_WAIT_BUDGET(30s)で別 budget 化
  されているため、emit stall は test 側 give-up(abort_and_drain + diag)
  で処理する。

### 3. outcome 分類(classify)

| 期待 mode | 受理する resolution | 拒否する resolution(→ `diag_fail`) |
| --- | --- | --- |
| `exit-tree` ("ok") | `Ok`(state=completed, exitCode=0) / `Err` code=Timeout | `Err` その他・`JoinError` |
| `fail-tree` ("fail") | `Err` code=ToolExecutionFailure && details.exitCode==7 && `find_cheaper_error(err).message()=="managed process failed"` / `Err` code=Timeout | 同上 |
| `deep-tree` ("abort") | abort 後の `err.to_string()=="outer tree cancellation"` / `Err` code=Timeout | `Ok`・上記以外の `Err` |
| `deep-stubborn` ("timeout") | `Err` code=Timeout(**期待 outcome**) | `Ok`・`Err` その他 |

`fail-tree` の message 照合が必要な理由: cleanup-failed arm
(monitor.rs:319-327、`"managed process cleanup failed"`)は `exitCode` を
含む**同一 details** を運ぶため code+exitCode だけでは区別できない —
cleanup-failed は `diag_fail` へ送る。

- 自然解決(Ok / ToolExecutionFailure)の場合は **全 expected key の
  emit を要求**する(post-`collect_emitted` grace 後の emitted set で
  判定 — deep mode の自然解決なら `{parent,child,grandchild}` 全 3 件が
  揃う。自然解決した fixture は全 emit を終えているため、不足 = fixture
  が途中で死んだ = 異常 → `diag_fail`)。
- `Timeout` 解決時の emit floor(post-collect の emitted set に適用)は
  mode designation に依存する(「60s-armed」は timeout 値ではなく
  mode identity で key される):
  - **60s-armed mode(exit/fail/deep-tree)は emit ≥1 件を要求** — emit は
    fixture の最初の観測可能行動であり、60s 時点まで managed process が
    生きていれば emit は完了している。0-emit Timeout = broken
    fixture/plumbing の証拠 → `diag_fail`。
  - **0-emit Timeout の受理は deep-stubborn(2s)と sibling 500ms test
    のみ** — timeout clock が fixture の emit 前に発火しうる genuine
    pre-emit kill の window が残るため、emit 済み分を全件検証する規則の
    0 件ケースとして受理する。

### 4. post-resolution verification

`settle` 後(early Timeout を含む全経路):

1. `collect_emitted(dir, keys, POST_RESOLVE_EMIT_GRACE)` — kill と競合する
   straggler emit を bounded 回収。
2. `verify_emitted_dead` — `live_pids` が空になるまで集合 budget
   (DEAD_BUDGET)+ poll 100ms で待ち、残存なら `diag_fail`。
3. 全 verification pass 後に `run.disarm()` — `TaskRun` の Drop guard を
   解除し、正常経路で `kill_emitted` が走らないようにする。

emit されていない stage は検証対象が 0 件になるだけで、emit 済み分の
「descendant 消滅」invariant は弱めない(0-emit 受理は §3 の per-mode
emit floor で制限済み)。

### 5. leak containment(panic/unwind 経路)

tokio の runtime drop は spawned task を **poll せず drop** する
(current_thread `close_and_shutdown_all`、tokio 1.53.2 の
`scheduler/current_thread` shutdown 経路を確認済み)。したがって
controller abort を投げるだけでは `drive()` の abort→cleanup は走らない
— abort が signal を立てても、task が再 poll されなければ `wait_aborted`
select は発火しない。

このため containment は 2 系統で構成する:

- **runtime 生存中の正式 give-up 経路(`abort_and_drain`)**: test が
  wait/settle の budget 超過を検出した時点で `controller.abort` →
  `timeout(DRAIN_BUDGET, handle)`。spawned task が runtime 上で poll される
  間に production の abort→`start_cleanup`→`tree.terminate` が完走し、
  emit 有無・pid 知識に依存せず tree を kill する(pre-emit 含む
  **全 window を覆う**主機構)。
- **unwind 中の最終防衛(`TaskRun::Drop` → `kill_emitted`)**: `armed`
  のまま panic で test fn が unwind すれば emitted pid を同期的に kill
  する(disarm 済みの正常経路では発火しない)。
  emit 前 panic は ≤1 process(parent)が残りうる **accepted residual**
  — window は spawn→entry emit(~ms 級)に限られ、parent 未 emit 時は
  branch/grandchild も未 spawn のため leak は parent 単独に bounded される。

fixture 内部でも同じ構造を持つ: `SpawnedChild` が fixture process の
panic(unwind)時に direct child を kill する(`try_wait()` で未終了のみ
発火。Windows は `{SystemRoot}\System32\taskkill.exe` の `/T` で
subtree、unix は direct — unix では parent の死亡が production の
`kill(-pgid)` cleanup を誘発し transitive はそちらが処理する)。

### 6. timeout budget の階層

| budget | 値 | 役割 |
| --- | --- | --- |
| task `timeout`(exit/fail/abort) | `"60s"` | fixture の観測可能な全 lifetime(spawn chain + emit wait 30s + abort/exit + 解決)を超える値 — timeout が lifetime と同じ clock で競合しない |
| task `timeout`(deep-stubborn) | `"2s"` | kill 検証に必要な最小値(現行値維持 — emit が観測される程度には tree が立ち上がる) |
| task `timeout`(sibling tests) | `"500ms"`(現行維持) | timeout 経路そのものの検証 — 早期 Timeout が legitimate 化されたため小さいまま race にならない |
| `EMIT_WAIT_BUDGET` | 30s | 全 key emit 待ち(abort mode の abort 前 / emit 検証) |
| `POST_RESOLVE_EMIT_GRACE` | 5s | resolution 後の straggler emit 回収 |
| `DEAD_BUDGET` | 30s | emit 済み pid 集合の消滅待ち(集合 budget・poll 100ms) |
| `SETTLE_BUDGET` | 180s | `settle()` の wall-clock bound(task timeout 60s + cleanup worst ~90s + slack) |
| `DRAIN_BUDGET` | 120s | give-up 時の abort→cleanup 完走待ち(下記 Windows terminate worst ~90s + slack) |
| poll cadence | file wait 20ms / pid-death wait 100ms | R5 self-load 削減(最大 ~10 tasklist/s/待機へ) |

`DRAIN_BUDGET`/`SETTLE_BUDGET` の導出: Windows `WinTree::terminate`
(tree.rs:465-499)の worst case は各 helper call が `HELPER_TIMEOUT_MS`
=10s(tree.rs:17)の deadline まで伸びる積算 — 初回 eager snapshot ~10s +
collect ~10s + `kill_verified`(non-force、taskkill + 再 collect)~20s +
grace-loop ~2×10s + `kill_verified`(force)~20s + final collect ~10s ≈
**~90s**。`DRAIN_BUDGET`=120s はこの ~90s cleanup に slack を乗せ、
`SETTLE_BUDGET`=180s は task timeout 60s + ~90s cleanup + slack を覆う。

## Decisions

### D1: per-key incremental emit は owner process が atomic tmp+rename で書く

`pids.d/{parent,child,grandchild}`。parent/branch は entry で自身の pid を
emit し、grandchild は branch が `leaf.id()` を spawn 返却直後に emitする
(`ready` 行を待たない — OS pid は spawn 成功時点で確定)。`tree` mode も
`pids.d/{parent,child}` へ統一する(単一 `pids.json` を残すと protocol が
二系統になり、同じ TOCTOU/unwrap 修正を別表現で二重に持つ — 表現ドリフト
防止)。`graceful` の `pids.d/parent.armed` も同じ atomic helper で書く。

pids.d dir の伝達は `PID_DIR_ENV` env 一本 — `fixture_task` が全
tree-family mode(`stubborn`/`graceful` 含む)の task `env` に宣言し、
fixture は `env::var` で読み、`spawn_fixture` の env 継承で
descendant(branch/child)へ伝播する。argv の pid path 契約は廃止し、
emit 責務を持つ mode で env 未設定なら mode 名入り panic(fail-fast)。

- 代替案: `pids.json` を維持して parse を retry する — exists→parse の
  TOCTOU が残り、「emit 済み pid のみ検証」の粒度が取れない(全完備 write
  では early resolution 時に検証対象を特定できない)→ 不採用。
- 代替案: `ready`/`@@` 行を test 側同期に再利用する — fixture stdout は
  tool の bounded capture 経由で test へ戻るが、行単位の streaming 待ちは
  capture 完了後しか読めず early resolution と両立しない → 不採用
  (ファイル emit は kill 済み process の書き残しも観測できる)。

### D2: task timeout は timeout 期待 mode のみ小さく、他は全 lifetime を上回る

exit/fail/abort は `"60s"`(emit wait 30s + abort/exit + 解決を含む全
lifetime を超える)、deep-stubborn は `"2s"`(kill 検証に必要な最小値)、
sibling test の `"500ms"` は維持。

- 代替案: 全 mode `"60s"` 統一 — deep-stubborn で毎回 60s 待つ wall-clock
  増加分が無駄(spec は timeout mode に最小値を要求)→ 不採用。
- 代替案: mode 別の個別 `#[tokio::test]` 化(ADR Option E)— 4 test 並列化で
  helper storm の peak 負荷を増やし、ADR で棄却済み → 不採用(逐次 loop 維持)。

### D3: 早期 `Timeout` を legitimate outcome として受理し、emit 済み pid を全件検証

resolution を「mode 期待 outcome OR code=Timeout」の分類表(D3 表)で
裁き、Timeout 受理後は bounded emit 回収 → emit 済み全 pid の消滅検証を
行う。自然解決(Ok/Fail)は **全 expected key の emit を要求**し、
不足は異常として `diag_fail` —
「fixture が走った」の最低証拠を残して vacuous-pass を防ぐ。
`Timeout` 解決にも per-mode の emit floor を課す: **60s-armed mode
(exit/fail/deep-tree)は emit ≥1 件を要求**し、0-emit Timeout は broken
fixture/plumbing として `diag_fail`(emit は最初の観測可能行動 —
60s 生存した fixture は emit 済みのはず)。0-emit Timeout の受理は
genuine pre-emit kill が起こりうる deep-stubborn(2s)と sibling 500ms
test に限る。

`fail-tree` の受理は `code==ToolExecutionFailure && exitCode==7` に加え
`find_cheaper_error(err).message()=="managed process failed"` を要求する
— cleanup-failed arm(monitor.rs:319-327)が `exitCode` を含む同一
details を運ぶため、message で区別しないと cleanup 失敗を fail-tree の
期待 outcome として誤受理する。

- 代替案: emit 完備を resolution の前提に戻す — R1 の構造 race そのものを
  保持する → 不採用。
- 代替案: Timeout 受理時に検証自体を skip する — 早期 kill が descendant を
  残す regression を見逃す → 不採用(spec 要件: emit 済み全件検証)。

### D4: Windows liveness は unfiltered `tasklist /NH /FO CSV` の client 側集合照合

`live_pids(want)` が poll ごとに 1 回の `tasklist /NH /FO CSV` snapshot
で `want` 集合全員を照合する。parse 規則: 各行は `"` prefix を要求して
`","` で split、field index 1 = bare PID(field 0 = image name —
`kill_emitted` の image 照合に共用。memory 列 `"12,345 K"` は `","` を
含まないため split は安全)。`/FI` 複数併記は AND 評価で常に空を返し
vacuous-false 化するため**禁止**(spec 明記)。nonconforming output・
helper 失敗時は全員 alive 扱い(conservative — wait budget で bound
され、誤 pass にも誤 panic にもならない)。unix は `kill(pid,0)` +
`/proc` zombie 判定を `live_pids` 内に per-pid で畳む — 旧 `alive()`
wrapper は残さず全 callsite を `live_pids` へ置換する(dead helper を
残すと `cargo clippy -D warnings` が `--all-targets` 下で fail する)。
poll cadence は file 系 20ms・pid-death 系 100ms に分離し、R5 の
self-load(~50 spawn/s)を ~10/s 以下へ下げる。

- 代替案: per-pid `/FI` 維持 + cadence 緩和のみ — 集約されず spawn 総量は
  key 数倍のまま → 不採用(R5 を弱めるだけで消せない)。
- 代替案: `OpenProcess`+exit code 照合等の API 化 — test helper に新たな
  unsafe/handle 管理を持ち込む。YAGNI(tasklist snapshot で十分) → 不採用。

### D5: wait は `wait_until` + `diag_fail` で帰属を自明にし、context-free `unwrap()` を排除

wait は非 panic の `wait_until` に統一し(panic 版 `wait_for`・
`wait_emitted` は作らない)、失敗は `dump_failure` → `diag_fail` が
`[{mode}] ... timed out after {budget:?} (poll {poll:?}): {what}`
shape の context 付き message で panic する — 「どの mode/test の何を
何秒待って何が駄目だったか」を panic 単体で判別可能にする。
`"{key} gone"`(:582/:653)の non-interpolated literal は
`verify_emitted_dead` への置換で消去する。pid file の read/parse・
`spawn_fixture` の `.spawn()`・`live_pids` 系の `.output()` は
context 付き `expect`/`diag_fail` へ。

### D6: panic-path leak containment は JoinHandle + drop-guard 同期 kill

`TaskRun` が `process_tool_definition(options)` の
`(def.execute)(input, Some(controller.signal()))` から得る 'static
future を `tokio::spawn` した JoinHandle を保持する — registry が使う
canonical `CanonicalExecute` seam(process/mod.rs:166-181)と同一経路。
`tool.execute(&input, ..)` を直接 `tokio::spawn` することはできない
(`&self`/`&Value` borrow を持つ future は `'static` を満たさない)。
- 正常系: `settle()`/`abort_and_drain()` で runtime 生存中に production
  cleanup を完走させる(pre-emit も覆う主機構)。
- unwind 系: `Drop` が `armed` のときのみ `controller.abort(teardown)` +
  同期 `kill_emitted` を行い、`handle.abort()` は **呼ばない**(future
  drop は cleanup を走らせず orphan 化する)。`disarm()` は
  `abort_and_drain` の give-up branch(自身の `kill_emitted` 後)と
  test の post-resolution verification pass 後に呼び、完走済み経路での
  二重 kill を防ぐ。

fixture 側は `SpawnedChild` kill-on-drop(`try_wait()` で未終了のみ発火、
Windows `{SystemRoot}\System32\taskkill.exe` の `/T` subtree / unix
direct kill)。

- 代替案: guard が `controller.abort` のみ投げる(ADR の例示そのまま)—
  runtime drop が spawned task を **poll せず drop** する(tokio 1.53.2
  `close_and_shutdown_all` をソースで確認済み)ため、unwind 中に cleanup は
  走らず leak が残る → `kill_emitted` の同期 kill を主機構に据える。
- 代替案: test 側で fixture の process group 化など更なる OS 仕掛け —
  production 側に無い仕組みを test 側に複製するだけで検証価値が薄い →
  不採用。
- 残 residual: parent 未 emit(~ms window)での panic は parent ≤1 process
  が残りうる — spec (d) の containment 要件を満たす範囲内の受容残余として
  記録する(window は emit-first 配置で構造的に最小化済み)。

### D7: failure diagnostics は `dump_failure`/`diag_fail` に集約

signature は `dump_failure(err: Option<&ToolError>, ctx)` /
`diag_fail(ctx) -> !`、ctx = `{mode, what/budget, elapsed, emitted set}`。
`dump_failure` が error `source()` chain・`find_cheaper_error` の
`details()` pretty JSON・ctx 項目を stderr へ出し、`diag_fail` が
context 付き message で panic する — error を持たない pure-wait failure
は `err: None` で同じ経路を通り、wait/verify を含む **全 failure 出口が
`dump_failure` → `diag_fail` を経由**する(「panic 経路を含む全 failure
経路」が文字通り成立)。
`node_task_resolves_via_path`(:448)は `.unwrap()` を `match` へ変え、
Err 経路で `diag_fail` — 次回 exit 255 が再発したとき
`"managed process cleanup failed"`(tool 側 helper 帰属)と node 起因
(state/exitCode/spawnCode 系 details)を output から判別できる(R4)。
diagnostics が tool 側帰属を示した場合の判断記録・follow-up issue 化は
spec の AC に従う運用とし、design の構造要件はここまで。

### D8: 未変更で残す test

`executes_literal_argv_...`・`fails_nonzero_exit_...`・
`caps_combined_output_...`・`touch_with_matching_write_scope_...` は
park しない短命 fixture で race/leak 経路を持たず、assertion 意味を
変えない(spec Out of Scope「上記 3 test 以外の assertion 意味の変更」
に対し、これらは対象外のまま)。`wait_until` 化と
`alive` → `live_pids` 置換に伴う機械的な callsite 更新のみ行う。

## File/Line Edit Map

対象: `crates/cheaper-tools/tests/proc_process.rs`(現行 717 行)。
行番号は現行 file の anchor(編集後は shift する)。

| 現行行 | 箇所 | 変更 |
| --- | --- | --- |
| :31-34 | const 群 | `PID_DIR_ENV`・budget/poll 定数を追加 |
| :102-105 | `stubborn` arm | `write_pid` → `pids.d/parent` atomic emit(dir は `env::var(PID_DIR_ENV)` — argv path 契約廃止、未設定は mode 名入り panic) |
| :106-124 | `graceful` arm | `pids.d/parent`(`PID_DIR_ENV` 由来)atomic emit + signal install 直後の `parent.armed` + SIGTERM 時 `parent.terminated` の atomic 化 |
| :133-147 | `branch` arm | `PID_DIR_ENV` 由来 dir に `child`(entry)・`grandchild`(spawn 直後 `leaf.id()`)を emit。`@@` 行は維持 |
| :148-159 | `tree` arm | `pids.json` 一括 write を `pids.d/{parent,child}`(`PID_DIR_ENV` 由来)per-key emit へ置換 |
| :160-186 | deep mode arm | `pids.d/parent` を arm 先頭(`ignore_sigterm` より前)に emit — dir は `env::var(PID_DIR_ENV)`(未設定は mode 名入り panic、argv path 契約廃止)。`@@` wait・exit/park は維持 |
| :198-200 | `write_pid` | atomic tmp+rename の `emit_marker`/`emit_pid` へ内製化 |
| :219-229 | `wait_line` | (fixture 内) — 変更不要 |
| :233-255 | `spawn_fixture` | 戻りを `SpawnedChild` guard(`try_wait()` gate)へ、`.spawn()` を context 付き expect へ、descendant stderr を `Stdio::inherit()` へ(panic が managed parent の capture stderr → `details.stderr.text` → `dump_failure` に乗る) |
| :291-306 | `fixture_task` | `env[PID_DIR_ENV]` を全 tree-family mode(stubborn/graceful 含む)で宣言する `pids_dir` 指定を追加 + `outputs` 宣言を `["pids.d/**"]` へ統一(`pids.d/parent(.armed|.terminated)` も dir 内に置くため Subtree が正直 — `**` 末尾 → `ResourceScope::Subtree`、policy/exec.rs:759-788) |
| :324-346 | `tool_with` | spawn 経路用に `process_tool_definition(options)` を返す variant を追加(canonical `CanonicalExecute` seam と同一経路 — spawned test は `tool.execute` を直接 `tokio::spawn` しない) |
| :360-368 | `wait_for` | `WaitCtx` 化 + `wait_until` 統一(panic 版は作らず失敗は `diag_fail` へ) |
| :371-400 | `alive` | `live_pids` へ置換・`alive` は廃止(Windows unfiltered tasklist CSV parse `"`/`","`/field 1 / unix per-pid `kill(pid,0)`+`/proc` zombie — wrapper を残すと `clippy -D warnings`(`--all-targets`)が fail) |
| — | 新規 helper block | `emitted_pids`/`collect_emitted`/`verify_emitted_dead`/`TaskRun`(armed/disarm)/`kill_emitted`(live∩emitted∧image 照合)/`SpawnedChild`/`dump_failure`/`diag_fail` |
| :448-470 | `node_task_resolves_via_path` | `.unwrap()` → `match` + Err 経路 `diag_fail`(exitCode/state/error chain 採取) |
| :529-550 | `times_out_native_process_and_cleans_tree` | `PID_DIR_ENV` task + `TaskRun` + Timeout 分類 + bounded emit 回収 + emit 済み pid の消滅検証(500ms mode は 0-emit Timeout も受理 — genuine pre-emit kill) |
| :552-585 | `abort_kills_managed_parent_and_descendant_and_preserves_reason` | `pids.d` 化 + `TaskRun` + `select!` 内 `wait_until` emit 待ち→abort→verify dead + `"{key} gone"` 消去 |
| :587-657 | `cleans_live_descendants...` | mode 別 timeout + `TaskRun` + outcome 分類表(fail-tree は `message=="managed process failed"` 照合)+ early Timeout 受理 + per-mode emit floor + bounded collect + 全 emitted pid 検証 |
| :692-717 | `allows_sigterm_exit_within_grace_before_force_kill` | `killGrace` `1s`→`10s` + `PID_DIR_ENV` task + `TaskRun` + Timeout 分類 + `.armed` marker 分岐(kill-before-arming=受理 / armed-but-no-`.terminated`=real failure) |

## Test-by-Test Behavior Table

| test | fixture | task timeout | 受理する resolution | emit 契約 | post-resolution 検証 |
| --- | --- | --- | --- | --- | --- |
| `times_out_native_process_and_cleans_tree` | `stubborn`(park) | `500ms` | `Err` code=Timeout のみ(他は `diag_fail`) | `pids.d/parent` を emit すれば collect、未 emit も受理(500ms mode は genuine pre-emit kill window あり) | emit 済み pid の消滅(0 件も受理) |
| `allows_sigterm_exit_within_grace_before_force_kill` (unix) | `graceful` | `500ms` / grace `10s` | `Err` code=Timeout のみ | `pids.d/parent` + `parent.armed` marker | `.armed` 有 → `parent.terminated`=="graceful" 必須 + emitted pid 消滅;`.armed` 無 → 受理(診断 note) |
| `abort_kills_managed_parent_and_descendant_and_preserves_reason` | `tree`(2 段 park) | なし(killGrace 200ms) | parent+child 全 emit 後の abort → `"stop managed tree"`;早期解決は `diag_fail` | `pids.d/{parent,child}` 全件必須 | emit 済み 2 件の消滅 |
| `cleans_live_descendants`(exit-tree) | 3 段、parent 自然 exit 0 | `60s` | `Ok` or `Err` Timeout | 自然解決なら全 3 件、Timeout なら 1-3 件(60s-armed は 0-emit `diag_fail`) | emit 済み全件の消滅 |
| 〃 (fail-tree) | 3 段、parent exit 7 | `60s` | `Err` ToolExecutionFailure+exitCode 7+`message=="managed process failed"` or Timeout | 同上 | 同上 |
| 〃 (deep-tree) | 3 段 park、test abort | `60s` | 全 emit 後 abort → `"outer tree cancellation"` or Timeout(早期受理) | abort 前に全 3 件必須 / Timeout 早期解決なら 1-3 件(0-emit `diag_fail`) | 同上 |
| 〃 (deep-stubborn) | 3 段 park、SIGTERM 無視 | `2s` | `Err` code=Timeout(**期待**) | 0-3 件(0-emit 受理は 2s/500ms mode のみ) | emit 済み全件の消滅 |
| `node_task_resolves_via_path` | ambient `node -e` | なし | `Ok` exitCode=0 のみ | なし | Err 経路で `diag_fail`(chain/details 採取 → R4 帰属判別) |
| 残り 4 test | inspect/fail/output/touch | — | 変更なし | — | — |

## Acceptance Mapping

| SPEC requirement / Acceptance Criteria | 本 design の対応 |
| --- | --- |
| mode 別 outcome と descendant 消滅の既存検証を弱めない | D3 分類表 + D4 `live_pids` + `verify_emitted_dead`(emit 済み全件検証、早期 Timeout でも弱めない) |
| task timeout が fixture の観測可能な全 lifetime を上回る | D2 — `"60s"` は emit wait 30s + abort/exit + 解決を含む全 lifetime を上回り、deep-stubborn のみ `"2s"` |
| 早期 `Timeout` を期待 outcome として受理 + emit 済み全件の消滅検証 | D3 — 受理しても消滅検証は全件行い、emit 0 件受理は genuine pre-emit kill window の残る 2s/500ms mode のみ(60s-armed mode の 0-emit Timeout は `diag_fail`) |
| fixture pid 通知を incremental per-key atomic emit へ | D1 — `pids.d/{parent,child,grandchild}`、owner-emit、tmp+rename、`PID_DIR_ENV` が唯一の伝達路(env 宣言 + spawn env 継承) |
| sibling test に同構造適用・`armed` marker で区別 | D1(`.armed`)+ test-by-test 表の graceful 行 — armed 有で `.terminated` 無しは real failure |
| `wait_for` panic が mode/key/budget context を含む・`"{key} gone"` 消去 | D5 — `wait_until` + `diag_fail` の context 付き panic、`verify_emitted_dead` へ集約 |
| failure diagnostics が error chain/details/mode/elapsed を採取 | D7 — `dump_failure`/`diag_fail` を全 failure 経路で使用 |
| `node_task_resolves_via_path` のテスト意図維持 + 失敗時帰属判別 | D7 + test-by-test 表 — ambient spawn 維持、Err 時に exitCode/state/chain 記録 |
| panic mid-`drive()` leak の containment | D6 — `TaskRun`(canonical seam の 'static future spawn + armed/disarm drop-guard 同期 kill)+ `abort_and_drain` + `SpawnedChild` |
| `tasklist /FI` 複数併記の禁止・unfiltered snapshot 集約 | D4 — `/NH /FO CSV` 1 回/poll の client 照合(`"` prefix・`","` split・field 1=PID)、helper 失敗は conservative |
| `#[tokio::test]` 非同期構造・env-driven mode・`@@` sentinel 維持 | 全変更が同 file の既存 pattern 内 — fixture argv/env 契約・`@@` 行は内部同期として維持 |
| liveness/kill の PID-reuse は accepted residual | Risks and Trade-offs で documented, not fixed を明記 |

## Non-Functional Constraints

- **決定性**: 全 assertion が wall-clock/ambient 負荷に依存しない。
  emit は file artifact(死んだ process の書き残しも観測可)、resolution は
  task の実結果、kill は同期 OS call で検証する。
- **自己負荷削減**: tasklist spawn は待機あたり最大 ~10/s(poll 100ms・
  集合 snapshot)へ — 旧 ~50/s/pid 待機から削減(R5)。
- **bounded**: 全待機は明示 budget(file 20ms poll / pid-death 100ms poll・
  emit 30s・post-resolve 5s・dead 30s・settle 180s・drain 120s)。
  helper(tasklist/taskkill)は `.output()` の完了まで待つ — fixture の
  `SpawnedChild`/`kill_emitted` は unwind/診断経路のみで呼ばれ
  happy path に乗らない。
- **新規依存なし**: std・`libc`・`tokio`・`serde_json` の既存依存のみ
  (`fastrand` も既存 dev-dep)。
- **POSIX/Windows parity**: emit/collect/verify の protocol は共通、
  kill/liveness のみ `cfg` 分岐。両 platform で同じ lifecycle invariant
  を検証する。

## Risks and Trade-offs

- **早期 `Timeout` 受理は genuine regression を一部 mask しうる**
  (exit/fail/abort mode で本当は自然解決すべきところを Timeout で受理)。
  緩和: 自然解決時の全 expected key emit 要求に加え、60s-armed mode の
  Timeout 解決にも emit ≥1 を要求(0-emit は broken fixture/plumbing として
  `diag_fail`)・`Timeout` は「受理」だが mode 期待は別途 assert・
  diagnostics が再発時の帰属を採取する。spec が明示的に選んだ
  trade-off で、flake 判別コストより coverage の実効を取る。
- **liveness/kill の PID-reuse accepted residual(spec (c))**: test 側
  `live_pids` は creation-time guard を持たず、kill 後に同 pid が再利用
  されると消滅待ちが誤判定しうる。`tasklist` に安価な born-time oracle
  が無く、production(`tree.rs` `born_before` 照合)相当を test に複製する
  コストに見合わないため **documented, not fixed** — wait budget で
  bounded される誤判定として残余受容し、必要なら phase-2 で再検討する。
  kill 側は `kill_emitted` の live ∩ emitted ∧ image 一致(`current_exe`
  filename)限定と `SpawnedChild` の `try_wait()` gate で誤 kill window
  を狭めるが、**同 image(fixture test binary)で再利用された pid は
  killable のまま** — spec (c) に並ぶ accepted residual として受容する。
- **pre-emit panic の parent leak ≤1 process(D6 residual)**: parent 未
  emit(~ms window)での panic は drop-guard も kill 対象を持たない。
  `abort_and_drain`(runtime 生存中の give-up)が主機構としてこの window
  も覆うため、残るのは「abort_and_drain 自体が poll されない unwind 中の
  panic」に限られる — emit-first 配置で window を構造的に最小化し、
  残余を受容する。
- **fixture mode 追加 emit の失敗**: emit write が失敗すると fixture が
  panic → 早期解決 → `diag_fail` で帰属が採れる(panic message に
  emit key を含める)。emit は test 観測の基盤のため失敗は fail-fast が
  正しい。
- **`SpawnedChild` の unix direct-kill は transitive を持たない**:
  parent fixture panic 時に branch の子(grandchild)は残るが、parent 死亡
  が production の `kill(-pgid)` cleanup を誘発し group 全体を処理する。
  Windows 側は `taskkill /T` で subtree 完備。

## Deferred Design Concerns

- `WinTree::terminate` の helper 連鎖軽減(snapshot 回数削減・dead pid 直接
  照合・taskkill "not found" exit の成功扱い・eager snapshot の遅延起動)—
  diagnostic が tool 側帰属を示した場合の conditional phase-2(別
  ADR/issue)。spec Deferred Design Concerns と同一。re-engagement
  trigger: `diag_fail` 採取後の failure 記録に `"process tree helper
  failed"` 由来の cleanup-failed が現れること。
- `ProcessTree` eager snapshot の production cost(全 execute が最低 1 回の
  powershell CIM spawn を支払う)— 同上、tool 側帰属確認後の論点。
- test 側 `live_pids` への born-time 照合(PID-reuse guard)— accepted
  residual として残余受容。再発が観測されたら phase-2 で再検討。

## Verification Notes

- [ ] `mise exec -- cargo test -p cheaper-tools --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test proc_process` が pass する(Windows・POSIX 両方)
- [ ] `mise run check` を少なくとも 3 連続実行し `proc_process` 系 test が失敗しない(smoke signal — ~1 件/日 rate に対する証明ではない、spec AC 参照)
- [ ] failure diagnostics の実効確認 — 意図的に budget を潰した一時的変更で `diag_fail` が error chain/details/mode/elapsed を出力することを目視確認し、revert する
- [ ] `cleans_live_descendants` で全 4 mode が mode 別 timeout・早期 Timeout 受理・emit 済み全件消滅検証の経路を通ることを、強制 early-timeout(一時的に timeout を極小にする変更)で経路確認して revert する
- [ ] 再発時運用 — diagnostic が tool 側帰属(`"process tree helper failed"`/`"managed process cleanup failed"` 系)を示した場合、判断記録を docs へ追記 + follow-up issue 化する方針を spec/ADR と整合確認する
