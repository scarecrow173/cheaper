---
id: "DESIGN-034MB9MsJSa0KMiqf3O8Tc"
type: "design"
status: "approved"
title: "Remove ambient Node from Rust test fixtures (issue #188)"
created: "2026-10-26"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/188"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/remove-ambient-node-test-coupling-issue-188.md"
    - "docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
    - "docs/designs/node-dev-tooling-boundary-contract-issue-182.md"
  derived-by:
    - "docs/plans/remove-ambient-node-from-rust-tests-issue-188.md"
    - "docs/test-specs/ambient-node-free-rust-test-contract-issue-188.md"
  verifies: []
  verified-by:
    - "docs/test-specs/ambient-node-free-rust-test-contract-issue-188.md"
  references: []
  defers: []
  deferred-by: []
---

# Remove ambient Node from Rust test fixtures (issue #188)

## Context

The accepted Node dev-tooling decision keeps Node pinned for repository
tooling, but does not require Rust integration tests to depend on the ambient
Node executable. Three test sites currently do so: a required process-tool
PATH test, its differential golden fixture, and a workflow test that silently
returns when Node is absent. The approved parent design assigns those sites to
F1 and requires the bare-name → PATH resolution → spawn contract to remain
covered while `cargo test` works without Node.

The contract belongs to the process executable resolver and its real spawn
path. The implementation will use the existing `diff_fixture_child` binary
as the controlled executable for process and differential fixtures. A nested
test process will receive a PATH containing only that binary's directory, so
the test exercises a bare executable name without changing the parent test
process environment. The workflow test's child script is only a way to
exercise materialization and execution; it will use the existing Rust test
binary as a fixture child and always run.

## Scope

### In Scope

- Replace `node_task_resolves_via_path`'s Node command with a bare
  `diff_fixture_child` command, using a nested test process and a controlled
  PATH containing the fixture binary directory.
- Translate the literal `node` process fixture to `diff_fixture_child` in the
  differential harness while retaining the frozen golden output
  (`bare node\n`, exit code 0).
- Replace the workflow test's Node lookup and silent skip with an always-run
  Rust fixture child while preserving compile, materialize, execute, read,
  and resource validation assertions.
- Translate the literal `node` task declaration used in `runtime.json` through
  the same narrow fixture-only mapping; leave the frozen input corpus intact.
- Add an explicit `bare` mode to `diff_fixture_child` that emits exactly
  `bare node\n` and exits successfully.
- Update the approved Node boundary design's §1.3 and §3.1–3.2 per-site
  descriptions, counts, and no-Node implications, plus the inventory's ambient
  site aggregate, call graph, and risk notes so no stale ambient coupling
  claim remains.

### Out of Scope

- Changing the pinned Node toolchain, production process resolver, policy
  surface, or the `mise run check` pipeline.
- Broad edits to process fixture semantics or unrelated corpus cases.
- Changing the expected stdout/exit-code golden contract.

## Components and Boundaries

- `cheaper-tools/tests/proc_process.rs` owns the externally observable
  process-tool contract: configured bare name resolves through the controlled
  PATH, is pinned, and spawns successfully.
- `cheaper-tools/tests/differential.rs` owns only translation from the frozen
  fixture's narrow Node-script idioms to `diff_fixture_child` modes; the JSON
  corpus remains the source fixture and expected golden output is unchanged.
- `cheaper-tools/src/bin/diff_fixture_child.rs` owns the explicit `bare` mode
  used by the differential fixture and controlled-PATH process test.
- `cheaper-workflow/tests/process.rs` owns workflow compile/materialize/execute
  integration. Its fixture child is language-independent and has no optional
  dependency gate.
- `cheaper-tools/src/tools/proc/executable.rs` and production policy code are
  unchanged; existing resolver behavior remains the owner of PATH lookup.

## Data and Control Flow

1. The process PATH-contract test re-executes its test binary with a marker and
   PATH set to the directory containing `diff_fixture_child`; it does not
   mutate PATH in a process shared by parallel tests.
