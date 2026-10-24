---
id: "ADR-034LFaJmc8htAt3pDMjMQ2"
type: "adr"
status: "accepted"
title: "Resolve proc_process test flake at the test boundary"
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
  implements:
    - "../specs/deterministic-proc-process-tests-issue-180.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/deterministic-proc-process-tests-issue-180.md"
    - "resolve-route-test-flake-via-test-side-determinism.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/proc-process-test-flake-under-parallel-load.md"
  derived-by:
    - "../designs/deterministic-proc-process-tests-issue-180.md"
    - "../plans/fix-proc-process-test-flake-via-test-side-determinism-issue-180.md"
    - "../test-specs/deterministic-proc-process-test-assertions-for-parallel-load-flake.md"
  verifies: []
  verified-by: []
  references:
    - "../impl/ir/opt-in-exec-deny-patterns-issue-164.md"
    - "../impl/ir/unified-exec-grant-model-issue-165.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Resolve proc_process test flake at the test boundary

## Context and Problem Statement

`crates/cheaper-tools/tests/proc_process.rs` の process-tree 系 test が
full-suite 並列負荷下で 2 件の transient fail を記録した(issue #180、
2026-10-23、Windows local)。コード読解で
`cleans_live_descendants_on_exit_and_failure_and_abort_and_timeout` は
task `timeout: "2s"` が 3 段 fixture の観測可能な全 lifetime
(spawn 準備 → pid emit → exit/park)を巻き込む **構造 race**
であることが確定し(discovery R1 — 構造の確定であり、観測 instance
の R1-R3 間帰属は panic message 未採取で未確定)、
`node_task_resolves_via_path` の exit 255 は Windows tree helper
失敗(`"managed process cleanup failed"`)が最有力だが diagnostic
未採取で未確定(R4)。共通因として Windows の
`WinTree::terminate` が powershell CIM snapshot + taskkill の subprocess
連鎖を全 execute(通常 exit 含む)に載せる helper 増幅があり、POSIX に
存在しないこの経路が Windows 限定観測と一致する。

## Decision Drivers

* 違反している invariant は「test の assertion が制御下にない wall-clock/
  ambient 状態(3 段 spawn 準備遅延・helper subprocess 連鎖遅延)を
  timing budget 内と仮定して観測する」ことで、責務境界の第一義は test 側。
* 確定済みの race(R1)と未確定の帰属(R4)が混在する — 確定分は構造的に
  直し、未確定分は再発時に自明化する仕組みを同時に入れる。
