---
id: "TASK-034LIwoKlNw7BOwWV21TuC"
type: "task"
status: "todo"
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

- [ ] `proc_process` test target 全体が pass する(Windows・POSIX —
      POSIX locus を `gh workflow run ci` または local POSIX env の
      どちらで確認したか記録済み)
- [ ] `mise run check` が少なくとも 3 連続 pass する
- [ ] 一時的変更で `diag_fail` が error chain/details/mode/elapsed を
      出力することを確認し revert した
- [ ] 一時的な極小 timeout で emit floor が mode identity 基点で機能
      すること(deep-stubborn の 0-emit 受理・60s-armed mode の
      0-emit `diag_fail`・emit ≥1 経路の受理 + 消滅検証)を確認し
      revert した
- [ ] diff review で timing/ambient 依存 assertion・`/FI`・`alive`
      wrapper・変換対象経路の context-free `unwrap()` が残って
      いないことを確認した
- [ ] 再発時運用(tool 側帰属 → 判断記録 + follow-up issue)が spec/ADR
      と整合することを確認した

## Verification

- [ ] `mise exec -- cargo test -p cheaper-tools --test proc_process` exits 0
- [ ] `mise run check` exits 0 ×3 連続
- [ ] `mise exec -- cargo clippy -p cheaper-tools --all-targets -- -D warnings` exits 0
- [ ] Manual: diagnostics/early-timeout の一時的変更確認 → revert(supplement)
- [ ] Manual: diff review で `"{key} gone"`・`/FI`・`alive(`・変換対象経路の context-free `unwrap()` が残っていないことを確認(supplement)
