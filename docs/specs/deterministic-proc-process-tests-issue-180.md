---
id: "SPEC-034LFaC05pNn26SBFu5KUC"
type: "spec"
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
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../adr/resolve-proc-process-test-flake-at-the-test-boundary.md"
    - "../plans/fix-proc-process-test-flake-via-test-side-determinism-issue-180.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/resolve-proc-process-test-flake-at-the-test-boundary.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/proc-process-test-flake-under-parallel-load.md"
  derived-by:
    - "../designs/deterministic-proc-process-tests-issue-180.md"
    - "../plans/fix-proc-process-test-flake-via-test-side-determinism-issue-180.md"
    - "../test-specs/deterministic-proc-process-test-assertions-for-parallel-load-flake.md"
  verifies: []
  verified-by:
    - "../test-specs/deterministic-proc-process-test-assertions-for-parallel-load-flake.md"
  references:
    - "../impl/ir/opt-in-exec-deny-patterns-issue-164.md"
    - "../impl/ir/unified-exec-grant-model-issue-165.md"
  defers: []
  deferred-by: []
---

# Deterministic proc_process tests for parallel-load flake (issue #180)

## What Should Be Built

`crates/cheaper-tools/tests/proc_process.rs` の process-tree 系 test を、
full-suite 並列負荷下の wall-clock 変動に依存しない deterministic な
構成へ修正する。対象は (a) `cleans_live_descendants_on_exit_and_failure_
and_abort_and_timeout`(:588)の mode 別 lifecycle 検証、(b) 同じ
armed-timeout race を共有する sibling test
(`times_out_native_process_and_cleans_tree` :529-550・unix のみ
`allows_sigterm_exit_within_grace_before_force_kill` :692-717)、
(c) `proc_fixture_child` fixture(test コードであり変更対象)と
`node_task_resolves_via_path`(:448)を含む同ファイルの
wait_for/alive helper 全体。加えて、failure 時に error chain・
details・mode・elapsed を採取する diagnostic を test 内に実装し、
root cause が test 固有でないと判明した場合の判断記録を行える状態にする。

## Why Now

