---
id: "DISC-034LFaxMQ43rlWH4Tn8KS5"
type: "discovery"
status: "resolved"
title: "proc_process.rs process-tree test flake under parallel load (issue #180)"
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
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "ubuntu-ci-route-test-flake.md"
    - "../specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
    - "../adr/resolve-route-test-flake-via-test-side-determinism.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/deterministic-proc-process-tests-issue-180.md"
    - "../adr/resolve-proc-process-test-flake-at-the-test-boundary.md"
    - "../designs/deterministic-proc-process-tests-issue-180.md"
  verifies: []
  verified-by: []
  references:
    - "../impl/ir/opt-in-exec-deny-patterns-issue-164.md"
    - "../impl/ir/unified-exec-grant-model-issue-165.md"
  defers: []
  deferred-by: []
metadata:
  discovery:
    question: "proc_process.rs の process-tree 系 test が並列負荷下で断続失敗する原因を特定し、閾値引上げ/polling 改善/構造分離のどれが根本解決かを決めることができるか"
    scope: "node_task_resolves_via_path(helper exit 255)と cleans_live_descendants_on_exit_and_failure_and_abort_and_timeout(timeout)の 2 件、および両者が共有する proc::run/drive/tree の managed-process lifecycle 経路"
    outcome: "部分確定 — cleans_live_descendants は task timeout: 2s が fixture の観測可能な全 lifetime を巻き込む test 側構造 race と読解で確定(観測 instance の race 間帰属は panic message 未採取で未確定)。node exit 255 は tree helper 失敗(cleanup-failed)が最有力だが diagnostic 未採取で未確定のため、test 側 determinism + failure diagnostics で再発時に自明化する方針"
---

# proc_process.rs process-tree test flake under parallel load (issue #180)

## Exploration Goal

