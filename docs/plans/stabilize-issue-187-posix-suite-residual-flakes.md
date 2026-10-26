---
id: "PLAN-034My32omcNgMPYj4q0kUz"
type: "plan"
status: "approved"
title: "Stabilize issue 187 POSIX suite residual flakes"
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
    - "docs/specs/stabilize-posix-suite-residual-flakes-issue-187.md"
  implemented-by:
    - "../tasks/synchronize-resources-abort-window-regression.md"
    - "../tasks/separate-hook-capability-error-diagnostics.md"
    - "../tasks/use-stable-windows-process-handle-termination-oracle.md"
    - "../tasks/run-issue-187-cross-platform-verification-and-closeout.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-187-posix-suite-residual-flake-stabilization.md"
    - "docs/adr/prefer-deterministic-test-boundary-diagnosis-for-residual-suite-flakes.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/resources-abort-window-phase-synchronization.md"
    - "../test-specs/hook-capability-diagnostic-error-shape.md"
    - "../test-specs/windows-process-handle-termination-oracle.md"
    - "../test-specs/policy-gated-command-tool.md"
  references: []
  defers: []
  deferred-by: []
---

# Stabilize issue 187 POSIX suite residual flakes Implementation Plan

## Goal

Stabilize the #187 resources, hook-config, and Windows process-liveness test
boundaries with deterministic synchronization and bounded diagnostics while
preserving each existing behavioral assertion and leaving unresolved causes
explicit.

## Scope

This is one suite-stabilization stream because all three work areas share the
same release intent and the required final POSIX/Windows verification gates.
The resources unit test, hook-config capability diagnostic, and Windows
process test are independently implementable and reviewable slices. The route
test is excluded from edits because #157 already covers its reported symptom.
Issue #188 Node cleanup and issue #162 host-environment work are out of scope.

## Upstream Documents

- [Approved spec](../specs/stabilize-posix-suite-residual-flakes-issue-187.md)
- [Accepted ADR](../adr/prefer-deterministic-test-boundary-diagnosis-for-residual-suite-flakes.md)

## Design Inputs

- [Approved design](../designs/issue-187-posix-suite-residual-flake-stabilization.md)

## File Map

| Path | Responsibility | Owner slice |
| --- | --- | --- |
| `crates/cheaper-tools/src/resources.rs` | Add private instance-scoped `#[cfg(test)]` one-shot phase hook and deterministic regression at successful pre-check / before enqueue; bound phase, release, request, result, and worker completion waits | Resources |
| `crates/cheaper-cli/src/hook_config.rs` | Separate `Command::output` `Err(io::Error)` evidence from `Ok(Output)` diagnostics at the capability-check owner | Hook diagnostics |
| `crates/cheaper-cli/tests/integration/hook_config.rs` | Preserve legacy-conflict warning assertion and exercise bounded failure-only diagnostic shape under the reported concurrent condition where available | Hook diagnostics |
| `crates/cheaper-tools/tests/proc_command.rs` | Replace PID-substring liveness oracle with same-handle bounded Windows termination observation and correct the Unix-only in-workspace symlink fixture used to reach resolved-basename exec-deny validation | Process command tests |
| `docs/discovery/posix-suite-residual-flakes-issue-187.md` | Record evidence and disposition for each symptom without claiming unresolved root causes | Evidence / final disposition |
| `docs/impl/ir/stabilize-issue-187-posix-suite-residual-flakes.md` | Record actual changes, attempts, bounded reproduction results, and final supported disposition | Final serialized closeout |

## Implementation Sequence

