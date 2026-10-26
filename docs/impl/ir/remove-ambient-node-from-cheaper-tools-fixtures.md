---
id: "IMPL-034MCFBcPwAhqtiC177bnN"
type: "impl"
status: "completed"
title: "Remove ambient Node from cheaper-tools fixtures"
created: "2026-10-26"
updated: "2026-10-26"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-tools/src/bin/diff_fixture_child.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/differential.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_process.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/remove-ambient-node-from-cheaper-tools-fixtures-issue-188.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Remove ambient Node from cheaper-tools fixtures

## Summary

Added an explicit `bare` mode to `diff_fixture_child`, translated the frozen
literal `node` declarations in differential process policies, and moved the
real bare-name resolve/pin/spawn proof into an isolated nested test process.
The process and runtime fixture JSON files remain unchanged.

## Context

Implements Task `TASK-034MCBEcBex2dohPIO7AKU` under the approved issue #188
plan and design. The differential translator handles the existing literal
`node -e console.log('bare node')` declaration, including its shared runtime
fixture instances, at test resolution time. PATH coverage is owned solely by
`node_task_resolves_via_path`.

## Implementation

### Changes

- `diff_fixture_child` emits exactly `bare node\n` and exits 0 for mode `bare`.
- The differential harness maps the exact known literal `node` command to
  `diff_fixture_child` with an absolute executable path and `bare` mode. The
  existing `<NODE>` script translation remains intact. Unknown literal Node
  commands are not silently mapped.
- `node_task_resolves_via_path` starts an exact-filter re-execution with PATH
  containing only the fixture binary directory. In that child, a configured
  bare `diff_fixture_child` command flows through the existing grant pinning
  and process spawn path; the test asserts completed state, exit code 0, and
  exact stdout `bare node\n`. The child requires the exact re-exec marker and
  asserts its PATH equals the fixture directory. The parent test process PATH
  is not mutated.
- Process and runtime JSON fixtures were not edited.

### Decisions During Implementation

Kept PATH resolution coverage in the dedicated process test. Differential
fixture translation uses an absolute binary path and therefore preserves the
golden output without claiming that the differential case tests PATH lookup.
The restricted PATH is applied only to the nested test process so parallel
tests and the invoking process retain their environment.

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

Windows, through `mise exec`, passed:

- `mise exec -- cargo test -p cheaper-tools --test proc_process node_task_resolves_via_path -- --exact`
- `mise exec -- cargo test -p cheaper-tools --test differential process_fixtures -- --exact`
- `mise exec -- cargo test -p cheaper-tools --test differential runtime_fixtures -- --exact`

WSL verification passed with a process-local PATH assembled from
`/home/akiyoshi/.cargo/bin/rustup which cargo` and `rustup which rustc` plus
Linux system directories. The proof checked that resolved `cargo`/`rustc`
matched those paths, that PATH had no mise/shim entries, and that `node` was
not available. Under that PATH, the same three cargo test filters passed.
The WSL command also ran
`git diff --exit-code -- crates/cheaper-tools/tests/fixtures/process.json crates/cheaper-tools/tests/fixtures/runtime.json`,
which exited 0.

## Risks

The differential mapping is intentionally limited to the exact known literal
Node script in the frozen fixtures. An unknown literal Node declaration fails
closed until an explicit translation and corresponding test coverage are
added.

## Follow-ups

No Task 1 follow-up. Broader boundary documentation and workflow fixture work
remain in their separate issue #188 tasks.
