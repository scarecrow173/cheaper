---
id: "TASK-034LIwoKlNw7BOwWV21TuC"
type: "task"
status: "done"
title: "Verify proc_process determinism and run mise run check"
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
  implements:
    - "docs/plans/fix-proc-process-test-flake-via-test-side-determinism-issue-180.md"
  implemented-by: []
  depends-on:
    - "docs/plans/fix-proc-process-test-flake-via-test-side-determinism-issue-180.md"
    - "docs/tasks/implement-pids-d-emit-protocol-and-test-helper-layer.md"
  blocks: []
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

# Verify proc_process determinism and run mise run check

## Work

merged task(emit protocol + helper 層 + 4 test 変換)の着地済み変更に
対し、対象 test の連続安定実行と canonical verification contract
(`mise run check`)の pass を確認する収束 task。新規 code 変更は無い。

確認観点:

1. `mise exec -- cargo test -p cheaper-tools --test proc_process` が
   pass する(Windows・POSIX 両方 — unix のみの `allows_sigterm` test は
   POSIX で動作確認)。POSIX 側の確認は `gh workflow run ci`
   (ubuntu `workflow_dispatch`)で行うか、local POSIX env(WSL・
   Linux 実機等)がある場合はそこで実行する — いずれかを本 task で
   明示して記録する。
2. `mise run check` を少なくとも 3 連続実行し、`proc_process` 系 test が
   失敗しない(smoke signal — ~1 件/日 rate に対する証明ではない、
   spec AC と同じ受容)。
3. diagnostics 実効確認 — 意図的に budget を潰した一時的変更で
   `diag_fail` が error chain/details/mode/elapsed/emitted set を出力
   することを目視確認し、revert する。
4. emit floor の分類正当性確認 — `cleans_live_descendants` の task
   timeout を極小にした一時的変更で、emit floor が mode identity
   基点で正しく働くことを確認する。「60s-armed」は timeout **値**
   ではなく mode 指定そのもの(exit-tree/fail-tree/deep-tree は
   designation 上 60s-armed であり、一時的に `"60s"` 以外へ変えても
   floor の key は mode identity で解決される):
   - tiny timeout 化 → `deep-stubborn` は 0-emit Timeout 受理のまま
     (genuine pre-emit kill window)を確認
   - 60s-armed mode(exit-tree/fail-tree/deep-tree)で 0-emit
     Timeout となれば `diag_fail` = floor が機能していることを確認
   - emit ≥1 で解決した mode は受理 + emit 済み全件の消滅検証
     (`verify_emitted_dead`)を通ることを確認
   確認後 revert する。
5. diff review で、`"{key} gone"`・`tasklist /FI`・`alive()` wrapper・
   **変換対象経路の** context-free `unwrap()` が残っていないこと、
   wait/verify の失敗が `diag_fail` の context 付き panic(mode/
   what/budget/elapsed/emitted set)を通ることを確認する。
   (context-free `unwrap()` の監査対象は変換済みの process-tree 系
   経路に限定 — touch 等の未変換 test の `read_to_string` は対象外)。
6. 再発時運用の確認 — diagnostic が tool 側帰属(`"process tree helper
   failed"`/`"managed process cleanup failed"` 系)を示した場合の
   判断記録追記 + follow-up issue 化の方針を spec/ADR と整合確認する。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/fix-proc-process-test-flake-via-test-side-determinism-issue-180.md (T4 slice)
- docs/designs/deterministic-proc-process-tests-issue-180.md (Verification Notes)

## Dependencies

- docs/tasks/implement-pids-d-emit-protocol-and-test-helper-layer.md

## Affected Files

- なし(検証のみ。既存変更の確認対象:
  crates/cheaper-tools/tests/proc_process.rs)

## Done When

- [x] `proc_process` test target 全体が pass する(Windows・POSIX —
      POSIX locus を `gh workflow run ci` または local POSIX env の
      どちらで確認したか記録済み — Notes 参照: Windows local で 9
      passed/0 failed、POSIX は compile smoke + CI locus 記録)
- [x] `mise run check` が少なくとも 3 連続 pass する(3/3 連続 exits 0)
- [x] 一時的変更で `diag_fail` が error chain/details/mode/elapsed を
      出力することを確認し revert した(Notes の
      forced-classification 証跡参照 — `dump_failure` が
      mode/what/budget/poll/elapsed・emitted set・`source()`
      chain・cheaper code/details JSON を stderr 採取し
      `[{mode}] {what}: timed out after ...` panic が続いた)
- [x] 一時的な極小 timeout で emit floor が mode identity 基点で機能
      すること(deep-stubborn の 0-emit 受理・60s-armed mode の
      0-emit `diag_fail`・emit ≥1 経路の受理 + 消滅検証)を確認し
      revert した
- [x] diff review で timing/ambient 依存 assertion・`/FI`・`alive`
      wrapper・変換対象経路の context-free `unwrap()` が残って
      いないことを確認した
- [x] 再発時運用(tool 側帰属 → 判断記録 + follow-up issue)が spec/ADR
      と整合することを確認した(spec AC・design Deferred Design
      Concerns の re-engagement trigger・IR Follow-ups で同一方針 —
      diagnostics が `"process tree helper failed"`/`"managed
      process cleanup failed"` 系の tool 側帰属を示した場合は
      判断記録を docs へ追記し tool 側 hardening を別
      issue(conditional phase-2)で起票する運用)

## Verification

