---
id: "PLAN-034MBPFnNY5JMgJzLEsXN8"
type: "plan"
status: "approved"
title: "Remove ambient Node from Rust test fixtures"
created: "2026-10-26"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    modified:
      - type: "file"
        path: "crates/cheaper-tools/src/bin/diff_fixture_child.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/differential.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_process.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/process.rs"
      - type: "file"
        path: "docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
      - type: "file"
        path: "docs/designs/node-dev-tooling-boundary-contract-issue-182.md"
      - type: "file"
        path: "docs/discovery/node-dev-tooling-inventory-issue-182.md"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    added:
      - type: "file"
        path: "docs/test-specs/ambient-node-free-rust-test-contract-issue-188.md"
      - type: "file"
        path: "docs/tasks/remove-ambient-node-from-cheaper-tools-fixtures-issue-188.md"
      - type: "file"
        path: "docs/tasks/remove-ambient-node-from-workflow-fixture-issue-188.md"
      - type: "file"
        path: "docs/tasks/synchronize-node-boundary-docs-after-issue-188.md"
      - type: "file"
        path: "docs/impl/ir/remove-ambient-node-from-cheaper-tools-fixtures.md"
      - type: "file"
        path: "docs/impl/ir/remove-ambient-node-from-workflow-fixture.md"
      - type: "file"
        path: "docs/impl/ir/synchronize-node-boundary-records-after-issue-188.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/remove-ambient-node-test-coupling-issue-188.md"
  implemented-by:
    - "docs/tasks/remove-ambient-node-from-cheaper-tools-fixtures-issue-188.md"
    - "docs/tasks/remove-ambient-node-from-workflow-fixture-issue-188.md"
    - "docs/tasks/synchronize-node-boundary-docs-after-issue-188.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/remove-ambient-node-from-rust-tests-issue-188.md"
    - "docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ambient-node-free-rust-test-contract-issue-188.md"
  references: []
  defers: []
  deferred-by: []
---

# Remove ambient Node from Rust test fixtures Implementation Plan

## Goal

Make the three Rust test sites in issue #188 run their existing process and
workflow guarantees without requiring ambient Node, while keeping differential
goldens unchanged.

## Scope

One implementation stream: the process tool contract, differential fixtures,
workflow e2e, and their shared boundary documentation form the single F1
acceptance surface. They must be delivered together so the recorded contract
matches actual test coverage. Do not change product runtime, the Node pin, or
the frozen fixture JSON.

## Upstream Documents

- `docs/specs/remove-ambient-node-test-coupling-issue-188.md`
- `docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md`
- `docs/designs/node-dev-tooling-boundary-contract-issue-182.md`

## Design Inputs

- `docs/designs/remove-ambient-node-from-rust-tests-issue-188.md`

## File Map

- `crates/cheaper-tools/tests/proc_process.rs` — controlled-PATH nested test
  proving bare fixture name resolution, pinning, and spawn.
- `crates/cheaper-tools/tests/differential.rs` — translate the frozen literal
  Node task to the explicit fixture mode using an absolute fixture path.
- `crates/cheaper-tools/src/bin/diff_fixture_child.rs` — add a `bare` mode
  emitting `bare node\n` with exit status 0.
- `crates/cheaper-workflow/tests/process.rs` — replace Node lookup/skip with an
  exact-filter test-binary fixture that writes `out.txt`; retain workflow and
  resource assertions.
- `docs/designs/node-dev-tooling-boundary-contract-issue-182.md` — revise §1.3,
  §§3.1–3.3, and §5 to describe actual post-change coverage.
- `docs/discovery/node-dev-tooling-inventory-issue-182.md` — revise ambient-site
  aggregate, call graph, and risk notes; do not change unrelated Node inventory.
- `AGENTS.md` — replace the stale three-site ambient Node warning with the
  verified issue188 disposition while retaining the Node tooling pin/removal
  gate and no-new-coupling policy.
