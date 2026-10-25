---
id: "IMPL-034LKRxcfODmOipPN5bAKx"
type: "impl"
status: "completed"
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
    - "docs/tasks/implement-pids-d-emit-protocol-and-test-helper-layer.md"
  implemented-by: []
  depends-on:
    - "docs/plans/fix-proc-process-test-flake-via-test-side-determinism-issue-180.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/deterministic-proc-process-test-assertions-for-parallel-load-flake.md"
    - "docs/discovery/proc-process-test-flake-under-parallel-load.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/fix-proc-process-test-flake-via-test-side-determinism-issue-180.md"
    - "docs/designs/deterministic-proc-process-tests-issue-180.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/specs/deterministic-proc-process-tests-issue-180.md"
    - "docs/adr/resolve-proc-process-test-flake-at-the-test-boundary.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement deterministic proc_process test rewrite

## Summary

issue #180(full-suite 並列負荷下の process-tree 系 transient fail)に対し、
DESIGN-034LGDPeqMOtv5azFEm4Ei(D1-D8)どおり
`crates/cheaper-tools/tests/proc_process.rs` を単一の atomic な変更として
確定形へ着地させた。変更は test file のみで、`src/tools/proc/` の本番
実装は一切変更していない(ADR Option A — test 側 boundary での修正)。

主な構成:

- fixture emit protocol(D1): `pids.d/{parent,child,grandchild}` の
  incremental per-key atomic emit(`{key}.tmp` → `fs::rename`)。伝達路は
  `CHEAPER_PROC_PID_DIR`(`PID_DIR_ENV`)env のみで、argv の pid path 契約
  (deep/tree/stubborn/graceful の `args.first()` pid path)は全廃。emit
  責務 mode で env 未設定なら mode 名入り panic(fail-fast)。
- deterministic helper 層(D4/D5/D7): 非 panic `wait_until` +
  `dump_failure`/`diag_fail` への全 failure 出口集約、`emitted_pids`/
  `collect_emitted`/`verify_emitted_dead`/`kill_emitted`/`live_pids`
  (Windows unfiltered `tasklist /NH /FO CSV` snapshot 照合、unix
  `kill(pid,0)` + `/proc` zombie)。
- panic-path leak containment(D6): `TaskRun` guard(canonical
  `CanonicalExecute` seam の 'static future を `tokio::spawn`、
  armed/disarm・`settle`/`abort`/`abort_and_drain`・Drop で
  `controller.abort` + 同期 `kill_emitted`、`handle.abort()` 非呼出し)
  と fixture 側 `SpawnedChild` kill-on-drop(`try_wait()` gate)。

## Context

TASK-034LIwmO9NxUouuOoCwC0g。PLAN-034LIvc25ZGY86L0WGpFmy /
DESIGN-034LGDPeqMOtv5azFEm4Ei / SPEC-034LFaC05pNn26SBFu5KUC /
ADR(test boundary)に従う。flake の構造 race は
DISC-034LFaxMQ43rlWH4Tn8KS5 で確定済み(R1: task timeout が fixture の
観測可能な全 lifetime を巻き込む、R5: `alive()` の poll ごとの tasklist
spawn self-load、panic 経路の context 欠如、panic mid-`drive()` leak)。

## Implementation

### Changes

`crates/cheaper-tools/tests/proc_process.rs`(717 行 → ~1370 行の全面
再構成):

- const 群に `PID_DIR_ENV`・budget/poll 定数(`EMIT_WAIT_BUDGET` 30s・
  `POST_RESOLVE_EMIT_GRACE` 5s・`DEAD_BUDGET` 30s・`SETTLE_BUDGET` 180s・
  `DRAIN_BUDGET` 120s・`FILE_POLL` 20ms・`PID_POLL` 100ms)を追加。
- fixture emit protocol:
  - `emit_pid(dir,key,pid)`(atomic tmp+rename、初回 `create_dir_all`)と
    unix `emit_marker(path,text)`(`{path}.tmp-<pid>` → rename)。
    `write_pid` は廃止。
  - `pid_dir(mode)` — `env::var(PID_DIR_ENV)` が唯一の伝達路。未設定・
    空値は mode 名入り panic。
  - deep 4 mode: arm 先頭で `pids.d/parent` emit(`ignore_sigterm` より前)
    → `spawn_fixture("branch")` → `@@` 行待ち(内部同期として維持)→
    exit/park。`pids.json` 一括 write と ids parse は廃止。
  - `branch`: entry で `pids.d/child` emit → `spawn_fixture("child")` →
    `leaf.id()` で `pids.d/grandchild` を spawn 返却直後に emit(`ready`
    待ちより先)→ `ready` 待ち → `@@{child,grandchild}` 行 → park。
  - `tree`: `pids.d/parent` emit → `spawn_fixture("child")` →
    `child.id()` で `pids.d/child` emit → `ready` 待ち → park。
  - `stubborn`/`graceful`: `pids.d/parent` atomic emit。`graceful`(unix)
    は signal install 直後に `pids.d/parent.armed`、SIGTERM 捕捉時に
    `pids.d/parent.terminated` を atomic write。
