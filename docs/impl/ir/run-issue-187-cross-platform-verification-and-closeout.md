---
id: "IMPL-034MyXn5zz5WT1tcl9CQvd"
type: "impl"
status: "in-progress"
title: "Run issue 187 cross-platform verification and closeout"
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
    - "docs/tasks/run-issue-187-cross-platform-verification-and-closeout.md"
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

# Run issue 187 cross-platform verification and closeout

## Summary

Platform verification gates and final evidence synchronization are complete. Historical
failure causes remain unresolved where bounded evidence did not establish them;
the stabilized test contracts and discovered Unix fixture mismatch are recorded
separately. The graph router still selects a briefing repair edge at
followup-triage; its focused gate evidence is awaiting separate read-only
diagnosis, so graph exit is not claimed complete.

## Context

Run `mise install`, `mise run setup`, and `mise run check` on the Windows host
three consecutive times. Run the canonical POSIX sequence in Ubuntu CI. Record
host, command, run identifiers, outcomes, and unresolved conditions.
Historical causes remain unresolved unless new direct evidence proves
otherwise.

## Changes

- Synchronized the discovered POSIX exec-deny fixture failure, baseline and
  green reproduction, final Ubuntu CI run, Windows run chronology, and
  unresolved symptom dispositions into the plan, discovery, task, and IR.

## Implementation

This closeout task made documentation-only updates after the code snapshot
`660cf138541bb77c0318885ad9b50ffb2dbe7dde` passed canonical Ubuntu CI and the
Windows run set was complete on the unchanged Windows code path at
`16f922a994afe8c41205bdf40eb02759f0f4d670`. No product code or test was changed
for this documentation checkpoint.

## Related Experiments

### Adopted

- The exact WSL baseline/fixed comparison and canonical CI outcomes were
  adopted as evidence for the Unix fixture correction and cross-platform
  disposition.

### Rejected

- Treating the missing Windows run 1 numeric exit code as observed exit 0 was
  rejected; the full-pipeline completion is recorded separately from explicit
  exit codes 0 for runs 2 and 3.

## Validation

- `mise install` and `mise run setup` passed on Windows at SHA `2de497f`; APM emitted a
  non-fatal warning that private policy repo `scarecrow173/.github-private` was
  unavailable. Workspace build, hook install, and local Git LFS setup completed.
- First `mise run check` on SHA `1f45fb4` passed Biome/fmt but failed Clippy on
  the test-only hook type alias; not counted toward the required three runs.
- First `mise run check` on SHA `2de497f` passed Biome/fmt but failed Clippy on
  an unnecessary borrow in the long-path assertion; not counted. Targeted full
  workspace Clippy then passed after removing the borrow.
- Ubuntu run `40310673489` targeted SHA `1f45fb4`; it was cancelled after the
  known Clippy failure. Ubuntu run `40310922367` targeted SHA `2de497f`; it was
  cancelled after the second known Clippy failure. Neither is POSIX verification
  evidence. A fresh run must target the corrected immutable head.
- Windows host `DESKTOP-NH00KUI` (mise 2026.10.1, Rust/Cargo 1.99) ran canonical `mise run check` three consecutive times on SHA
  `16f922a994afe8c41205bdf40eb02759f0f4d670`, following successful `mise
  install` and `mise run setup`. Run 1/session `74340` completed the pipeline
  with no error marker and all final stages successful, but the wrapper did not
  retain its numeric exit code. Runs 2/session `56212` and 3/session `87696`
  each retained explicit exit code 0. The later `660cf13` change only affects a
  `#[cfg(unix)]` fixture and is not compiled on Windows.
- Canonical Ubuntu workflow `40311338267` on `16f922a` failed the Unix
  `security_exec_deny_pin_blast_radius` fixture with `workspace escape:
  symlink-denied`; WSL reproduced the failure on baseline and passed after the
  fixture-only `WithinWorkspace` correction. This did not weaken the deny
  assertion or change production policy.
- Canonical Ubuntu workflow `40314463285` passed on code SHA
  `660cf138541bb77c0318885ad9b50ffb2dbe7dde`: `mise install`, `mise run setup`,
  `mise run check`, and cleanup succeeded.
- Earlier bounded Windows focused checks passed for the ETXTBSY diagnostic
  branch, but no concurrent writer was injected and ETXTBSY did not recur. The
  historical resources stall mechanism and Windows liveness failure cause also
  remain unproven. The #157 route behavior is already covered; no route change
  was made. No retry or skip workaround was added.
- The code snapshot that received final cross-platform gates is
  `660cf138541bb77c0318885ad9b50ffb2dbe7dde` for POSIX and
  `16f922a994afe8c41205bdf40eb02759f0f4d670` for Windows; the only intervening
  code change is the Unix-only fixture correction described above.
- Focused route command:
  `node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js --current followup-triage --signal followup-terminal --focus docs/plans/stabilize-issue-187-posix-suite-residual-flakes.md --json`.
  It returned `followup-triage-to-briefing-repair` / `briefing-not-pass`, with
  no task-graph issues; all four plan tasks are `done`. The requested
  followup-terminal signal did not override the earlier gate-repair edge. The
  focused gate's supporting artifact IDs/statuses remain under separate
  read-only diagnosis; no unrelated briefing repair is included here.

## Risks

- Full verification may expose environment-specific failures; preserve the
  exact output and distinguish environmental limitation from code failure.
- A non-reproducing symptom remains an unresolved cause.

## Follow-ups

- Update issue #187 discovery evidence and this record after each bounded gate.
- No further code change is justified by the bounded evidence. Preserve the
  historical unresolved causes in any issue closeout; do not state that the
  original symptoms were all reproduced or root-caused. Do not mark graph exit
  complete until the briefing gate route finding is resolved or explicitly
  classified by the canonical router protocol.
