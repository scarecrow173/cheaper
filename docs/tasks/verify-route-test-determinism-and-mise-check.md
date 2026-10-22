---
id: "TASK-034yW3DRPtMmttnAEru9Ns"
type: "task"
status: "todo"
title: "Verify route test determinism and run mise run check"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/157"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/fix-route-test-flake-via-test-side-determinism.md"
  implemented-by: []
  depends-on:
    - "docs/plans/fix-route-test-flake-via-test-side-determinism.md"
    - "docs/tasks/make-exec-infra-failure-test-deterministic.md"
    - "docs/tasks/pin-sweep-test-deletion-order-with-explicit-mtimes.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/fix-route-test-flake-via-test-side-determinism.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Verify route test determinism and run mise run check

## Work

T1・T2 の着地済み変更に対し、対象 test の連続安定実行と canonical
verification contract(`mise run check`)の pass を確認する収束 task。
新規 code 変更は無い。

確認観点:

1. 両対象 test が複数回実行で安定 pass する(連続 2-3 回の targeted run
   で flake 兆候が無いこと)。
2. `route_run::` module filter・`route::log_store` filter が pass する。
3. `mise run check` が pass する(biome・`cargo fmt --check`・clippy・
   version/doc-id check・`cargo test --workspace`・`node --test`・
   lockfile/payload check の全 step)。
4. diff review で、`exec_infra_failure_returns_error` に `marker.exists()`
   等の timing 依存 assertion が残っていないこと、sweep test の assertion
   が厳密集合で `read_dir` 列挙順に依存しないことを確認する(ubuntu CI で
   の marker 未作成・readdir 順不定条件での fail フリーは、この review
   確認で代替可能 — design Verification Notes)。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/fix-route-test-flake-via-test-side-determinism.md (T3 slice)

## Dependencies

- docs/tasks/make-exec-infra-failure-test-deterministic.md
- docs/tasks/pin-sweep-test-deletion-order-with-explicit-mtimes.md

## Affected Files

- なし(検証のみ。既存変更の確認対象:
  crates/cheaper-cli/tests/integration/route_run.rs、
  crates/cheaper-cli/src/route/log_store.rs)

## Done When

- [ ] 両対象 test が連続実行で安定 pass する
- [ ] `route_run::`・`route::log_store` filter run が pass する
- [ ] `mise run check` が pass する
- [ ] diff review で timing/enumeration 依存 assertion が残っていないことを
      確認した

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_run::` exits 0
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam route::log_store` exits 0
- [ ] `mise run check` exits 0
- [ ] Manual: diff で `marker.exists()`・readdir 順依存 assertion
      (disjunction / existence XOR)が残っていないことを確認(supplement)