* 本番 `proc::tree` の cleanup semantics を、test でしか観測されていない
  事象への対応として先走って変えない(#157 と同型の境界)。
* 修正で検証対象の lifecycle invariant(mode 別 outcome・descendant
  消滅・ambient PATH 解決)を弱めない。

## Considered Options

### Option A: test 側 determinism + failure diagnostics

`cleans_live_descendants` の task `timeout` を mode 別に分離し
(timeout を期待しない mode は fixture の観測可能な全 lifetime を
上回る値へ)、fixture の pid 通知を incremental な per-key atomic
emit(`pids.d/{parent,child,grandchild}` 等)へ変えて早期解決を
期待 outcome として受理しつつ emit 済み pid 全件の消滅検証を保つ。
同じ armed-timeout race を持つ sibling test
(`times_out_native_process_and_cleans_tree`・unix の
`allows_sigterm_exit_within_grace_before_force_kill` — 後者は
"armed" marker 追加)にも同構造を適用する。`wait_for` budget の
余裕確保・context 付き panic message、`alive()` の unfiltered
`tasklist` snapshot 集約/低頻度化、panic mid-`drive()` leak の
containment(JoinHandle + drop guard abort・fixture 子の
kill-on-drop guard)を行い、failure 時に error `source()` chain・
details JSON・mode・elapsed を stderr へ採取する diagnostic helper
を test 内に実装する。diagnostic が tool 側帰属を示した場合は判断
記録 + 別 issue で tool 側 hardening に逃がす条件付き第 2 段を用意する。

### Option B: tool 側 helper robustness 先行

`WinTree::terminate` の snapshot 回数削減、dead pid の直接照合、
taskkill "not found" exit の成功扱い、eager snapshot の遅延起動等で
helper 増幅自体を潰す。production の cleanup semantics を変更する。

### Option C: diagnostics-first のみ

failure 時採取だけ実装し、root cause を確定してから修正を決める。
確定済みの R1 race はこの段階では修正しない。

### Option D: 閾値引上げ/poll 改善のみ

`wait_for` を 60s 等へ増やし poll 周期を緩めるだけで構造は据え置く。

### Option E: test 分割

4 mode loop を独立 `#[tokio::test]` へ分離し failure の帰属を
test 名で即座に判別可能にする。

### Option F: intra-file serialization

tree-heavy test 群を shared `static Mutex` で直列化し、同ファイル内の
self-load(並走する tasklist/helper spawn)を削る。

## Decision Outcome

Chosen option: "Option A: test 側 determinism + failure diagnostics",
because R1 は読解で確定した test 側の構造 race であり、invariant を
壊している層(test)で直すのが正しい境界。Option B は未確定の R4 を
前提に本番 semantics を変えるため根拠不足 — diagnostic で tool 側帰属が
確認された場合の条件付き第 2 段として位置づける。Option C は確定済み
race を放置し、Option D は R1 の構造 race を消せず、Option E は
4 test 並列化で helper storm の peak 負荷を増やす(failure の mode
帰属は現行 panic 経路には無く、Option A の diagnostics 要件が供給
する)、Option F は self-load 削減が Option A の poll 集約に包含され
R1 を消せない。diagnostics を同時に入れることで、残る未確定要素
(exit 255 帰属)も再発時に自己説明的になる — issue #180 の AC
「root cause 特定の記録」「test 固有でないと判明した場合の判断記録」
の両方をこの形で満たす。

### Consequences

* Good, because 確定 race(R1)を構造的に除去し、wait_for/poll の
  改善で R2-R3/R5 の寄与要因も同時に弱める。
* Good, because diagnostic helper が failure を self-explaining にし、
  exit 255 の帰属が次回観測で確定できる。
* Good, because 本番コードを変更しないため検証範囲が test file に
  閉じ、リスクが最小。
* Bad, because exit 255 が tool 側 helper 失敗なら第 1 段だけでは
  再発しうる — ただしその場合は diagnostic が帰属を確定させ、
  第 2 段の tool 側対応の正当化根拠になる。
* Bad, because mode 別 timeout の分離は timeout mode の壁時計を
  長くしうる(ただし kill 検証に必要な最小値へ抑える)。

### Confirmation

* `mise exec -- cargo test -p cheaper-tools --test proc_process` が
  pass すること。
* `mise run check` を少なくとも 3 連続実行し、対象 test が失敗しない
  こと(issue #180 AC)。
* 再発時は diagnostic 出力で帰属を判別し、tool 側帰属なら判断記録を
  docs へ追記 + follow-up issue 化する運用を確認する。

## Implementation Plan

* Affected paths: `crates/cheaper-tools/tests/proc_process.rs`
  (`proc_fixture_child` — deep mode の incremental pid emit 化・unix
  `graceful` mode の "armed" marker、`spawn_fixture` :233、
  `cleans_live_descendants_on_exit_and_failure_and_abort_and_timeout`
  :588、`times_out_native_process_and_cleans_tree` :529-550、
  `allows_sigterm_exit_within_grace_before_force_kill` :692-717、
  `node_task_resolves_via_path` :448、helpers `wait_for` :360 /
  `alive` :371)。
* Patterns to follow: #157 の test-side determinism
  (docs/adr/resolve-route-test-flake-via-test-side-determinism.md)。
  mode 別に `json!({"timeout": ..., "killGrace": ...})` を差し替える
  構成。fixture の per-key pid file は tmp+rename の atomic write、
  test は parseable 内容の到着を file ごとに待ち、早期 `Timeout` 後は
  bounded post-resolution wait で emit 済み pid 全件の消滅を検証する。
  `alive()` の集約は unfiltered `tasklist /NH`(`/FO CSV` 可)
  snapshot 1 回/poll の client 側照合。panic mid-`drive()` の leak 対策
  は execute の JoinHandle 化 + drop guard の controller abort、
  `spawn_fixture` 子の kill-on-drop guard。error chain は
  `find_cheaper_error` + `Error::source` walk で採取。`wait_for` の
  panic message は mode/key/budget の context を取る。
* Patterns to avoid: `proc/` 本番コードへの変更。`std::env::set_var`
  で PATH を書き換える test(並列 test と競合する process-global
  状態)。`pending` 早期解決を一律異常扱いする assert の温存。
  `tasklist /FI` filter の複数併記(AND 評価で常に空を返し
  `alive()` が vacuous 化)。failure 経路の context-free `unwrap()`。
* Dependencies or configuration: 新規依存なし。fixture は既存
  `proc_fixture_child` の env-driven mode 機構を拡張する形で変更する
  (test コードであり spec scope 内)。
* Conditional phase 2(別 issue): diagnostic が tool 側帰属を示した場合、
  `WinTree::terminate` の helper 連鎖削減・taskkill "not found" 処理・
  stale snapshot 対処を本番 `tree.rs` に適用するかを別 ADR/issue で
  判断する。本 ADR では実施しない。

## Verification

* [ ] `mise exec -- cargo test -p cheaper-tools --test proc_process` が
  pass する
* [ ] `mise run check` 3 連続で `proc_process` 系 test が失敗しない
  (smoke signal — ~1 件/日 rate に対する証明ではない点は spec AC
  参照)
* [ ] `cleans_live_descendants` で mode 別 timeout 分離・incremental
  per-key pid emit・早期解決受理 + emit 済み pid 全件の消滅検証が
  実装され、fixture 準備/emit/exit が timeout と競合しない
* [ ] sibling test(`times_out_native_process_and_cleans_tree`・
  `allows_sigterm_exit_within_grace_before_force_kill`)も同構造で
  直り、後者は "armed" marker で kill-before-arming と
  armed-but-no-`.terminated` を区別する
* [ ] failure diagnostics helper が error chain・details・mode・elapsed
  を出力し、`wait_for` panic が mode/key/budget context を含む

## Pros and Cons of the Options

### Option A: test 側 determinism + failure diagnostics

* Good, because 確定した構造 race を責務のある層で直す。
* Good, because 未確定要素を diagnostic で確定可能にする。
* Good, because 本番コード無変更でリスク最小。#157 実績パターン。
* Bad, because tool 側帰因の flake(exit 255 が helper 失敗の場合)は
  第 2 段まで残りうる。
* Bad, because timeout mode の壁時計が伸びうる。

### Option B: tool 側 helper robustness 先行

* Good, because helper 増幅そのものを減らし全 execute を軽くする。
* Bad, because test 観測のみの事象に本番 semantics 変更を先行させる。
* Bad, because R1 の test 側 race は残る(構造 race は別問題)。
* Bad, because `tree.rs` の POSIX/Windows parity・TS parity への影響
  評価が必要で変更面が大きい。

### Option C: diagnostics-first のみ

* Good, because 変更最小で根拠を集められる。
* Bad, because 確定済み R1 race を放置し flake が残る。
* Bad, because 再現頻度が低く(1 件/日以下)確定まで長期化しうる。

### Option D: 閾値引上げ/poll 改善のみ

* Good, because diff が最小。
* Bad, because `timeout: "2s"` の構造 race と早期解決 assert を消せない。
* Bad, because helper 増幅の共通因は残る。

### Option E: test 分割

* Good, because failure の mode 帰属が test 名で即座に分かる —
  ただし同等の帰属は Option A の diagnostics 要件
  (`wait_for` context 付き panic message)でも得られる。現行の
  panic 経路には mode が無く、`"{key} gone"` は展開されない
  literal なので「assert message で既に取れる」は誤り。
* Bad, because 4 test 並列化で helper storm の peak が上がる。

### Option F: intra-file serialization

* Good, because test 自身の self-load が減り R2-R3/R5 の window が
  縮む。
* Bad, because complementary 止まり — self-load を減らしても R1 の
  構造 race(task timeout が fixture の全 lifetime を巻き込む)は
  消せず、self-load 削減項は Option A の poll 集約/cadence 緩和が
  既にカバーする。

## More Information

* issue #180: <https://github.com/scarecrow173/cheaper/issues/180>
* 調査記録: `docs/discovery/proc-process-test-flake-under-parallel-load.md`
* spec: `docs/specs/deterministic-proc-process-tests-issue-180.md`
* 前例 ADR(#157): `docs/adr/resolve-route-test-flake-via-test-side-determinism.md`
* 失敗観測: `docs/impl/ir/unified-exec-grant-model-issue-165.md`・
  `docs/impl/ir/opt-in-exec-deny-patterns-issue-164.md` の triage note
