---
id: "IMPL-034Kqau86OzwuLBm22fbxx"
type: "impl"
status: "completed"
title: "Fix route test determinism for ubuntu CI flake issue 157"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/157"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/log_store.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/make-exec-infra-failure-test-deterministic.md"
    - "docs/tasks/pin-sweep-test-deletion-order-with-explicit-mtimes.md"
    - "docs/tasks/verify-route-test-determinism-and-mise-check.md"
  implemented-by: []
  depends-on:
    - "docs/plans/fix-route-test-flake-via-test-side-determinism.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/deterministic-route-test-assertions.md"
    - "docs/discovery/ubuntu-ci-route-test-flake.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/fix-route-test-flake-via-test-side-determinism.md"
    - "docs/designs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/adr/resolve-route-test-flake-via-test-side-determinism.md"
    - "docs/specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Fix route test determinism for ubuntu CI flake issue 157

## Summary

issue #157 の 2 件の flaky route test を、ADR Option A（test 側で
determinism を確保する）に従い修正した。変更は test コードのみで、
本番実装の変更はゼロ。

1. `route_run.rs` の `exec_infra_failure_returns_error`:
   実行到達証明を `marker.exists()`（child が kill 前に marker を
   書けたかの timing 依存観測）から、`run_route` の戻り `Err` の
   `code() == ErrorCode::ToolExecutionFailure` + message
   `contains("routed command did not complete")` という決定的な
   assert へ置き換えた（design D1）。
2. `log_store.rs` の `sweep_enforces_ttl_count_size_and_skips_tmp`:
   `set_mtime` helper（`File::set_modified`、新規 dep 無し）を
   `mod tests` に追加し、単一の `now` 起点で (a) TTL・(b) count
   sweep・(c) size sweep の対象 file に 1 秒超離れた一意な mtime
   順序を付与。assertion を厳密集合（removed 数 + 各 file の
   exists/non-exists）へ絞り、`fs::read_dir` 列挙順への暗黙依存を
   排除した（design D2–D5）。

## Context

TASK-03404zg7SVQPn5JeZ0lwDD（T1）・TASK-0340C4KaPSmy4CrDagevUC（T2）・
TASK-034yW3DRPtMmttnAEru9Ns（T3）。PLAN-034foupf7ivrjlU7zVsv6M /
DESIGN-034Kp4aEcwrsOReUSPyxU9 / SPEC-034KoUwVbN6YjxD7CjpYkf /
ADR-034KoUAHJ1vI8BbyJoL0Q9 に従う。ubuntu CI での flake 原因は
DISC-Gvis7cxcHxPQwsvoSGmuox で「test が制御下にない外部状態
（child の marker 書込み timing・readdir 列挙順）を観測する」と
確定済み。

## Implementation

### Changes

- `crates/cheaper-cli/tests/integration/route_run.rs`（test のみ）:
  `use cheaper_core::errors::{CheaperError, ErrorCode};` を use 群へ
  追加（先例: `tests/integration/envelope.rs`）。`is_err()` assert と
  `marker.exists()` assert を除去し、`.err().expect("route must fail:
  grant timeout is 1ms")` で `Err` を取り出して code + message を
  assert する形へ置き換えた。`timeout: '1ms'`・policy.yaml の
  `CHEAPER_ROUTE_CHILD_MODE: io` → `sleep` 書換え・`reject_fx` 配線・
  `marker` 変数（fixture 構築物として残存）・usage log の `"route"`
  record 非含有 assert は不変。
- `crates/cheaper-cli/src/route/log_store.rs`（`#[cfg(test)] mod tests`
  のみ）:
  - `set_mtime(path, time)` helper を追加し、test 先頭で
    `let now = SystemTime::now();` を 1 回だけ取得。
  - (a) TTL: inline `fs::File::options()...set_modified(past)` を
    `set_mtime(&old, now - 3600s)` へ統一（表現ドリフト防止）。
  - (b) count sweep: `a = now-40s` / `fresh = now-30s` /
    `b = now-20s` / `c = now-10s` の一意順序を付与し、
    `removed == 2`・`!a`・`!fresh`・`b`・`c` の厳密集合へ。
    `b.exists() || c.exists()` の disjunction は廃止。
  - (c) size sweep: 直前 cleanup loop を維持した上で
    `big = now-20s`（古）・`small = now-10s`（新）を付与し、
    `removed == 1`・`!big`・`small` へ厳密化。
    `big.exists() != small.exists()` の XOR は廃止。
  - `.tmp` 除外・TTL assert・sweep 引数・本番実装 `sweep_with` は不変。

