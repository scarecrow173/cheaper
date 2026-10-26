---
id: "TASK-034MysZAmZ6pNgfgX08O8Q"
type: "task"
status: "done"
title: "Separate hook capability error diagnostics"
created: "2026-10-26"
updated: "2026-10-26"
owners: ["scarecrow173"]
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
    - "docs/plans/stabilize-issue-187-posix-suite-residual-flakes.md"
  implemented-by: []
  depends-on:
    - "docs/plans/stabilize-issue-187-posix-suite-residual-flakes.md"
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
    - "docs/test-specs/hook-capability-diagnostic-error-shape.md"
  references: []
  defers: []
  deferred-by: []
---

# Separate hook capability error diagnostics

## Work

At the `check_capability` boundary in `crates/cheaper-cli/src/hook_config.rs`,
emit bounded failure-only diagnostics that distinguish
`Command::output() -> Err(io::Error)` from `Ok(Output)`. Identify both the
source executable and copied PATH executable, plus bounded metadata/timestamps.
For `Err`, include `ErrorKind`, raw OS code, and message without fabricated
child/status/stdout/stderr. For `Ok`, record status and bounded stdout/stderr.
Preserve `hook_config_sync_warns_on_legacy_conflict` behavior; do not add retries
or infer an `ETXTBSY` writer without captured evidence.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/stabilize-issue-187-posix-suite-residual-flakes.md`
- `docs/specs/stabilize-posix-suite-residual-flakes-issue-187.md`
- `docs/designs/issue-187-posix-suite-residual-flake-stabilization.md`
- `docs/adr/prefer-deterministic-test-boundary-diagnosis-for-residual-suite-flakes.md`

## Dependencies

- Requires the approved plan and linked test-spec; no other code task is a prerequisite.

## Affected Files

- `crates/cheaper-cli/src/hook_config.rs`
- `crates/cheaper-cli/tests/integration/hook_config.rs`

## Done When

- [x] `Err(io::Error)` diagnostics identify `ErrorKind`, raw OS code, message, source path, copied executable path, and bounded metadata/timestamps only.
- [x] `Ok(Output)` diagnostics report status and bounded stdout/stderr separately; legacy conflict warning assertion remains.
- [x] Attempt the reported parallel condition with bounded reproduction; if `ETXTBSY` does not recur, record exact conditions and uncertainty, and obtain an independently reviewed supported disposition.
- [x] No retry, skip, assertion weakening, or speculative production behavior change is introduced.

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_config::` exits 0.
- [x] `mise exec -- cargo fmt --check` exits 0.

## Implementation Evidence (2026-10-26)

- On the Windows host, the focused documented command completed twice with 27
  passed, 0 failed each run. Default test-thread scheduling exercised parallel
  hook-config fixtures that each copied and executed a PATH shim; `ETXTBSY` did
  not recur in either run.
- These were two bounded suite runs. No concurrent writer was intentionally
  injected, so it does not establish the historical trigger or exclude a
  writer race. Judge accepted the supported unresolved disposition; the cause
  remains unknown.
- The Windows spawn-error contract test passed with an invalid executable
  image (`ErrorKind::Uncategorized`, raw OS error 216); the Unix nonzero-output
  shell-script case is platform-gated and was not run on this host.
- After review, the Err test now launches the known `CARGO_BIN_EXE_cheaper`
  binary as a subprocess and confirms that source path beside the invalid PATH
  executable path. It passed once after the change. Long paths retain both
  ends plus an explicit truncation marker, byte length, and stable digest; the
  regression also asserts that the OS error message field is present.
- A new unit test covers a 601-byte path and checks the 512-byte cap, retained
  prefix/suffix, explicit truncation length, and stable identifier; focused
  diagnostic unit tests passed (2 passed, including strict text cap).
- Final focused hook-config module run on the Windows host passed all 27 tests
  after the follow-up changes. Unix-specific `Ok(Output)` shell test remains
  part of the canonical POSIX verification task.
- `mise exec -- cargo fmt --check` passed.