- `docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md` — add a dated
  factual correction for the F1 limitation recorded at decision time; preserve
  the accepted Node-tooling decision and historical rationale.
- `docs/specs/README.md`, `docs/designs/README.md`, and `docs/plans/README.md` —
  synchronize generated index status rows after artifact status changes.
- `docs/test-specs/README.md` and `docs/tasks/README.md` — synchronize generated
  indexes when the post-approval test spec and tasks are created.
- `docs/impl/ir/README.md` — add the three required per-task IRs to the
  implementation-record index (the impl-doc conventions require a manual
  README update; `audit_impl_record.js` validates the resulting index).

## Implementation Sequence

1. Update `diff_fixture_child` and differential translation for the known
   literal Node task; preserve frozen JSON and expected output. Run both
   `process_fixtures` and `runtime_fixtures` before and after to establish the
   Node-free golden parity.
2. Rework `node_task_resolves_via_path` to re-execute the exact test with PATH
   set only to the fixture binary directory; make the inner test use the bare
   `diff_fixture_child` name and assert successful `bare node\n` output.
3. Replace workflow `resolve_node`/silent skip with the exact test-binary
   fixture child contract (`--exact workflow_fixture_child --nocapture
   out.txt`, `CHEAPER_WORKFLOW_PROCESS_FIXTURE=generate`, `inputs: []`); keep
   producer/consumer and resource validation assertions.
4. Update the parent design's §§1.3, 3.1–3.3, and §5, the inventory's
   ambient-site aggregate/call graph/risk notes, and `AGENTS.md` to the verified
   contracts; retain no claim that F1 is pending. Add a dated ADR note correcting
   the decision-time consequence that cargo tests require ambient Node without
   changing the accepted decision.
5. Regenerate the affected spec/design/plan/task indexes from current front
   matter; preview test-spec/ADR/discovery indexes and apply only if their
   previews report changes. Manually update the implementation-record README
   as each IR is created. Run targeted document audits and assert that the
   frozen process/runtime fixture JSON files are byte-unchanged.

## Dependencies and Risks

- No external dependency, toolchain update, or product API change is required.
- Node remains in the pinned development toolchain under the accepted ADR.

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| Process-wide PATH mutation races with other tests. | Medium | High | Set PATH only on the nested exact test process; do not call `set_var`. |
| A literal `node` declaration in a fixture is pinned even without being invoked. | Medium | High | Translate every literal `node` command in differential process policies, including six shared `runtime.json` declarations. |
| Differential fixture output parity is mistaken for PATH-resolution coverage. | Low | Medium | Keep the dedicated controlled-PATH process test as the single bare-name resolve/pin/spawn proof. |
| Expected output is weakened while editing frozen fixture input. | Low | High | Keep process/runtime JSON unchanged and assert `git diff --exit-code` against base `e5e26284de8a21ac9433e6470811ffd049448853` on both files. |
| Node becomes visible in the no-Node verification environment. | Low | High | Replace WSL PATH in the test process with only Rust bin and Linux system directories, then fail if `command -v node` succeeds. |

## Task Breakdown

1. `cheaper-tools` process and differential fixture task: add the explicit
   `bare` child mode, translate literal Node tasks in process/runtime fixture
   policies, and prove bare-name resolve/pin/spawn under the isolated PATH.
2. Workflow process fixture task: replace Node lookup and silent return with
   the exact test-binary fixture child and retain compile/materialize/execute/
   read/resource assertions.
3. Boundary record synchronization task, dependent on tasks 1 and 2: update the
   parent design, inventory, ADR historical note, and generated indexes; run
   focused doc audits and the frozen-input diff assertion.

Tasks 1 and 2 are independently verifiable; task 3 starts only after both
runtime contracts pass. They remain one issue188 delivery so no partial state
leaves the canonical boundary declaration stale.

## Planning Flow Applied