2. The marked test configures a bare `diff_fixture_child` task. Existing grant
   pinning resolves that name from PATH and the process tool spawns the
   resolved fixture binary, which emits `bare node\n` in its selected mode.
3. The differential translator recognizes the frozen literal `node` case and
   maps its known script to the same fixture mode using the fixture binary's
   absolute path. Unknown scripts continue to fail loudly; the expected
   normalized result remains unchanged. This fixture case no longer exercises
   PATH lookup; that contract is consolidated in
   `node_task_resolves_via_path`, which will exercise bare-name lookup, pinning,
   and spawn together under a controlled PATH. The `bare` mode emits exactly
   `bare node\n` and exits with code 0.
4. The workflow producer invokes its test-binary fixture child at
   `current_exe` with arguments `--exact workflow_fixture_child --nocapture
   out.txt` and env marker `CHEAPER_WORKFLOW_PROCESS_FIXTURE=generate`. The
   child test writes `generated` to `out.txt` in its inherited workspace cwd;
   the consumer reads that file. The policy keeps `outputs: ["out.txt"]` and
   `inputs: []` because the fixture no longer executes a script. The test
   always runs, including when Node is absent.

The differential harness applies the literal `node` translation to every
fixture process policy, including the six shared declarations in
`runtime.json`. Those declarations are not removed: keeping the frozen corpus
unchanged avoids changing captured inputs while ensuring no fixture grant is
pinned to ambient Node.

## Non-Functional Constraints

- Tests must not mutate process-wide PATH, depend on ambient Node, or silently
  skip coverage.
- Keep the test-only alias translation explicit and narrow. Do not introduce
  Node emulation into product code or expose a new production option.
- Preserve cross-platform executable naming and spawning (`.exe` on Windows,
  executable fixture binary on POSIX).
- Keep the golden stdout and exit status stable and verify the bare-name path
  through actual process-tool execution, not only resolver unit coverage.
- The differential case preserves output and exit-code parity but transfers
  PATH-resolution coverage to the dedicated process-tool contract test; it
  must not be described as independently retaining PATH-resolution coverage.

## Risks and Trade-offs

- Nested test re-execution adds a small process startup cost; bound it by
  invoking the exact named test with a marker and checking its exit status.
- The differential mapping models the one frozen `node -e` program, not
  general JavaScript. Keep unknown scripts fail-closed and preserve golden
  output comparison. The literal executable name is not part of the contract;
  the guarantee is name-independent bare executable lookup through PATH.
- A separate Rust fixture in the workflow test duplicates a small amount of
  child behavior, but avoids introducing cross-crate fixture infrastructure.

## Deferred Design Concerns

No new follow-up is introduced. The six literal `node` declarations in
`runtime.json` are resolved while grants are pinned even if no operation calls
them; translating them is therefore required for the runtime fixture target
to pass without Node.

## Verification Notes

- [ ] Process-tool integration proves bare fixture name → isolated PATH lookup
  → spawn and does not use Node.
- [ ] Differential process corpus passes with unchanged expected output and no
  literal ambient `node` fixture command.
- [ ] The fixture case's absolute-path translation is explicit, and the
  dedicated process-tool contract test covers bare-name PATH resolution and
  spawn.
- [ ] Workflow process test executes rather than returns early when PATH has no
  Node and still validates produced content and resource inference.
- [ ] In the Node-free WSL Rust environment, confirm `command -v node` returns
  no result, then pass each of these commands:

  ```sh
  cargo test -p cheaper-tools --test proc_process node_task_resolves_via_path -- --exact
  cargo test -p cheaper-tools --test differential process_fixtures -- --exact
  cargo test -p cheaper-tools --test differential runtime_fixtures -- --exact
  cargo test -p cheaper-workflow --test process compiles_policy_process_producer_and_executes_materialized_jobs -- --exact
  ```
