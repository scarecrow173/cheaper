---
id: "PLAN-034M8k2N0x4RKepPhz3J4M"
type: "plan"
status: "approved"
title: "Implement issue 184 usage route and error analysis"
created: "2026-10-26"
updated: "2026-10-26"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/184"
  changes:
    added:
      - type: "file"
        path: "docs/plans/implement-usage-route-error-analysis-issue-184.md"
      - type: "file"
        path: "docs/test-specs/usage-route-error-analysis-contract-issue-184.md"
      - type: "file"
        path: "docs/tasks/pin-usage-route-error-contract-tests.md"
      - type: "file"
        path: "docs/tasks/implement-usage-route-window-and-by-route.md"
      - type: "file"
        path: "docs/tasks/implement-usage-by-error-aggregation.md"
      - type: "file"
        path: "docs/tasks/wire-usage-group-by-flags-text-and-guide.md"
      - type: "file"
        path: "docs/tasks/measure-usage-jsonl-peak-memory.md"
      - type: "file"
        path: "docs/tasks/verify-usage-analysis-chain-and-audit.md"
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/args.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/cli.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/text_format.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/args.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_log.rs"
      - type: "file"
        path: "docs/guides/usage.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/exp/usage-jsonl-peak-memory-measurement.jsonl"
  implements:
    - "docs/specs/usage-analysis-scope-issue-184.md"
  implemented-by:
    - "docs/tasks/pin-usage-route-error-contract-tests.md"
    - "docs/tasks/implement-usage-route-window-and-by-route.md"
    - "docs/tasks/implement-usage-by-error-aggregation.md"
    - "docs/tasks/wire-usage-group-by-flags-text-and-guide.md"
    - "docs/tasks/measure-usage-jsonl-peak-memory.md"
    - "docs/tasks/verify-usage-analysis-chain-and-audit.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/usage-analysis-scope-issue-184.md"
    - "docs/designs/usage-grouped-route-error-analysis-issue-184.md"
  derived-by:
    - "docs/test-specs/usage-route-error-analysis-contract-issue-184.md"
    - "docs/tasks/pin-usage-route-error-contract-tests.md"
    - "docs/tasks/implement-usage-route-window-and-by-route.md"
    - "docs/tasks/implement-usage-by-error-aggregation.md"
    - "docs/tasks/wire-usage-group-by-flags-text-and-guide.md"
    - "docs/tasks/measure-usage-jsonl-peak-memory.md"
    - "docs/tasks/verify-usage-analysis-chain-and-audit.md"
  verifies: []
  verified-by:
    - "docs/test-specs/usage-route-error-analysis-contract-issue-184.md"
  references:
    - "docs/discovery/usage-analysis-audit-issue-184.md"
    - "docs/adr/grouped-route-and-error-analysis-in-cheaper-usage.md"
    - "docs/guides/usage.md"
    - "docs/impl/exp/README.md"
  defers: []
  deferred-by: []
---

# Implement issue 184 usage route and error analysis Implementation Plan

## Goal

Implement issue #184's route-window filtering, per-route and per-error usage
reports, CLI flags, and text/JSON output while preserving existing run filters,
usage record schema, privacy allowlists, and no-flag report behavior.

## Scope

One implementation stream: parser flags, report aggregation, text formatting,
integration coverage, and user documentation share one CLI contract and one
release intent. Keep writer/schema changes, new dependencies, and streaming
aggregation out of scope.

## Upstream Documents

- `docs/specs/usage-analysis-scope-issue-184.md` (approved)
- `docs/adr/grouped-route-and-error-analysis-in-cheaper-usage.md` (accepted)

## Design Inputs

- `docs/designs/usage-grouped-route-error-analysis-issue-184.md` (approved)

## File Map

- `crates/cheaper-cli/src/args.rs` — `--route`/`--group-by` parse, usage-only
  option guard, exact error contracts.
- `crates/cheaper-cli/src/usage_report.rs` — route window and route filter,
  owner join/orphan logic, unkeyed counting, per-route and by-error aggregates,
  existing run aggregation and report callers.
- `crates/cheaper-cli/src/cli.rs` — usage help text and parsed option handoff.
- `crates/cheaper-cli/src/text_format.rs` — route section and flag-gated
  grouped sections with JSON parity.
- `crates/cheaper-cli/tests/integration/args.rs` — CLI option and exact guard
  message tests.
- `crates/cheaper-cli/tests/integration/usage_report.rs` — report JSONL
  fixtures, contract edges, privacy, compatibility, and report formatting.
- `crates/cheaper-cli/tests/integration/usage_log.rs` — existing report
  builder callers and output contract updates.
- `crates/cheaper-cli/tests/integration/main.rs` — module declarations only
  if new integration test modules are needed; prefer existing modules.
- `docs/guides/usage.md` — flags, route/error sections, windows, denominator
  meanings, and measured versus estimated values.
- `docs/impl/exp/usage-jsonl-peak-memory-measurement.jsonl` — bounded
  representative large JSONL fixture dimensions and peak-memory measurement
  evidence as an `experiment_event.v1` note (`benchmarks/results/` is
  reserved for `cheaper-bench` harness `schemaVersion: 2` run artifacts and
  does not fit ad-hoc measurement notes — convention confirmed via
  `benchmarks/results/README.md` and `docs/impl/exp/README.md`).

## Implementation Sequence

1. Add failing focused tests for route window/scoping/join, error grouping,
   CLI guards, privacy, legacy behavior, nonfinite values, and text parity;
   include the omitted edge cases listed in the Verification Matrix below.