Independent plan review approved this plan. The planning flow created the
issue188 test spec and linked it through `relations.verified-by`, then created
three task documents with explicit Task Graph dependencies. The cheaper-tools
and workflow tasks link to the issue188 test spec; the documentation task
depends on both test tasks. `relations.implemented-by` and `verified-by` were
synchronized before the task-graph audit, which reported no issues. Each task
created an in-progress Implementation Record before its first code or
documentation mutation.

## Verification

- [x] Task 1 Node-free WSL verification: use an explicit process-local Linux
  PATH, assert Node is unavailable, then prove bare-name resolve/pin/spawn and
  unchanged differential process/runtime outputs without `mise exec`:

  ```sh
  set -e
  cargo_bin="$(rustup which cargo)"
  rustc_bin="$(rustup which rustc)"
  cargo_dir="$(dirname "$cargo_bin")"
  rustc_dir="$(dirname "$rustc_bin")"
  export PATH="$cargo_dir:$rustc_dir:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/lib/wsl/lib"
  test "$(command -v cargo)" = "$cargo_bin"
  test "$(command -v rustc)" = "$rustc_bin"
  case "$PATH" in *mise*|*shims*) echo "unexpected shim path" >&2; exit 1;; esac
  if command -v node >/dev/null 2>&1; then echo "unexpected node on PATH" >&2; exit 1; fi
  export CARGO_TARGET_DIR="$PWD/target/wsl-issue188"
  cargo test -p cheaper-tools --test proc_process node_task_resolves_via_path -- --exact
  cargo test -p cheaper-tools --test differential process_fixtures -- --exact
  cargo test -p cheaper-tools --test differential runtime_fixtures -- --exact
  git diff --exit-code e5e26284de8a21ac9433e6470811ffd049448853 -- crates/cheaper-tools/tests/fixtures/process.json crates/cheaper-tools/tests/fixtures/runtime.json
  ```
- [x] Task 2 Node-free WSL verification: independently establish the explicit
  process-local Linux PATH and `command -v node` fail check, then run:

  ```sh
  set -e
  cargo_bin="$(rustup which cargo)"
  rustc_bin="$(rustup which rustc)"
  cargo_dir="$(dirname "$cargo_bin")"
  rustc_dir="$(dirname "$rustc_bin")"
  export PATH="$cargo_dir:$rustc_dir:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/lib/wsl/lib"
  test "$(command -v cargo)" = "$cargo_bin"
  test "$(command -v rustc)" = "$rustc_bin"
  case "$PATH" in *mise*|*shims*) echo "unexpected shim path" >&2; exit 1;; esac
  if command -v node >/dev/null 2>&1; then echo "unexpected node on PATH" >&2; exit 1; fi
  export CARGO_TARGET_DIR="$PWD/target/wsl-issue188"
  cargo test -p cheaper-workflow --test process compiles_policy_process_producer_and_executes_materialized_jobs -- --exact
  ```
- [x] Task 3 documentation verification: after reviewing each index preview,
  apply the affected canonical indexes; then run `doc_status.js audit --type
  all --json` and
  `.agents/skills/impl-doc/scripts/audit_impl_record.js --json`. The latter
  checks IR schema, relations, sections, and README presence; `doc_status`
  checks relation/index consistency. Filter both reports for the issue188
  spec/design/plan/test-spec/tasks/IRs and updated parent design/inventory/ADR/AGENTS.
  Pass when there is no blocking finding and no target-file relation/index
  warning. Search the three updated source docs:

  ```powershell
  rg -n 'ambient `node` (依存|結合) 3 site|2 件 fail|2 required / 1 silent-skip|silent-skip|F1.*(pending|未実施|未着手)' AGENTS.md docs/designs/node-dev-tooling-boundary-contract-issue-182.md docs/discovery/node-dev-tooling-inventory-issue-182.md docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md
  ```

  Review every match; none may claim the old fail/skip behavior or that F1 is
  pending. Intentional historical context must be explicitly dated.
