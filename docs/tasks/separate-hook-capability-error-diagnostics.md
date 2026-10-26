---
id: "TASK-034MysZAmZ6pNgfgX08O8Q"
type: "task"
status: "in-progress"
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

- [ ] `Err(io::Error)` diagnostics identify `ErrorKind`, raw OS code, message, source path, copied executable path, and bounded metadata/timestamps only.
- [ ] `Ok(Output)` diagnostics report status and bounded stdout/stderr separately; legacy conflict warning assertion remains.
- [ ] Attempt the reported parallel condition with bounded reproduction; if `ETXTBSY` does not recur, record exact conditions and uncertainty, and obtain an independently reviewed supported disposition.
- [ ] No retry, skip, assertion weakening, or speculative production behavior change is introduced.

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_config::` exits 0.
- [ ] `mise exec -- cargo fmt --check` exits 0.