`crates/cheaper-tools/tests/proc_process.rs` の process-tree 系 test が
full-suite 並列負荷下で 1 回ずつ transient fail した 2 件の観測(issue #180、
#164/#165 implementation record の triage note 由来)について、実コードを
読んで race window を特定し、閾値引上げ・polling 改善・構造分離のどの
修正クラスが根本解決になるかを判断材料として整理するのが目的。#157
(mtime tie-break / 1ms timeout race)と同様に「test が制御していない外部
状態を観測している」型の flake か、本番側の欠陥かを切り分ける。

## Key Issues and Assumptions

### 観測された失敗(記録済み事実)

- `node_task_resolves_via_path`(proc_process.rs:448)— 2026-10-23、
  `cargo test -p cheaper-tools` 並列負荷下で「helper exit 255」の transient
  fail 1 回(unified-exec-grant-model-issue-165.md triage note)。単独再実行・
  `mise run check` 内再実行は pass。failure 時の error chain / details は
  未採取。
- `cleans_live_descendants_on_exit_and_failure_and_abort_and_timeout`
  (proc_process.rs:588)— 同日、full-suite 負荷下で timeout 1 回
  (opt-in-exec-deny-patterns-issue-164.md triage note)。同じく再実行では
  pass。
- 両件とも Windows local での観測 — `cargo test -p cheaper-tools` /
  targeted test 実行を full-suite 並列負荷下で行った際の transient
  fail。`mise run check` 内再実行は pass しており、こちらは失敗では
  なく pass した再実行の context。CI(Ubuntu)での観測記録は現時点で
  ない。

### test 側の構造(confirmed — コード読解)

- `wait_for(cond, timeout, what)` は `tokio::time::sleep(20ms)` polling で
  wall-clock 超過時に `panic!("wait_for timed out: {what}")`
  (proc_process.rs:360-368)。
  使用 budget は pids.json 待ち 15s・pid 消滅待ち 20s。
- `alive(pid)` の Windows backend は **poll ごとに `tasklist.exe /FI`
  subprocess を同期 spawn** する(proc_process.rs:371-380)。20ms 周期の
  待機中は秒間最大 ~50 個の tasklist spawn を発生させ、それ自体が full-suite
  の ambient 負荷を増幅する。
- `node_task_resolves_via_path`(:448)は `tool_with` → `pin_exec_grants`
  (exec/mod.rs:139-153)→ `resolve_process_executable`(`search_path: None`
  = ambient PATH、exec/mod.rs:171-180・executable.rs:104-141)で bare
  `node` を **pin 時点で realpath 済み絶対 path へ解決**する。したがって
  PATH 探索の揺らぎは pin 後の spawn には入らない — exit 255 が出るとすれば
  pin 済み `node.exe` の実行自体か、spawn 後の tree cleanup 経路である。
- `cleans_live_descendants`(:588)は exit/fail/abort/timeout の 4 mode を
  1 test 内で逐次 loop し、mode ごとに TempDir + 3 段 fixture tree
  (parent=test binary re-exec → `branch` → `child`、各段が
  `proc_fixture_child` の libtest init 付き process spawn)を張る。
  task 設定は全 mode 共通で `timeout: "2s"`、`killGrace: "100ms"`。
- panic 経路の context 欠如: `wait_for` の panic message は `what`
  のみで mode を含まず、pid 消滅待ちの `"{key} gone"`(:582/:653)は
  interpolation されない literal。pids.json の read/parse
  (`Path::exists()` 通過後に `read_to_string().unwrap()` — :573/:625、
  exists→parse の TOCTOU あり)、`alive()` の `.output()`(:376)、
  `spawn_fixture` の `.spawn()`(:252)も context-free `unwrap()` の
  ため、failure の mode/key 帰属が panic からは取れない。

### runtime 側の構造(confirmed — コード読解)

- timeout clock は `drive()` 冒頭の `sleep(timeout_ms)`(monitor.rs:225)で、
  `command.spawn()` **成功後**に arm される(mod.rs:328,425)。つまり
  `timeout: "2s"` は parent 自体の CreateProcess こそ含まないが、fixture
  の **観測可能な全 lifetime**(branch spawn → child spawn → `ready`
  行 → `@@` 行 → pids.json write → exit/park)の全遅延を巻き込む。
- `create_process_tree(child.id())` は spawn 直後に呼ばれ(monitor.rs:223)、
  Windows では `WinTree::new` が **eager に `powershell.exe -NoProfile
  Get-CimInstance Win32_Process` snapshot を background task として起動**
  する(tree.rs:434-438)。短時間で終わる task(node `-e` 等)でも全 execute
  が最低 1 回の powershell CIM 起動を支払う。
- `start_cleanup` は abort/timeout だけでなく **通常 exit でも必ず**
  `tree.terminate(kill_grace_ms)` を走らせる(monitor.rs:263-266、
  cleanup.rs:16-30)。
- `WinTree::terminate`(tree.rs:465-499)は初回 snapshot await →
  `collect_snapshot`(powershell 新規 spawn)→ `taskkill /T` → grace loop
  (50ms 周期で `collect_snapshot` 再取得)→ `taskkill /T /F` → 最終
  `collect_snapshot`。1 execute あたりの powershell CIM snapshot は
  eager `WinTree::new`(:434-438)+ terminate 冒頭の
  `collect_snapshot`(:473)で **最低 2 回**、live tree の typical
  terminate(grace loop :481-491 + 最終 :493)で **~4-5 回**、
  `kill_verified` の retry 用再 snapshot(:451-463)まで含めると
  **~7+ 回**、taskkill は **1〜2 回**の subprocess 連鎖になる。各
  helper は `HELPER_TIMEOUT_MS = 10s` deadline + 1MB output cap 付き
  (tree.rs:17-19,147-244)。
- helper(powershell/taskkill)が非 0 exit すると `"process tree helper
  failed ({code})"` → `cleanup_error` → `Err(ToolExecutionFailure
  "managed process cleanup failed")` で終了する(monitor.rs:319-327)。
  `kill_verified`(tree.rs:451-463)は taskkill 失敗を再 snapshot が空の
  場合だけ成功扱いにするが、**CIM snapshot の staleness で dead pid が
  残っていると error が伝播する** — 負荷下で WMI/CIM の反映遅延が
  起きうる経路。
- POSIX backend は `kill(-pid)` signal + `/proc` zombie 走査のみで
  subprocess helper を持たない(tree.rs:29-117)。helper 増幅は
  **Windows 固有**であり、2 件とも Windows local 観測という非対称と一致する
  (#157 discovery でも同じ非対称が確認済み)。

### 特定した race window

- **R1(confirmed): task timeout が fixture の観測可能な全 lifetime を
  巻き込む。** `cleans_live_descendants` は mode ごとに test binary を
  3 連鎖で spawn し、`timeout: "2s"` は `drive()` 冒頭で arm されて
  から fixture の spawn chain 準備・pids.json write・exit/park まで
  全てに乗る。発火位置で panic 形が変わる:
  (a) pids.json write 前に発火 → abort/timeout mode では `pending`
      が先に解決して `assert!(matches!(expected, "ok"|"fail"))` の
      "fixture died early" panic(:617-620)、exit/fail mode では
      pids.json 未作成の `read_to_string().unwrap()` panic(:624-625);
  (b) write 後・自然 exit 前に発火(exit/fail mode)→ `pending` が
      Timeout で解決し wrong-error panic — expected "ok" 側は
      `panic!("{other} expected success")`(:648)、expected "fail"
      側は `assert_eq!(code(&err), …ToolExecutionFailure)` の
      mismatch(:642);
  (c) test 側 abort 前に発火(deep-tree)→ write 前なら (a) の
      died-early、write 後なら `assert_eq!(err.to_string(),
      "outer tree cancellation")` の reason-mismatch(:646)。
  deep-stubborn(timeout 期待)では write 後の発火が期待 outcome で、
  write 前発火は (a) の died-early になる。これは確率的でなく
  **構造的に組み込まれた race** で、load があるほど発火しやすい。
  なお同じ armed-timeout-during-prep race は sibling test にも存在する
  — `times_out_native_process_and_cleans_tree`(:529-550、
  `timeout: "500ms"` + `:543` の `read_to_string().unwrap()`)と
  unix の `allows_sigterm_exit_within_grace_before_force_kill`
  (:692-717、`timeout: "500ms"`、`.terminated` marker 待ち)は、
  timeout が fixture の emit/handler-install を待たずに発火しうる
  同一構造を持つ。
- **R2(confirmed): `wait_for(pids.json, 15s)` は上記 chain + eager CIM
  snapshot と競合する。** fixture chain 自体に加え、`ProcessTree` の初回
  powershell snapshot が execute と並走して CPU/IO を食う。
- **R3(confirmed): `wait_for(!alive(pid), 20s)` は terminate の helper
  連鎖と直列する。** `killGrace: "100ms"` の下、各 `collect_snapshot` が
  負荷で数百 ms〜秒かかると grace loop は即座に打ち切られ force kill に
  進むが、terminate 全体(初回 await + snapshot×n + taskkill×2)は負荷次第で
  秒〜十数秒かかりうる。pid 消滅待ちは key ごとに **逐次** の
  `wait_for`(:651-655)で、各 key が ~20ms 周期で tasklist を spawn
  するため待機中は概ね 20ms ごと 1 回の tasklist spawn が続く。
  20s budget の余裕は小さい。
- **R4(未確定): exit 255 の帰属。** (a) tree helper(powershell/taskkill)の
  非 0 exit → `"process tree helper failed (255)"` → cleanup-failed、
  (b) pin 済み `node.exe` が負荷下の resource pressure(handle/memory/AV
  介入)で起動自体に失敗し exit 255。diagnostic が無いため現行記録からは
  判別不能。再現時に error chain を採取する instrumentation が前提になる。
- **R5(寄与要因): test 自身が負荷源。** `alive()` の tasklist poll(最大
  ~50 spawn/s/待機)が full-suite の ambient 負荷を増やし、R1-R3 の
  window を押し広げる(heartbeat emit は policy default 30s interval —
  proc/mod.rs:402-423 — で ≤20s test あたり高々 ~1 emit のため負荷寄与は
  negligible と判断し、列挙から外す)。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. test 側 determinism — mode 別 timeout の分離(exit/fail/abort は fixture の観測可能な全 lifetime を上回る値、timeout mode だけ必要 budget)、早期解決を期待値として受理(incremental pid emit で emit 済み全件の消滅検証を保持)、wait_for budget 拡大 + `alive` poll 集約/低頻度化、failure 時に error chain/details を dump する diagnostic | 確認済みの R1 race を構造的に除去。#157 と同じ「test が制御しない外部状態への依存を切る」境界で責務が正しい。diagnostic 追加で R4 も次回発生時に自明化 | exit 255 が tool 側 helper 失敗なら test 側だけでは再発しうる(ただし diagnostic で判別→条件付き tool 側対応へ逃がせる) | yes |
| B. tool 側 helper robustness — `WinTree::terminate` の snapshot 回数削減・dead pid の直接照合・taskkill "not found" exit の成功扱い・eager snapshot の遅延起動 | R1-R4 を共通因で抑えられる。production 利用でも cleanup が軽くなる | 未確定の R4 ありきで本番 semantics(process-tree cleanup の挙動)に触る。fixture test でしか観測していない事象に本番変更を先行させるのは根拠不足 | conditional |
| C. diagnostics-first のみ — failure 時採取だけ入れて再現を待つ | 変更最小・root cause を確定してから直せる | R1 は読解で確定済みなのに放置される。再現が不定期(1 件/日以下)だと長期間 flake が残る | no |
| D. 閾値引上げ/poll 改善のみ — wait_for を 60s 等にし poll 間隔を広げる | diff 最小 | `timeout: "2s"` の構造 race(R1)を消せない。helper 増幅も残る | no |
| E. test 分割 — 4 mode loop を独立 `#[tokio::test]` へ分離 | failure の mode 帰属が test 名で直ちに分かる・1 mode の遅延が他に伝播しない | 4 test 並列化で helper storm の peak はむしろ上がる。mode 帰属は現行 panic 経路には無い(`wait_for` panic に mode 無し・`"{key} gone"` は非展開 literal)が、同等の帰属は Option A の diagnostics で供給できる | no |
| F. intra-file serialization — tree-heavy test 群を shared `static Mutex` で直列化 | test 自身の self-load が減る | complementary 止まり — R1 の構造 race は消せず、self-load 削減項は Option A の poll 集約が既にカバーする | no |

## Tentative Conclusions and Hypotheses

- **confirmed(race 構造)**: `cleans_live_descendants` の flake 要因
  として test 側の構造 race(R1)が読解で確定 — `timeout: "2s"` が
  3 段 fixture の観測可能な全 lifetime(spawn 準備 → pids.json
  write → exit/park)を budget に含め、発火位置に応じて died-early・
  未作成ファイル・wrong-error の panic を生む。単独実行で pass・
  並列負荷でのみ fail という挙動と整合する。ただし **観測された
  instance の R1-R3 間の帰属は未確定** — failure 時の panic
  message が採取されていないため、どの window で落ちたかは記録に
  残っていない。
- **hypothesis(最有力)**: `node_task_resolves_via_path` の exit 255 は
  Windows tree helper(powershell CIM または taskkill)の非 0 exit が
  `"managed process cleanup failed"` として表面化したもので、共通因は
  **helper 増幅**(eager snapshot + terminate 連鎖)が full-suite 並列負荷で
  deadline/staleness window に入ること。POSIX にはこの経路がなく、
  Windows local 限定の観測と一致する。
- **hypothesis(対抗)**: exit 255 は `node.exe` 自身の起動失敗(resource
  pressure)。可能性は残るが pin は ambient PATH 解決済みの絶対 path で
  PATH 揺らぎの筋は潰せており、次善仮説。
- **confirmed(寄与)**: test 側の `alive()` tasklist polling が ambient
  負荷を自己増幅させる(R5)。poll 集約・周期緩和は独立して効く。
- **hypothesis → 記録済み**: tool 側 helper robustness(Option B)は
  diagnostic で tool 側帰属が確認された場合の条件付き後段とする。先走って
  本番 `tree.rs` の cleanup semantics を変えない(#157 の境界と同型)。

## Open Questions

- exit 255 の帰属(tree helper vs node.exe)— failure 時 diagnostic(error
  chain・details・mode・elapsed)採取で初めて判別可能。spec 側の必須要件化
  で解く。
- CI(Ubuntu)で同種 flake が観測されるか。現行 2 件は Windows local のみ。
  POSIX 経路は helper を持たないため、観測されるとすれば R1 系のみの想定。
- helper 増幅を恒常的に減らす tool 側 hardening の要否 — diagnostic で
  helper 帰属が確認された時点で別途判断(本 discovery では条件付き記録)。

## Promotion Candidates

- [x] spec-doc needed: proc_process.rs process-tree test の deterministic 化
  要件 + failure diagnostic 採取の記録 —
  `docs/specs/deterministic-proc-process-tests-issue-180.md`
- [x] adr-doc needed: 修正境界(test 側 vs tool 側)の選択記録 —
  `docs/adr/resolve-proc-process-test-flake-at-the-test-boundary.md`

## Sources and Relations

- issue #180: <https://github.com/scarecrow173/cheaper/issues/180>
- 失敗観測: `docs/impl/ir/unified-exec-grant-model-issue-165.md`
  triage note(node exit 255)、`docs/impl/ir/opt-in-exec-deny-patterns-
  issue-164.md` triage note(cleans_live_descendants timeout)
- 検証対象コード:
  `crates/cheaper-tools/tests/proc_process.rs`(31-255 fixture、261-287
  TempDir、360-400 wait_for/alive、448-470 node test、529-550
  times_out_native_process、587-657 cleans_live_descendants、692-717
  sigterm grace)、
  `crates/cheaper-tools/src/tools/proc/monitor.rs`(205-347 drive)、
  `crates/cheaper-tools/src/tools/proc/cleanup.rs`(16-51)、
  `crates/cheaper-tools/src/tools/proc/tree.rs`(17-19 helper 定数、
  29-117 POSIX、123-501 Windows backend、517-542 public surface)、
  `crates/cheaper-tools/src/tools/exec/mod.rs`(139-211 pin)、
  `crates/cheaper-tools/src/tools/proc/executable.rs`(104-141 resolve)
- 前例(#157 chain): `docs/discovery/ubuntu-ci-route-test-flake.md`、
  `docs/specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md`、
  `docs/adr/resolve-route-test-flake-via-test-side-determinism.md`