- [x] Cross-platform Windows targeted verification:

  ```powershell
  mise exec -- cargo test -p cheaper-tools --test proc_process node_task_resolves_via_path -- --exact
  mise exec -- cargo test -p cheaper-tools --test differential process_fixtures -- --exact
  mise exec -- cargo test -p cheaper-tools --test differential runtime_fixtures -- --exact
  mise exec -- cargo test -p cheaper-workflow --test process compiles_policy_process_producer_and_executes_materialized_jobs -- --exact
  ```
- [x] Preview canonical index updates for affected types with:

  ```powershell
  node apm_modules/scarecrow173/apm-packages/packages/doc-driven-dev/.apm/skills/doc-maintenance/scripts/doc_maintenance.js plan --type spec --json
  node apm_modules/scarecrow173/apm-packages/packages/doc-driven-dev/.apm/skills/doc-maintenance/scripts/doc_maintenance.js plan --type design --json
  node apm_modules/scarecrow173/apm-packages/packages/doc-driven-dev/.apm/skills/doc-maintenance/scripts/doc_maintenance.js plan --type plan --json
  node apm_modules/scarecrow173/apm-packages/packages/doc-driven-dev/.apm/skills/doc-maintenance/scripts/doc_maintenance.js plan --type adr --json
  node apm_modules/scarecrow173/apm-packages/packages/doc-driven-dev/.apm/skills/doc-maintenance/scripts/doc_maintenance.js plan --type discovery --json
  node apm_modules/scarecrow173/apm-packages/packages/doc-driven-dev/.apm/skills/doc-maintenance/scripts/doc_maintenance.js plan --type test-spec --json
  node apm_modules/scarecrow173/apm-packages/packages/doc-driven-dev/.apm/skills/doc-maintenance/scripts/doc_maintenance.js plan --type task --json
  ```

  After reviewing those previews, apply the affected index updates and audit:

  ```powershell
  node apm_modules/scarecrow173/apm-packages/packages/doc-driven-dev/.apm/skills/doc-maintenance/scripts/doc_maintenance.js apply --type spec --json
  node apm_modules/scarecrow173/apm-packages/packages/doc-driven-dev/.apm/skills/doc-maintenance/scripts/doc_maintenance.js apply --type design --json
  node apm_modules/scarecrow173/apm-packages/packages/doc-driven-dev/.apm/skills/doc-maintenance/scripts/doc_maintenance.js apply --type plan --json
  node apm_modules/scarecrow173/apm-packages/packages/doc-driven-dev/.apm/skills/doc-maintenance/scripts/doc_maintenance.js apply --type test-spec --json
  node apm_modules/scarecrow173/apm-packages/packages/doc-driven-dev/.apm/skills/doc-maintenance/scripts/doc_maintenance.js apply --type task --json
  node apm_modules/scarecrow173/apm-packages/packages/doc-driven-dev/.apm/skills/doc-status/scripts/doc_status.js audit --type all --json
  ```
- [x] Run the canonical `mise run check` once after implementation and all
  document/index updates are complete.

## Open Questions / Gaps

None. The issue, accepted ADR, and approved issue188 design define the expected
behavior, file boundaries, fixture modes, and Node-free verification path.

## Self-Review

- [x] Every spec requirement maps to a concrete task, test-spec, and evidence.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: delegated execution with review checkpoints, as already
  authorized for issue #188.
- Independent work streams: `cheaper-tools` process/differential fixtures and
  workflow fixture; documentation synchronization depends on both.
- User approval required before dispatch: already granted in the issue188
  delivery authorization.
- Implementation was dispatched task-by-task with review checkpoints; the
  implementation records and verification matrix capture the completed work.

## Review Handoff

- Verify that differential fixtures preserve expected bytes but no longer own
  PATH-resolution coverage; the dedicated process test must resolve and spawn
  the bare fixture name with only its binary directory in PATH.
- Verify the six runtime fixture grant declarations are translated without
  editing the frozen JSON corpus.
- Verify workflow no-Node execution runs the producer and consumer assertions
  rather than succeeding through an early return.
- Verify frozen fixture JSON and expected golden outputs remain unchanged, and
  the ADR correction preserves its decision-time rationale.