1. **Resources test seam:** add a minimal test-only seam interface to the instance path so the regression compiles without a missing-symbol error; keep the seam instance-scoped and private.
2. **Resources test (red):** write the phase-controlled regression against that seam, asserting the observed event is after successful pre-check and before enqueue; retain abort rejection, original reason, and subsequent manager usability.
3. **Resources red run:** first run the phase-controlled test against the current runtime and record its real outcome. If it passes, temporarily bypass only the existing post-registration abort recheck in a local, uncommitted mutation; run the focused test and require a bounded behavioral failure at the abort-result assertion. Restore the production recheck immediately and rerun green. Keep mutation evidence separate from any claim about reproducing the historical Ubuntu stall; a missing-symbol compile failure is not red evidence.
4. **Resources implementation (green):** wire the one-shot hook at the actual successful pre-check / before-enqueue boundary with bounded phase, release, request, result, and worker-finished waits. Resume through real enqueue/listener/recheck logic; only join after `is_finished()`; make no second-lock contention claim.
5. **Resources green run:** rerun the focused test and confirm phase observation, bounded completion, original abort reason, and manager reuse.
6. **Hook diagnostic test (red):** add focused assertions for distinct `Err(io::Error)` versus `Ok(Output)` evidence shapes while retaining the legacy-conflict warning assertion.
7. **Hook diagnostics implementation:** report both source and copied executable paths plus bounded metadata/timestamps around the capability invocation; include `ErrorKind`, raw OS code, and message for `Err(io::Error)` without fabricated child/status/stdout/stderr, and status plus bounded stdout/stderr for `Ok(Output)`. Keep paths whole within their bound; longer paths explicitly mark truncation and retain leading/trailing text, original byte length, and a stable non-cryptographic identifier.
8. **Hook diagnostic green run:** run the focused hook-config module and confirm its warning behavior and each diagnostic branch contract. Attempt the reported parallel condition with bounded reproduction; if `ETXTBSY` does not recur, record attempts as uncertainty and obtain independently reviewed supported disposition without retry workaround.
9. **Windows process test (red):** add assertions using a test-local `SYNCHRONIZE | PROCESS_TERMINATE` process handle opened before abort; use the same handle to prove the child is initially live and retain the abort-reason/termination contract. The red result must be a bounded behavior/assertion failure, not a missing symbol or compile error.
10. **Windows watchdog and cleanup implementation:** bound pending result to 20 seconds with operation, phase, elapsed time, allowed budget, and PID diagnostics. On expiry never await/join pending again; abort it and use separate maximum 5-second same-handle `TerminateProcess` fixture cleanup. Use RAII for all handle paths.
11. **Windows green run:** on Windows run the focused test and confirm the normal termination wait on the same handle is also bounded to 20 seconds; timeout fixture cleanup remains a separate maximum 5 seconds, and all assertions and handle cleanup pass.
12. **POSIX exec-deny fixture correction (scope refinement from canonical CI run `40311338267`):** `security_exec_deny_pin_blast_radius` intentionally uses an in-workspace symlink so the resolved executable basename differs from the declared basename. Keep the deny contract and assertion unchanged; set only this fixture to `WithinWorkspace` because its target is inside the fixture root. Record the exact CI baseline failure and WSL focused red/green reproduction. Do not change production symlink policy or deny behavior.
13. **Serialized integration:** verify current route test remains on the #157 contract and make no route code edit. Review all slices for scope, bounded waits, preserved assertions, and evidence wording; update discovery with exact observed attempts and explicit unresolved dispositions.
14. **Serialized platform verification:** on provisioned Ubuntu POSIX CI run `mise install`, `mise run setup`, `mise run check`; on Windows run `mise install` then `mise run check` three consecutive times. Record host, start/end, command, and each result. Focused passes do not substitute for these gates.
15. **Closeout:** create implementation record with impl-doc, record commit evidence, and re-project graph before exit audit. Do not self-approve any downstream gate.

## Dependency Graph

| Step | Depends on | Blocks |
| --- | --- | --- |
| Resources test and implementation | Plan approval; resources test-spec; then resources task | Cross-slice integration review |
| Hook diagnostic test and implementation | Plan approval; hook test-spec; then hook task | Cross-slice integration review |
| Windows process test and POSIX fixture correction | Plan approval; Windows oracle test-spec and existing resolved-basename deny test-spec; then process-command task | Cross-slice integration review |
| Cross-slice review and evidence/disposition update | All three implementation slices | Ubuntu and Windows full verification |
| Ubuntu canonical verification | Cross-slice review; provisioned Ubuntu POSIX CI | Completion |
| Windows three-run canonical verification | Cross-slice review; Windows host | Completion |
| Implementation record and graph exit audit | Both platform verification gates | Completion |