### Decisions During Implementation

- `unwrap_err()` ではなく `.err().expect(...)` を使用 —
  `RouteRunOutput: Debug` 未実装のため（design の確定形どおり）。
- `ErrorCode::Timeout` ではなく `ToolExecutionFailure` を assert する —
  review で design が修正された点（commit `81274b5`）。engine が job
  失敗を `outcome.error` として `ToolExecutionFailure` に collapse
  するため、`Err` 側へは `Timeout` code が届かず、仮に `Timeout` を
  assert すると全 platform で決定的 fail になる。message
  `"routed command did not complete"` は `captured() == None &&
  outcome.error.is_some()` 分岐固有の文字列で、「workflow が exec job
  を失敗まで実行した」post-admission 証拠として `marker.exists()` の
  代替になる。
- `marker` 変数自体は残す — `reject_fx` signature が `&marker` を
  要求し、`CHEAPER_ROUTE_CHILD_MARKER` は sibling test と共有する
  fixture 構築物のため、assert 対象から外すだけで配線は維持した
  （design D5）。
- (b) の削除集合 `{a, fresh}` は現行コメントの「fresh + a が消える」
  意図を維持しつつ、mtime tie 由来の「a/b/c のどれか」と fresh の
  tie 不確定性を排除した。(c) の順序は big-older が必須 — 逆順だと
  small 先行削除でも残量 128B > 100 で `removed == 2` になり意図が
  変わる。

## Related Experiments

### Adopted

なし（known-solution task。design D1–D5 が assertion 形状・mtime 値・
helper 形まで確定済みのため Experiment Log は未作成）。

### Rejected

なし。

## Validation

- `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_run::exec_infra_failure_returns_error -- --exact`: pass
- `cargo test ... --test integration route_run::`: 25 test 全 pass
- `cargo test ... route::log_store`: 6 test 全 pass（`--lib` 全体も非
  regress）
- 両対象 test は連続 3 回の targeted run で flake 兆候無く安定 pass
- `cargo fmt --check`: pass
- `mise run check`: 全 step exit 0（biome・`cargo fmt --check`・
  clippy・version/doc-id check・`cargo test --workspace` 全 target
  0 failed・`node --test`・lockfile/apm-payload check）
- diff review: `marker.exists()` 系 assertion が残っていないこと、
  sweep assertion が厳密集合で `read_dir` 順に依存しないことを確認
  （ubuntu CI での fail フリーは review 確認で代替 — design
  Verification Notes）

証跡 commit: `34a391a`（briefing）→ `43186dc`（design）→ `81274b5`
（design review 修正: infra-failure assert を `Timeout` から
`ToolExecutionFailure` + message assert へ訂正）→ `ed7f56f`
（spec/design/adr 承認）→ `d9b13d5`（plan/test-spec/task）→
`9de23f8`（本実装 + T1/T2 task 記録）→ `f79e01f`（T3 検証記録）。

## Risks

- `sweep_with` の (mtime, path) secondary sort key による実装側
  hardening は spec/design の Deferred Design Concerns で scope 外確定
  （YAGNI）。test 側で一意 mtime を与えるため tie は発生しない。
- ubuntu CI での実検証は merge 後の CI 実行で確認する — 本 change は
  assertion が外部状態を参照しないことの review 確認をもって代替済み。

## Follow-ups

followup-triage 分類: `followup-terminal` — 本 implementation chain
（issue #157）由来の未分類・未処理の follow-up はない。

- `route run` の cwd-mismatch fail-closed 観測は issue #148 の
  verification で記録済みの既知 Open Question であり、canonical 記録先
  `docs/discovery/devin-pretooluse-rewrite-runtime-verification.md`
  （Open Questions + Promotion Candidates「別 issue 候補 — 起票は
  所有者判断」）に既に存在する。#157 の作業から派生した leftover では
  なく、分類済みのため新規 follow-up artifact は起こさない。
- `sweep_with` の secondary sort key hardening は Deferred Design
  Concerns として spec/design で scope 外確定済みのため follow-up 化
  しない。
