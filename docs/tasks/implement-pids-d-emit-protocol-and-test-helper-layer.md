---
id: "TASK-034LIwmO9NxUouuOoCwC0g"
type: "task"
status: "done"
title: "Implement deterministic proc_process test rewrite"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
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
  implements:
    - "docs/plans/fix-proc-process-test-flake-via-test-side-determinism-issue-180.md"
  implemented-by:
    - "docs/impl/ir/implement-deterministic-proc-process-test-rewrite.md"
  depends-on:
    - "docs/plans/fix-proc-process-test-flake-via-test-side-determinism-issue-180.md"
  blocks:
    - "docs/tasks/verify-proc-process-determinism-and-mise-check.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/fix-proc-process-test-flake-via-test-side-determinism-issue-180.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/deterministic-proc-process-test-assertions-for-parallel-load-flake.md"
  references:
    - "docs/designs/deterministic-proc-process-tests-issue-180.md"
  defers: []
  deferred-by: []
---

# Implement deterministic proc_process test rewrite

## Work

`crates/cheaper-tools/tests/proc_process.rs` の fixture emit protocol・
deterministic helper 層・対象 4 test(+`node_task_resolves_via_path`)の
再構成を**単一の atomic な変更**として実装する(design D1-D8)。

emit protocol の producer(fixture 各 arm)と consumer(test 側 emit 観測
helper)が同じ file に同居し、protocol の変更と consumer の実装を別 task
に分けると中間状態で compile/test が成立しない(旧 `pids.json` 読み側が
残る段階では対象 test が確実に fail し、consumer が無い helper 群は
dead code で `clippy -D warnings` に引っかかる)。そのため本 task で
file 全体を design の確定形へ一度に着地させる。旧 T1/T2/T3 の 3 分割は
この理由で merged task へ統合した。

design との差分(justified deviation):

- **panic 版 wait helper を作らない** — design :199-200「panic 版
  (`wait_for`/`wait_emitted`)は panic→`TaskRun::Drop` が意図の経路で
  ある場所に限る」と :221-223「wait/verify 系の failure 出口は panic
  版 wait を通さず全て `diag_fail` へ集約」が内部矛盾するため、
  design wording を修正し `wait_until` + `diag_fail` を authoritative
  形として統一した。`wait_for` panic の message format
  `[{mode}] ... timed out after {budget:?} (poll {poll:?}): {what}` は
  `diag_fail` の message に fold する。
- **自然解決の emit floor は全 expected key** — design の outcome
  分類表(自然解決なら全 3 件)を採用し、§3 の emit ≥1 記述を揃えた。
- **dev-seam feature flag を付けない** — design の verification
  command に含まれる `--features cheaper-runtime/dev-seam,cheaper-cli/
  dev-seam` は `cheaper-tools` が両 crate に依存しないため
  `-p cheaper-tools` の cargo コマンドでは解決不能。`-p cheaper-tools`
  の全コマンドから落とす(`mise run check` 経路では design どおり
  feature 付きで走る)。

### 1. fixture emit protocol(D1)

1. const 群(:31-34)に `PID_DIR_ENV`(`CHEAPER_PROC_PID_DIR`)と
   budget/poll 定数を追加 — `EMIT_WAIT_BUDGET` 30s・
   `POST_RESOLVE_EMIT_GRACE` 5s・`DEAD_BUDGET` 30s・`SETTLE_BUDGET`
   180s・`DRAIN_BUDGET` 120s・file poll 20ms / pid-death poll 100ms。
2. `emit_pid(dir,key,pid)`/`emit_marker(path,text)` — `{key}.tmp` →
   `fs::rename` の atomic emit。owner process が 1 回だけ書き、初回は
   `create_dir_all`。`write_pid`(:198-200)は内製化して廃止。
3. fixture 各 arm を `pids.d/` per-key emit へ。emit dir は
   `env::var(PID_DIR_ENV)` が唯一の伝達路(argv pid path 契約は廃止)—
   emit 責務 mode で env 未設定なら **mode 名入り panic**(fail-fast)。
   emit 順序は「pid を知っている process が知った時点で書く」:
   - deep(`exit-tree`/`fail-tree`/`deep-tree`/`deep-stubborn`、
     :160-186): arm 先頭で `pids.d/parent` を emit(`ignore_sigterm`
     より前 — pre-emit window を構造的に最小化)→
     `spawn_fixture("branch")` → `@@` 行待ち → mode 別 exit/park。
   - `branch`(:133-147): `pids.d/child` を entry で emit →
     `spawn_fixture("child")` → `pids.d/grandchild` を `leaf.id()` で
     spawn 返却直後に emit(`ready` 待ちより先 — OS pid は spawn
     成功時点で確定)→ `ready` 待ち → `@@{child,grandchild}` 行
     (内部同期として維持)→ park。
   - `tree`(:148-159): `pids.d/parent` emit → `spawn_fixture("child")`
     → `pids.d/child` を `child.id()` で emit → `ready` 待ち → park
     (単一 `pids.json` write 廃止)。
   - `stubborn`/`graceful`(:102-124): `pids.d/parent` atomic emit。
     `graceful`(unix)は signal install 直後に `pids.d/parent.armed`、
     SIGTERM 捕捉時に `pids.d/parent.terminated` を atomic write。