- [x] `mise exec -- cargo test -p cheaper-tools --test proc_process` exits 0(9 passed/0 failed、31.90s、Windows local)
- [x] `mise run check` exits 0 ×3 連続(proc_process leg は run1 33.09s / run2 36.54s / run3 32.06s で 9 passed/0 failed、全 run で `test result: FAILED` 0 件)
- [x] `mise exec -- cargo clippy -p cheaper-tools --all-targets -- -D warnings` exits 0
- [x] Manual: diagnostics/early-timeout の一時的変更確認 → revert(supplement)
- [x] Manual: diff review で `"{key} gone"`・`/FI`・`alive(`・変換対象経路の context-free `unwrap()` が残っていないことを確認(supplement)

## Notes

### Verification evidence(2026-10-25、Windows local、branch `bugfix/proc-process-flake-issue-180`)

- Targeted: `mise exec -- cargo test -p cheaper-tools --test
  proc_process` → exit 0、**9 passed / 0 failed**(31.90s)。
- Canonical: `mise run check` × **3 連続** 全 exit 0(biome →
  fmt --check → clippy → version/doc-id → `cargo test
  --workspace` → `node --test` scripts+benchmarks → lockfile →
  apm-payload)。各 run の proc_process leg: run1 = 9/0
  (33.09s)、run2 = 9/0 (36.54s)、run3 = 9/0 (32.06s)。spec AC
  どおり smoke signal であり ~1 件/日 rate への証明ではない。
- Forced-classification check(一時変更 → revert 済み):
  - **60s-armed floor 発火**: timeout `"60s"`→`"1ms"` + fixture
    emit dir を `hidden.d` へ redirect(観測側 `pids.d` を空に
    する)の一時変更で `cleans_live_descendants` を実行 →
    exit-tree が `Err` code=Timeout + emitted `{}` で emit
    floor に入り、`dump_failure` dump(mode: exit-tree /
    what / budget 5s / poll 20ms / elapsed / `emitted: {}` /
    error `source()` chain / `cheaper code: TIMEOUT` /
    details JSON)に続く `diag_fail` panic `[exit-tree]
    60s-armed timeout requires at least one emit: timed out
    after 5s (poll 20ms); elapsed 255.3µs` を確認(test
    FAILED、exit 101 = 意図どおり)。floor が timeout **値**
    ではなく mode identity(`mode != "deep-stubborn"`)で key
    されることを実証。
  - **deep-stubborn 0-emit 受理**: deep-stubborn のみ
    `"2s"`→`"1ms"` + emit redirect、他 mode は `"60s"` +
    通常 emit dir の一時変更 → 全 4 mode pass(32.13s)。
    deep-stubborn は 0-emit Timeout が期待 outcome として受理
    され(genuine pre-emit kill window)、exit/fail/deep-tree
    は自然解決 + 全 key emit + `verify_emitted_dead` 経路を
    通過。
  - **emit ≥1 受理の副次観測**: timeout のみ `"60s"`→`"1ms"`
    (redirect 無し)の一時変更でも test は pass(28.49s)—
    kill 配送(monitor→cleanup→taskkill 連鎖)が entry-emit
    latency より長いため 60s-armed mode は emit ≥1 で解決し、
    floor 受理 + emitted 全件消滅検証を通る。timeout 値だけ
    では 0-emit を起こせないことは、0-emit Timeout = broken
    fixture/plumbing とする design 判断と整合。
  - revert 後の再実行: `cleans_live_descendants` 1 passed/0
    failed(31.65s)、`git status` に proc_process.rs の diff
    無し。
- Diff review: `"{key} gone"` 0 件・`/FI` 0 件・`alive(` 0 件。
  `.unwrap()` 残存は変換対象外(fixture 内部・test setup
  helper・非 tree test・touch marker)のみ — 変換済み
  process-tree 経路は `wait_until` + `dump_failure`/`diag_fail`
  ・context 付き `expect`(`spawn_fixture({mode})`・
  `parent.terminated readable after marker wait`)へ集約済み。
  `read_to_string` は whitelist(`emitted_pids` 内部・
  `/proc/<pid>/stat`・touch marker・`.terminated` 内容確認)のみ。
- POSIX locus: `allows_sigterm_exit_within_grace_before_force_kill`
  および unix `live_pids`/`kill_emitted` leg は `#[cfg(unix)]`。
  実 POSIX 実行は **local POSIX env (WSL2 Ubuntu + rust 1.99.0 +
  node 26)** で実施済み — `cargo test -p cheaper-tools --test
  proc_process --test proc_invocation` が **proc_process 10/10・
  proc_invocation 18/18 pass**(unix `allows_sigterm`・
  `live_pids`/`kill_emitted` leg 含む、2026-10-25)。
  `cargo check --target x86_64-unknown-linux-gnu` compile smoke
  も pass(IR Validation 記録済み)。
- CI `gh workflow run ci` は 3 回 dispatch したが unix 既存 flake
  で完走せず: (a) `proc_invocation` unix コンパイルエラー(本 PR
  で修正済み)、(b) `resources::tests::abort_landing_...` lib
  test hang(run 40149087832 / 40153767045 両方で 60 分超 stall、
  本差分とは無関係の src/ unittest)、(c) `route_run::exec_infra_
  failure_returns_error` は main 2026-10-22 run で既 fail。
  これらは issue #187 として filed 済み。`hook_config` ETXTBSY
  flake(WSL 全件 run で 1 回観測、単体 rerun pass)も同 issue
  に記録。
- 再発時運用の整合確認: spec AC(「diagnostics が tool 側帰属を
  示した場合は判断記録を docs へ追記し tool 側 follow-up
  issue 化」)・design Deferred Design Concerns(re-engagement
  trigger = `diag_fail` 採取後の failure 記録に `"process tree
  helper failed"` 由来 cleanup-failed)・IR
  Follow-ups(conditional phase-2、別 issue)は同一方針で整合。
  `dump_failure` が `find_cheaper_error` の code/details +
  `source()` chain を採取するため tool 側帰属(helper 失敗
  arm)と test 側帰属を判別可能。
