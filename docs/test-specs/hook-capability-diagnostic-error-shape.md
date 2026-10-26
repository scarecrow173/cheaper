---
id: "TSPEC-034MyrGbD5n5Bu9SnpiKXF"
type: "test-spec"
status: "draft"
title: "Hook capability diagnostic error shape"
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
    - "docs/adr/prefer-deterministic-test-boundary-diagnosis-for-residual-suite-flakes.md"
  derived-by: []
  verifies:
    - "docs/specs/stabilize-posix-suite-residual-flakes-issue-187.md"
    - "docs/designs/issue-187-posix-suite-residual-flake-stabilization.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Hook capability diagnostic error shape

## Purpose

Preserve truthful failure evidence for the hook capability check. Keep this
guarantee while `Command::output` is used at this boundary; retire it only if
the invocation boundary changes and replacement diagnostics preserve the same
distinction.

## Feature

Feature: capability diagnostics describe only the observed command result

## Rules

- Rule: `Err(io::Error)` reports `ErrorKind`, raw OS code, message, source path,
  copied executable path, and bounded metadata/timestamps for the executable.
  Paths are complete within their bound; longer paths explicitly mark
  truncation and retain leading/trailing text, original byte length, and a
  stable non-cryptographic identifier.
- Rule: an `Err(io::Error)` diagnostic does not report a child, exit status,
  stdout, or stderr because no `Output` exists.
- Rule: `Ok(Output)` reports its status and bounded stdout/stderr separately;
  a nonzero status is not described as a spawn error.
- Rule: diagnostics preserve the legacy-conflict warning and are failure-only
  and bounded.

## Examples

- Example: Given execution returns `Err(io::Error::from_raw_os_error(...))`,
  when capability diagnostics are rendered, then they include both source and
  copy paths with OS error fields and omit child/status/output fields.
- Example: Given execution returns `Ok(Output)` with a nonzero status, when
  diagnostics are rendered, then they include status and bounded captured
  streams without labeling the result as a spawn error.

## Guarantees

- Error diagnostics reflect only fields present in the corresponding Rust
  result variant and identify the exact executable artifacts inspected.
- The existing warning behavior remains asserted.

## Non-goals

- The external writer or operation that caused historical `ETXTBSY` when no
  failure-time evidence identifies it.
- Retry behavior or a speculative filesystem/runtime fix.

## Risk

- Mixing `Err` and `Ok` evidence invents process facts and prevents a maintainer
  from locating which copied executable failed.
