---
id: "TSPEC-034MCAP6oUmUL9GA7GfisH"
type: "test-spec"
status: "draft"
title: "Ambient Node free Rust test contracts"
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
    - "docs/designs/remove-ambient-node-from-rust-tests-issue-188.md"
  derived-by: []
  verifies:
    - "docs/specs/remove-ambient-node-test-coupling-issue-188.md"
    - "docs/designs/remove-ambient-node-from-rust-tests-issue-188.md"
  verified-by:
    - "docs/tasks/remove-ambient-node-from-cheaper-tools-fixtures-issue-188.md"
    - "docs/tasks/remove-ambient-node-from-workflow-fixture-issue-188.md"
    - "docs/tasks/synchronize-node-boundary-docs-after-issue-188.md"
  references: []
  defers: []
  deferred-by: []
---

# Ambient Node free Rust test contracts

## Purpose

Preserve the process, differential-fixture, and workflow guarantees currently
covered by Rust tests that implicitly depend on Node. This test spec can be
retired only if those contracts are intentionally removed or replaced by an
equivalent approved contract.

## Feature

Feature: Rust integration tests exercise their process contracts without
ambient Node.

## Rules

- Rule: the process-tool fixture resolves a bare executable name from a
  process-local controlled PATH, pins it, and successfully spawns it without
  mutating process-global PATH.
- Rule: differential process and runtime fixtures preserve their frozen JSON
  inputs and expected output semantics while using the explicit Rust fixture
  executable for the literal Node cases.
- Rule: workflow process coverage executes its fixture child and retains
  workflow/resource assertions when Node is absent; it must not silently skip.

## Examples

- Example: Given a child process PATH containing the fixture directory but no
  Node, when the process tool receives the fixture's bare filename, then grant
  pinning and spawn succeed and return the expected `bare node\n` output.
- Example: Given the checked-in process/runtime fixture JSON, when differential
  tests translate literal Node commands, then golden output remains unchanged
  and the JSON files remain byte-identical.
- Example: Given Node is absent, when the workflow process test runs, then its
  fixture child creates the declared output and producer/consumer/resource
  assertions execute instead of returning early.

## Guarantees

- The three required Rust test sites pass with a process-local PATH that cannot
  resolve Node.
- The dedicated process-tool test retains a real bare-name lookup, grant pin,
  and spawn proof; differential output parity is not counted as that proof.
- The six runtime fixture Node declarations are translated so fixed grant
  pinning succeeds without changing frozen fixture inputs.
- The differential golden outputs and workflow behavior remain semantically
  unchanged.

## Non-goals

- Removing Node from mise, changing repository tooling, emulating arbitrary
  JavaScript/Node behavior, or changing product/distribution runtime behavior.

## Risk

- A future fixture edit could reintroduce environment-dependent Rust tests,
  weaken the bare-name process contract, or mask workflow regressions behind a
  silent skip.
