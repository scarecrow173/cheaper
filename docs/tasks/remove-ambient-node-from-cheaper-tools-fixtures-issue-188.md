---
id: "TASK-034MCBEcBex2dohPIO7AKU"
type: "task"
status: "done"
title: "Remove ambient Node from cheaper-tools fixtures"
created: "2026-10-26"
updated: "2026-10-26"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/remove-ambient-node-from-rust-tests-issue-188.md"
  implemented-by: []
  depends-on:
    - "docs/plans/remove-ambient-node-from-rust-tests-issue-188.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ambient-node-free-rust-test-contract-issue-188.md"
  references: []
  defers: []
  deferred-by: []
---

# Remove ambient Node from cheaper-tools fixtures

## Work

Replace Node usage in the `cheaper-tools` process and differential fixture
tests. Add an explicit Rust child mode that preserves `bare node\n`, translate
the literal Node cases (including six shared runtime declarations) in the test
translation layer, and retain an isolated real bare-name PATH lookup/pin/spawn
proof in `node_task_resolves_via_path`. Keep fixture JSON byte-identical.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/remove-ambient-node-from-rust-tests-issue-188.md`
- `docs/specs/remove-ambient-node-test-coupling-issue-188.md`
- `docs/designs/remove-ambient-node-from-rust-tests-issue-188.md`

## Dependencies

- Requires the approved issue188 plan; no task dependency.

## Affected Files

- `crates/cheaper-tools/src/bin/diff_fixture_child.rs`
- `crates/cheaper-tools/tests/differential.rs`
- `crates/cheaper-tools/tests/proc_process.rs`
- `crates/cheaper-tools/tests/fixtures/process.json` (assert unchanged)
- `crates/cheaper-tools/tests/fixtures/runtime.json` (assert unchanged)
- `docs/impl/ir/remove-ambient-node-from-cheaper-tools-fixtures.md`

## Done When

- [x] Bare-name PATH resolution, grant pinning, and spawn succeed in an
  isolated child PATH with Node absent and without global PATH mutation.
- [x] Process/runtime differential outputs remain unchanged and both fixture
  JSON files remain byte-identical.
- [x] No target test in this slice requires ambient Node.

## Verification

- [x] In WSL with process-local PATH from `rustup which cargo/rustc`, verified
  Node absent, run `cargo test -p cheaper-tools --test proc_process node_task_resolves_via_path -- --exact`.
- [x] Under the same Node-free WSL PATH, run `cargo test -p cheaper-tools --test differential process_fixtures -- --exact` and `cargo test -p cheaper-tools --test differential runtime_fixtures -- --exact`.
- [x] `git diff --exit-code e5e26284de8a21ac9433e6470811ffd049448853 -- crates/cheaper-tools/tests/fixtures/process.json crates/cheaper-tools/tests/fixtures/runtime.json` exits 0.
- [x] Windows: all three exact tests pass through `mise exec --` (`proc_process node_task_resolves_via_path`, `differential process_fixtures`, `differential runtime_fixtures`).