4. `spawn_fixture`(:233-255): 戻りを `SpawnedChild` kill-on-drop
   guard へ — Drop で `try_wait()` が未終了を示す場合のみ発火(既
   exit の pid 再利用を誤 kill しない)。Windows は
   `{SystemRoot}\System32\taskkill.exe` absolute path で
   `/PID <pid> /T /F`(subtree)+ `child.kill()` belt、unix は
   direct `child.kill()`(SIGKILL — transitive は parent 死亡が
   誘発する production `kill(-pgid)` cleanup が処理)。`id()`
   passthrough を提供。`.spawn()` を context 付き `expect` へ、
   stderr を `Stdio::inherit()` へ(descendant panic が managed
   parent の capture stderr → `details.stderr.text` → `dump_failure`
   に乗る)。
5. `fixture_task`(:291-306): optional `pids_dir` 引数を追加 —
   `Some` を渡す caller(tree-family mode: stubborn/graceful/tree/
   deep)のみ `env[PID_DIR_ENV]` と `outputs: ["pids.d/**"]` を宣言
   する。`inspect`/`fail`/`output`/`touch` の caller は `None` で
   従来どおり各自の outputs を渡す(`touch` は `outputs:["out"]` +
   `write_scope{"out"}` 不変 — emit 伝達路を持たない task に
   pids.d scope を宣言しない)。
6. `tool_with`(:324-346): `process_tool_definition(options)` を返す
   variant を追加 — registry canonical `CanonicalExecute` seam
   (process/mod.rs:166-181)と同一の 'static future を得て
   `tokio::spawn` する(`tool.execute` の `&self`/`&Value` borrow は
   `'static` を満たさない)。spawn しない test は従来どおり
   `tool.execute(&input, None).await` の direct 呼出し維持。

### 2. deterministic helper 層(D4・D5・D6・D7)

wait/verify の全失敗出口を `diag_fail` に集約する。**panic 版
`wait_for`・`wait_emitted` は作らない**(上記 deviation 参照)—
`wait_until` が `false` を返した全経路で `dump_failure` → `diag_fail`
を呼ぶ。

1. `WaitCtx`/`wait_until`: `WaitCtx { mode, what, budget, poll }`。
   `wait_until(ctx, cond)` は満了時 `false` を返す非 panic 版のみ —
   `select!` 内外を問わず全 wait が同じ primitive を使い、失敗は
   caller が `diag_fail` へ送る(`select!` 内に panic する wait を
   置けない構造問題が消える)。poll cadence は file-wait 20ms /
   pid-death 100ms。
2. emit 観測: `emitted_pids(dir,keys)`(存在かつ parseable な
   `{key: pid}` 集合)・`collect_emitted(dir,keys,grace)`(全 key
   parseable か bounded grace 経過まで `wait_until` する straggler
   回収)。
3. `live_pids(want)`(:371-400): Windows は unfiltered
   `tasklist /NH /FO CSV` を poll ごとに 1 回だけ spawn し client 側で
   `want` 集合と照合 — 各行 `"` prefix 必須・`","` split・field 1 =
   bare PID(field 0 image name は `kill_emitted` の image 照合に
   共用)。`tasklist /FI` 複数併記は禁止(AND 評価で常に空)。
   nonconforming output・helper 失敗は全員 alive 扱い(conservative)。
   unix は `kill(pid,0)` + `/proc` zombie を per-pid で畳む。
   `alive` wrapper は残さず全 callsite を置換(dead helper で
   `clippy -D warnings` fail 回避)。
4. `verify_emitted_dead(ctx, emitted)`: `wait_until` で
   `live_pids(emitted).is_empty()` まで待機(DEAD_BUDGET・poll
   100ms)、残存なら live pid 一覧付きで `diag_fail`。
   `"{key} gone"`(:582/:653)の uninterpolated literal はこの集約で
   消去する。