- `spawn_fixture` は `SpawnedChild` kill-on-drop guard を返す
  (Drop で `try_wait()` が未終了を示す場合のみ発火 — Windows は
  `{SystemRoot}\System32\taskkill.exe` absolute path の
  `/PID <pid> /T /F` subtree + `child.kill()` belt、unix は direct
  `kill()`)。`.spawn()` は mode 名入り expect 化、stderr は
  `Stdio::null()` → `Stdio::inherit()`(descendant panic が managed
  parent の capture stderr → `details.stderr.text` → `dump_failure` に
  乗る)。
- `fixture_task(mode, args, outputs, pids_dir: Option<&Path>)`:
  `Some` の caller(tree-family mode)のみ `env[PID_DIR_ENV]` と
  `outputs: ["pids.d/**"]` を宣言。inspect/fail/output/touch は `None`
  で従来 outputs を維持。
- `tool_options`(`tool_with` から options 構築を分離)+ `TaskRun::spawn`
  が `process_tool_definition(options)` の `(def.execute)(input,
  Some(controller.signal()))` で 'static future を取得して
  `tokio::spawn`(registry の `CanonicalExecute` seam と同一経路)。
- wait/verify/diag 層:
  - `WaitCtx { mode, what, budget, poll, started, pids_dir, keys }`。
    `wait_until(ctx, cond)` は `false` を返す非 panic 版のみ — panic 版
    `wait_for`/`wait_emitted` は作らず、全失敗経路を `dump_failure` →
    `diag_fail` へ集約(task doc の justified deviation どおり)。
  - `emitted_pids`/`collect_emitted`(bounded straggler 回収)・
    `verify_emitted_dead`(集合 budget + live pid 一覧付き diag)。
  - `live_pids`: Windows は unfiltered `tasklist /NH /FO CSV` を poll
    ごと 1 回 spawn し client 側照合(`"` prefix・`","` split・
    field 1 = bare PID、field 0 = image name)。filter 併記は禁止。
    nonconforming output・helper 失敗は conservative に全員 alive 扱い。
    unix は per-pid `kill(pid,0)` + `/proc` zombie。`alive` wrapper は
    全 callsite 置換で廃止。
  - `kill_emitted`: live ∩ emitted ∧ `current_exe` filename image 一致に
    限定。Windows は per-pid absolute `taskkill /PID <pid> /T /F`、unix
    は `parent` key があれば `kill(-(pid), SIGKILL)` group kill + 他
    emitted pid の `kill(pid, SIGKILL)` belt。
  - `dump_failure(err: Option<&ToolError>, ctx)`: `source()` chain・
    `find_cheaper_error` の code/details pretty JSON・pids.d entries・
    emitted set を stderr 採取。`diag_fail(ctx) -> !` が context 付き
    message で panic。
- test 変換:
  - `cleans_live_descendants_...`: mode 別 timeout(exit/fail/deep-tree
    `"60s"`、deep-stubborn `"2s"`)・`TaskRun` 駆動(TempDir 先行宣言)・
    outcome 分類表(期待 outcome OR code=Timeout — fail-tree は
    `message=="managed process failed"` 照合で cleanup-failed arm を
    区別)・deep-tree abort mode は `wait_until` emit 待ちを
    `&mut run.handle` と `select!`(handle 先解決は early resolution、
    emit stall は `abort_and_drain` → `diag_fail`)・per-mode emit
    floor(自然解決 = 全 3 key、60s-armed Timeout = emit ≥1、
    deep-stubborn の 0-emit Timeout は受理)・post-resolution で
    `collect_emitted` → `verify_emitted_dead` → `run.disarm()`。
  - `times_out_native_process_and_cleans_tree`: `stubborn` + `pids_dir` +
    `TaskRun` + Timeout 分類のみ受理 + 0-emit 受理(500ms mode の
    genuine pre-emit kill window)+ emit 済み消滅検証。旧 `:543` の
    context-free `read_to_string().unwrap()` は除去。
  - `abort_kills_managed_parent_and_descendant_and_preserves_reason`:
    `tree` fixture の `pids.d/{parent,child}` 全件 emit を `wait_until` で
    handle と `select!` → `run.abort("stop managed tree")` → `settle` →
    reason assert → emit 済み 2 件の消滅検証。`"{key} gone"` literal は
    `verify_emitted_dead` への置換で消去。
  - `allows_sigterm_exit_within_grace_before_force_kill`(unix):
    `killGrace` `1s`→`10s` + `pids_dir` + `TaskRun` + Timeout 分類 +
    `.armed` marker 分岐(armed 有 → `parent.terminated`=="graceful" 必須
    + emitted 消滅、armed 無 → kill-before-arming 受理)。
  - `node_task_resolves_via_path`: `.unwrap()` を `match` へ — Err 経路は
    `dump_failure` → `diag_fail`(exitCode/state/error chain/details
    採取で R4 帰属判別)。受理 resolution は `Ok` exitCode=0 のみ。