issue #180: 2026-10-23 に 2 件の transient fail が Windows local の
`cargo test -p cheaper-tools` / targeted test 実行(full-suite 並列
負荷下)で観測された(#164/#165 IR の triage note)。単独再実行・
`mise run check` 内再実行では両件 pass するため再現条件は full-suite
並列負荷のみで、CI の検証信号として残る限り「本物の regression」か
「flake」かの判別コストが継続する。#157 で同型 flake を test 側
determinism で解消した先例があり、本件でも同じ境界の修正が読解で
確認した構造 race(discovery R1)に対して有効と分かっている。

## Users and Value

この repo で作業する developer と parent agent が受益者。`mise run
check` の process-tree test を負荷非依存にし、かつ万が一再発しても
failure diagnostic が self-explaining になることで、triage のたびに
並列実行を当てにせず root cause 判別へ進める。

## Scope

### In Scope

- `proc_fixture_child` fixture(test ファイル内の test コードであり、
  本 spec の変更対象):
  - deep mode(`exit-tree`/`fail-tree`/`deep-tree`/`deep-stubborn`)の
    pid 通知を単一 `pids.json` の全完備 write から **incremental emit**
    へ変える — 例えば `pids.d/{parent,child,grandchild}` の per-key
    file を各 stage が pid を知った時点で atomic(tmp file + rename)
    に書く。parent は起動直後に自身の pid を書き、branch は child /
    grandchild を spawn 次第それぞれ書く。test は file ごとに
    parseable な内容が出るまで待つ。
  - unix `graceful` mode に SIGTERM handler install 直後を示す
    "armed" marker(例: `{pid_path}.armed`)を追加する。
- `cleans_live_descendants_on_exit_and_failure_and_abort_and_timeout`(:588):
  - task `timeout` を mode 別に分離し、timeout を期待しない mode
    (exit/fail/abort)では task timeout が fixture の **観測可能な全
    lifetime**(spawn chain 準備 → per-key emit → exit/park)を上回る
    値にする — "fixture 準備分だけ余裕を持たせる"は不十分で、emit・
    exit 待ちも同じ clock に乗る。timeout mode のみ kill 検証に必要な
    最小値。
  - 早期解決(`pending` が全 pid emit 前に `Timeout` で解決)を異常
    ではなく期待 outcome として扱う判定へ直す。早期 `Timeout` 後は
    emitted artifact を bounded post-resolution wait で回収し、
    **emit 済みの全 pid が死んでいることを検証する**。未 spawn の
    stage はそもそも何も emit しないため検証対象が自然に 0 件になり、
    skip や fudge にはならない — 早期 `Timeout` の受理と descendant
    検証の不弱化は incremental emit で両立させる。
  - 4 mode 逐次 loop 構造は維持してよい(分割は任意 — peak 負荷を
    増やさないため)。
- 同じ armed-timeout-during-prep race(timeout が `drive()` 冒頭で
  spawn 直後に arm される — monitor.rs:225・proc/mod.rs:328/425)を
  持つ sibling test:
  - `times_out_native_process_and_cleans_tree`(:529-550、
    `timeout: "500ms"`): 早期 `Timeout` を legitimate な outcome と
    して受理し、`:543` の context-free `read_to_string(...).unwrap()`
    を bounded wait 付きの emitted-pid 回収へ直す — pid file が emit
    されていればその pid の消滅を検証し、emit 前に kill されていた
    場合は検証対象なし(未 emit で panic しない)。
  - `allows_sigterm_exit_within_grace_before_force_kill`(:692-717、
    unix のみ、`timeout: "500ms"`): 同様に早期 `Timeout` を受理。
    fixture の "armed" marker で handler install 済みを区別し、
    arming 前に kill されたケースは acceptable early outcome、
    armed なのに `.terminated` が出ないケースは従来どおり real
    failure とする。
- `wait_for`/`alive` helper:
  - 待機 budget の余裕確保と、poll 集約または cadence 緩和。Windows
    `alive()` の集約に **`tasklist /FI "PID eq X"` filter を複数
    並べる方式は使わない** — 複数 `/FI` filter は AND 評価され常に
    空を返すため、`alive()` が vacuous に false 化して「何も検証せず
    pass」になる。代わりに **unfiltered な `tasklist /NH`(`/FO CSV`
    可)snapshot を poll ごとに 1 回だけ**取り、監視対象の pid 集合
    全体を client 側で照合するか、poll cadence を緩和する。
  - `wait_for` に context 引数(mode / key / budget 等)を持たせ、
    panic message が失敗の帰属を自明に示すようにする — 現行の
    `"{key} gone"`(:582/:653)は interpolation されない literal で、
    panic には mode も key も出ない。
  - failure-relevant 経路の context-free `unwrap()` を diagnostics
    付きの `expect` / helper 呼出しへ置き換える(pid file の read /
    parse、`alive()` の `.output()`、`spawn_fixture` の `.spawn()` 等)。
- failure diagnostics: unwrap/panic 前に error の `source()` chain・
  `find_cheaper_error` の details JSON・観測 mode・経過時間を stderr
  出力する helper を同ファイルに追加し、process-tree 系 test の failure
  経路(panic 経路を含む)で必ず呼ぶ。
- 上記の修正で flake が再発した場合に備え、diagnostic 出力から tool 側
  帰属か test 側帰属かを判別できる粒度を保つこと。

### Out of Scope

- `crates/cheaper-tools/src/tools/proc/`(`run`/`drive`/`cleanup`/
  `tree`/`spawn`/`executable`)の本番実装変更 — helper 増幅への
  tool 側 hardening は diagnostic で tool 側帰属が確認された場合に
  別 issue で行う(Deferred Design Concerns)。
- `node` の ambient PATH 解決自体の変更 — `node_task_resolves_via_path`
  が PATH 解決 seam を end-to-end で覆う意図は維持する。
- 他 test file・CI 構成(runs-on・retry・OS matrix)の変更。
- 上記 3 test 以外の process 系 test の assertion 意味の変更(同等の
  wait_for/diagnostics 修正は許容)。

## Requirements

- mode 別 outcome(exit=exitCode 0・fail=exitCode 7・abort=outer
  cancellation・timeout=Timeout code)と descendant 消滅
  (parent/child/grandchild の非稼働)の既存検証を弱めない —
  修正は timing 依存の除去であり、検証対象の lifecycle invariant は
  保持する。早期 `Timeout` を受理しても **emit 済み pid の消滅検証は
  全件行う**(未 spawn の stage は emit 自体が無いため免除にも
  skip にもならない)。
- fixture 準備(3 段 spawn chain → per-key pid emit)が task timeout と
  同じ clock で競合しない構造にする。timeout を期待しない mode では
  **task timeout が fixture の観測可能な全 lifetime(prep → emit →
  exit/park)を上回る**こと — 「prep の分だけ積む」は不十分。
- sibling test(`times_out_native_process_and_cleans_tree`・
  `allows_sigterm_exit_within_grace_before_force_kill`)でも同構造を
  適用し、早期 `Timeout` 後の emitted-artifact 検証を bounded wait で
  行う。後者は "armed" marker で kill-before-arming と
  armed-but-no-`.terminated` を区別する。
- 全 `wait_for` panic は「どの mode/test の何を何秒待ってどうなら
  なかったか」を含む message で失敗し(`"{key} gone"` 型の
  uninterpolated literal を残さない)、加えて failure diagnostics
  helper が error chain・details・mode を採取する。
- `node_task_resolves_via_path` は ambient `node` spawn という
  テスト意図を維持しつつ、失敗時に `exitCode`/`state`/error chain を
  記録できる形にする(再発時に exit 255 の帰属が判別できること)。
- 修正は `#[tokio::test]` の非同期構造・`proc_fixture_child` の
  env-driven mode 切替・`@@` sentinel という既存 pattern に従う。

### Residual Race Vectors and Dispositions

コード読解で確認した flake 要因の全量と、本 spec での処置:

- (a) `Path::exists()` → `read_to_string()`/`parse` の TOCTOU
  (exists 通過後に未完備の file を読んで panic)— **In Scope の
  incremental emit で解消**: per-key file を tmp+rename で atomic に
  書き、test は parseable な内容が出るまで file ごとに待つ。
- (b) context-free `unwrap()` と uninterpolated `"{key} gone"` —
  **In Scope の diagnostics/context 要件で解消**: panic 経路に
  mode/key/budget の context を届ける。
- (c) `alive()` の PID reuse 無検査 — test 側 `alive()` は
  creation-time guard を持たない(production 側は `tree.rs:354-364`
  の `born_before` 照合 + `:429` の `now_ms()` 起点で PID reuse を
  防いでいる)。`tasklist` には安価な born-time oracle がなく、test
  側で同等の照合を実装するコストに見合わないため **accepted
  residual** とする — wait budget で bounded される誤判定リスクと
  して残余受容し、必要なら phase-2 hardening で再検討する。
- (d) panic mid-`drive()` による fixture process leak — test 側で
  panic(assert/wait_for 失敗)して `pending` を drop しても managed
  process は kill されず(`kill_on_drop` 無し)、park 中の fixture が
  最大 3 process(parent/child/grandchild)残留する。fixture の cwd
  は TempDir のため Windows では `remove_dir_all` も失敗する。
  **containment を必須とする** — 例: `execute` を JoinHandle 上で
  駆動し panic 時に drop guard が controller を abort して abort
  経路の tree kill を働かせる、`spawn_fixture` の子に kill-on-drop
  guard を付ける、等。

## Acceptance Criteria

- [ ] flake の root cause が特定され記録されている —
  `docs/discovery/proc-process-test-flake-under-parallel-load.md` に
  `cleans_live_descendants` の構造 race(task timeout が fixture の
  観測可能な全 lifetime を巻き込む)が confirmed として記録され、
  exit 255 の帰属仮説(tree helper vs node.exe)とその判別手段も
  記録されている。
- [ ] transient fail が再発しないことを確認する修正 or 根因が test
  固有でないと判明した場合の判断記録 — test 側の timing 依存が除去
  され、failure diagnostics が error chain/details/mode を採取する。
  diagnostics が tool 側帰属(例: `"process tree helper failed"` 由来
  cleanup-failed)を示した場合は判断記録を docs へ追記し tool 側
  follow-up issue 化の方針を残す。
- [ ] 修正した場合 `mise run check` 連続実行で flake 非再現を確認 —
  少なくとも 3 連続の `mise run check` で対象 test が失敗しないこと。
  加えて `mise exec -- cargo test -p cheaper-tools --test proc_process`
  の targeted 実行が pass すること。ただし 3 連続 green は ~1 件/日
  の flake rate に対する **smoke signal であって証明ではない**。
  node exit-255 leg についての success は「次回再発時に diagnostic
  出力が帰属(tool 側 helper vs ambient node)を特定できること」で
  あり、条件付き phase-2 への分岐は follow-up issue 経由で行う。
  phase-2 着手前の再発は expected-tolerated として記録する。

## Deferred Design Concerns

- `WinTree::terminate` の helper 連鎖(eager CIM snapshot・poll ごとの
  powershell spawn・stale snapshot への stale-row 対処)を軽量化する
  tool 側 hardening — 本 spec scope では diagnostic による帰属判別を
  優先し、production semantics を先走って変えない。diagnostic が tool
  側帰属を示した時点で別 issue として再検討する。
- `ProcessTree` 自体の production 利用側での cost(eager snapshot が全
  execute に乗る)— 同じく tool 側帰属確認後の論点。

## Sources and Relations

- issue #180: <https://github.com/scarecrow173/cheaper/issues/180>
- 原因調査: `docs/discovery/proc-process-test-flake-under-parallel-load.md`
- 修正境界の選択記録:
  `docs/adr/resolve-proc-process-test-flake-at-the-test-boundary.md`
- 失敗観測: `docs/impl/ir/unified-exec-grant-model-issue-165.md`・
  `docs/impl/ir/opt-in-exec-deny-patterns-issue-164.md` の triage note
- 前例: `docs/specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md`
