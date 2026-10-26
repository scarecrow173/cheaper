---
id: "SPEC-034MBouypEDoaGeJu56EWi"
type: "spec"
status: "approved"
title: "Remove ambient Node coupling from Rust test fixtures (issue #188)"
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
  implemented-by:
    - "docs/plans/remove-ambient-node-from-rust-tests-issue-188.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines:
    - "docs/designs/node-dev-tooling-boundary-contract-issue-182.md"
  refined-by: []
  derives-from:
    - "docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
  derived-by:
    - "docs/designs/remove-ambient-node-from-rust-tests-issue-188.md"
    - "docs/test-specs/ambient-node-free-rust-test-contract-issue-188.md"
  verifies: []
  verified-by:
    - "docs/test-specs/ambient-node-free-rust-test-contract-issue-188.md"
  references: []
  defers: []
  deferred-by: []
---

# Remove ambient Node coupling from Rust test fixtures (issue #188)

## What Should Be Built

Rust test coverage for process and workflow behavior must remain complete when
Node is absent from the environment running `cargo test`. The tests must retain
the process tool's executable-name resolution and spawn guarantee and preserve
the current differential fixture outputs.

## Why Now

Issue #188 implements follow-up F1 from the approved Node dev-tooling boundary
design. Today, two `cheaper-tools` test paths require Node to pass, while a
`cheaper-workflow` test silently skips when Node is absent. The silent skip
reduces coverage without making the test command fail. This coupling is not
needed by the test contracts and makes Rust verification depend on an
unrelated repository tooling runtime.

## Users and Value

- Rust contributors running crate tests outside the full mise-managed toolchain
  get deterministic process and workflow coverage without an ambient Node
  executable.
- Maintainers retain visible, required coverage for executable resolution and
  workflow materialization instead of accepting a silent skip.

## Scope

### In Scope

- The process-tool contract for a bare executable name resolving through PATH
  and spawning successfully.
- The process differential fixture's expected stdout and exit-code behavior.
- The workflow compile/materialize/execute/read contract running on every test
  environment.
- No uninvoked or shared process-task fixture declaration may cause the
  differential suite to resolve ambient Node while pinning executable grants.
- The ambient-site table in the approved issue-182 design reflects the new
  test contracts after implementation.

### Out of Scope

- Removing Node from `mise.toml`, changing its pinned version, or migrating repo
  tooling.
- Changing production process policy or making literal `node` executable
  names generally exempt from policy.
- General JavaScript emulation or broad edits to the frozen fixture corpus.

## Requirements

- The three test sites named in issue #188 must complete successfully when
  their test process PATH has no Node executable.
- The process-tool test must exercise bare-name lookup through PATH and a
  successful spawn; only preserving an equivalent output is insufficient.
- Differential outputs, including `bare node\n` and exit code 0, remain
  unchanged.
- The workflow test must execute the full producer, consumer, and resource
  assertions rather than return early based on host tool availability.
- Existing literal Node commands in process fixture policies must not be
  resolved as ambient Node, including shared declarations used by the
  differential runtime fixture suite.

## Acceptance Criteria

- [ ] `cheaper-tools` process resolution and differential fixture tests pass
  with Node excluded from the environment.
- [ ] The process-tool integration test proves bare executable name → PATH
  resolution → spawn, with no process-wide PATH mutation visible to sibling
  tests.
- [ ] `process.json` and `runtime.json` expected golden output remains
  unchanged while their literal Node task commands no longer require Node.
- [ ] The workflow producer/consumer test runs to completion with Node absent
  and asserts generated file contents and resource behavior.
- [ ] The approved issue-182 design's §1.3 and §3.1–3.2, plus inventory
  aggregates, call graph, and risk notes, accurately describe per-site coverage
  and the no-Node behavior.

## Deferred Design Concerns

Node remains a pinned repository dev-tool and product/runtime boundaries remain
unchanged under accepted ADR-034LRjX5iYZHrubmKgKAL1.

## Sources and Relations

Issue #188 narrows the F1 follow-up already specified in
`docs/designs/node-dev-tooling-boundary-contract-issue-182.md` §5. The accepted
Node runtime decision remains in
`docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md`; this spec does not
reopen that decision.
