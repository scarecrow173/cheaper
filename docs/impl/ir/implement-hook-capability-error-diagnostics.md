---
id: "IMPL-034MyAvIcswWUdbS5yKM0n"
type: "impl"
status: "completed"
title: "Implement hook capability error diagnostics"
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
    - "docs/tasks/separate-hook-capability-error-diagnostics.md"
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

# Implement hook capability error diagnostics

## Summary

Implemented the failure-only capability diagnostics and contract coverage.
The task remains in progress pending the canonical POSIX and three-run Windows
verification gates. Judge accepted the bounded `ETXTBSY` non-reproduction as an
unresolved disposition; it does not identify or resolve the historical cause.

## Context

Follows `docs/tasks/separate-hook-capability-error-diagnostics.md`, the
approved #187 plan, its hook diagnostic test-spec, and the accepted
deterministic test-boundary ADR.

## Implementation

### Changes

- At `check_capability`, `Command::output() -> Err(io::Error)` reports the
  `ErrorKind`, raw OS code, bounded message, current source executable and PATH
  executable paths, file length, and modified/accessed/created timestamps. It
  does not claim a child, status, stdout, or stderr.
- A nonzero `Ok(Output)` reports the process status, bounded stdout and stderr
  separately, and the same source/PATH executable metadata. It is not labeled a
  spawn error.
- Diagnostic text is bounded: path (512 bytes), captured stream (96 bytes),
  component text (192 bytes), and total message (2,048 bytes including any
  truncation marker).
- Added a portable invalid-executable integration case for the `Err` branch and
  a Unix executable-script case for nonzero `Output`. Retained the pre-existing
  `hook_config_sync_warns_on_legacy_conflict` test unchanged.

### Decisions During Implementation

- Used `current_exe()` as the source executable and the resolved PATH candidate
  as the copied/selected executable. Both metadata records are gathered at the
  failure boundary; no writer or retry behavior is inferred.
- The local host is Windows, so the Unix-only nonzero shell-script test could
  not run here. The generic spawn-error case uses an invalid executable image
  and exercised Windows `ErrorKind::Uncategorized` / raw OS error 216.

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_config::hook_config_capability_spawn_error_reports_only_io_error_evidence -- --exact` — passed (1 test). Before implementation the same test failed because the generic spawn diagnostic lacked the required fields.
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_config::` — passed twice (27 passed, 0 failed each run), including the legacy conflict warning assertion. The hook-config cases ran concurrently; historical `ETXTBSY` did not recur in either bounded run.
- `mise exec -- cargo fmt --check` — passed.
- The focused run was on the Windows host, current worktree revision after planning commit `b88c9a1`; no Unix-host result is claimed.
- Independent review noted that the Err assertion did not protect the OS error message field and the original 128-byte path cap could obscure long executable paths. Added a message-field assertion and raised the per-path bound to 512 bytes.
- Follow-up resolved the caller/source ambiguity at the fixture boundary: the Err case now launches `env!("CARGO_BIN_EXE_cheaper")` as a subprocess while PATH resolves to the deliberately invalid fixture executable. The product `current_exe()` is therefore the actual source CLI binary for this case; no source-copy lineage is inferred by product code.
- Long executable paths now retain both ends plus an explicit truncation marker, byte length, and stable FNV-1a digest inside the path bound. The Err focused test passed after these changes (1 passed).
- Added a unit test for a 601-byte path that asserts the bound, explicit truncation marker, preserved prefix/suffix, and digest presence; the focused unit-test passed (1 passed).
- The integration path assertions now accept exact values within 512 bytes and validate the explicit bounded identity form for longer fixture paths, avoiding false failure solely from a long checkout path.
- The overall text helper now reserves bytes for the truncation marker, so the rendered error does not exceed the declared 2,048-byte cap. Its focused boundary tests passed (2 passed, including multibyte text).
- `mise exec -- cargo fmt --check` passed after the final test formatting changes.

## Risks

- The historical writer/operation behind `ETXTBSY` remains unknown. No recurrence in
  two concurrent focused suites is not a root-cause resolution. Independent review
  of this supported unresolved disposition is still required.
- The Unix `Ok(Output)` shell-script test remains unexecuted on this Windows host.

## Follow-ups

- Obtain coordinator/independent review of the bounded no-recurrence evidence and
  unresolved `ETXTBSY` disposition before marking the task complete.