5. `kill_emitted(dir)`: emit 済み pid の best-effort 同期 kill。
   pids.d 不存在・parse 不能は tolerate し対象 0 件で return。
   kill 対象は live ∩ emitted ∧ `current_exe` filename image 一致に
   限定。Windows は per-key `taskkill /PID <pid> /T /F`、unix は
   `parent` key があれば `kill(-(pid), SIGKILL)` group kill + 他
   emitted pid への `kill(pid, SIGKILL)` belt。
6. `TaskRun`: definition seam の 'static future `tokio::spawn` した
   JoinHandle + `AbortController` + `pids_dir`/`keys`/`mode`/`armed`
   (default `true`)を保持する guard。`TempDir` は `TaskRun` より
   **先に**宣言する(kill_emitted が pids.d を読めるうちに guard を
   発火させる宣言順ルール)。
   - `abort(reason)`: `controller.abort(AbortReason::from(
     CheaperError::new(ErrorCode::Aborted, reason)))` の wrap。
   - `settle()`: `tokio::time::timeout(SETTLE_BUDGET, &mut handle)` —
     超過時は `abort_and_drain` → `diag_fail`。
   - `abort_and_drain(why)`: `controller.abort` →
     `timeout(DRAIN_BUDGET, &mut handle)` — runtime 生存中に
     production の abort→cleanup(tree kill)を完走させる正式
     give-up 経路。give-up branch(DRAIN_BUDGET 超過)は
     `kill_emitted` + stderr note + `disarm()`(guard による二重
     kill 防止)の上 `diag_fail`。
   - `disarm()`: `armed = false` — `abort_and_drain` give-up branch
     (自身の `kill_emitted` 後)と test の post-resolution
     verification pass 後に呼ぶ。
   - `Drop`(armed のみ発火): `controller.abort(teardown)` + 同期
     `kill_emitted`。**`handle.abort()` は呼ばない**(JoinHandle
     abort は future を次 yield で drop するだけで production
     cleanup は走らず leak を悪化させる)。
7. `dump_failure(err: Option<&ToolError>, ctx)`/`diag_fail(ctx) -> !`:
   error `source()` chain・`find_cheaper_error` の details pretty
   JSON・`{mode, what/budget, elapsed, emitted set}` を stderr 採取
   し、`diag_fail` が context 付き message で panic する(dump が
   panic より先 — panic message には timeout shape
   `[{mode}] ... timed out after {budget:?} (poll {poll:?}): {what}`
   を fold)。error を持たない pure-wait failure は `err: None` で
   同じ経路を通る。`settle()`/`select!` 由来の `JoinError` は
   `ToolError` ではないため `err: None` + message/display 欄で扱う。

### 3. `cleans_live_descendants_on_exit_and_failure_and_abort_and_timeout` 変換(:587-657、D2・D3)

4 mode 逐次 loop 構造は維持(分割は peak 負荷を増やすため不採用 —
design D2)。

1. mode 別 task timeout — `exit-tree`/`fail-tree`/`deep-tree` は
   `"60s"`(emit wait 30s + abort/exit + 解決を含む fixture の観測
   可能な全 lifetime を上回る)、`deep-stubborn` は `"2s"` 維持(kill
   検証に必要な最小値)。
2. `TaskRun` で駆動 — `TempDir` を `TaskRun` より先に宣言。
3. outcome 分類表(D3):
   - `exit-tree`: `Ok`(state=completed, exitCode=0)/
     `Err` code=Timeout。
   - `fail-tree`: `Err` code=ToolExecutionFailure &&
     details.exitCode==7 && `find_cheaper_error(err).message()==
     "managed process failed"` / `Err` code=Timeout — cleanup-failed
     arm(monitor.rs:319-327)が同一 details を運ぶため message 照合
     必須。
   - `deep-tree`(abort): emit 全件後 `run.abort("outer tree
     cancellation")` → `err.to_string()=="outer tree cancellation"` /
     `Err` code=Timeout。
   - `deep-stubborn`: `Err` code=Timeout(**期待 outcome**)。
   - 早期 `Timeout` は発火位置を問わず legitimate outcome として受理
     —「期待 outcome OR Timeout」の二択。それ以外の `Err`・
     `JoinError` は `diag_fail`。
4. abort mode(deep-tree): emit 待ち(parent+child+grandchild、
   `EMIT_WAIT_BUDGET` 30s、`wait_until`)と `&mut handle` を
   `select!` — **select! 内では `wait_until` のみ使う**(panic 版
   wait は存在しない)。全 emit 到着で `run.abort(...)` → `settle()`
   → reason assert。emit stall は `abort_and_drain("emit stall")` →
   `diag_fail`。handle が先に解決すれば early resolution として分類へ。