## Dependencies and Risks

The resources, hook-diagnostics, and Windows-process slices may proceed independently after plan approval; their focused contract checks and review are prerequisites to serialized integration. POSIX full verification and the three-run Windows full verification are final serialized gates. Ubuntu CI availability is external; Windows host access is also an external prerequisite. Causes of the historical resource stall, hook `ETXTBSY`, and Windows liveness observation remain unconfirmed; absence of recurrence is not resolution.

| Risk | Likelihood / impact | Mitigation |
| --- | --- | --- |
| Historical failure does not recur, leaving cause unknown | Medium / medium | Record bounded attempts and evidence; keep a supported disposition explicit and independently reviewed; do not claim root-cause resolution |
| New synchronization hook tests a synthetic path instead of real transition | Low / high | Place it immediately after actual successful pre-check and before enqueue; resume through real enqueue/listener/recheck; review exact ownership and scope |
| Watchdog or fixture cleanup becomes unbounded or masks runtime result | Low / high | Separate 20-second runtime result watchdog from 5-second fixture cleanup; never reawait pending after abort; report cleanup separately |
| Platform full-check hosts unavailable | Medium / high | Treat required Ubuntu and Windows gates as blockers to implementation completion; do not substitute local focused runs |
| Data loss / breaking behavior / dependency availability | Low / medium | No data migration, production behavior change absent causal evidence, or new dependency is planned; preserve APIs and assertions |

## Task Breakdown

The graph planning-flow requires a plan approval yield before it creates test-specs or tasks. After approval, create one test-spec per verifiable behavior, add those documents to front matter `relations.verified-by`, then create tasks before exiting planning. Do not waive this sequence or mark the draft approved with a fabricated test-spec link. Task ownership should be: resources slice owns only `resources.rs`; hook slice owns `hook_config.rs` and `hook_config` integration test; Windows slice owns `proc_command.rs`; serialized integration/verification owns discovery and implementation records. The three code slices have no file overlap and may be delegated independently if the user authorizes delegation; cross-slice review and canonical platform verification remain coordinator-owned and serialized.

## Verification

- [ ] Resources: `mise exec -- cargo test -p cheaper-tools --lib abort_landing_before_listener_registration_still_rejects` passes with explicit phase, release, request, result, and worker-finished budgets; abort reason and subsequent manager usability remain asserted.
- [ ] Resources detector: when the current-runtime baseline passes, temporarily bypass only the post-registration abort recheck, rerun the focused test and observe a bounded abort-result failure, restore the check, then rerun green; record the controlled mutation separately from historical reproduction evidence and verify the production source has no remaining mutation.
- [ ] Hook diagnostics: `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_config::` passes; `Err(io::Error)` output has `ErrorKind`, raw OS code, message, source/copy paths whole within their diagnostic bound (explicit truncation identity beyond it), and bounded metadata/timestamps with no child/status/stdout/stderr fields; `Ok(Output)` reports status and bounded stdout/stderr; legacy warning assertion remains.
- [ ] Windows process: `mise exec -- cargo test -p cheaper-tools --test proc_command abort_kills_and_preserves_reason` passes on Windows; pending watchdog is 20 seconds and reports operation/phase/elapsed/allowed budget/PID; normal termination wait is at most 20 seconds on the same handle; fixture cleanup is separate and at most 5 seconds with API result/error code; RAII closes handle.
- [ ] Route disposition: inspect `crates/cheaper-cli/tests/integration/route_run.rs` and its history to confirm `ToolExecutionFailure` plus `routed command did not complete`; no new #187 route code is introduced.
- [ ] Review: independently review the implementation and supported dispositions; no unresolved confirmed P0/P1 remains and no retry, skip, weakened assertion, or timeout-only mitigation is present.
- [x] Ubuntu POSIX gate: canonical workflow `40314463285` passed `mise install`, `mise run setup`, and `mise run check` on code SHA `660cf138541bb77c0318885ad9b50ffb2dbe7dde`.
- [x] Windows gate: three consecutive `mise run check` runs followed `mise install`/`mise run setup` on SHA `16f922a994afe8c41205bdf40eb02759f0f4d670`; run 1/session `74340` completed all stages without an error marker but its numeric exit code was not retained, while sessions `56212` and `87696` retained exit code 0. The only later code change is a `#[cfg(unix)]` fixture correction.
- [ ] Closeout: `docs/impl/ir/run-issue-187-cross-platform-verification-and-closeout.md` records actual changes, attempts, supported dispositions, commits, and platform evidence. Focused implementation-record audit is clean and doc-status reports zero blocking findings; graph router currently selects a briefing repair edge and its focused gate evidence is under separate read-only diagnosis.