2. Implement route window filtering and grouped route aggregation in
   `usage_report.rs`, including string-route owner requirements, window-wide
   orphan/unkeyed diagnostics, and the documented `format_number` key path.
3. Implement error-code aggregation and canonical execution value maps while
   preserving empty error codes and existing failure denominator semantics.
4. Wire CLI flags/help and exact validation messages; update all report
   builder callers found in `usage_report.rs`, `usage_report.rs` tests, and
   `usage_log.rs` tests.
5. Add text sections and user guide documentation; verify flag-gated JSON/text
   parity and no-flag backward compatibility.
6. Generate a bounded, representative large JSONL input, measure peak memory
   of current full-file parsing with a documented reproducible command, and
   record fixture size, record mix, environment, and result in the
   `docs/impl/exp` measurement note before deciding whether streaming
   remains deferred.
7. Run focused tests, then canonical `mise run check`; review the final diff
   for scope, privacy, doc-index, and contract consistency.

## Dependencies and Risks

- The plan is draft and requires designated review before implementation.
- No dependency changes are planned. Existing `usage.jsonl` writer schema and
  ordering remain untouched.
- Route totals change under `--since` and `--route`; tests and the guide must
  pin this observable behavior.
- Fake records can reuse a `logRef`; first window record wins and must not
  double-attribute a refetch.
- Error detail and command/path fields are sensitive; aggregation keys and
  output must remain restricted to the design allowlist.
- Large JSONL memory results may show full-file parsing is not viable; preserve
  the recorded evidence and route any streaming redesign back through design.
- External verification dependency risk: `mise`-managed tools must be
  available; no live provider or network access is required for this feature.

## Task Breakdown

- T1: Pin CLI args and report contract tests (args, report, writer callers).
  → docs/tasks/pin-usage-route-error-contract-tests.md
- T2: Implement route window, route scope, join, orphan/unkeyed, and per-route
  aggregates.
  → docs/tasks/implement-usage-route-window-and-by-route.md
- T3: Implement by-error aggregates and canonical execution value keys.
  → docs/tasks/implement-usage-by-error-aggregation.md
- T4: Implement CLI handoff, help, and text formatting; update guide and
  compatibility snapshots.
  → docs/tasks/wire-usage-group-by-flags-text-and-guide.md
- T5: Measure representative large JSONL peak memory and record evidence;
  decide whether streaming remains deferred.
  → docs/tasks/measure-usage-jsonl-peak-memory.md
- T6: Run focused and canonical verification, doc-status checks, and final
  scope/privacy audit.
  → docs/tasks/verify-usage-analysis-chain-and-audit.md
All implementation tasks depend on T1's regression expectations, but the
chain is strictly serialized: T1 → T2 → T3 → T4 → T5 → T6. T2 and T3 both
edit the same `build_usage_report` tail and the same
`tests/integration/usage_report.rs` file, so T3 depends on T2 (parallel
execution would guarantee a merge conflict). T4 depends on T2/T3. T5's
verification (`cargo test … usage_report::`) cannot go green until T4 lands
the last pin tests, so T5 depends on T4. T6 depends on all implementation
and evidence tasks as the chain gate. Landing model: all six tasks land on
`feature/usage-analysis-issue-184` before merge — `mise run check` is
expected-red between T1 and T4 by TDD pin-test design (new pins fail while
their implementation is still pending), and the canonical check is T6's
gate, not a per-task gate. The behavior test spec is
docs/test-specs/usage-route-error-analysis-contract-issue-184.md, linked from
this plan's `relations.verified-by` and from each task's `verified-by`.

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_report::` exits 0.
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration args::` exits 0.
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_log::` exits 0.
- [ ] `mise run check` exits 0.
- [ ] A bounded representative large JSONL measurement records input dimensions and peak memory using a reproducible command; evidence is checked in as `docs/impl/exp/usage-jsonl-peak-memory-measurement.jsonl` (`experiment_event.v1` note convention, per `docs/impl/exp/README.md`).
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` reports no new blocking findings for the issue #184 artifacts.

## Open Questions / Gaps

- The representative dataset size and acceptable peak-memory threshold are
  not specified by the issue. Establish a bounded input anchored to the
  observed scale of a real `usage.jsonl` (naming the actual file used, or
  the recorded fallback assumption when none is available) and report
  measurement without inventing a pass threshold; if evidence indicates
  unacceptable memory use, return to design before deferring streaming.
- The measurement artifact destination is fixed: an `experiment_event.v1`
  note at `docs/impl/exp/usage-jsonl-peak-memory-measurement.jsonl`
  (`benchmarks/results/` is the `cheaper-bench` harness's append-only
  `schemaVersion: 2` store and does not accept ad-hoc notes).

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation with review
  checkpoints after tests, implementation, and memory evidence.
- Work streams are serialized, not parallel: route (T2) and error (T3)
  aggregation share the `build_usage_report` tail and one test file, so T3
  follows T2; formatting/docs (T4) follows both; measurement (T5) is a
  bounded slice sequenced after T4; verification (T6) gates the chain.
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- Review focus: confirm the two window-wide diagnostics, unkeyed route owner
  behavior, canonical number formatter path, all CLI/report callers, privacy
  allowlist, backward compatibility, and the bounded memory measurement
  evidence. The plan remains draft; review and approval are required before
  creating test specs or tasks in planning-flow.