5. per-mode emit floor(post-`collect_emitted` grace 後の emitted set
   に適用):
   - **自然解決(Ok/ToolExecutionFailure)は全 expected key の emit
     必須** — deep mode の自然解決なら `{parent,child,grandchild}` 全
     3 件が揃う(自然解決した fixture は全 emit を終えている。
     不足 = fixture が途中で死んだ = 異常 → `diag_fail`)。
   - 60s-armed mode の Timeout 解決は emit ≥1 必須 — 「60s-armed」は
     timeout 値ではなく **mode identity で指定**する
     (exit-tree/fail-tree/deep-tree が designation 上 60s-armed。
     emit は最初の観測可能行動 — 60s 生存した fixture は emit 済みの
     はず。0-emit Timeout = broken fixture/plumbing → `diag_fail`)。
   - 0-emit Timeout の受理は deep-stubborn(2s)のみ(genuine
     pre-emit kill window が残る)。
6. post-resolution verification(settle 後、early Timeout 含む全経路):
   `collect_emitted(dir,keys,POST_RESOLVE_EMIT_GRACE 5s)` →
   `verify_emitted_dead`(DEAD_BUDGET 30s・poll 100ms、残存なら live
   pid 一覧付き `diag_fail`)→ 全 verification pass 後に
   `run.disarm()`。

### 4. sibling test + `node_task_resolves_via_path` 変換

1. `times_out_native_process_and_cleans_tree`(:529-550): `stubborn`
   fixture + `pids_dir` 付き `fixture_task` + `TaskRun` + Timeout 分類
   + bounded emit 回収。受理 resolution は `Err` code=Timeout のみ
   (他は `diag_fail`)。emit 済みならその pid の消滅を
   `verify_emitted_dead` で検証、0-emit も受理(500ms mode は
   genuine pre-emit kill window あり)。`:543` の context-free
   `read_to_string(...).unwrap()` は bounded wait 付きの emitted-pid
   回収へ置き換える。
2. `abort_kills_managed_parent_and_descendant_and_preserves_reason`
   (:552-585): `tree` fixture の `pids.d/{parent,child}` 全件必須。
   emit 待ち `wait_until`(`EMIT_WAIT_BUDGET`、全 `{parent,child}`
   key)を `&mut handle` と `select!`:
   - handle が先に解決 → early resolution として `diag_fail`
     (abort test に timeout は armed されず、早期解決は異常)。
   - emit stall → `run.abort_and_drain("emit stall")` → `diag_fail`。
   - 全 key 到着 → `run.abort("stop managed tree")` → `settle()` →
     `err.to_string()=="stop managed tree"` assert →
     `verify_emitted_dead`(emit 済み 2 件の消滅)。
   `"{key} gone"`(:582)literal は `verify_emitted_dead` への置換で
   消去。
3. `allows_sigterm_exit_within_grace_before_force_kill`(:692-717、
   unix のみ): `killGrace` `1s`→`10s` + `pids_dir` 付き `fixture_task`
   + `TaskRun` + Timeout 分類 + `.armed` marker 分岐:
   - `.armed` 有 → `pids.d/parent.terminated`=="graceful" 必須 +
     emitted pid 消滅(armed なのに `.terminated` が出ない = real
     failure)
   - `.armed` 無 → kill-before-arming = acceptable early outcome
     (診断 note)
   受理 resolution は `Err` code=Timeout のみ。
4. `node_task_resolves_via_path`(:448-470): `.unwrap()` を `match`
   へ — Err 経路で `diag_fail`(exitCode/state/error chain/details
   採取 → R4 帰属判別)。受理 resolution は `Ok` exitCode=0 のみ。
   ambient `node -e` spawn のテスト意図は維持。

維持するもの(変更禁止):

- `#[tokio::test]` 非同期構造・env-driven mode 切替・`@@` sentinel・
  stdin null/stdout piped。
- `executes_literal_argv_...`・`fails_nonzero_exit_...`・
  `caps_combined_output_...`・`touch_with_matching_write_scope_...` の
  assertion 意味 — `wait_until` 化・`alive` → `live_pids` 置換に伴う
  機械的 callsite 更新のみ。
- sibling test の `timeout: "500ms"`(現行維持 — timeout 経路そのもの
  の検証)。abort test の task timeout なし(killGrace 200ms)。
- `node_task_resolves_via_path` の ambient `node` spawn(PATH 解決
  seam の e2e 検証意図)。
- 新規依存追加なし(std・`libc`・`tokio`・`serde_json`・`fastrand`
  のみ)。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/fix-proc-process-test-flake-via-test-side-determinism-issue-180.md (implementation slice)