## Open Questions / Gaps

The exact historical owner/cause for the resources stall, `ETXTBSY` writer or operation, and the Windows dead-to-alive `tasklist` observation remains unknown after bounded diagnostics. The required Ubuntu and Windows full-check gates have completed; see the closeout IR for run evidence. No #162 environment work is implied by this plan.

## Self-Review

- [x] Every spec requirement maps to a task or explicit serialized verification gate.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Review and Approval Record

- Reviewer A and Reviewer B completed independent plan reviews.
- Validator confirmed the missing normal-path 20-second Windows handle wait and
  missing source/copy executable path; both were added. Validator rejected a
  draft `verified-by` blocker because planning-flow yields for plan approval
  before creating test-specs.
- Final Judge: `ACCEPT WITH NON-BLOCKING NOTES`; no confirmed P0/P1. The
  remaining implementation note is to make the resources red check exercise
  the real test seam and behavioral checkpoint, never treat a missing symbol
  compile failure as red evidence.
- Implementation Judge: `ACCEPT WITH NON-BLOCKING NOTES`; no confirmed P0/P1.
  The bounded-path contract clarification and implementation slices are
  accepted. Canonical Ubuntu POSIX and three-run Windows checks remain required
  before #187 closeout; the historical `ETXTBSY` cause remains unknown.
- POSIX scope-refinement review: independent A/B, Validator, and Judge accepted
  the fixture-only `WithinWorkspace` correction with no confirmed P0/P1/P2
  blocker; canonical Ubuntu workflow `40314463285` then passed on SHA
  `660cf138541bb77c0318885ad9b50ffb2dbe7dde`.
- Final Windows verification: three sequential checks completed on SHA
  `16f922a994afe8c41205bdf40eb02759f0f4d670`. Runs 2 and 3 have explicit exit
  code 0; run 1 completed the full pipeline with no error marker but its numeric
  exit code was not retained. The intervening Unix-only fixture change does
  not compile on Windows.
- Approval: user-delegated approval authority was applied by the parent
  coordinator on 2026-10-26 based on the independent Judge decision. This
  approves the plan for test-spec/task generation and implementation handoff;
  it does not establish that any code has been implemented or verified.

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation with per-slice review checkpoints.
- Independent work streams: resources, hook diagnostics, and Windows process test slices; serialized integration review and cross-platform verification remain coordinator-owned.
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

Assumptions: the named current test ownership is retained; no dependency or public runtime API is needed. Missing prerequisites: plan approval, followed by behavior test specs, then task documents; execution of #187 code is not approved by this plan draft. Reviewer focus: resource hook placement and instance scope; truthful distinction between `Err(io::Error)` and `Ok(Output)`; same-handle Windows identity, 20-second watchdog/5-second cleanup separation and RAII; unresolved causes and non-recurrence wording; exact required full-check gates; exclusion of #188 and #162.