### Decisions During Implementation

- panic 版 wait helper(`wait_for`/`wait_emitted`)は作らない — task doc
  の justified deviation(design wording の内部矛盾を task 側で修正済み)。
  `wait_until` + `dump_failure`/`diag_fail` を authoritative 形とした。
- `emit_marker` は `#[cfg(unix)]` のみ — `.armed`/`.terminated` marker は
  unix `graceful` arm だけが使うため、Windows build の `dead_code`
  lint(`clippy -D warnings`)を避けるために cfg 化した。
- `TaskRun.keys` は `WaitCtx.keys` へ流し、`dump_failure` の emitted
  listing が対象 key 集合を正確に出すようにした(field 未使用警告回避も
  兼ねる)。
- `fixture_task` の `outputs` 引数は `pids_dir: Some` のとき
  `["pids.d/**"]` で上書きされる(tree-family caller は `&[]` を渡す)。
  emit 伝達路を持たない task へ pids.d scope を宣言しない原則を維持。
- unix `kill_emitted` の image 照合は linux `/proc/<pid>/exe` link の
  filename 一致。oracle が無い他 unix は emitted ∧ live 限定で残余受容
  (spec residual (c))。read_link 失敗は fail-closed(false)。
- `diag_fail` の panic message は task doc pin の timeout shape
  `[{mode}] {what}: timed out after {budget:?} (poll {poll:?});
  elapsed {elapsed:?}` を全 failure 出口で統一使用 — 非 wait 失敗では
  `what` が失敗性質を記述し dump が実値(result/error chain)を示す。

## Related Experiments

### Adopted

なし(known-solution task — design D1-D8 が protocol・signature・
budget 値・分類表まで確定済みのため Experiment Log は未作成)。

### Rejected

なし。

## Validation

- `mise exec -- cargo fmt -p cheaper-tools -- --check`: pass
- `mise exec -- cargo clippy -p cheaper-tools --all-targets -- -D warnings`:
  pass(dead helper なし)
- `mise exec -- cargo test -p cheaper-tools --test proc_process`:
  9 passed / 0 failed(2 連続実行で green、~31-36s)
- targeted filters(全て pass):
  `times_out_native_process`(7.03s)・`abort_kills_managed`(6.81s)・
  `node_task_resolves_via_path`(2.01s)・`cleans_live_descendants`
  (full-run 内で pass)
- POSIX compile smoke: `rustup target add x86_64-unknown-linux-gnu` +
  `mise exec -- cargo check -p cheaper-tools --test proc_process --target
  x86_64-unknown-linux-gnu`: pass(unix cfg 枝 — emit_marker/unix_live/
  unix_image_matches/kill(-pgid)/graceful arm — の構文を linux 向け
  compile で検証)
- narrowed greps(全て clean):
  - `grep -n '{key} gone' proc_process.rs` → 0 件
  - `grep -n '/FI\|alive(' proc_process.rs` → 0 件(`/FI` の doc comment
    言及も除去済み)
  - `grep -n 'read_to_string'` → whitelist のみ(read_stdin の stdin
    read・`emitted_pids` 内部・unix `/proc/<pid>/stat`・touch marker
    照合・`.terminated` 内容確認)

## Risks

- **早期 `Timeout` 受理の mask 可能性**(design Risks): 60s-armed mode の
  emit ≥1 floor と自然解決の全 key 要求で vacuous-pass を防ぐ構造は
  design どおり。再発時は `dump_failure` の採取情報で帰属判別。
- **PID-reuse / image oracle の無い環境**(spec residual (c)): `live_pids`
  は creation-time guard を持たず、kill 側は image 一致 + `try_wait()`
  gate で window を狭める。同 image で再利用された pid は killable のまま
  — documented, not fixed。
- **pre-emit panic の parent leak ≤1 process**(D6 residual): emit-first
  配置で構造的に最小化済み。`abort_and_drain` が主機構として全 window
  を覆う。
- `cleans_live_descendants` の wall-clock 所要は mode あたり最大
  ~数十秒(deep-stubborn の 2s timeout + Windows cleanup worst ~90s の
  settle/drain budget が支配)— full-suite 所要時間が伸びる点は受容。

## Follow-ups

followup-triage 分類: `followup-terminal` — 本 task 由来の未処理
follow-up はない。

- `WinTree::terminate` helper 連鎖軽減・eager snapshot cost・test 側
  born-time 照合は spec/design の Deferred Design Concerns(conditional
  phase-2、別 issue)— diagnostics が tool 側帰属
  (`"process tree helper failed"`/`"managed process cleanup failed"` 系)
  を示した時点で再検討する運用。
- 3 連続 `mise run check` による flake 非再現確認は後続 task
  `verify-proc-process-determinism-and-mise-check` の責務(本 task の
  scope 外)。