- docs/designs/deterministic-proc-process-tests-issue-180.md (D1-D8)

## Dependencies

- docs/plans/fix-proc-process-test-flake-via-test-side-determinism-issue-180.md

## Affected Files

- crates/cheaper-tools/tests/proc_process.rs

## Done When

- [ ] `pids.d/{parent,child,grandchild}` の incremental per-key atomic
      emit が全 tree-family mode で定められた emit 順序で動作し、
      `PID_DIR_ENV` が唯一の伝達路(emit 責務 mode で env 未設定なら
      mode 名入り panic)
- [ ] `SpawnedChild` が `spawn_fixture` の戻りを包み、`try_wait()`
      gate で未終了時のみ Windows `taskkill /PID /T /F` subtree +
      `child.kill()` belt / unix direct kill する
- [ ] `fixture_task` の `pids_dir` 指定を持つ caller のみ
      `env[PID_DIR_ENV]` + `outputs:["pids.d/**"]` を宣言し、
      touch/inspect/fail/output が従来の outputs を維持している
- [ ] wait は `wait_until` のみで構成され、panic 版 `wait_for`・
      `wait_emitted`・`"{key} gone"` literal が残っていない
- [ ] `live_pids` が unfiltered tasklist CSV 照合(unix `kill(pid,0)`
      + zombie)で動作し、`alive` wrapper が残っていない
- [ ] `emitted_pids`/`collect_emitted`・`verify_emitted_dead`・
      `kill_emitted`・`TaskRun`(abort/settle/abort_and_drain/disarm/
      Drop — `handle.abort()` 非呼出し)・`dump_failure`/`diag_fail`
      が実装されている
- [ ] `cleans_live_descendants` が mode 別 timeout(60s/2s)・「期待
      outcome OR Timeout」分類表(`fail-tree` は
      `message=="managed process failed"` 照合)・abort mode の
      `wait_until`+`select!` 構造・per-mode emit floor(自然解決 =
      全 3 件、60s-armed Timeout = emit ≥1、deep-stubborn = 0-emit
      受理)・settle 後 bounded collect → `verify_emitted_dead` →
      `run.disarm()` で動く
- [ ] sibling 3 test が変換済み(times_out は 0-emit 受理 + emit 済み
      消滅検証、abort sibling は mandated select! 構造 +
      `"stop managed tree"` + 2 件消滅、allows_sigterm は
      `killGrace` `10s` + `.armed` marker 分岐)
- [ ] `node_task_resolves_via_path` の Err 経路が `diag_fail`
      (exitCode/state/error chain)を通る
- [ ] file が compile し、全 test を含む `cargo test --test
      proc_process` が pass する

## Verification

- [ ] `mise exec -- cargo test -p cheaper-tools --test proc_process` exits 0
- [ ] `mise exec -- cargo test -p cheaper-tools --test proc_process cleans_live_descendants` exits 0
- [ ] `mise exec -- cargo test -p cheaper-tools --test proc_process times_out_native_process` exits 0
- [ ] `mise exec -- cargo test -p cheaper-tools --test proc_process abort_kills_managed` exits 0
- [ ] `mise exec -- cargo test -p cheaper-tools --test proc_process node_task_resolves_via_path` exits 0
- [ ] `mise exec -- cargo fmt --check` exits 0
- [ ] `mise exec -- cargo clippy -p cheaper-tools --all-targets -- -D warnings` exits 0(`alive` 等の dead helper 残存で fail するため)
- [ ] POSIX compile smoke(Windows 開発環境では linux 向け compile のみ確認 — unix cfg 枝の構文破綻を CI 以前に検出):
      `rustup target add x86_64-unknown-linux-gnu && mise exec -- cargo check -p cheaper-tools --test proc_process --target x86_64-unknown-linux-gnu`。
      toolchain が local に install できない場合は `gh workflow run ci`(ubuntu `workflow_dispatch`)での確認を fallback とする
- [ ] Manual: `grep -n '{key} gone' crates/cheaper-tools/tests/proc_process.rs` が空を返す(supplement)
- [ ] Manual: `grep -n '/FI\|alive(' crates/cheaper-tools/tests/proc_process.rs` が `/FI` filter と `alive(` wrapper を返さない(supplement)
- [ ] Manual: `grep -n 'read_to_string' crates/cheaper-tools/tests/proc_process.rs` の生存箇所が whitelist のみ — touch の marker 照合・unix `live_pids` の `/proc/<pid>/stat` 読取・`emitted_pids`/`collect_emitted`/marker 照合(`.terminated` 内容確認を含む)の内部(supplement)
